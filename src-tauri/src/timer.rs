use crate::*;

pub(crate) fn nt_success(status: i32) -> bool {
    status >= 0
}

pub(crate) fn round_ms(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

pub(crate) fn hundred_ns_to_ms(value: u32) -> f32 {
    round_ms(value as f32 / 10_000.0)
}

pub(crate) fn ms_to_hundred_ns(value_ms: f32) -> u32 {
    let clamped = value_ms.clamp(0.5, 1000.0);
    (clamped * 10_000.0).round() as u32
}

pub(crate) fn query_timer_resolution() -> Result<(u32, u32, u32), AppError> {
    let mut max_100ns = 0u32;
    let mut min_100ns = 0u32;
    let mut current_100ns = 0u32;
    let status = unsafe {
        NtQueryTimerResolution(&mut max_100ns, &mut min_100ns, &mut current_100ns)
    };
    if !nt_success(status) {
        return Err(AppError::Message(format!(
            "NtQueryTimerResolution failed with NTSTATUS 0x{status:08X}"
        )));
    }
    Ok((max_100ns, min_100ns, current_100ns))
}

pub(crate) fn build_timer_resolution_dto(runtime: &RuntimeControlState) -> Result<TimerResolutionDto, AppError> {
    let (max_100ns, min_100ns, current_100ns) = query_timer_resolution()?;
    let requested_100ns = runtime
        .timer_resolution
        .lock()
        .map_err(|_| AppError::Message("timer state lock poisoned".to_owned()))?
        .requested_100ns;

    Ok(TimerResolutionDto {
        minimum_ms: hundred_ns_to_ms(min_100ns),
        maximum_ms: hundred_ns_to_ms(max_100ns),
        current_ms: hundred_ns_to_ms(current_100ns),
        requested_ms: requested_100ns.map(hundred_ns_to_ms),
        enabled: requested_100ns.is_some(),
    })
}

pub(crate) fn apply_timer_resolution_request(
    runtime: &RuntimeControlState,
    requested_100ns: Option<u32>,
) -> Result<TimerResolutionDto, AppError> {
    let mut guard = runtime
        .timer_resolution
        .lock()
        .map_err(|_| AppError::Message("timer state lock poisoned".to_owned()))?;

    if let Some(current_request) = guard.requested_100ns {
        if Some(current_request) == requested_100ns {
            drop(guard);
            return build_timer_resolution_dto(runtime);
        }

        let mut current_after_release = 0u32;
        let status = unsafe { NtSetTimerResolution(current_request, 0, &mut current_after_release) };
        if !nt_success(status) {
            return Err(AppError::Message(format!(
                "NtSetTimerResolution(release) failed with NTSTATUS 0x{status:08X}"
            )));
        }
        guard.requested_100ns = None;
    }

    if let Some(next_request) = requested_100ns {
        let mut current_after_set = 0u32;
        let status = unsafe { NtSetTimerResolution(next_request, 1, &mut current_after_set) };
        if !nt_success(status) {
            return Err(AppError::Message(format!(
                "NtSetTimerResolution(set) failed with NTSTATUS 0x{status:08X}"
            )));
        }
        guard.requested_100ns = Some(next_request);
    }

    drop(guard);
    build_timer_resolution_dto(runtime)
}

pub(crate) fn release_timer_resolution(runtime: &RuntimeControlState) {
    let Ok(mut guard) = runtime.timer_resolution.lock() else {
        error!("Failed to acquire timer state lock during shutdown");
        return;
    };

    let Some(requested_100ns) = guard.requested_100ns.take() else {
        return;
    };

    let mut current_after_release = 0u32;
    let status = unsafe { NtSetTimerResolution(requested_100ns, 0, &mut current_after_release) };
    if !nt_success(status) {
        error!(
            "NtSetTimerResolution(release on shutdown) failed with NTSTATUS 0x{status:08X}"
        );
    }
}

pub(crate) fn disable_process_power_throttling() -> Result<(), AppError> {
    let throttling = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            | PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION,
        StateMask: 0,
    };

    unsafe {
        SetProcessInformation(
            GetCurrentProcess(),
            ProcessPowerThrottling,
            (&throttling as *const PROCESS_POWER_THROTTLING_STATE).cast::<c_void>(),
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )
    }
    .map_err(|e| AppError::Message(format!("SetProcessInformation(ProcessPowerThrottling) failed: {e}")))
}
