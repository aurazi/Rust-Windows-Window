// https://docs.microsoft.com/en-us/windows/win32/learnwin32/your-first-windows-program
// the way its designed is finikiy.... i don't really like it
// maybe i'll figure something else out

#![allow(non_snake_case)]

use ::std::mem;
use ::std::sync::atomic::AtomicI64;
use ::std::sync::atomic::Ordering;
use ::std::sync::Arc;
use ::std::thread;
use crossbeam::channel::{Receiver, Sender};
use parking_lot::RwLock;
use windows::{
    core::{HSTRING, PWSTR},
    w,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{Environment::GetCommandLineW, LibraryLoader::GetModuleHandleW, Performance::*},
        UI::WindowsAndMessaging::*,
    },
};

const FPS: usize = 30;

enum Message {
    Render(HWND),
    QuitRender(Sender<()>),
}
struct AppState(Sender<Message>, RwLock<SharedApp>);
struct SharedApp {
    dt: f64,
    elapsed: f64,
}
struct HighResolutionTimer {
    t_beginning_of_time: i64,
    t_before: AtomicI64,
    t_after: AtomicI64,
    c_frequency: f64,
} // https://docs.microsoft.com/en-us/windows/win32/winmsg/about-timers
impl HighResolutionTimer {
    fn new() -> Self {
        let mut freq = 0;
        if !unsafe { QueryPerformanceFrequency(&mut freq).as_bool() } {
            panic!("cry you have no support for HRT");
        }
        let mut t_beginning_of_time = 0;
        unsafe {
            QueryPerformanceCounter(&mut t_beginning_of_time);
        }
        Self {
            t_beginning_of_time: t_beginning_of_time,
            t_before: AtomicI64::new(t_beginning_of_time),
            t_after: AtomicI64::new(t_beginning_of_time),
            c_frequency: freq as f64,
        }
    }
    fn set_start(&mut self) {
        unsafe {
            let mut f = 0;
            QueryPerformanceCounter(&mut f);
            self.t_before.store(f, Ordering::Release);
        }
    }
    fn set_end(&mut self) {
        unsafe {
            let mut f = 0;
            QueryPerformanceCounter(&mut f);
            self.t_after.store(f, Ordering::Release);
        }
    }
    fn get_delta(&self) -> f64 {
        (self.t_after.load(Ordering::Acquire) as f64 - self.t_before.load(Ordering::Acquire) as f64)
            / self.c_frequency
    }
    fn get_elapsed(&self) -> f64 {
        (self.t_after.load(Ordering::Acquire) as f64 - self.t_beginning_of_time as f64)
            / self.c_frequency
    }
}

fn BeginListening(recv: Receiver<Message>, appstate: Arc<AppState>) {
    thread::spawn(move || {
        let mut screen_buffer: Vec<(u8, u8, u8)> = vec![];

        loop {
            let received_message = recv.recv();
            match received_message {
                Ok(message) => match message {
                    Message::Render(hwnd) => unsafe {
                        let rl = appstate.1.read();
                        let dt = rl.dt;
                        let elapsed = rl.elapsed;
                        mem::drop(rl);

                        let mut window_rect = RECT::default();
                        GetWindowRect(hwnd, &mut window_rect);
                        let (window_width, window_height) = (
                            window_rect.right - window_rect.left,
                            window_rect.bottom - window_rect.top,
                        );
                        let (middle_w, middle_h) = (window_width / 2, window_height / 2);
                        let buffer_size = window_width * window_height;
                        if buffer_size as usize > screen_buffer.capacity() {
                            let cost = buffer_size as usize - (screen_buffer.capacity());
                            screen_buffer.reserve_exact(cost);
                            screen_buffer.resize(buffer_size as usize, (0, 0, 0));
                        }
                        // do not iterate over entire buffer or you'll be editing parts that
                        // are not being rendered.

                        let breathing = (0xFF as f64 * (f64::sin(elapsed) * 0.5 + 0.5)) as u8;
                        for index in 0i32..buffer_size {
                            let (px, py) = (index % window_width, index / window_width);
                            let distance_to_middle =
                                f64::sqrt(((px - middle_w) ^ 2 + (py - middle_h) ^ 2) as f64);
                            let (b, g, r) = &mut screen_buffer[index as usize];
                            // danger of overflowing oh well!
                            // however, it creates some sweet patterns!
                            let breathing = (breathing as f64 * distance_to_middle) as u8;
                            *r = breathing;
                            *g = breathing;
                            *b = breathing;
                        }

                        let mut bitmapi = BITMAPINFO::default();
                        bitmapi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
                        bitmapi.bmiHeader.biBitCount = 24;
                        bitmapi.bmiHeader.biCompression = BI_RGB as u32;
                        bitmapi.bmiHeader.biPlanes = 1; // has to be 1 idk why
                        bitmapi.bmiHeader.biWidth = window_width;
                        bitmapi.bmiHeader.biHeight = window_height;

                        // BeginPaint and EndPaint is weird..
                        // I get flickers kek~
                        // anyway, just don't use this
                        // for WM_PAINT and instead fallback
                        // to DefWindowProcW

                        let hdc = GetDC(hwnd);
                        SetStretchBltMode(hdc, COLORONCOLOR);
                        StretchDIBits(
                            hdc,
                            0,
                            0,
                            window_width,
                            window_height,
                            0,
                            0,
                            window_width,
                            window_height,
                            screen_buffer.as_ptr() as *const _,
                            &bitmapi,
                            DIB_RGB_COLORS,
                            SRCCOPY,
                        );
                        ReleaseDC(hwnd, hdc);
                    },
                    Message::QuitRender(sender) => {
                        println!("goodbye world");
                        let _ = sender.send(());
                        break;
                    }
                },
                Err(err) => println!("{err}"),
            }
        }
    });
}

fn BeginEngine(sender: Sender<Message>, hwnd: HWND, appstate: Arc<AppState>) {
    let mut timer = HighResolutionTimer::new();
    let fps_r = 1.0 / FPS as f64;
    thread::spawn(move || loop {
        timer.set_end();
        let delta = timer.get_delta();
        let elapsed = timer.get_elapsed();
        if delta >= fps_r {
            let _ = sender.try_send(Message::Render(hwnd));
            timer.set_start();
        }
        let mut wl = appstate.1.write();
        wl.dt = delta;
        wl.elapsed = elapsed;
    });
}

fn wWinMain(
    hInstance: HINSTANCE,
    _hPrevInstance: HINSTANCE,
    _pCmdLine: PWSTR,
    nCmdShow: i32,
) -> i32 {
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

    let (sender, receiver) = crossbeam::channel::bounded(FPS);
    let appstate = Arc::new(AppState(
        sender.clone(),
        RwLock::new(SharedApp {
            dt: 0.0,
            elapsed: 0.0,
        }),
    ));
    let appstate_message_r = appstate.clone();

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
            &appstate_message_r as *const Arc<AppState> as *const _,
        )
    };
    if hwnd.0 == 0 {
        dbg!("Failed to create window instance");
        return 1;
    }

    BeginListening(receiver, appstate.clone());
    BeginEngine(sender, hwnd, appstate.clone());

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
    let appstate: &Arc<AppState> = unsafe {
        if message == WM_CREATE {
            let pCreate = lparam.0 as *const CREATESTRUCTW;
            let pState = (*pCreate).lpCreateParams as *const Arc<AppState>;
            SetWindowLongPtrW(window, GWLP_USERDATA, pState as isize);
            &*pState
        } else {
            &*(GetWindowLongPtrW(window, GWLP_USERDATA) as *const Arc<AppState>)
        }
    };

    unsafe {
        match message as u32 {
            WM_EXITSIZEMOVE | WM_SIZING | WM_SHOWWINDOW => {
                let _ = appstate.0.try_send(Message::Render(window));
                LRESULT(0)
            }
            WM_DESTROY => {
                // haha im so cheeky
                let (sender, reciever) = crossbeam::channel::bounded::<()>(1);
                let _ = appstate.0.send(Message::QuitRender(sender));
                let _ = reciever.recv();
                PostQuitMessage(0); // Push special message to queue, value 0, tells GetMessageW to stop in the next iteration
                LRESULT(0)
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}
