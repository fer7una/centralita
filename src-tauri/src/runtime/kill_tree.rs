use std::{
    io,
    process::{Child, ExitStatus},
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

const FORCED_WAIT: Duration = Duration::from_secs(2);
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub fn stop_process_tree(child_handle: &Arc<Mutex<Child>>) -> io::Result<Option<ExitStatus>> {
    if let Some(status) = try_wait_child(child_handle)? {
        return Ok(Some(status));
    }

    let pid = lock_mutex(child_handle).id();

    #[cfg(windows)]
    let tree_kill_result = run_taskkill(pid);

    #[cfg(not(windows))]
    let tree_kill_result = kill_child_root(child_handle);

    if let Some(status) = wait_for_exit(child_handle, FORCED_WAIT)? {
        return Ok(Some(status));
    }

    let root_kill_result = kill_child_root(child_handle);
    if let Some(status) = wait_for_exit(child_handle, FORCED_WAIT)? {
        return Ok(Some(status));
    }

    tree_kill_result?;
    root_kill_result?;

    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("process tree for pid {pid} did not exit after stop request"),
    ))
}

fn try_wait_child(child_handle: &Arc<Mutex<Child>>) -> io::Result<Option<ExitStatus>> {
    lock_mutex(child_handle).try_wait()
}

fn wait_for_exit(
    child_handle: &Arc<Mutex<Child>>,
    timeout: Duration,
) -> io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(status) = try_wait_child(child_handle)? {
            return Ok(Some(status));
        }

        let now = Instant::now();
        if now >= deadline {
            return Ok(None);
        }

        thread::sleep(EXIT_POLL_INTERVAL.min(deadline - now));
    }
}

fn kill_child_root(child_handle: &Arc<Mutex<Child>>) -> io::Result<()> {
    let mut child = lock_mutex(child_handle);
    if child.try_wait()?.is_none() {
        child.kill()?;
    }

    Ok(())
}

fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(windows)]
fn run_taskkill(pid: u32) -> io::Result<()> {
    let mut command = std::process::Command::new("taskkill");
    command.args(["/F", "/T", "/PID", &pid.to_string()]);
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "taskkill failed for pid {pid} with status {status}"
        )))
    }
}
