use crate::metrics::Metrics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub message: String,
    pub level: AlertLevel,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum AlertRule {
    CpuUsage { threshold: f32, level: AlertLevel },
    MemUsage { threshold: f32, level: AlertLevel },
    DiskUsage { threshold: f32, level: AlertLevel },
    NetRx { threshold: u64, level: AlertLevel },
    NetTx { threshold: u64, level: AlertLevel },
}

pub struct AlertManager {
    pub rules: Vec<AlertRule>,
    pub active_alerts: Vec<Alert>,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            rules: vec![
                AlertRule::CpuUsage { threshold: 90.0, level: AlertLevel::Warning },
                AlertRule::MemUsage { threshold: 90.0, level: AlertLevel::Warning },
                AlertRule::DiskUsage { threshold: 95.0, level: AlertLevel::Warning },
                AlertRule::NetRx { threshold: 1024 * 1024, level: AlertLevel::Info }, // 1MB/s
                AlertRule::NetTx { threshold: 1024 * 1024, level: AlertLevel::Info },
            ],
            active_alerts: Vec::new(),
        }
    }

    pub fn check(&mut self, metrics: &Metrics) {
        self.active_alerts.clear();
        let now = std::time::Instant::now();
        for rule in &self.rules {
            match rule {
                AlertRule::CpuUsage { threshold, level } => {
                    if metrics.cpu_total > *threshold {
                        self.active_alerts.push(Alert {
                            message: format!("CPU usage high: {:.1}%", metrics.cpu_total),
                            level: level.clone(),
                            timestamp: now,
                        });
                    }
                }
                AlertRule::MemUsage { threshold, level } => {
                    let percent = metrics.mem_used as f32 / metrics.mem_total as f32 * 100.0;
                    if percent > *threshold {
                        self.active_alerts.push(Alert {
                            message: format!("Memory usage high: {:.1}%", percent),
                            level: level.clone(),
                            timestamp: now,
                        });
                    }
                }
                AlertRule::DiskUsage { threshold, level } => {
                    for disk in &metrics.disks {
                        let used = disk.total - disk.available;
                        let percent = used as f32 / disk.total as f32 * 100.0;
                        if percent > *threshold {
                            self.active_alerts.push(Alert {
                                message: format!("Disk {} usage high: {:.1}%", disk.name, percent),
                                level: level.clone(),
                                timestamp: now,
                            });
                        }
                    }
                }
                AlertRule::NetRx { threshold, level } => {
                    if metrics.net_rx > *threshold {
                        self.active_alerts.push(Alert {
                            message: format!("High network download: {:.2} KB/s", metrics.net_rx as f64 / 1024.0),
                            level: level.clone(),
                            timestamp: now,
                        });
                    }
                }
                AlertRule::NetTx { threshold, level } => {
                    if metrics.net_tx > *threshold {
                        self.active_alerts.push(Alert {
                            message: format!("High network upload: {:.2} KB/s", metrics.net_tx as f64 / 1024.0),
                            level: level.clone(),
                            timestamp: now,
                        });
                    }
                }
            }
        }
    }
} 