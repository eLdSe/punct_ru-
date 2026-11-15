use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber, OpenClipboard,
    SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_MENU,
    VK_C, VK_V,
};

const CF_UNICODETEXT: u32 = 13;

static SAVED_CLIPBOARD: Mutex<Option<String>> = Mutex::new(None);

pub fn get_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT);
        let result = if let Ok(h) = handle {
            if h.0.is_null() {
                None
            } else {
                let ptr = GlobalLock(HGLOBAL(h.0)) as *const u16;
                if ptr.is_null() {
                    None
                } else {
                    let mut len = 0;
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(ptr, len);
                    let text = String::from_utf16_lossy(slice);
                    let _ = GlobalUnlock(HGLOBAL(h.0));
                    Some(text)
                }
            }
        } else {
            None
        };
        let _ = CloseClipboard();
        result
    }
}

pub fn set_clipboard_text(text: &str) -> bool {
    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes_len = utf16.len() * std::mem::size_of::<u16>();

    unsafe {
        let h_mem = match GlobalAlloc(GMEM_MOVEABLE, bytes_len) {
            Ok(h) => h,
            Err(_) => return false,
        };

        let ptr = GlobalLock(h_mem) as *mut u16;
        if ptr.is_null() {
            return false;
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
        let _ = GlobalUnlock(h_mem);

        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return false;
        }
        let _ = EmptyClipboard();
        let set_res = SetClipboardData(CF_UNICODETEXT, HANDLE(h_mem.0));
        let _ = CloseClipboard();
        set_res.is_ok()
    }
}

fn send_key_event(vk: u16, key_up: bool) {
    let flags = if key_up { KEYEVENTF_KEYUP } else { Default::default() };
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: crate::assistant::MAGIC_EXTRA_INFO,
            },
        },
    };
    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn copy_selection() -> Option<String> {
    // Save user's current clipboard text before overwriting it with Ctrl+C
    let prev_clipboard = get_clipboard_text();
    if let Ok(mut guard) = SAVED_CLIPBOARD.lock() {
        *guard = prev_clipboard;
    }

    // Release Alt modifier so Ctrl+C is not sent as Ctrl+Alt+C
    send_key_event(VK_MENU.0, true);
    thread::sleep(Duration::from_millis(20));

    let seq_before = unsafe { GetClipboardSequenceNumber() };

    // Send Ctrl+C
    send_key_event(VK_CONTROL.0, false);
    send_key_event(VK_C.0, false);
    send_key_event(VK_C.0, true);
    send_key_event(VK_CONTROL.0, true);

    // Wait for clipboard update (up to 300ms)
    let mut updated = false;
    for _ in 0..15 {
        thread::sleep(Duration::from_millis(20));
        let seq_after = unsafe { GetClipboardSequenceNumber() };
        if seq_after != seq_before {
            updated = true;
            break;
        }
    }

    if !updated {
        return None;
    }

    get_clipboard_text()
}

pub fn restore_previous_clipboard() {
    let saved = SAVED_CLIPBOARD.lock().ok().and_then(|mut g| g.take());
    if let Some(old_text) = saved {
        let _ = set_clipboard_text(&old_text);
    }
}

pub fn paste_text(text: &str) -> bool {
    if !set_clipboard_text(text) {
        restore_previous_clipboard();
        return false;
    }
    thread::sleep(Duration::from_millis(20));

    // Send Ctrl+V
    send_key_event(VK_CONTROL.0, false);
    send_key_event(VK_V.0, false);
    send_key_event(VK_V.0, true);
    send_key_event(VK_CONTROL.0, true);

    // Give target app 75ms to process WM_PASTE before restoring user's original clipboard
    thread::sleep(Duration::from_millis(75));
    restore_previous_clipboard();
    true
}
