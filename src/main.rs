// https://docs.microsoft.com/en-us/windows/win32/learnwin32/your-first-windows-program

#![allow(non_snake_case)]

use ::std::mem;
use ::std::ptr::null;
use ::std::thread;
use crossbeam::channel::{Receiver, Sender};
use windows::{
    core::{HSTRING, PWSTR},
    w,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{Environment::GetCommandLineW, LibraryLoader::GetModuleHandleW},
        UI::WindowsAndMessaging::*,
    },
};

enum Message {
    Render,
    QuitRender,
}
struct AppState(Sender<Message>);

fn BeginListening(recv: Receiver<Message>) {
    thread::spawn(move || loop {
        let received_message = recv.recv();
        match received_message {
            Ok(message) => match message {
                Message::Render => {
                    println!("render request!");
                }
                Message::QuitRender => {
                    println!("goodbye world");
                    break;
                }
            },
            Err(err) => println!("{err}"),
        }
    });
}

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

    let (sender, receiver) = crossbeam::channel::bounded(30);
    let appstate = AppState(sender);

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
            &appstate as *const AppState as *const _,
        )
    };
    if hwnd.0 == 0 {
        dbg!("Failed to create window instance");
        return 1;
    }

    BeginListening(receiver);

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
    let appstate: &AppState = unsafe {
        if message == WM_CREATE {
            let pCreate = lparam.0 as *const CREATESTRUCTW;
            let pState = (*pCreate).lpCreateParams as *const AppState;
            SetWindowLongPtrW(window, GWLP_USERDATA, pState as isize);
            &*pState
        } else {
            &*(GetWindowLongPtrW(window, GWLP_USERDATA) as *const AppState)
        }
    };

    unsafe {
        match message as u32 {
            WM_PAINT => {
                let _ = appstate.0.try_send(Message::Render);
                LRESULT(0)
            }
            WM_DESTROY => {
                let _ = appstate.0.send(Message::QuitRender);
                PostQuitMessage(0); // Push special message to queue, value 0, tells GetMessageW to stop in the next iteration
                LRESULT(0)
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}
