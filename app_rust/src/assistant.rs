use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_CONTROL, VK_HOME, VK_SHIFT, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PostMessageW, HHOOK, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN,
    WM_SYSKEYDOWN,
};

pub const WM_APPLY_DOUBLE_SPACE: u32 = WM_APP + 2;
pub const MAGIC_EXTRA_INFO: usize = 0x5055_4E43;

#[derive(Clone, Copy)]
pub struct SafeHwnd(pub HWND);
unsafe impl Send for SafeHwnd {}
unsafe impl Sync for SafeHwnd {}

#[derive(Clone, Copy)]
pub struct SafeHhook(pub HHOOK);
unsafe impl Send for SafeHhook {}
unsafe impl Sync for SafeHhook {}

static MAIN_HWND: Mutex<Option<SafeHwnd>> = Mutex::new(None);
static KEYBOARD_HOOK: Mutex<Option<SafeHhook>> = Mutex::new(None);
static LAST_SPACE_TIME: Mutex<Option<Instant>> = Mutex::new(None);
pub static DOUBLE_SPACE_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn has_cyrillic(text: &str) -> bool {
    text.chars().any(|c| matches!(c, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё'))
}

pub fn is_active_layout_cyrillic() -> bool {
    unsafe {
        let fg = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
        let thread_id = if fg.0.is_null() {
            0
        } else {
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(fg, None)
        };
        let hkl = windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout(thread_id);
        let lang_id = (hkl.0 as usize & 0xFFFF) as u16;
        // 0x0419: Russian, 0x0422: Ukrainian, 0x0423: Belarusian
        matches!(lang_id, 0x0419 | 0x0422 | 0x0423)
    }
}

pub unsafe fn init_assistant(main_hwnd: HWND, h_instance: HINSTANCE) {
    *MAIN_HWND.lock().unwrap() = Some(SafeHwnd(main_hwnd));

    let hook = windows::Win32::UI::WindowsAndMessaging::SetWindowsHookExW(
        WH_KEYBOARD_LL,
        Some(low_level_keyboard_proc),
        h_instance,
        0,
    );
    if let Ok(h) = hook {
        *KEYBOARD_HOOK.lock().unwrap() = Some(SafeHhook(h));
    }
}

pub unsafe fn cleanup_assistant() {
    if let Some(h) = KEYBOARD_HOOK.lock().unwrap().take() {
        let _ = windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(h.0);
    }
}

pub fn send_key_press(vk: VIRTUAL_KEY) {
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: MAGIC_EXTRA_INFO,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: MAGIC_EXTRA_INFO,
                },
            },
        },
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn select_line_to_home() {
    let inputs = [
        // Press Ctrl
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    dwExtraInfo: MAGIC_EXTRA_INFO,
                    ..Default::default()
                },
            },
        },
        // Press Shift
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_SHIFT,
                    dwExtraInfo: MAGIC_EXTRA_INFO,
                    ..Default::default()
                },
            },
        },
        // Press Home
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_HOME,
                    dwExtraInfo: MAGIC_EXTRA_INFO,
                    ..Default::default()
                },
            },
        },
        // Release Home
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_HOME,
                    dwFlags: KEYEVENTF_KEYUP,
                    dwExtraInfo: MAGIC_EXTRA_INFO,
                    ..Default::default()
                },
            },
        },
        // Release Shift
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_SHIFT,
                    dwFlags: KEYEVENTF_KEYUP,
                    dwExtraInfo: MAGIC_EXTRA_INFO,
                    ..Default::default()
                },
            },
        },
        // Release Ctrl
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    dwFlags: KEYEVENTF_KEYUP,
                    dwExtraInfo: MAGIC_EXTRA_INFO,
                    ..Default::default()
                },
            },
        },
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if n_code >= 0 && (wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN) {
        let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);

        // 1. Completely ignore injected keystrokes (prevents recursive loops)
        if (kbd.flags.0 & 0x10) != 0 || kbd.dwExtraInfo == MAGIC_EXTRA_INFO {
            return CallNextHookEx(None, n_code, wparam, lparam);
        }

        let vk = VIRTUAL_KEY(kbd.vkCode as u16);

        if vk == VK_SPACE {
            if !DOUBLE_SPACE_ENABLED.load(Ordering::Relaxed) {
                return CallNextHookEx(None, n_code, wparam, lparam);
            }

            // 2. Only intercept space if active layout is Cyrillic!
            if !is_active_layout_cyrillic() {
                *LAST_SPACE_TIME.lock().unwrap() = None;
                return CallNextHookEx(None, n_code, wparam, lparam);
            }

            let mut last_space = LAST_SPACE_TIME.lock().unwrap();
            let now = Instant::now();

            if let Some(prev_time) = *last_space {
                if now.duration_since(prev_time) < Duration::from_millis(450) {
                    // Double space detected!
                    *last_space = None;
                    drop(last_space);

                    // INTERCEPT second space
                    if let Some(main_hwnd) = *MAIN_HWND.lock().unwrap() {
                        let _ = PostMessageW(main_hwnd.0, WM_APPLY_DOUBLE_SPACE, WPARAM(0), LPARAM(0));
                    }
                    return LRESULT(1);
                }
            }

            *last_space = Some(now);
            return CallNextHookEx(None, n_code, wparam, lparam);
        }

        // Any non-space key resets double space timer
        *LAST_SPACE_TIME.lock().unwrap() = None;
    }

    CallNextHookEx(None, n_code, wparam, lparam)
}
