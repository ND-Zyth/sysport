use crate::metrics::Metrics;
use std::fs::File;
use std::io::{Write, Read};

pub enum ExportFormat {
    Json,
    Csv,
}

pub fn export_metrics(history: &[Metrics], format: ExportFormat, path: &str) -> std::io::Result<()> {
    match format {
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(&history_as_serializable(history))?;
            let mut file = File::create(path)?;
            file.write_all(json.as_bytes())?;
        }
        ExportFormat::Csv => {
            let mut wtr = csv::Writer::from_path(path)?;
            wtr.write_record(&["timestamp","cpu_total","mem_used","mem_total","net_rx","net_tx"])?;
            for m in history {
                wtr.write_record(&[
                    format!("{:?}", m.timestamp),
                    format!("{:.2}", m.cpu_total),
                    m.mem_used.to_string(),
                    m.mem_total.to_string(),
                    m.net_rx.to_string(),
                    m.net_tx.to_string(),
                ])?;
            }
            wtr.flush()?;
        }
    }
    Ok(())
}

pub fn import_capture(path: &str) -> std::io::Result<Vec<Metrics>> {
    let mut file = File::open(path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    let data: Vec<SerializableMetrics> = serde_json::from_str(&buf)?;
    Ok(data.into_iter().map(|s| s.into()).collect())
}

pub fn export_capture(history: &[Metrics], path: &str) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(&history_as_serializable(history))?;
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

pub fn import_log(path: &str) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

pub fn export_log(log: &str, path: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(log.as_bytes())?;
    Ok(())
}

// Helper for serializing Metrics (since Instant is not serializable)
use serde::Serialize;
#[derive(Serialize, serde::Deserialize)]
struct SerializableMetrics {
    timestamp: u128,
    cpu_total: f32,
    mem_total: u64,
    mem_used: u64,
    net_rx: u64,
    net_tx: u64,
}

impl From<SerializableMetrics> for Metrics {
    fn from(s: SerializableMetrics) -> Self {
        Metrics {
            timestamp: std::time::Instant::now(),
            cpu_total: s.cpu_total,
            mem_total: s.mem_total,
            mem_used: s.mem_used,
            net_rx: s.net_rx,
            net_tx: s.net_tx,
            ..Default::default()
        }
    }
}

fn history_as_serializable(history: &[Metrics]) -> Vec<SerializableMetrics> {
    history.iter().map(|m| SerializableMetrics {
        timestamp: m.timestamp.elapsed().as_millis(),
        cpu_total: m.cpu_total,
        mem_total: m.mem_total,
        mem_used: m.mem_used,
        net_rx: m.net_rx,
        net_tx: m.net_tx,
    }).collect()
} 