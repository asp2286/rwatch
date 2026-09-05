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

    Ok(SystemSnapshot {
        system_name: read_system_name()?,
        cpu_name: read_cpu_name()?,
        uptime: format_uptime(uptime_seconds),
        loadavg: loadavg_raw.trim().to_string(),
        memory: parse_memory_info(&meminfo_raw)?,
        cpu: read_cpu_snapshots()?,
    })
}

fn find_cpuinfo_value<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    input.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;

        if name.trim() == key {
            Some(value.trim())
        } else {
            None
        }
    })
}

fn parse_uptime_seconds(input: &str) -> Result<u64, String> {
    let first_value = input
        .split_whitespace()
        .next()
        .ok_or("Invalid /proc/uptime")?;

    let seconds = first_value
        .parse::<f64>()
        .map_err(|_| format!("Invalid uptime value: {first_value}"))?;

    Ok(seconds as u64)
}

fn parse_memory_info(input: &str) -> Result<MemoryInfo, String> {
    let total_kib = input
        .lines()
        .find(|line| line.starts_with("MemTotal:"))
        .ok_or("MemTotal not found")?;

    let available_kib = input
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))
        .ok_or("MemAvailable not found")?;

    Ok(MemoryInfo {
        total_kib: parse_meminfo_value(total_kib)?,
        available_kib: parse_meminfo_value(available_kib)?,
    })
}

fn parse_meminfo_value(line: &str) -> Result<u64, String> {
    let value = line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("Invalid meminfo line: {line}"))?;

    value
        .parse::<u64>()
        .map_err(|_| format!("Invalid memory value: {value}"))
}

fn read_cpu_snapshots() -> Result<Vec<CpuSnapshot>, Box<dyn Error>> {
    let stat = fs::read_to_string("/proc/stat")?;

    let mut result = Vec::new();

    for line in stat.lines() {
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };

        if !name.starts_with("cpu") {
            break;
        }

        result.push(CpuSnapshot {
            name: name.to_string(),
            times: parse_cpu_times(line)?,
        });
    }

    Ok(result)
}

fn parse_cpu_times(line: &str) -> Result<CpuTimes, String> {
    let values: Result<Vec<u64>, _> = line
        .split_whitespace()
        .skip(1)
        .map(str::parse::<u64>)
        .collect();

    let values =
        values.map_err(|_| format!("Invalid CPU line: {line}"))?;

    if values.len() < 4 {
        return Err(format!("Not enough CPU values: {line}"));
    }

    let user = values[0];
    let nice = values[1];
    let system = values[2];
    let idle = values[3];

    let iowait = values.get(4).copied().unwrap_or(0);
    let irq = values.get(5).copied().unwrap_or(0);
    let softirq = values.get(6).copied().unwrap_or(0);
    let steal = values.get(7).copied().unwrap_or(0);

    let idle_total = idle + iowait;

    let total =
        user
            + nice
            + system
            + idle
            + iowait
            + irq
            + softirq
            + steal;

    Ok(CpuTimes {
        idle: idle_total,
        total,
    })
}

fn read_cpu_name() -> Result<String, Box<dyn Error>> {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;

    // x86/x86_64
    if let Some(name) = find_cpuinfo_value(&cpuinfo, "model name") {
        return Ok(name.to_string());
    }

    // ARM
    let implementer = find_cpuinfo_value(&cpuinfo, "CPU implementer");
    let part = find_cpuinfo_value(&cpuinfo, "CPU part");

    if let (Some(implementer), Some(part)) = (implementer, part) {
        if let Some(name) = arm_cpu_name(implementer, part) {
            return Ok(name.to_string());
        }
    }

    Ok("Unknown CPU".to_string())
}

fn arm_cpu_name(
    implementer: &str,
    part: &str,
) -> Option<&'static str> {
    match (implementer, part) {
        ("0x41", "0xd03") => Some("ARM Cortex-A53"),
        ("0x41", "0xd08") => Some("ARM Cortex-A72"),
        ("0x41", "0xd0b") => Some("ARM Cortex-A76"),
        ("0x41", "0xd41") => Some("ARM Cortex-A78"),
        _ => None,
    }
}

fn read_system_name() -> Result<String, Box<dyn Error>> {
    if let Ok(model) = fs::read_to_string("/proc/device-tree/model") {
        let model = model.trim_matches('\0').trim();

        if !model.is_empty() {
            return Ok(model.to_string());
        }
    }

    let vendor = fs::read_to_string("/sys/class/dmi/id/sys_vendor")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let product = fs::read_to_string("/sys/class/dmi/id/product_name")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    match (vendor, product) {
        (Some(vendor), Some(product)) => {
            Ok(format!("{vendor} {product}"))
        }

        (None, Some(product)) => Ok(product),

        (Some(vendor), None) => Ok(vendor),

        _ => {
            let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;

            if let Some(model) = find_cpuinfo_value(&cpuinfo, "Model") {
                return Ok(model.to_string());
            }

            Ok("Unknown system".to_string())
        }
    }
}
