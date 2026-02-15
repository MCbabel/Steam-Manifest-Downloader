use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "windows")]
use std::sync::Arc;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::services::AppState;

// ---------------------------------------------------------------------------
// Windows Job Object wrapper – ensures child process trees are killed reliably
// Uses raw FFI to avoid version-specific windows-sys feature issues.
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
pub mod win_job {
    use std::ffi::c_void;
    use std::ptr;

    type HANDLE = *mut c_void;
    type BOOL = i32;
    type DWORD = u32;

    const PROCESS_SET_QUOTA: DWORD = 0x0100;
    const PROCESS_TERMINATE: DWORD = 0x0001;
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x2000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: DWORD = 9;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct IO_COUNTERS {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: DWORD,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: DWORD,
        affinity: usize,
        priority_class: DWORD,
        scheduling_class: DWORD,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION_STRUCT {
        basic_limit_information: JOBOBJECT_BASIC_LIMIT_INFORMATION,
        io_info: IO_COUNTERS,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    extern "system" {
        fn CreateJobObjectW(
            lp_job_attributes: *const c_void,
            lp_name: *const u16,
        ) -> HANDLE;
        fn SetInformationJobObject(
            h_job: HANDLE,
            job_object_information_class: DWORD,
            lp_job_object_information: *const c_void,
            cb_job_object_information_length: DWORD,
        ) -> BOOL;
        fn AssignProcessToJobObject(h_job: HANDLE, h_process: HANDLE) -> BOOL;
        fn TerminateJobObject(h_job: HANDLE, u_exit_code: u32) -> BOOL;
        fn OpenProcess(dw_desired_access: DWORD, b_inherit_handle: BOOL, dw_process_id: DWORD) -> HANDLE;
        fn CloseHandle(h_object: HANDLE) -> BOOL;
    }

    pub struct JobObject {
        handle: HANDLE,
    }

    impl JobObject {
        pub fn new() -> Option<Self> {
            unsafe {
                let handle = CreateJobObjectW(ptr::null(), ptr::null());
                if handle.is_null() {
                    return None;
                }

                // Configure job to kill all processes when the job handle is closed
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION_STRUCT = std::mem::zeroed();
                info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

                let result = SetInformationJobObject(
                    handle,
                    JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                    &info as *const _ as *const c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION_STRUCT>() as DWORD,
                );

                if result == 0 {
                    CloseHandle(handle);
                    return None;
                }

                Some(JobObject { handle })
            }
        }

        pub fn assign_process(&self, pid: u32) -> bool {
            unsafe {
                let process_handle = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
                if process_handle.is_null() {
                    return false;
                }
                let result = AssignProcessToJobObject(self.handle, process_handle);
                CloseHandle(process_handle);
                result != 0
            }
        }

        pub fn terminate(&self) {
            unsafe {
                TerminateJobObject(self.handle, 1);
            }
        }
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    // SAFETY: The HANDLE is only used behind Arc and through &self methods
    unsafe impl Send for JobObject {}
    unsafe impl Sync for JobObject {}
}

/// Progress event payload emitted to the frontend via "download-progress" event.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProgressEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "jobId")]
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "depotId")]
    pub depot_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "depotCount")]
    pub depot_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "appId")]
    pub app_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "lastUpdated")]
    pub last_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "freeGB")]
    pub free_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "manifestId")]
    pub manifest_id: Option<String>,
}

impl ProgressEvent {
    pub fn new(event_type: &str, job_id: &str) -> Self {
        Self {
            event_type: event_type.to_string(),
            job_id: job_id.to_string(),
            step: None,
            depot_id: None,
            current: None,
            total: None,
            output: None,
            message: None,
            stream: None,
            command: None,
            results: None,
            depot_count: None,
            app_id: None,
            last_updated: None,
            free_gb: None,
            drive: None,
            filename: None,
            manifest_id: None,
        }
    }
}

/// Emit a progress event to the frontend.
pub fn emit_progress(app: &AppHandle, event: &ProgressEvent) {
    if let Err(e) = app.emit("download-progress", event) {
        eprintln!("[DepotRunner] Failed to emit progress event: {}", e);
    }
}

/// Depot configuration for running DepotDownloaderMod.
#[derive(Debug, Clone)]
pub struct DepotRunConfig {
    pub depot_id: String,
    pub manifest_id: String,
}

/// Platform-specific executable name for display purposes.
#[cfg(target_os = "windows")]
const DDM_DISPLAY_NAME: &str = "DepotDownloaderMod.exe";
#[cfg(target_os = "linux")]
const DDM_DISPLAY_NAME: &str = "DepotDownloaderMod";

/// Run DepotDownloaderMod for a single depot. Streams stdout/stderr to frontend.
///
/// Returns Ok(true) if the process exited with code 0, Ok(false) if non-zero.
pub async fn run_depot_downloader(
    app: &AppHandle,
    exe_path: &Path,
    app_id: &str,
    depot: &DepotRunConfig,
    work_dir: &Path,
    extra_args: &[String],
    job_id: &str,
    state: &AppState,
) -> Result<bool, String> {
    let manifest_file = format!("{}_{}.manifest", depot.depot_id, depot.manifest_id);
    let keys_file = "steam.keys";

    let mut args = vec![
        "-app".to_string(),
        app_id.to_string(),
        "-depot".to_string(),
        depot.depot_id.clone(),
        "-manifest".to_string(),
        depot.manifest_id.clone(),
        "-depotkeys".to_string(),
        keys_file.to_string(),
        "-manifestfile".to_string(),
        manifest_file,
    ];
    args.extend_from_slice(extra_args);

    let command_display = format!(
        "{} {}",
        DDM_DISPLAY_NAME,
        args.join(" ")
    );

    // Emit running status
    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("running_downloader".to_string());
    event.depot_id = Some(depot.depot_id.clone());
    event.command = Some(command_display);
    emit_progress(app, &event);

    // Create Windows Job Object before spawning
    #[cfg(target_os = "windows")]
    let job_object = win_job::JobObject::new().map(Arc::new);

    // Spawn the process
    // On Windows: run via exe directly (dotnet-dependent app with .exe entry point)
    // On Linux: run the self-contained binary directly
    let mut cmd = Command::new(exe_path);
    cmd.args(&args)
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    // CREATE_NO_WINDOW on Windows
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    // Create new process group on Linux for reliable cleanup
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to start DepotDownloaderMod for depot {}: {}", depot.depot_id, e))?;

    // Track the PID and assign to Job Object
    if let Some(pid) = child.id() {
        #[cfg(target_os = "windows")]
        if let Some(ref jo) = job_object {
            jo.assign_process(pid);
        }

        let mut jobs = state.active_jobs.lock().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.child_pid = Some(pid);
            #[cfg(target_os = "windows")]
            {
                job.job_object = job_object.clone();
            }
        }
    }

    // Stream stdout with throttling
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_stdout = app.clone();
    let job_id_stdout = job_id.to_string();
    let depot_id_stdout = depot.depot_id.clone();

    let stdout_handle = tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut last_emit = tokio::time::Instant::now();
            let mut buffer: Vec<String> = Vec::new();
            let throttle_interval = tokio::time::Duration::from_millis(150);

            while let Ok(Some(line)) = lines.next_line().await {
                buffer.push(line);

                let now = tokio::time::Instant::now();
                if now.duration_since(last_emit) >= throttle_interval || buffer.len() >= 50 {
                    let combined = buffer.join("\n");
                    let mut event = ProgressEvent::new("output", &job_id_stdout);
                    event.depot_id = Some(depot_id_stdout.clone());
                    event.stream = Some("stdout".to_string());
                    event.output = Some(combined);
                    emit_progress(&app_stdout, &event);
                    buffer.clear();
                    last_emit = now;
                }
            }

            // Emit remaining buffered lines
            if !buffer.is_empty() {
                let combined = buffer.join("\n");
                let mut event = ProgressEvent::new("output", &job_id_stdout);
                event.depot_id = Some(depot_id_stdout.clone());
                event.stream = Some("stdout".to_string());
                event.output = Some(combined);
                emit_progress(&app_stdout, &event);
            }
        }
    });

    let app_stderr = app.clone();
    let job_id_stderr = job_id.to_string();
    let depot_id_stderr = depot.depot_id.clone();

    let stderr_handle = tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut last_emit = tokio::time::Instant::now();
            let mut buffer: Vec<String> = Vec::new();
            let throttle_interval = tokio::time::Duration::from_millis(150);

            while let Ok(Some(line)) = lines.next_line().await {
                buffer.push(line);

                let now = tokio::time::Instant::now();
                if now.duration_since(last_emit) >= throttle_interval || buffer.len() >= 50 {
                    let combined = buffer.join("\n");
                    let mut event = ProgressEvent::new("output", &job_id_stderr);
                    event.depot_id = Some(depot_id_stderr.clone());
                    event.stream = Some("stderr".to_string());
                    event.output = Some(combined);
                    emit_progress(&app_stderr, &event);
                    buffer.clear();
                    last_emit = now;
                }
            }

            // Emit remaining buffered lines
            if !buffer.is_empty() {
                let combined = buffer.join("\n");
                let mut event = ProgressEvent::new("output", &job_id_stderr);
                event.depot_id = Some(depot_id_stderr.clone());
                event.stream = Some("stderr".to_string());
                event.output = Some(combined);
                emit_progress(&app_stderr, &event);
            }
        }
    });

    // Wait for process to complete
    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for DepotDownloaderMod: {}", e))?;

    // Wait for stream readers to finish
    let _ = stdout_handle.await;
    let _ = stderr_handle.await;

    // Clear the PID and job object
    {
        let mut jobs = state.active_jobs.lock().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.child_pid = None;
            #[cfg(target_os = "windows")]
            {
                job.job_object = None;
            }
        }
    }

    Ok(status.success())
}

/// Run DepotDownloaderMod for all depots sequentially.
/// Checks for cancellation between each depot.
pub async fn run_all_depots(
    app: &AppHandle,
    exe_path: &Path,
    app_id: &str,
    depots: &[DepotRunConfig],
    work_dir: &Path,
    extra_args: &[String],
    job_id: &str,
    state: &AppState,
) -> Result<Vec<serde_json::Value>, String> {
    let mut results = Vec::new();
    let total = depots.len();

    for (i, depot) in depots.iter().enumerate() {
        // Check for cancellation
        {
            let jobs = state.active_jobs.lock().await;
            if let Some(job) = jobs.get(job_id) {
                if job.status == "cancelled" {
                    let mut event = ProgressEvent::new("cancelled", job_id);
                    event.message = Some("Download cancelled by user.".to_string());
                    emit_progress(app, &event);
                    break;
                }
            }
        }

        // Emit progress
        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("running_downloader".to_string());
        event.depot_id = Some(depot.depot_id.clone());
        event.current = Some(i + 1);
        event.total = Some(total);
        emit_progress(app, &event);

        match run_depot_downloader(app, exe_path, app_id, depot, work_dir, extra_args, job_id, state).await {
            Ok(success) => {
                results.push(serde_json::json!({
                    "depotId": depot.depot_id,
                    "success": success,
                    "error": if success { serde_json::Value::Null } else {
                        serde_json::Value::String(format!("DepotDownloader exited with non-zero code for depot {}", depot.depot_id))
                    }
                }));

                let mut event = ProgressEvent::new("depot_complete", job_id);
                event.depot_id = Some(depot.depot_id.clone());
                event.current = Some(i + 1);
                event.total = Some(total);
                emit_progress(app, &event);
            }
            Err(e) => {
                // Check if cancelled
                {
                    let jobs = state.active_jobs.lock().await;
                    if let Some(job) = jobs.get(job_id) {
                        if job.status == "cancelled" {
                            let mut event = ProgressEvent::new("cancelled", job_id);
                            event.message = Some("Download cancelled by user.".to_string());
                            emit_progress(app, &event);
                            break;
                        }
                    }
                }

                results.push(serde_json::json!({
                    "depotId": depot.depot_id,
                    "success": false,
                    "error": e
                }));

                let mut event = ProgressEvent::new("error", job_id);
                event.depot_id = Some(depot.depot_id.clone());
                event.message = Some(e);
                emit_progress(app, &event);
            }
        }
    }

    Ok(results)
}

/// Kill the active process for a job.
/// On Windows: terminates via Job Object, then falls back to taskkill.
/// On Linux: kills the entire process group via SIGKILL.
pub async fn kill_job(state: &AppState, job_id: &str) -> bool {
    let mut pid = None;
    #[cfg(target_os = "windows")]
    let mut job_object_opt: Option<Arc<win_job::JobObject>> = None;

    {
        let mut jobs = state.active_jobs.lock().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = "cancelled".to_string();
            pid = job.child_pid.take();
            #[cfg(target_os = "windows")]
            {
                job_object_opt = job.job_object.take();
            }
        }
    }

    let mut killed = false;

    // --- Windows kill path ---
    #[cfg(target_os = "windows")]
    {
        // Step 1: Terminate via Job Object (kills all child processes)
        if let Some(jo) = job_object_opt {
            jo.terminate();
            killed = true;
        }

        // Step 2: Kill by PID as fallback
        if !killed {
            if let Some(pid) = pid {
                let mut cmd = std::process::Command::new("taskkill");
                cmd.args(["/pid", &pid.to_string(), "/f", "/t"]);
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                match cmd.output() {
                    Ok(output) => {
                        killed = output.status.success();
                    }
                    Err(_) => {}
                }
            }
        }

        // Step 3: Fallback - kill by process name
        if !killed {
            let mut cmd = std::process::Command::new("taskkill");
            cmd.args(["/im", "DepotDownloaderMod.exe", "/f", "/t"]);
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            match cmd.output() {
                Ok(output) => {
                    killed = output.status.success();
                }
                Err(_) => {}
            }
        }
    }

    // --- Linux kill path ---
    #[cfg(target_os = "linux")]
    {
        if let Some(child_pid) = pid {
            // Kill entire process group (we used process_group(0) on spawn)
            unsafe {
                let result = libc::kill(-(child_pid as i32), libc::SIGKILL);
                killed = result == 0;
            }

            // Fallback: kill by PID directly
            if !killed {
                unsafe {
                    let result = libc::kill(child_pid as i32, libc::SIGKILL);
                    killed = result == 0;
                }
            }
        }

        // Fallback: kill by process name
        if !killed {
            match std::process::Command::new("killall")
                .args(["-9", "DepotDownloaderMod"])
                .output()
            {
                Ok(output) => {
                    killed = output.status.success();
                }
                Err(_) => {}
            }
        }
    }

    killed
}

/// Get the path to the DepotDownloaderMod executable.
/// First tries embedded extraction, then falls back to external paths.
pub async fn get_exe_path_async() -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "windows")]
    const EXE_NAME: &str = "DepotDownloaderMod.exe";
    #[cfg(target_os = "linux")]
    const EXE_NAME: &str = "DepotDownloaderMod";

    // Try embedded extraction first (works for both installer and portable)
    match crate::services::embedded_tools::ensure_extracted().await {
        Ok(path) => {
            eprintln!("[DepotRunner] Using embedded DepotDownloaderMod: {:?}", path);
            return Ok(path);
        }
        Err(e) => {
            eprintln!("[DepotRunner] Embedded extraction failed: {}, trying external paths...", e);
        }
    }

    // Fallback: look next to the executable
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(parent) = exe_dir.parent() {
            let exe_path = parent.join("DepotDownloaderMod").join(EXE_NAME);
            if exe_path.exists() {
                return Ok(exe_path);
            }
        }
    }

    // Fallback: current working directory
    let local_path = std::path::PathBuf::from("DepotDownloaderMod").join(EXE_NAME);
    if local_path.exists() {
        return Ok(local_path);
    }

    Err(format!("{} not found.", EXE_NAME))
}
