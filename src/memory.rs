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

pub fn kib_to_gib(kib: u64) -> f64 {
    kib as f64 / 1024.0 / 1024.0
}