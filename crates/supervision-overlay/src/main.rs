#[cfg(windows)]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
    ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, MSG,
    SW_SHOW, WNDCLASSW, WS_EX_TOPMOST, WS_POPUP,
};

fn main() {
    println!("Supervision Overlay: Displaying restriction warning...");

    #[cfg(windows)]
    {
        run_windows_overlay();
    }

    #[cfg(not(windows))]
    {
        println!("\n==============================================");
        println!("       SUPERVISION ACCESS RESTRICTED          ");
        println!("  This application or website is restricted.  ");
        println!("       Return to your productive work.        ");
        println!("==============================================\n");
    }
}

#[cfg(windows)]
fn run_windows_overlay() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    unsafe {
        let class_name: Vec<u16> = OsStr::new("SupervisionOverlayWindow")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe extern "system" fn wnd_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                WM_DESTROY, WM_PAINT, PostQuitMessage, BeginPaint, EndPaint, PAINTSTRUCT,
            };
            use windows_sys::Win32::Graphics::Gdi::{
                CreateSolidBrush, FillRect, SetBkMode, SetTextColor, DrawTextW, RECT,
                DT_CENTER, DT_VCENTER, DT_SINGLELINE, TRANSPARENT,
            };

            match msg {
                WM_PAINT => {
                    let mut ps: PAINTSTRUCT = std::mem::zeroed();
                    let hdc = BeginPaint(hwnd, &mut ps);
                    let bg_brush = CreateSolidBrush(0x001A1A1A); // Dark charcoal background
                    FillRect(hdc, &ps.rcPaint, bg_brush);

                    SetBkMode(hdc, TRANSPARENT as i32);
                    SetTextColor(hdc, 0x004545EE); // Vibrant red text

                    let msg_str: Vec<u16> = "ACCESS RESTRICTED\n\nSupervision Digital Self-Control Active\nReturning you to productive focus..."
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();

                    let mut rect: RECT = ps.rcPaint;
                    DrawTextW(hdc, msg_str.as_ptr(), -1, &mut rect, DT_CENTER | DT_VCENTER);

                    EndPaint(hwnd, &ps);
                    0
                }
                WM_DESTROY => {
                    PostQuitMessage(0);
                    0
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: 0,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            800,
            500,
            0,
            0,
            0,
            std::ptr::null(),
        );

        if hwnd != 0 {
            ShowWindow(hwnd, SW_SHOW);

            // Auto dismiss after 2 seconds
            let hwnd_copy = hwnd;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(2000));
                windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                    hwnd_copy,
                    windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                    0,
                    0,
                );
            });

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
