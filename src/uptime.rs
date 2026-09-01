pub fn parse_uptime_seconds(input: &str) -> Result<u64, String> {
    let first_value = input
        .split_whitespace()
        .next()
        .ok_or("Invalid /proc/uptime")?;

    let seconds = first_value
        .parse::<f64>()
        .map_err(|_| format!("Invalid uptime value: {first_value}"))?;

    Ok(seconds as u64)
}

pub fn format_uptime(total_seconds: u64) -> String {
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;

    format!("{days}d {hours:02}h {minutes:02}m {seconds:02}s")
}
