use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWindowInfo {
    pub process_id: u32,
    pub process_name: String,
    pub window_title: String,
    pub executable_path: Option<String>,
}

pub struct WindowMonitor;

impl WindowMonitor {
    pub fn get_foreground_window() -> Option<ActiveWindowInfo> {
        #[cfg(windows)]
        {
            Self::get_foreground_window_windows()
        }

        #[cfg(target_os = "linux")]
        {
            Self::get_foreground_window_linux()
        }

        #[cfg(target_os = "macos")]
        {
            Self::get_foreground_window_macos()
        }

        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    #[cfg(windows)]
    fn get_foreground_window_windows() -> Option<ActiveWindowInfo> {
        use windows_sys::Win32::Foundation::{HWND, HANDLE};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
        };
        use windows_sys::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }

            // 1. Get window title
            let mut title_buf = [0u16; 512];
            let title_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
            let window_title = if title_len > 0 {
                String::from_utf16_lossy(&title_buf[..title_len as usize])
            } else {
                String::new()
            };

            // 2. Get Process ID
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 {
                return None;
            }

            // 3. Query executable image path
            let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            let mut exe_path = None;
            let mut process_name = format!("pid-{}", pid);

            if !handle.is_null() {
                let mut path_buf = [0u16; 1024];
                let mut size = path_buf.len() as u32;
                if QueryFullProcessImageNameW(handle, 0, path_buf.as_mut_ptr(), &mut size) != 0 {
                    let path = String::from_utf16_lossy(&path_buf[..size as usize]);
                    if let Some(filename) = std::path::Path::new(&path).file_name() {
                        process_name = filename.to_string_lossy().to_string();
                    }
                    exe_path = Some(path);
                }
                windows_sys::Win32::Foundation::CloseHandle(handle);
            }

            Some(ActiveWindowInfo {
                process_id: pid,
                process_name,
                window_title,
                executable_path: exe_path,
            })
        }
    }

    #[cfg(target_os = "linux")]
    fn get_foreground_window_linux() -> Option<ActiveWindowInfo> {
        // Query active window via xdotool or /proc if available
        let output = std::process::Command::new("xdotool")
            .args(["getwindowfocus", "getwindowpid"])
            .output()
            .ok()?;

        if output.status.success() {
            let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Ok(pid) = pid_str.parse::<u32>() {
                let comm_path = format!("/proc/{}/comm", pid);
                let process_name = std::fs::read_to_string(comm_path)
                    .unwrap_or_else(|_| "unknown".to_string())
                    .trim()
                    .to_string();

                let title_output = std::process::Command::new("xdotool")
                    .args(["getwindowfocus", "getwindowname"])
                    .output()
                    .ok();

                let window_title = title_output
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();

                return Some(ActiveWindowInfo {
                    process_id: pid,
                    process_name,
                    window_title,
                    executable_path: None,
                });
            }
        }
        None
    }

    #[cfg(target_os = "macos")]
    fn get_foreground_window_macos() -> Option<ActiveWindowInfo> {
        // Query active app via osascript
        let script = r#"
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set appName to name of frontApp
                set appPid to unix id of frontApp
                return appName & "|||" & appPid
            end tell
        "#;
        let output = std::process::Command::new("osascript")
            .args(["-e", script])
            .output()
            .ok()?;

        if output.status.success() {
            let str_out = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = str_out.trim().split("|||").collect();
            if parts.len() == 2 {
                let process_name = parts[0].to_string();
                let pid = parts[1].trim().parse::<u32>().unwrap_or(0);
                return Some(ActiveWindowInfo {
                    process_id: pid,
                    process_name,
                    window_title: String::new(),
                    executable_path: None,
                });
            }
        }
        None
    }
}
