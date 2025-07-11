use std::time::Instant;

#[derive(Clone)]
pub struct Metrics {
    pub timestamp: Instant,
    pub cpu_usage: Vec<f32>, // per core
    pub cpu_total: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub disks: Vec<DiskMetrics>,
    pub net_rx: u64,
    pub net_tx: u64,
    pub selected_interface: Option<String>,
    pub interfaces: Vec<String>,
    pub net_per_interface: Vec<NetInterfaceStats>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            cpu_usage: vec![],
            cpu_total: 0.0,
            mem_total: 0,
            mem_used: 0,
            disks: vec![],
            net_rx: 0,
            net_tx: 0,
            selected_interface: None,
            interfaces: vec![],
            net_per_interface: vec![],
        }
    }
}

#[derive(Clone, Default)]
pub struct DiskMetrics {
    pub name: String,
    pub total: u64,
    pub available: u64,
}

#[derive(Clone, Default)]
pub struct NetInterfaceStats {
    pub name: String,
    pub rx: u64,
    pub tx: u64,
} 