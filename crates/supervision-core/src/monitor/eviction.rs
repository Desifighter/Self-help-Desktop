use tracing::{info, warn};

pub struct ProcessEviction;

impl ProcessEviction {
    pub fn evict(pid: u32, process_name: &str) -> bool {
        info!("Evicting restricted process: {} (PID: {})", process_name, pid);

        #[cfg(windows)]
        {
            Self::evict_windows(pid)
        }

        #[cfg(not(windows))]
        {
            Self::evict_unix(pid)
        }
    }

    #[cfg(windows)]
    fn evict_windows(pid: u32) -> bool {
        use windows_sys::Win32::Foundation::{HANDLE, HWND, BOOL, LPARAM};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, TerminateProcess, PROCESS_TERMINATE,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowThreadProcessId, PostMessageW, ShowWindow,
            WM_CLOSE, SW_MINIMIZE,
        };

        unsafe {
            // Stage 1 & 2: Minimize and post WM_CLOSE to all windows belonging to this PID
            struct EnumData {
                target_pid: u32,
            }

            unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let data = &*(lparam as *const EnumData);
                let mut win_pid: u32 = 0;
                GetWindowThreadProcessId(hwnd, &mut win_pid);
                if win_pid == data.target_pid {
                    // Minimize window
                    ShowWindow(hwnd, SW_MINIMIZE);
                    // Post WM_CLOSE message
                    PostMessageW(hwnd, WM_CLOSE, 0, 0);
                }
                1 // Continue enumeration
            }

            let data = EnumData { target_pid: pid };
            EnumWindows(Some(enum_window_callback), &data as *const _ as isize);

            // Give the app 200ms to exit cleanly, then forcefully terminate
            std::thread::sleep(std::time::Duration::from_millis(200));

            // Stage 3: TerminateProcess
            let handle: HANDLE = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if !handle.is_null() {
                let result = TerminateProcess(handle, 1);
                windows_sys::Win32::Foundation::CloseHandle(handle);
                if result != 0 {
                    info!("Successfully terminated restricted process PID {}", pid);
                    return true;
                }
            }
        }

        false
    }

    #[cfg(not(windows))]
    fn evict_unix(pid: u32) -> bool {
        // First try SIGTERM
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }

        std::thread::sleep(std::time::Duration::from_millis(200));

        // Then forceful SIGKILL
        let res = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        if res == 0 {
            info!("Successfully terminated restricted process PID {}", pid);
            return true;
        }

        false
    }
}
