use crate::cpu::CpuSnapshot;
use crate::memory::MemoryInfo;

pub struct SystemSnapshot {
    pub cpu_name: String,
    pub uptime: String,
    pub loadavg: String,
    pub memory: MemoryInfo,
    pub cpu: Vec<CpuSnapshot>,
}