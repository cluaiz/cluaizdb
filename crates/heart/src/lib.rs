use std::sync::Arc;
use tokio::sync::RwLock;
use sysinfo::{System, CpuRefreshKind, RefreshKind, MemoryRefreshKind};
use tracing::info;
use std::time::Duration;

pub mod booster;
use booster::BoosterState;

/// Real-time biological telemetry representing the system's hardware state.
#[derive(Debug, Clone, Copy)]
pub struct Telemetry {
    /// Heart Rate (BPM): Number of active API hits/jobs processing.
    pub bpm: u32,
    /// Blood Pressure (BP): CPU load (0-100%).
    pub bp_systolic: u8,
    /// Oxygen Level (SpO2): Available RAM percentage (0-100%).
    pub spo2: u8,
    /// Metabolic Rate: Background task / garbage collection intensity (0-100%).
    pub metabolic_rate: u8,
    /// GPU Load (0-100%).
    pub gpu_load: u8,
    /// Disk / SSD I/O wait or load percentage.
    pub ssd_load: u8,
    /// CPU Load of the current CLUAIZD process (0-100%).
    pub process_bp: u8,
    /// RAM Usage percentage of the current CLUAIZD process.
    pub process_spo2: u8,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            bpm: 60, // Resting heart rate
            bp_systolic: 120, // Normal systolic BP
            spo2: 98, // Healthy SpO2
            metabolic_rate: 10, // Low background activity
            gpu_load: 0,
            ssd_load: 0,
            process_bp: 0,
            process_spo2: 0,
        }
    }
}

/// Cluaizdb-HEART Autonomic Controller
pub struct Heart {
    pub telemetry: Arc<RwLock<Telemetry>>,
    pub booster_state: Arc<RwLock<BoosterState>>,
}

impl Heart {
    pub fn new(data_dir: &std::path::Path) -> Self {
        Self {
            telemetry: Arc::new(RwLock::new(Telemetry::default())),
            booster_state: Arc::new(RwLock::new(BoosterState::load_from_disk(data_dir))),
        }
    }

    /// Starts the background Tokio loop to measure real-time system metrics.
    pub fn start_heartbeat(&self) {
        let telemetry = Arc::clone(&self.telemetry);

        tokio::spawn(async move {
            info!("Cluaizdb-HEART Telemetry loop started.");

            // Configure sysinfo to fetch CPU, Memory, and Processes
            use sysinfo::ProcessRefreshKind;
            let mut sys = System::new_with_specifics(
                RefreshKind::new()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything())
                    .with_processes(ProcessRefreshKind::everything())
            );

            let pid = sysinfo::get_current_pid().unwrap_or(sysinfo::Pid::from_u32(0));

            // Give sysinfo a moment to get an initial reading
            tokio::time::sleep(Duration::from_millis(200)).await;

            loop {
                // Refresh system stats
                sys.refresh_cpu_usage();
                sys.refresh_memory();
                sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu());

                // 1. Calculate Blood Pressure (BP) from Global CPU Usage
                let mut total_cpu = 0.0;
                let cpus = sys.cpus();
                for cpu in cpus {
                    total_cpu += cpu.cpu_usage();
                }
                
                let avg_cpu = if cpus.is_empty() { 0.0 } else { total_cpu / cpus.len() as f32 };
                let bp = avg_cpu.clamp(0.0, 100.0) as u8;

                // 2. Calculate SpO2 (Oxygen) from Global Available RAM
                let total_mem = sys.total_memory();
                let free_mem = sys.available_memory();
                
                let spo2 = if total_mem > 0 {
                    ((free_mem as f64 / total_mem as f64) * 100.0).clamp(0.0, 100.0) as u8
                } else {
                    100
                };

                // 3. Process-Specific Telemetry
                let mut process_bp = 0.0;
                let mut process_spo2 = 0.0;
                if let Some(process) = sys.process(pid) {
                    // process.cpu_usage() returns total CPU usage (e.g. up to 800% on 8 cores). 
                    // Divide by number of cores to get a 0-100% value comparable to `bp`.
                    let p_cpu = process.cpu_usage() / if cpus.is_empty() { 1.0 } else { cpus.len() as f32 };
                    process_bp = p_cpu.clamp(0.0, 100.0);
                    
                    let p_mem = process.memory(); // in bytes
                    if total_mem > 0 {
                        process_spo2 = ((p_mem as f64 / total_mem as f64) * 100.0).clamp(0.0, 100.0);
                    }
                }

                // Update the shared telemetry
                {
                    let mut lock = telemetry.write().await;
                    lock.bp_systolic = bp;
                    lock.spo2 = spo2;
                    lock.process_bp = process_bp as u8;
                    lock.process_spo2 = process_spo2 as u8;
                    // BPM and Metabolic Rate can be updated by other components later
                }

                // Sleep before next reading
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }
}
