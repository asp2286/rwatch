use std::error::Error;
use std::fs;

#[derive(Debug)]
pub struct CpuTimes {
    pub idle: u64,
    pub total: u64,
}

#[derive(Debug)]
pub struct CpuSnapshot {
    pub name: String,
    pub times: CpuTimes,
}

pub fn read_cpu_snapshots() -> Result<Vec<CpuSnapshot>, Box<dyn Error>> {
    let stat = fs::read_to_string("/proc/stat")?;

    let mut result = Vec::new();

    for line in stat.lines() {
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };

        if !name.starts_with("cpu") {
            break;
        }

        let times = parse_cpu_times(line)?;

        result.push(CpuSnapshot {
            name: name.to_string(),
            times,
        });
    }

    Ok(result)
}

pub fn cpu_usage(before: &CpuTimes, after: &CpuTimes) -> f64 {
    let total_delta = after.total - before.total;
    let idle_delta = after.idle - before.idle;

    if total_delta == 0 {
        return 0.0;
    }

    (total_delta - idle_delta) as f64 / total_delta as f64 * 100.0
}

pub fn usage_bar(percent: f64, width: usize) -> String {
    let percent = percent.clamp(0.0, 100.0);

    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let empty = width - filled;

    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn parse_cpu_times(line: &str) -> Result<CpuTimes, String> {
    let values: Result<Vec<u64>, _> = line
        .split_whitespace()
        .skip(1)
        .map(str::parse::<u64>)
        .collect();

    let values = values.map_err(|_| format!("Invalid CPU line: {line}"))?;

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

    let total = user + nice + system + idle + iowait + irq + softirq + steal;

    Ok(CpuTimes {
        idle: idle_total,
        total,
    })
}
