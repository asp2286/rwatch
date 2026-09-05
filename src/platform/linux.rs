use std::error::Error;
use std::fs;

use crate::cpu::{CpuSnapshot, CpuTimes};
use crate::memory::MemoryInfo;
use crate::model::SystemSnapshot;
use crate::uptime::format_uptime;

pub fn collect_snapshot() -> Result<SystemSnapshot, Box<dyn Error>> {
    let uptime_raw = fs::read_to_string("/proc/uptime")?;
    let loadavg_raw = fs::read_to_string("/proc/loadavg")?;
    let meminfo_raw = fs::read_to_string("/proc/meminfo")?;

    let uptime_seconds = parse_uptime_seconds(&uptime_raw)?;
    let uptime = format_uptime(uptime_seconds);

    let memory = parse_memory_info(&meminfo_raw)?;
    let cpu = read_cpu_snapshots()?;

    Ok(SystemSnapshot {
        uptime,
        loadavg: loadavg_raw.trim().to_string(),
        memory,
        cpu,
    })
}