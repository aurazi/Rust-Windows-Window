// https://docs.microsoft.com/en-us/windows/win32/learnwin32/your-first-windows-program

#![allow(non_snake_case)]

use ::std::mem;
use ::std::ptr::null;
use windows::{
    core::{HSTRING, PWSTR},
    w,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, SetBkColor,
            SetTextColor, DT_CENTER, DT_NOCLIP, PAINTSTRUCT,
        },
        System::{Environment::GetCommandLineW, LibraryLoader::GetModuleHandleW},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
            RegisterClassW, ShowWindow, TranslateMessage, CW_USEDEFAULT, MSG, SHOW_WINDOW_CMD,
            SW_SHOWDEFAULT, WM_DESTROY, WM_PAINT, WNDCLASSW, WS_EX_ACCEPTFILES,
            WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
};

fn wWinMain(hInstance: HINSTANCE, hPrevInstance: HINSTANCE, pCmdLine: PWSTR, nCmdShow: i32) -> i32 {
    const WINDOWCLASSNAME: &HSTRING = w!("Main Window Class");
    const WINDOWNAME: &HSTRING = w!("Hello World Window");

    let mut window_class = WNDCLASSW {
        hInstance: hInstance,
        lpszClassName: WINDOWCLASSNAME.into(),
        lpfnWndProc: Some(wndproc),
        ..Default::default()
    };

    unsafe {
        let regc_r = RegisterClassW(&mut window_class);
        debug_assert!(regc_r != 0);
    };

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_ACCEPTFILES,
            WINDOWCLASSNAME,
            WINDOWNAME,
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            hInstance,
            null(),
        )
    };
    if hwnd.0 == 0 {
        dbg!("Failed to create window instance");
        return 1;
    }

    unsafe {
        ShowWindow(hwnd, SHOW_WINDOW_CMD(nCmdShow as u32));

        let mut message = MSG::default();
        while GetMessageW(&mut message, HWND(0), 0, 0).into() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    0
}

fn main() {
    /*
        Not sure if there is a way to link wWinMain
    */
    let _ = unsafe {
        let prochinstance =
            GetModuleHandleW(None).expect("Failed to get module handle of current process");
        debug_assert!(prochinstance.0 != 0);

        wWinMain(
            prochinstance,
            HINSTANCE(0),
            GetCommandLineW(),
            SW_SHOWDEFAULT.0 as i32,
        );
    };
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message as u32 {
            WM_PAINT => {
                let mut ps = mem::zeroed::<PAINTSTRUCT>();
                let hdc = BeginPaint(window, &mut ps);
                let color_brush = CreateSolidBrush(0x000000FF);

                FillRect(hdc, &ps.rcPaint, color_brush);

                SetTextColor(hdc, 0x00FFFFFF);
                SetBkColor(hdc, 0x00FF00FF);
                let mut TextRect = RECT {
                    left: 10,
                    right: 100,
                    top: 50,
                    bottom: 25,
                };
                DrawTextW(
                    hdc,
                    w!("Hello World").as_wide(),
                    &mut TextRect,
                    DT_CENTER | DT_NOCLIP,
                );

                DeleteObject(color_brush);
                EndPaint(window, &ps);
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0); // Push special message to queue, value 0, tells GetMessageW to stop in the next iteration
                LRESULT(0)
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}
