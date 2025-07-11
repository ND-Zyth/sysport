mod metrics;
mod alert;
mod export;
mod theme;
mod packet_stats;
mod remote;
mod plugins;
mod app;
use eframe::{egui, epi};
use egui::plot::{Plot, Line, Values, Value};
use sysinfo::{System, SystemExt, DiskExt, NetworkExt, NetworksExt};
use std::time::Instant;
use crate::app::SysPortApp;
use std::fs;
use std::path::Path;
use eframe::epi::IconData;

fn load_icon() -> Option<IconData> {
    let icon_path = Path::new("sysport.png");
    if let Ok(bytes) = fs::read(icon_path) {
        if let Ok(image) = image::load_from_memory(&bytes) {
            let image = image.to_rgba8();
            let (width, height) = image.dimensions();
            let pixels = image.into_raw();
            return Some(IconData { width, height, rgba: pixels });
        }
    }
    None
}

fn main() {
    let mut options = eframe::NativeOptions::default();
    if let Some(icon) = load_icon() {
        options.icon_data = Some(icon);
    }
    let mut plugin_system = plugins::PluginSystem::new();
    plugin_system.load_plugins("../plugins/sample_plugin/target/release"); // Adjust path as needed
    eframe::run_native(Box::new(SysPortApp::default()), options);
}

// In plotting, use:
// let cpu_points: Vec<Value> = history.iter().enumerate().map(|(i, m)| Value::new(i as f64, m.cpu_total as f64)).collect();
// Line::new(Values::from_values(cpu_points))
// ...
// update signature: fn update(&mut self, ctx: &egui::Context, _frame: &eframe::epi::Frame)
// For sysinfo, use sys.cpus() as a slice, e.g. sys.cpus().len() or sys.cpus()[i].cpu_usage()

// If Metrics is defined here, implement Default manually:
// Remove duplicate Metrics, DiskMetrics, and SysPortApp struct definitions from main.rs. Use the ones from the modules instead.
