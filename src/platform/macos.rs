use std::error::Error;
use std::ffi::CString;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{mem, ptr};

use crate::cpu::{CpuSnapshot, CpuTimes};
use crate::memory::MemoryInfo;
use crate::model::SystemSnapshot;

use mach2::mach_init::mach_host_self;
use mach2::traps::mach_task_self;

use crate::uptime::format_uptime;

pub fn collect_snapshot() -> Result<SystemSnapshot, Box<dyn Error>> {
    let total_kib = read_total_memory_kib()?;
    let available_kib = read_available_memory_kib()?;

    Ok(SystemSnapshot {
        cpu_name: read_cpu_name()?,
        uptime: format_uptime(read_uptime_seconds()?),
        loadavg: read_loadavg()?,
        memory: MemoryInfo {
            total_kib,
            available_kib,
        },
        cpu: read_cpu_snapshots()?,
    })
}

fn read_loadavg() -> Result<String, Box<dyn Error>> {
    let mut values = [0.0_f64; 3];

    let count = unsafe { libc::getloadavg(values.as_mut_ptr(), values.len() as i32) };

    if count != 3 {
        return Err(format!("getloadavg returned {count}, expected 3").into());
    }

    Ok(format!(
        "{:.2} {:.2} {:.2}",
        values[0], values[1], values[2]
    ))
}

fn read_uptime_seconds() -> Result<u64, Box<dyn Error>> {
    let name = CString::new("kern.boottime")?;

    let mut boot_time: libc::timeval = unsafe { mem::zeroed() };
    let mut size = mem::size_of::<libc::timeval>();

    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            &mut boot_time as *mut _ as *mut libc::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    Ok(now - boot_time.tv_sec as u64)
}

fn read_total_memory_kib() -> Result<u64, Box<dyn Error>> {
    Ok(sysctl_u64("hw.memsize")? / 1024)
}

fn read_available_memory_kib() -> Result<u64, Box<dyn Error>> {
    let page_size = sysctl_u64("hw.pagesize")?;

    unsafe {
        let host = mach_host_self();

        let mut stats: libc::vm_statistics64 = mem::zeroed();

        let mut count = (mem::size_of::<libc::vm_statistics64>()
            / mem::size_of::<libc::integer_t>())
            as libc::mach_msg_type_number_t;

        let result = libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            &mut stats as *mut _ as libc::host_info64_t,
            &mut count,
        );

        if result != libc::KERN_SUCCESS {
            return Err(format!("host_statistics64 failed: {result}").into());
        }

        let available_pages = stats.free_count as u64 + stats.inactive_count as u64;

        let available_bytes = available_pages * page_size;

        Ok(available_bytes / 1024)
    }
}

fn sysctl_u64(name: &str) -> Result<u64, Box<dyn Error>> {
    let name = CString::new(name)?;

    let mut value: u64 = 0;
    let mut size = mem::size_of::<u64>();

    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            &mut value as *mut _ as *mut libc::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    Ok(value)
}

fn read_total_cpu_snapshot() -> Result<CpuSnapshot, Box<dyn Error>> {
    unsafe {
        let host = mach_host_self();

        let mut cpu_load: libc::host_cpu_load_info_data_t = mem::zeroed();

        let mut count = (mem::size_of::<libc::host_cpu_load_info_data_t>()
            / mem::size_of::<libc::integer_t>())
            as libc::mach_msg_type_number_t;

        let result = libc::host_statistics(
            host,
            libc::HOST_CPU_LOAD_INFO,
            &mut cpu_load as *mut _ as libc::host_info_t,
            &mut count,
        );

        if result != libc::KERN_SUCCESS {
            return Err(format!("host_statistics failed: {result}").into());
        }

        let user = cpu_load.cpu_ticks[libc::CPU_STATE_USER as usize] as u64;

        let system = cpu_load.cpu_ticks[libc::CPU_STATE_SYSTEM as usize] as u64;

        let idle = cpu_load.cpu_ticks[libc::CPU_STATE_IDLE as usize] as u64;

        let nice = cpu_load.cpu_ticks[libc::CPU_STATE_NICE as usize] as u64;

        let total = user + system + idle + nice;

        Ok(CpuSnapshot {
            name: "cpu".to_string(),
            times: CpuTimes { idle, total },
        })
    }
}

fn read_per_core_cpu_snapshots() -> Result<Vec<CpuSnapshot>, Box<dyn Error>> {
    unsafe {
        let host = mach_host_self();

        let mut processor_count: libc::natural_t = 0;
        let mut processor_info: libc::processor_info_array_t = ptr::null_mut();
        let mut processor_info_count: libc::mach_msg_type_number_t = 0;

        let result = libc::host_processor_info(
            host,
            libc::PROCESSOR_CPU_LOAD_INFO,
            &mut processor_count,
            &mut processor_info,
            &mut processor_info_count,
        );

        if result != libc::KERN_SUCCESS {
            return Err(format!("host_processor_info failed: {result}").into());
        }

        let mut snapshots = Vec::with_capacity(processor_count as usize);

        for cpu_index in 0..processor_count as usize {
            let base = cpu_index * libc::CPU_STATE_MAX as usize;

            let user = *processor_info.add(base + libc::CPU_STATE_USER as usize) as u64;

            let system = *processor_info.add(base + libc::CPU_STATE_SYSTEM as usize) as u64;

            let idle = *processor_info.add(base + libc::CPU_STATE_IDLE as usize) as u64;

            let nice = *processor_info.add(base + libc::CPU_STATE_NICE as usize) as u64;

            snapshots.push(CpuSnapshot {
                name: format!("cpu{cpu_index}"),
                times: CpuTimes {
                    idle,
                    total: user + system + idle + nice,
                },
            });
        }

        let byte_size = processor_info_count as usize * mem::size_of::<libc::integer_t>();

        let deallocate_result = libc::vm_deallocate(
            mach_task_self(),
            processor_info as libc::vm_address_t,
            byte_size as libc::vm_size_t,
        );

        if deallocate_result != libc::KERN_SUCCESS {
            return Err(format!("vm_deallocate failed: {deallocate_result}").into());
        }

        Ok(snapshots)
    }
}

fn read_cpu_snapshots() -> Result<Vec<CpuSnapshot>, Box<dyn Error>> {
    let total = read_total_cpu_snapshot()?;
    let cores = read_per_core_cpu_snapshots()?;

    let mut snapshots = Vec::with_capacity(1 + cores.len());

    snapshots.push(total);
    snapshots.extend(cores);

    Ok(snapshots)
}

fn sysctl_string(name: &str) -> Result<String, Box<dyn Error>> {
    let name = CString::new(name)?;

    let mut size: usize = 0;

    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            ptr::null_mut(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    let mut buffer = vec![0u8; size];

    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    if buffer.last() == Some(&0) {
        buffer.pop();
    }

    Ok(String::from_utf8(buffer)?)
}

fn read_cpu_name() -> Result<String, Box<dyn Error>> {
    sysctl_string("machdep.cpu.brand_string")
}
