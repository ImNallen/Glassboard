use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;

#[derive(Default)]
struct Totals {
    cpu_time: Duration,
    memory: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceUsage {
    memory_bytes: u64,
    cpu_percent: Option<f32>,
}

/// The previous sample's time and total CPU time, since CPU use needs two samples.
#[derive(Default)]
pub(crate) struct Usage(Mutex<Option<(Instant, Duration)>>);

// Async so the process scan runs off the main thread.
#[tauri::command]
pub(crate) async fn get_resource_usage(app: tauri::AppHandle) -> ResourceUsage {
    let state = app.state::<Usage>();
    let mut previous = state.0.lock().unwrap();
    let totals = owned();
    let sample = (Instant::now(), totals.cpu_time);
    let usage = ResourceUsage {
        memory_bytes: totals.memory,
        cpu_percent: cpu_percent(*previous, sample),
    };
    *previous = Some(sample);
    usage
}

/// Percent of one core, as Activity Monitor and top report it. The total can
/// shrink when a webview process exits, which reads as idle rather than negative.
fn cpu_percent(
    previous: Option<(Instant, Duration)>,
    (now, cpu_time): (Instant, Duration),
) -> Option<f32> {
    let (then, previous_cpu_time) = previous?;
    let wall = now.saturating_duration_since(then);
    Some(cpu_time.saturating_sub(previous_cpu_time).as_secs_f32() / wall.as_secs_f32() * 100.0)
}

// WebKit's WebContent, GPU, and Networking processes are children of launchd,
// but macOS records Glassboard as responsible for them. Memory is the physical
// footprint Activity Monitor shows, not resident size.
#[cfg(target_os = "macos")]
fn owned() -> Totals {
    extern "C" {
        fn responsibility_get_pid_responsible_for_pid(pid: libc::pid_t) -> libc::pid_t;
    }
    let own = std::process::id() as libc::pid_t;
    let estimate = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    // Room for processes started between the two calls.
    let mut pids = vec![0 as libc::pid_t; estimate.max(0) as usize + 64];
    let count = unsafe {
        libc::proc_listallpids(
            pids.as_mut_ptr().cast(),
            std::mem::size_of_val(pids.as_slice()) as libc::c_int,
        )
    };
    pids.truncate(count.max(0) as usize);
    // libc deprecates its mach bindings in favor of the mach2 crate.
    #[allow(deprecated)]
    let (numer, denom) = {
        let mut timebase = libc::mach_timebase_info { numer: 0, denom: 0 };
        unsafe { libc::mach_timebase_info(&mut timebase) };
        (u128::from(timebase.numer), u128::from(timebase.denom))
    };
    let mut totals = Totals::default();
    for pid in pids {
        if pid != own && unsafe { responsibility_get_pid_responsible_for_pid(pid) } != own {
            continue;
        }
        let mut info: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
        let buffer = (&mut info as *mut libc::rusage_info_v2).cast();
        if unsafe { libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V2, buffer) } != 0 {
            continue;
        }
        // Mach absolute time units, which are not nanoseconds on Apple Silicon.
        let ticks = u128::from(info.ri_user_time + info.ri_system_time);
        totals.cpu_time += Duration::from_nanos((ticks * numer / denom) as u64);
        totals.memory += info.ri_phys_footprint;
    }
    totals
}

// WebView2 and WebKitGTK processes descend from Glassboard's own process.
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn owned() -> Totals {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        // Linux threads would otherwise be listed as processes sharing their parent's memory.
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_cpu()
            .without_tasks(),
    );
    let own = Pid::from_u32(std::process::id());
    let processes = system.processes();
    processes
        .iter()
        .filter(|(&pid, _)| {
            // Windows keeps an exited parent's pid, which can be reused, so ancestry can cycle.
            std::iter::successors(Some(pid), |pid| processes.get(pid)?.parent())
                .take(processes.len())
                .any(|pid| pid == own)
        })
        .fold(Totals::default(), |totals, (_, process)| Totals {
            cpu_time: totals.cpu_time + Duration::from_millis(process.accumulated_cpu_time()),
            memory: totals.memory + process.memory(),
        })
}

#[cfg(test)]
mod tests {
    use super::cpu_percent;
    use std::time::{Duration, Instant};

    #[test]
    fn cpu_percent_is_of_one_core_between_samples() {
        let start = Instant::now();
        let at = |seconds| start + Duration::from_secs(seconds);
        let cpu = Duration::from_millis;
        assert_eq!(cpu_percent(None, (start, cpu(500))), None);
        assert_eq!(
            cpu_percent(Some((start, cpu(500))), (at(2), cpu(510))),
            Some(0.5)
        );
        assert_eq!(
            cpu_percent(Some((start, cpu(0))), (at(2), cpu(3000))),
            Some(150.0)
        );
        assert_eq!(
            cpu_percent(Some((start, cpu(900))), (at(2), cpu(400))),
            Some(0.0)
        );
    }
}
