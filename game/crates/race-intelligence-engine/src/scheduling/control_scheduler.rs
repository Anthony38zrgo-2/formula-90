#[derive(Debug, Clone, Copy)]
pub struct ControlScheduler {
    pub control_interval_seconds: f64,
    pub tactical_interval_seconds: f64,
    pub accumulated_control_seconds: f64,
    pub accumulated_tactical_seconds: f64,
}

pub fn create_scheduler(
    control_interval_seconds: f64,
    tactical_interval_seconds: f64,
) -> Result<ControlScheduler, String> {
    if !control_interval_seconds.is_finite()
        || control_interval_seconds <= 0.0
        || control_interval_seconds > 1.0
    {
        return Err("Scheduler control interval is invalid".to_string());
    }
    if !tactical_interval_seconds.is_finite()
        || tactical_interval_seconds < control_interval_seconds
        || tactical_interval_seconds > 5.0
    {
        return Err("Scheduler tactical interval is invalid".to_string());
    }
    Ok(ControlScheduler {
        control_interval_seconds,
        tactical_interval_seconds,
        accumulated_control_seconds: 0.0,
        accumulated_tactical_seconds: 0.0,
    })
}

pub fn consume_host_interval(
    scheduler: &mut ControlScheduler,
    host_interval_seconds: f64,
) -> Result<(bool, bool), String> {
    let (control_ticks, tactical_ticks) =
        consume_host_interval_ticks(scheduler, host_interval_seconds)?;
    Ok((control_ticks > 0, tactical_ticks > 0))
}

pub fn consume_host_interval_ticks(
    scheduler: &mut ControlScheduler,
    host_interval_seconds: f64,
) -> Result<(usize, usize), String> {
    create_scheduler(
        scheduler.control_interval_seconds,
        scheduler.tactical_interval_seconds,
    )?;
    if !host_interval_seconds.is_finite()
        || host_interval_seconds < 0.0
        || host_interval_seconds > 1.0
    {
        return Err("Scheduler host interval is invalid".to_string());
    }
    scheduler.accumulated_control_seconds += host_interval_seconds;
    scheduler.accumulated_tactical_seconds += host_interval_seconds;
    let control_ticks = ((scheduler.accumulated_control_seconds + 1e-12)
        / scheduler.control_interval_seconds)
        .floor() as usize;
    let tactical_ticks = ((scheduler.accumulated_tactical_seconds + 1e-12)
        / scheduler.tactical_interval_seconds)
        .floor() as usize;
    scheduler.accumulated_control_seconds = (scheduler.accumulated_control_seconds
        - control_ticks as f64 * scheduler.control_interval_seconds)
        .max(0.0);
    scheduler.accumulated_tactical_seconds = (scheduler.accumulated_tactical_seconds
        - tactical_ticks as f64 * scheduler.tactical_interval_seconds)
        .max(0.0);
    Ok((control_ticks, tactical_ticks))
}
