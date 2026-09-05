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
