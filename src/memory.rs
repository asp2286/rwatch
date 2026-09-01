pub struct MemoryInfo {
    pub total_kib: u64,
    pub available_kib: u64,
}

impl MemoryInfo {
    pub fn used_kib(&self) -> u64 {
        self.total_kib - self.available_kib
    }

    pub fn usage_percent(&self) -> f64 {
        self.used_kib() as f64 / self.total_kib as f64 * 100.0
    }
}

pub fn parse_memory_info(input: &str) -> Result<MemoryInfo, String> {
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

pub fn kib_to_gib(kib: u64) -> f64 {
    kib as f64 / 1024.0 / 1024.0
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
