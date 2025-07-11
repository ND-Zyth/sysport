use crate::metrics::{Metrics, DiskMetrics};
use crate::alert::{AlertManager, AlertLevel, AlertRule};
use crate::export::{export_log, export_metrics, ExportFormat};
use crate::theme::CustomTheme;
use crate::packet_stats::{PacketStats, decode_protocol};
use crate::remote::ExampleServers;
use crate::plugins::PluginSystem;

use eframe::{egui, epi};
use sysinfo::{System, SystemExt, DiskExt, NetworkExt, NetworksExt, ProcessorExt};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::net::IpAddr;
use pcap::Capture;
use maxminddb::geoip2;
use std::fs;
use regex::Regex;

// In plotting, use:
// let cpu_points: Vec<Value> = history.iter().enumerate().map(|(i, m)| Value::new(i as f64, m.cpu_total as f64)).collect();
// Line::new(Values::from_values(cpu_points))
// ...
// update signature: fn update(&mut self, ctx: &egui::Context, _frame: &eframe::epi::Frame)
// For sysinfo, use sys.cpus() as a slice, e.g. sys.cpus().len() or sys.cpus()[i].cpu_usage()

fn draw_gauge_arc(painter: &egui::Painter, center: egui::Pos2, radius: f32, start_angle: f32, sweep: f32, color: egui::Color32, thickness: f32) {
    let n = 64;
    let mut points = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let angle = start_angle + sweep * t;
        points.push(center + egui::Vec2::angled(angle) * radius);
    }
    painter.add(egui::Shape::line(points, egui::Stroke::new(thickness, color)));
}

pub struct RawPacketInfo {
    pub timestamp: std::time::Instant,
    pub data: Vec<u8>,
    pub src: Option<IpAddr>,
    pub dst: Option<IpAddr>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub protocol: String,
    pub country: Option<String>,
    pub app: Option<String>,
}

pub struct SysPortApp {
    pub metrics: Arc<Mutex<Metrics>>,
    pub history: Arc<Mutex<Vec<Metrics>>>,
    pub update_interval: f32, // seconds
    pub paused: bool,
    pub last_update: Instant,
    pub system: System,
    pub alert_manager: AlertManager,
    pub export_status: Option<String>,
    pub selected_interface: Option<String>,
    pub protocol_tcp: bool,
    pub protocol_udp: bool,
    pub protocol_icmp: bool,
    pub protocol_arp: bool,
    pub packet_log: Arc<Mutex<VecDeque<Vec<u8>>>>,
    pub max_packet_log: usize,
    pub proxy_port: u16,
    pub reverse_proxy_port: u16,
    pub reverse_proxy_target: String,
    pub dns_port: u16,
    pub proxy_running: bool,
    pub reverse_proxy_running: bool,
    pub dns_running: bool,
    pub packet_filter: String,
    pub packet_search: String,
    pub custom_theme: CustomTheme,
    pub use_custom_theme: bool,
    pub stats: PacketStats,
    pub plugin_system: PluginSystem,
    pub raw_packets: Arc<Mutex<VecDeque<RawPacketInfo>>>,
    pub geoip_reader: Option<maxminddb::Reader<Vec<u8>>>,
}

impl Default for SysPortApp {
    fn default() -> Self {
        let metrics = Arc::new(Mutex::new(Metrics::default()));
        let history = Arc::new(Mutex::new(Vec::with_capacity(300)));
        let update_interval = 1.0;
        let paused = false;
        let last_update = Instant::now();
        let mut system = System::new_all();
        system.refresh_all();
        let alert_manager = AlertManager::new();
        // Remove memory usage warnings
        let alert_manager = {
            let mut am = alert_manager;
            am.rules.retain(|rule| !matches!(rule, AlertRule::MemUsage { .. }));
            am
        };
        let custom_theme = CustomTheme::default();
        let export_status = None;
        let selected_interface = None;
        let protocol_tcp = true;
        let protocol_udp = true;
        let protocol_icmp = true;
        let protocol_arp = true;
        let packet_log = Arc::new(Mutex::new(VecDeque::with_capacity(1000)));
        let max_packet_log = 1000;
        let proxy_port = 8888;
        let reverse_proxy_port = 8889;
        let reverse_proxy_target = "127.0.0.1:80".to_string();
        let dns_port = 5353;
        let proxy_running = false;
        let reverse_proxy_running = false;
        let dns_running = false;
        let packet_filter = String::new();
        let packet_search = String::new();
        let use_custom_theme = true;
        let stats = PacketStats::default();
        let plugin_system = PluginSystem::new();
        let raw_packets = Arc::new(Mutex::new(VecDeque::with_capacity(10000)));
        let mut geoip_reader = None;
        if let Ok(data) = fs::read("GeoLite2-Country.mmdb") {
            if let Ok(reader) = maxminddb::Reader::from_source(data) {
                geoip_reader = Some(reader);
            }
        }

        // Spawn background thread for polling system metrics
        let metrics_clone = metrics.clone();
        let history_clone = history.clone();
        thread::spawn(move || {
            let mut sys = System::new_all();
            let mut last_rx = 0;
            let mut last_tx = 0;
            loop {
                thread::sleep(Duration::from_millis(200));
                sys.refresh_cpu();
                sys.refresh_memory();
                sys.refresh_disks_list();
                sys.refresh_disks();
                sys.refresh_networks();
                let cpus = sys.processors();
                let cpu_usages: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();
                let cpu_total = cpu_usages.iter().sum::<f32>() / cpu_usages.len().max(1) as f32;
                let mem_total = sys.total_memory();
                let mem_used = sys.used_memory();
                let disks = sys.disks().iter().map(|d| DiskMetrics {
                    name: d.name().to_string_lossy().to_string(),
                    total: d.total_space(),
                    available: d.available_space(),
                }).collect();
                let net = sys.networks();
                let rx = net.iter().map(|(_, data)| data.received()).sum();
                let tx = net.iter().map(|(_, data)| data.transmitted()).sum();
                let net_rx = if last_rx == 0 { 0 } else { rx - last_rx };
                let net_tx = if last_tx == 0 { 0 } else { tx - last_tx };
                last_rx = rx;
                last_tx = tx;
                let m = Metrics {
                    timestamp: Instant::now(),
                    cpu_usage: cpu_usages,
                    cpu_total,
                    mem_total,
                    mem_used,
                    disks,
                    net_rx,
                    net_tx,
                    selected_interface: None,
                    interfaces: vec![],
                    net_per_interface: vec![],
                };
                // Store latest metrics
                if let Ok(mut lock) = metrics_clone.lock() {
                    *lock = m.clone();
                }
                // Store history for plotting
                if let Ok(mut hist) = history_clone.lock() {
                    hist.push(m);
                    if hist.len() > 300 { hist.remove(0); }
                }
            }
        });

        // Spawn background thread for global packet capture (scaffold)
        let raw_packets_clone = raw_packets.clone();
        let geoip_reader_for_thread = geoip_reader.take();
        std::thread::spawn(move || {
            let device = pcap::Device::lookup().unwrap().unwrap();
            if let Ok(mut cap) = Capture::from_device(device.name.as_str()).unwrap().promisc(true).open() {
                while let Ok(packet) = cap.next_packet() {
                    let data = packet.data.to_vec();
                    // Parse IP/port/protocol (IPv4 only for now)
                    let (src, dst, src_port, dst_port, proto) = if data.len() > 34 && (data[12] == 0x08 && data[13] == 0x00) {
                        // IPv4
                        let src = IpAddr::from([data[26], data[27], data[28], data[29]]);
                        let dst = IpAddr::from([data[30], data[31], data[32], data[33]]);
                        let proto = data[23];
                        let (src_port, dst_port, proto_str) = match proto {
                            6 => (Some(u16::from_be_bytes([data[34], data[35]])), Some(u16::from_be_bytes([data[36], data[37]])), "TCP"),
                            17 => (Some(u16::from_be_bytes([data[34], data[35]])), Some(u16::from_be_bytes([data[36], data[37]])), "UDP"),
                            1 => (None, None, "ICMP"),
                            _ => (None, None, "IPv4"),
                        };
                        (Some(src), Some(dst), src_port, dst_port, proto_str.to_string())
                    } else {
                        (None, None, None, None, "Other".to_string())
                    };
                    let country = src.and_then(|ip| geoip_reader_for_thread.as_ref().and_then(|g| {
                        if let Ok(geo) = g.lookup::<geoip2::Country>(ip) {
                            geo.country.and_then(|c| c.iso_code).map(|s| s.to_string())
                        } else { None }
                    }));
                    let pkt = RawPacketInfo {
                        timestamp: std::time::Instant::now(),
                        data,
                        src,
                        dst,
                        src_port,
                        dst_port,
                        protocol: proto,
                        country,
                        app: None, // TODO: per-app mapping
                    };
                    let mut lock = raw_packets_clone.lock().unwrap();
                    if lock.len() > 10000 { lock.pop_front(); }
                    lock.push_back(pkt);
                }
            }
        });

        Self {
            metrics,
            history,
            update_interval,
            paused,
            last_update,
            system,
            alert_manager,
            export_status,
            selected_interface,
            protocol_tcp,
            protocol_udp,
            protocol_icmp,
            protocol_arp,
            packet_log,
            max_packet_log,
            proxy_port,
            reverse_proxy_port,
            reverse_proxy_target,
            dns_port,
            proxy_running,
            reverse_proxy_running,
            dns_running,
            packet_filter,
            packet_search,
            custom_theme,
            use_custom_theme,
            stats,
            plugin_system,
            raw_packets,
            geoip_reader,
        }
    }
}

impl SysPortApp {
    pub fn load_geoip(&mut self, path: &str) {
        if let Ok(data) = fs::read(path) {
            if let Ok(reader) = maxminddb::Reader::from_source(data) {
                self.geoip_reader = Some(reader);
            }
        }
    }
    pub fn lookup_country(&self, ip: &IpAddr) -> Option<String> {
        if let Some(reader) = &self.geoip_reader {
            if let Ok(geo) = reader.lookup::<geoip2::Country>(*ip) {
                if let Some(country) = geo.country.and_then(|c| c.iso_code) {
                    return Some(country.to_string());
                }
            }
        }
        None
    }
}

impl epi::App for SysPortApp {
    fn name(&self) -> &str {
        "SysPort - System Monitor"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &eframe::epi::Frame) {
        if self.use_custom_theme {
            self.custom_theme.apply(ctx);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Top bar: controls
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("SysPort").heading());
                    if ui.button(if self.paused { "Resume" } else { "Pause" }).clicked() {
                        self.paused = !self.paused;
                    }
                    if ui.button("Refresh").clicked() {
                        self.system.refresh_all();
                    }
                    ui.label("Update interval (s):");
                    ui.add(egui::Slider::new(&mut self.update_interval, 0.2..=5.0).text("s"));
                    // Theme switcher
                    // REMOVE ComboBox for theme switching
                });
                if let Some(msg) = &self.export_status {
                    ui.label(msg);
                }
                // Network interface selection
                let interfaces = self.metrics.lock().unwrap().interfaces.clone();
                egui::ComboBox::from_label("Network Interface")
                    .selected_text(self.selected_interface.clone().unwrap_or_else(|| "auto".to_string()))
                    .show_ui(ui, |ui| {
                        for iface in &interfaces {
                            ui.selectable_value(&mut self.selected_interface, Some(iface.clone()), iface);
                        }
                    });
                // Protocol toggles
                ui.checkbox(&mut self.protocol_tcp, "TCP");
                ui.checkbox(&mut self.protocol_udp, "UDP");
                ui.checkbox(&mut self.protocol_icmp, "ICMP");
                ui.checkbox(&mut self.protocol_arp, "ARP");
                ui.separator();
                ui.collapsing("Network Servers", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Transparent Proxy Port:");
                        ui.add(egui::DragValue::new(&mut self.proxy_port).clamp_range(1..=65535));
                        if ui.button(if self.proxy_running { "Stop Proxy" } else { "Start Proxy" }).clicked() {
                            if !self.proxy_running {
                                let log = self.packet_log.clone();
                                let port = self.proxy_port;
                                self.proxy_running = true;
                                std::thread::spawn(move || {
                                    let log = log.clone();
                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    let cb = Arc::new(Mutex::new(move |pkt: &[u8]| {
                                        let mut log = log.lock().unwrap();
                                        if log.len() > 1000 { log.pop_front(); }
                                        log.push_back(pkt.to_vec());
                                    }));
                                    rt.block_on(ExampleServers::start_transparent_proxy(port, cb));
                                });
                            } else {
                                // TODO: Stop proxy (not implemented)
                                self.proxy_running = false;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Reverse Proxy Port:");
                        ui.add(egui::DragValue::new(&mut self.reverse_proxy_port).clamp_range(1..=65535));
                        ui.label("Target:");
                        ui.text_edit_singleline(&mut self.reverse_proxy_target);
                        if ui.button(if self.reverse_proxy_running { "Stop Reverse Proxy" } else { "Start Reverse Proxy" }).clicked() {
                            if !self.reverse_proxy_running {
                                let log = self.packet_log.clone();
                                let port = self.reverse_proxy_port;
                                let target = self.reverse_proxy_target.clone();
                                self.reverse_proxy_running = true;
                                std::thread::spawn(move || {
                                    let log = log.clone();
                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    let cb = Arc::new(Mutex::new(move |pkt: &[u8]| {
                                        let mut log = log.lock().unwrap();
                                        if log.len() > 1000 { log.pop_front(); }
                                        log.push_back(pkt.to_vec());
                                    }));
                                    rt.block_on(ExampleServers::start_reverse_proxy(port, &target, cb));
                                });
                            } else {
                                // TODO: Stop reverse proxy (not implemented)
                                self.reverse_proxy_running = false;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("DNS Server Port:");
                        ui.add(egui::DragValue::new(&mut self.dns_port).clamp_range(1..=65535));
                        if ui.button(if self.dns_running { "Stop DNS" } else { "Start DNS" }).clicked() {
                            if !self.dns_running {
                                let log = self.packet_log.clone();
                                let port = self.dns_port;
                                self.dns_running = true;
                                std::thread::spawn(move || {
                                    let log = log.clone();
                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    let cb = Arc::new(Mutex::new(move |pkt: &[u8]| {
                                        let mut log = log.lock().unwrap();
                                        if log.len() > 1000 { log.pop_front(); }
                                        log.push_back(pkt.to_vec());
                                    }));
                                    rt.block_on(ExampleServers::start_dns_server(port, cb));
                                });
                            } else {
                                // TODO: Stop DNS (not implemented)
                                self.dns_running = false;
                            }
                        }
                    });
                });
                // Theme Editor section
                ui.separator();
                ui.collapsing("Theme Editor", |ui| {
                    ui.checkbox(&mut self.use_custom_theme, "Use Custom Theme");
                    if self.use_custom_theme {
                        ui.label("Edit all theme parameters below. Changes apply live.");
                        macro_rules! color_picker {
                            ($ui:expr, $field:expr, $label:expr) => {
                                $ui.horizontal(|ui| {
                                    ui.label($label);
                                    let mut color: egui::Color32 = (*$field).into();
                                    if ui.color_edit_button_srgba(&mut color).changed() {
                                        *$field = color.into();
                                    }
                                });
                            };
                        }
                        color_picker!(ui, &mut self.custom_theme.background, "Background");
                        color_picker!(ui, &mut self.custom_theme.foreground, "Foreground");
                        color_picker!(ui, &mut self.custom_theme.accent, "Accent");
                        color_picker!(ui, &mut self.custom_theme.warning, "Warning");
                        color_picker!(ui, &mut self.custom_theme.error, "Error");
                        color_picker!(ui, &mut self.custom_theme.info, "Info");
                        color_picker!(ui, &mut self.custom_theme.cpu_gauge, "CPU Gauge");
                        color_picker!(ui, &mut self.custom_theme.mem_gauge, "Memory Gauge");
                        color_picker!(ui, &mut self.custom_theme.net_gauge, "Network Gauge");
                        color_picker!(ui, &mut self.custom_theme.disk_gauge, "Disk Gauge");
                        color_picker!(ui, &mut self.custom_theme.bar_fill, "Bar Fill");
                        color_picker!(ui, &mut self.custom_theme.bar_bg, "Bar Background");
                        color_picker!(ui, &mut self.custom_theme.text_primary, "Text Primary");
                        color_picker!(ui, &mut self.custom_theme.text_secondary, "Text Secondary");
                        color_picker!(ui, &mut self.custom_theme.text_disabled, "Text Disabled");
                        color_picker!(ui, &mut self.custom_theme.border, "Border");
                        color_picker!(ui, &mut self.custom_theme.separator, "Separator");
                        color_picker!(ui, &mut self.custom_theme.button, "Button");
                        color_picker!(ui, &mut self.custom_theme.button_hovered, "Button Hovered");
                        color_picker!(ui, &mut self.custom_theme.button_active, "Button Active");
                        color_picker!(ui, &mut self.custom_theme.input_bg, "Input Background");
                        color_picker!(ui, &mut self.custom_theme.input_fg, "Input Foreground");
                        color_picker!(ui, &mut self.custom_theme.selection, "Selection");
                        color_picker!(ui, &mut self.custom_theme.highlight, "Highlight");
                        color_picker!(ui, &mut self.custom_theme.alert_info, "Alert Info");
                        color_picker!(ui, &mut self.custom_theme.alert_warning, "Alert Warning");
                        color_picker!(ui, &mut self.custom_theme.alert_critical, "Alert Critical");
                        ui.horizontal(|ui| {
                            ui.label("Font Family:");
                            ui.text_edit_singleline(&mut self.custom_theme.font_family);
                            ui.label("Font Size:");
                            ui.add(egui::DragValue::new(&mut self.custom_theme.font_size).clamp_range(8.0..=48.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Border Thickness:");
                            ui.add(egui::DragValue::new(&mut self.custom_theme.border_thickness).clamp_range(0.0..=10.0));
                            ui.label("Widget Rounding:");
                            ui.add(egui::DragValue::new(&mut self.custom_theme.widget_rounding).clamp_range(0.0..=32.0));
                            ui.label("Widget Padding:");
                            ui.add(egui::DragValue::new(&mut self.custom_theme.widget_padding).clamp_range(0.0..=32.0));
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Import Theme from JSON...").clicked() {
                                if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                                    if let Ok(data) = std::fs::read_to_string(&path) {
                                        if let Ok(theme) = serde_json::from_str::<CustomTheme>(&data) {
                                            self.custom_theme = theme;
                                        }
                                    }
                                }
                            }
                            if ui.button("Export Theme to JSON...").clicked() {
                                if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).set_file_name("custom_theme.json").save_file() {
                                    let _ = std::fs::write(&path, serde_json::to_string_pretty(&self.custom_theme).unwrap());
                                }
                            }
                            if ui.button("Reset to Default").clicked() {
                                self.custom_theme = CustomTheme::default();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Preset:");
                            if ui.button("Light").clicked() { self.custom_theme = CustomTheme::light(); }
                            if ui.button("Dark").clicked() { self.custom_theme = CustomTheme::dark(); }
                            if ui.button("Solarized Light").clicked() { self.custom_theme = CustomTheme::solarized_light(); }
                            if ui.button("Solarized Dark").clicked() { self.custom_theme = CustomTheme::solarized_dark(); }
                            if ui.button("VSCode Light").clicked() { self.custom_theme = CustomTheme::vscode_light(); }
                            if ui.button("VSCode Dark").clicked() { self.custom_theme = CustomTheme::vscode_dark(); }
                            if ui.button("Dracula").clicked() { self.custom_theme = CustomTheme::dracula(); }
                            if ui.button("Nord").clicked() { self.custom_theme = CustomTheme::nord(); }
                        });
                    }
                });
                ui.separator();
                ui.collapsing("Plugin System", |ui| {
                    ui.label("Manage and run Rust plugins. Place plugin .so/.dylib/.dll files in the plugins directory.");
                    if ui.button("Reload Plugins").clicked() {
                        self.plugin_system.loaded_plugins.clear();
                        self.plugin_system.load_plugins("../plugins/sample_plugin/target/release"); // Adjust path as needed
                    }
                    ui.label("Loaded plugins:");
                    for plugin in &self.plugin_system.loaded_plugins {
                        ui.label(&plugin.name);
                    }
                });
                // Alert panel
                let metrics = self.metrics.lock().unwrap().clone();
                // Disable all alerts
                self.alert_manager.active_alerts.clear();
                // --- Web Monitor Section (scaffold) ---
                ui.separator();
                ui.heading("Web Monitor (Preview)");
                ui.label("Monitor traffic by country, app, and port. Intercept global traffic. (Coming soon)");
                // TODO: Add real data and controls here
                // --- End Web Monitor Section ---
                ui.separator();
                // Main panel: metrics and charts
                let history = self.history.lock().unwrap().clone();
                ui.heading("System Metrics");
                ui.separator();
                    // CPU
                    ui.collapsing("CPU Usage", |ui| {
                        ui.label(format!("Total: {:.1}%", metrics.cpu_total));
                        for (i, usage) in metrics.cpu_usage.iter().enumerate() {
                            ui.add(egui::ProgressBar::new(*usage as f32 / 100.0).text(format!("Core {}: {:.1}%", i, usage)));
                        }
                        ui.horizontal(|ui| {
                            // REMOVE jagged line chart (Plot)
                            // REMOVE bar chart (Plot)
                            // Only show gauge
                            let gauge = metrics.cpu_total / 100.0;
                            ui.allocate_ui(egui::vec2(80.0, 80.0), |ui| {
                                let (rect, _response) = ui.allocate_exact_size(egui::vec2(80.0, 80.0), egui::Sense::hover());
                                let painter = ui.painter();
                                let center = rect.center();
                                let radius = 35.0;
                                let start_angle = std::f32::consts::PI * 1.25;
                                let end_angle = std::f32::consts::PI * 2.75;
                                let sweep = end_angle - start_angle;
                                let filled = sweep * gauge as f32;
                                painter.circle_stroke(center, radius, egui::Stroke::new(4.0, egui::Color32::DARK_GRAY));
                                draw_gauge_arc(&painter, center, radius, start_angle, filled, egui::Color32::LIGHT_BLUE, 8.0);
                                painter.text(center, egui::Align2::CENTER_CENTER, format!("{:.0}%", metrics.cpu_total), egui::FontId::proportional(16.0), egui::Color32::WHITE);
                            });
                        });
                    });
                    ui.separator();
                    // Memory
                    ui.collapsing("Memory Usage", |ui| {
                        let used_gb = metrics.mem_used as f64 / 1024.0 / 1024.0;
                        let total_gb = metrics.mem_total as f64 / 1024.0 / 1024.0;
                        ui.label(format!("{:.2} GB / {:.2} GB", used_gb, total_gb));
                        ui.add(egui::ProgressBar::new(metrics.mem_used as f32 / metrics.mem_total as f32).text("Used"));
                        ui.horizontal(|ui| {
                            // REMOVE jagged line chart (Plot)
                            // REMOVE bar chart (Plot)
                            // Only show gauge
                            let gauge = metrics.mem_used as f32 / metrics.mem_total as f32;
                            ui.allocate_ui(egui::vec2(80.0, 80.0), |ui| {
                                let (rect, _response) = ui.allocate_exact_size(egui::vec2(80.0, 80.0), egui::Sense::hover());
                                let painter = ui.painter();
                                let center = rect.center();
                                let radius = 35.0;
                                let start_angle = std::f32::consts::PI * 1.25;
                                let end_angle = std::f32::consts::PI * 2.75;
                                let sweep = end_angle - start_angle;
                                let filled = sweep * gauge;
                                painter.circle_stroke(center, radius, egui::Stroke::new(4.0, egui::Color32::DARK_GRAY));
                                draw_gauge_arc(&painter, center, radius, start_angle, filled, egui::Color32::LIGHT_GREEN, 8.0);
                                painter.text(center, egui::Align2::CENTER_CENTER, format!("{:.0}%", gauge * 100.0), egui::FontId::proportional(16.0), egui::Color32::WHITE);
                            });
                        });
                    });
                    ui.separator();
                    // Disks
                    ui.collapsing("Disk Usage", |ui| {
                        for disk in &metrics.disks {
                            let used = disk.total - disk.available;
                            let used_gb = used as f64 / 1024.0 / 1024.0 / 1024.0;
                            let total_gb = disk.total as f64 / 1024.0 / 1024.0 / 1024.0;
                            ui.label(format!("{}: {:.2} GB / {:.2} GB", disk.name, used_gb, total_gb));
                            ui.add(egui::ProgressBar::new(used as f32 / disk.total as f32).text("Used"));
                        }
                    });
                    ui.separator();
                    // Network
                    ui.collapsing("Network Throughput", |ui| {
                        ui.label(format!("Download: {:.2} KB/s", metrics.net_rx as f64 / 1024.0));
                        ui.label(format!("Upload: {:.2} KB/s", metrics.net_tx as f64 / 1024.0));
                        ui.horizontal(|ui| {
                            // REMOVE jagged line chart (Plot)
                            // REMOVE bar chart (Plot)
                            // Only show gauge
                            let max_rx = history.iter().map(|m| m.net_rx).max().unwrap_or(1) as f32 / 1024.0;
                            let gauge = metrics.net_rx as f32 / 1024.0 / max_rx.max(1.0);
                            ui.allocate_ui(egui::vec2(80.0, 80.0), |ui| {
                                let (rect, _response) = ui.allocate_exact_size(egui::vec2(80.0, 80.0), egui::Sense::hover());
                                let painter = ui.painter();
                                let center = rect.center();
                                let radius = 35.0;
                                let start_angle = std::f32::consts::PI * 1.25;
                                let end_angle = std::f32::consts::PI * 2.75;
                                let sweep = end_angle - start_angle;
                                let filled = sweep * gauge;
                                painter.circle_stroke(center, radius, egui::Stroke::new(4.0, egui::Color32::DARK_GRAY));
                                draw_gauge_arc(&painter, center, radius, start_angle, filled, egui::Color32::LIGHT_YELLOW, 8.0);
                                painter.text(center, egui::Align2::CENTER_CENTER, format!("{:.0} KB/s", metrics.net_rx as f64 / 1024.0), egui::FontId::proportional(14.0), egui::Color32::WHITE);
                            });
                        });
                    });
                    ui.separator();
                    // In the Live Packet Log section:
                    ui.collapsing("Live Packet Log", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Filter:");
                            ui.text_edit_singleline(&mut self.packet_filter);
                            ui.label("Search:");
                            ui.text_edit_singleline(&mut self.packet_search);
                            if ui.button("Export Filtered").clicked() {
                                if let Some(path) = rfd::FileDialog::new().set_title("Export Filtered Packets").save_file() {
                                    let mut filtered = Vec::new();
                                    let raw_packets = self.raw_packets.lock().unwrap();
                                    let mut packets: Vec<_> = raw_packets.iter().collect();
                                    packets.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                                    for pkt in packets.iter().take(100) {
                                        if !packet_matches_filter(pkt, &self.packet_filter, &self.packet_search) { continue; }
                                        filtered.push(pkt.data.clone());
                                    }
                                    let _ = std::fs::write(path, serde_json::to_string_pretty(&filtered).unwrap());
                                }
                            }
                        });
                        use std::collections::HashMap;
                        let raw_packets = self.raw_packets.lock().unwrap();
                        ui.label(format!("Total packets: {}", raw_packets.len()));
                        let mut packets: Vec<_> = raw_packets.iter().collect();
                        packets.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                        for pkt in packets.iter().take(100) {
                            if !packet_matches_filter(pkt, &self.packet_filter, &self.packet_search) { continue; }
                            let proto_color = match pkt.protocol.as_str() {
                                "TCP" => egui::Color32::LIGHT_BLUE,
                                "UDP" => egui::Color32::LIGHT_GREEN,
                                "ICMP" => egui::Color32::YELLOW,
                                "ARP" => egui::Color32::RED,
                                _ => egui::Color32::GRAY,
                            };
                            let title = format!(
                                "{} {}:{} → {}:{} [{}] {}",
                                pkt.timestamp.elapsed().as_secs(),
                                pkt.src.map(|ip| ip.to_string()).unwrap_or("?".to_string()),
                                pkt.src_port.map(|p| p.to_string()).unwrap_or("?".to_string()),
                                pkt.dst.map(|ip| ip.to_string()).unwrap_or("?".to_string()),
                                pkt.dst_port.map(|p| p.to_string()).unwrap_or("?".to_string()),
                                pkt.protocol,
                                pkt.country.as_deref().unwrap_or("")
                            );
                            egui::CollapsingHeader::new(title)
                                .default_open(false)
                                .show(ui, |ui| {
                                    ui.visuals_mut().widgets.noninteractive.bg_fill = proto_color;
                                    ui.label(format!("Timestamp: {:?}", pkt.timestamp));
                                    ui.label(format!("Source: {}:{}", pkt.src.map(|ip| ip.to_string()).unwrap_or("?".to_string()), pkt.src_port.map(|p| p.to_string()).unwrap_or("?".to_string())));
                                    ui.label(format!("Destination: {}:{}", pkt.dst.map(|ip| ip.to_string()).unwrap_or("?".to_string()), pkt.dst_port.map(|p| p.to_string()).unwrap_or("?".to_string())));
                                    ui.label(format!("Protocol: {}", pkt.protocol));
                                    if let Some(country) = &pkt.country {
                                        ui.label(format!("Country: {}", country));
                                    }
                                    if let Some(app) = &pkt.app {
                                        ui.label(format!("App: {}", app));
                                    }
                                    let hex = pkt.data.iter().map(|b| format!("{:02X} ", b)).collect::<String>();
                                    let ascii = pkt.data.iter().map(|&b| if b.is_ascii_graphic() { b as char } else { '.' }).collect::<String>();
                                    ui.label("Hex:");
                                    ui.label(hex.clone());
                                    ui.label("ASCII:");
                                    ui.label(ascii.clone());
                                });
                        }
                    });
                    ui.separator();
                    // Plugin import UI
                    // REMOVE the Plugins UI section that uses import_plugin_from_file
                    // Only keep the Lua plugin system UI (already implemented above)
                    ui.heading("Global Traffic Capture");
                    ui.label("Summary by Country, App, Port (Preview)");
                    let raw_packets = self.raw_packets.lock().unwrap();
                    use std::collections::HashMap;
                    let mut country_count = HashMap::new();
                    let mut port_count = HashMap::new();
                    for pkt in raw_packets.iter() {
                        if let Some(country) = &pkt.country {
                            *country_count.entry(country.clone()).or_insert(0) += 1;
                        }
                        if let Some(port) = pkt.dst_port {
                            *port_count.entry(port).or_insert(0) += 1;
                        }
                    }
                    ui.label(format!("Countries: {:?}", country_count));
                    ui.label(format!("Ports: {:?}", port_count));
            });
        });

        // Settings panel (bottom)
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("SysPort v0.1.0");
                ui.label("Platform: ");
                ui.label(std::env::consts::OS);
                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    ui.label("MIT License");
                });
            });
        });

        // Control update interval and pause
        if !self.paused && self.last_update.elapsed().as_secs_f32() > self.update_interval {
            self.last_update = Instant::now();
            self.system.refresh_all();
        }
        ctx.request_repaint();
    }
}

fn packet_matches_filter(pkt: &RawPacketInfo, filter: &str, search: &str) -> bool {
    use regex::Regex;
    let hex = pkt.data.iter().map(|b| format!("{:02X} ", b)).collect::<String>();
    let ascii = pkt.data.iter().map(|&b| if b.is_ascii_graphic() { b as char } else { '.' }).collect::<String>();
    let mut matches = true;
    // Multi-field filter: filter:proto:tcp, filter:country:US, etc.
    for part in filter.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if let Some(rest) = part.strip_prefix("proto:") {
            matches &= pkt.protocol.to_lowercase() == rest.to_lowercase();
        } else if let Some(rest) = part.strip_prefix("country:") {
            matches &= pkt.country.as_deref().unwrap_or("").to_lowercase() == rest.to_lowercase();
        } else if let Some(rest) = part.strip_prefix("src:") {
            matches &= pkt.src.map(|ip| ip.to_string()).unwrap_or_default().contains(rest);
        } else if let Some(rest) = part.strip_prefix("dst:") {
            matches &= pkt.dst.map(|ip| ip.to_string()).unwrap_or_default().contains(rest);
        } else if let Some(rest) = part.strip_prefix("sport:") {
            matches &= pkt.src_port.map(|p| p.to_string()).unwrap_or_default() == rest;
        } else if let Some(rest) = part.strip_prefix("dport:") {
            matches &= pkt.dst_port.map(|p| p.to_string()).unwrap_or_default() == rest;
        } else if let Ok(re) = Regex::new(part) {
            matches &= re.is_match(&hex) || re.is_match(&ascii);
        } else {
            matches &= hex.to_lowercase().contains(&part.to_lowercase())
                || ascii.to_lowercase().contains(&part.to_lowercase())
                || pkt.src.map(|ip| ip.to_string()).unwrap_or_default().to_lowercase().contains(&part.to_lowercase())
                || pkt.dst.map(|ip| ip.to_string()).unwrap_or_default().to_lowercase().contains(&part.to_lowercase())
                || pkt.src_port.map(|p| p.to_string()).unwrap_or_default().contains(&part.to_lowercase())
                || pkt.dst_port.map(|p| p.to_string()).unwrap_or_default().contains(&part.to_lowercase())
                || pkt.protocol.to_lowercase().contains(&part.to_lowercase())
                || pkt.country.as_deref().unwrap_or("").to_lowercase().contains(&part.to_lowercase());
        }
    }
    if !search.is_empty() {
        let search = search.to_lowercase();
        matches &= hex.to_lowercase().contains(&search)
            || ascii.to_lowercase().contains(&search);
    }
    matches
} 