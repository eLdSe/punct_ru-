#![windows_subsystem = "windows"]

mod assistant;
mod clipboard;
mod engine;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_SZ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_NOREPEAT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW,
    FindWindowW, GetCursorPos, GetMessageW, InsertMenuW, LoadIconW, PostQuitMessage,
    RegisterClassExW, SendMessageW, SetForegroundWindow, TrackPopupMenu, TranslateMessage, HICON,
    HMENU, IDI_APPLICATION, MF_BYPOSITION, MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING,
    MF_UNCHECKED, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY,
    WM_HOTKEY, WM_RBUTTONUP, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};

const WM_TRAY_CALLBACK: u32 = WM_APP + 1;
const HOTKEY_ID: i32 = 1001;

const MENU_INFO: usize = 1002;
const MENU_MODE_CASUAL: usize = 1003;
const MENU_MODE_FORMAL: usize = 1004;
const MENU_DOUBLE_SPACE: usize = 1005;
const MENU_AUTOSTART: usize = 1006;
const MENU_EXIT: usize = 1007;

static GLOBAL_ENGINE: Mutex<Option<engine::PunctEngine>> = Mutex::new(None);
static CASUAL_MODE: AtomicBool = AtomicBool::new(true);

fn load_settings() {
    unsafe {
        let subkey = w!("Software\\PunctRu");
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, &mut hkey).is_ok() {
            let mut val_type = REG_DWORD;
            let mut val: u32 = 0;
            let mut val_len = 4u32;
            if RegQueryValueExW(
                hkey,
                w!("CasualMode"),
                None,
                Some(&mut val_type),
                Some(&mut val as *mut u32 as *mut u8),
                Some(&mut val_len),
            ).is_ok() {
                CASUAL_MODE.store(val != 0, Ordering::Relaxed);
            }

            let mut ds_val: u32 = 1;
            let mut ds_len = 4u32;
            if RegQueryValueExW(
                hkey,
                w!("DoubleSpace"),
                None,
                Some(&mut val_type),
                Some(&mut ds_val as *mut u32 as *mut u8),
                Some(&mut ds_len),
            ).is_ok() {
                assistant::DOUBLE_SPACE_ENABLED.store(ds_val != 0, Ordering::Relaxed);
            }
            let _ = RegCloseKey(hkey);
        }
    }
}

fn save_settings(casual: bool) {
    CASUAL_MODE.store(casual, Ordering::Relaxed);
    unsafe {
        let subkey = w!("Software\\PunctRu");
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegCreateKeyW(
            HKEY_CURRENT_USER,
            subkey,
            &mut hkey,
        ).is_ok() {
            let val: u32 = if casual { 1 } else { 0 };
            let _ = RegSetValueExW(
                hkey,
                w!("CasualMode"),
                0,
                REG_DWORD,
                Some(std::slice::from_raw_parts(&val as *const u32 as *const u8, 4)),
            );
            let _ = RegCloseKey(hkey);
        }
    }
}

fn save_double_space_setting(enabled: bool) {
    assistant::DOUBLE_SPACE_ENABLED.store(enabled, Ordering::Relaxed);
    unsafe {
        let subkey = w!("Software\\PunctRu");
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegCreateKeyW(HKEY_CURRENT_USER, subkey, &mut hkey).is_ok() {
            let val: u32 = if enabled { 1 } else { 0 };
            let _ = RegSetValueExW(
                hkey,
                w!("DoubleSpace"),
                0,
                REG_DWORD,
                Some(std::slice::from_raw_parts(&val as *const u32 as *const u8, 4)),
            );
            let _ = RegCloseKey(hkey);
        }
    }
}

fn is_autostart_enabled() -> bool {
    unsafe {
        let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, &mut hkey).is_ok() {
            let res = RegQueryValueExW(hkey, w!("PunctRu"), None, None, None, None).is_ok();
            let _ = RegCloseKey(hkey);
            res
        } else {
            false
        }
    }
}

fn set_autostart(enable: bool) {
    unsafe {
        let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_WRITE, &mut hkey).is_ok() {
            if enable {
                if let Ok(exe_path) = std::env::current_exe() {
                    let path_wide: Vec<u16> = exe_path
                        .to_string_lossy()
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();
                    let _ = RegSetValueExW(
                        hkey,
                        w!("PunctRu"),
                        0,
                        REG_SZ,
                        Some(std::slice::from_raw_parts(
                            path_wide.as_ptr() as *const u8,
                            path_wide.len() * 2,
                        )),
                    );
                }
            } else {
                let _ = RegDeleteValueW(hkey, w!("PunctRu"));
            }
            let _ = RegCloseKey(hkey);
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            if wparam.0 as i32 == HOTKEY_ID {
                handle_hotkey();
            }
            LRESULT(0)
        }
        assistant::WM_APPLY_DOUBLE_SPACE => {
            handle_double_space_apply();
            LRESULT(0)
        }
        WM_TRAY_CALLBACK => {
            if lparam.0 as u32 == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xffff) as usize;
            match cmd_id {
                MENU_MODE_CASUAL => {
                    save_settings(true);
                    update_tray_tip(hwnd);
                }
                MENU_MODE_FORMAL => {
                    save_settings(false);
                    update_tray_tip(hwnd);
                }
                MENU_DOUBLE_SPACE => {
                    let current = assistant::DOUBLE_SPACE_ENABLED.load(Ordering::Relaxed);
                    save_double_space_setting(!current);
                    update_tray_tip(hwnd);
                }
                MENU_AUTOSTART => {
                    let current = is_autostart_enabled();
                    set_autostart(!current);
                }
                MENU_EXIT => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            assistant::cleanup_assistant();
            remove_tray_icon(hwnd);
            let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn handle_hotkey() {
    println!("[Hotkey Alt+P] Triggered!");

    // 1. Copy selected text
    let original_text = match clipboard::copy_selection() {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            clipboard::restore_previous_clipboard();
            println!("[Hotkey] No text selected in active window.");
            return;
        }
    };

    // Ignore text without Cyrillic (pure English, code, links, commands)
    if !original_text.chars().any(|c| matches!(c, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё')) {
        clipboard::restore_previous_clipboard();
        println!("[Hotkey] Skipped: text does not contain Russian/Cyrillic characters.");
        return;
    }

    println!("[Hotkey] Captured text: {:?}", &original_text);

    let casual = CASUAL_MODE.load(Ordering::Relaxed);

    // 2. Process via ONNX model
    let mut engine_guard = GLOBAL_ENGINE.lock().unwrap();
    if let Some(engine) = engine_guard.as_mut() {
        match engine.restore(&original_text, casual) {
            Ok(restored) => {
                println!("[Hotkey] Restored: {:?}", &restored);
                // 3. Paste back (paste_text automatically restores previous clipboard afterwards)
                if restored != original_text {
                    clipboard::paste_text(&restored);
                } else {
                    clipboard::restore_previous_clipboard();
                    println!("[Hotkey] Text is already properly formatted.");
                }
            }
            Err(e) => {
                clipboard::restore_previous_clipboard();
                eprintln!("[Hotkey] Inference error: {e}");
            }
        }
    } else {
        clipboard::restore_previous_clipboard();
        println!("[Hotkey] Engine is not loaded (waiting for model files).");
    }
}

fn handle_double_space_apply() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_BACK, VK_RIGHT, VK_SPACE};

    // 1. Delete the first space that was typed
    assistant::send_key_press(VK_BACK);
    std::thread::sleep(std::time::Duration::from_millis(15));

    // 2. Select from current caret to the start of the current line (Shift + Home)
    assistant::select_line_to_home();
    std::thread::sleep(std::time::Duration::from_millis(20));

    // 3. Copy the actual selected text via clipboard
    let original_text = match clipboard::copy_selection() {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            clipboard::restore_previous_clipboard();
            // Nothing copied, deselect and restore two spaces
            assistant::send_key_press(VK_RIGHT);
            assistant::send_key_press(VK_SPACE);
            assistant::send_key_press(VK_SPACE);
            return;
        }
    };

    // Safety guard: if selected text is enormous (>1500 chars, e.g. a whole document in Word),
    // deselect and don't touch it to avoid accidental massive overwrites.
    if original_text.chars().count() > 1500 {
        clipboard::restore_previous_clipboard();
        assistant::send_key_press(VK_RIGHT);
        assistant::send_key_press(VK_SPACE);
        assistant::send_key_press(VK_SPACE);
        return;
    }

    // 4. Check if text has Cyrillic. If it's English/code, don't touch it!
    if !assistant::has_cyrillic(&original_text) {
        clipboard::restore_previous_clipboard();
        assistant::send_key_press(VK_RIGHT);
        assistant::send_key_press(VK_SPACE);
        assistant::send_key_press(VK_SPACE);
        return;
    }

    // 5. Restore with model and paste back
    let casual = CASUAL_MODE.load(Ordering::Relaxed);
    let mut engine_guard = GLOBAL_ENGINE.lock().unwrap();
    if let Some(engine) = engine_guard.as_mut() {
        match engine.restore(&original_text, casual) {
            Ok(restored) => {
                let output = format!("{restored} ");
                clipboard::paste_text(&output);
            }
            Err(e) => {
                clipboard::restore_previous_clipboard();
                eprintln!("[Assistant] Inference error: {e}");
                assistant::send_key_press(VK_RIGHT);
                assistant::send_key_press(VK_SPACE);
                assistant::send_key_press(VK_SPACE);
            }
        }
    } else {
        clipboard::restore_previous_clipboard();
    }
}

unsafe fn show_tray_menu(hwnd: HWND) {
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);

    let hmenu = CreatePopupMenu().unwrap_or(HMENU(std::ptr::null_mut()));
    if hmenu.0.is_null() {
        return;
    }

    let is_casual = CASUAL_MODE.load(Ordering::Relaxed);
    let is_ds = assistant::DOUBLE_SPACE_ENABLED.load(Ordering::Relaxed);
    let is_auto = is_autostart_enabled();

    let _ = InsertMenuW(
        hmenu,
        0,
        MF_BYPOSITION | MF_STRING | MF_GRAYED,
        MENU_INFO,
        w!("Punctuation Tool (Alt+P) - Active"),
    );
    let _ = InsertMenuW(hmenu, 1, MF_BYPOSITION | MF_SEPARATOR, 0, PCWSTR::null());

    let casual_flags = MF_BYPOSITION | MF_STRING | if is_casual { MF_CHECKED } else { MF_UNCHECKED };
    let _ = InsertMenuW(
        hmenu,
        2,
        casual_flags,
        MENU_MODE_CASUAL,
        w!("💬 Режим: Чат / Мессенджер (без точки в конце)"),
    );

    let formal_flags = MF_BYPOSITION | MF_STRING | if !is_casual { MF_CHECKED } else { MF_UNCHECKED };
    let _ = InsertMenuW(
        hmenu,
        3,
        formal_flags,
        MENU_MODE_FORMAL,
        w!("📄 Режим: Документы / Деловой (со всеми точками)"),
    );

    let _ = InsertMenuW(hmenu, 4, MF_BYPOSITION | MF_SEPARATOR, 0, PCWSTR::null());

    let ds_flags = MF_BYPOSITION | MF_STRING | if is_ds { MF_CHECKED } else { MF_UNCHECKED };
    let _ = InsertMenuW(
        hmenu,
        5,
        ds_flags,
        MENU_DOUBLE_SPACE,
        w!("⚡ Авто-исправление по 2x Пробелу"),
    );

    let auto_flags = MF_BYPOSITION | MF_STRING | if is_auto { MF_CHECKED } else { MF_UNCHECKED };
    let _ = InsertMenuW(
        hmenu,
        6,
        auto_flags,
        MENU_AUTOSTART,
        w!("🚀 Автозагрузка с Windows"),
    );

    let _ = InsertMenuW(hmenu, 7, MF_BYPOSITION | MF_SEPARATOR, 0, PCWSTR::null());
    let _ = InsertMenuW(hmenu, 8, MF_BYPOSITION | MF_STRING, MENU_EXIT, w!("Выход"));

    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(
        hmenu,
        TPM_LEFTALIGN | TPM_BOTTOMALIGN,
        cursor.x,
        cursor.y,
        0,
        hwnd,
        None,
    );
    let _ = DestroyMenu(hmenu);
}

fn update_tray_tip(hwnd: HWND) {
    let is_casual = CASUAL_MODE.load(Ordering::Relaxed);
    let mode_str = if is_casual { "Чат / Мессенджер" } else { "Деловой" };
    let tip_text: Vec<u16> = format!("Punctuation Tool (Alt+P) | {}", mode_str)
        .encode_utf16()
        .collect();
    let mut tip = [0u16; 128];
    let len = tip_text.len().min(127);
    tip[..len].copy_from_slice(&tip_text[..len]);

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_TIP,
        szTip: tip,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &mut nid);
    }
}

fn add_tray_icon(hwnd: HWND, icon: HICON) {
    let is_casual = CASUAL_MODE.load(Ordering::Relaxed);
    let mode_str = if is_casual { "Чат / Мессенджер" } else { "Деловой" };
    let tip_text: Vec<u16> = format!("Punctuation Tool (Alt+P) | {}", mode_str)
        .encode_utf16()
        .collect();
    let mut tip = [0u16; 128];
    let len = tip_text.len().min(127);
    tip[..len].copy_from_slice(&tip_text[..len]);

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAY_CALLBACK,
        hIcon: icon,
        szTip: tip,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &mut nid);
    }
}

fn remove_tray_icon(hwnd: HWND) {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &mut nid);
    }
}

fn find_file(candidates: &[&str]) -> Option<std::path::PathBuf> {
    // 1. Check relative to current exe
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            for &c in candidates {
                let p = exe_dir.join(c);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    // 2. Check current working dir
    for &c in candidates {
        let p = Path::new(c);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    None
}

const EMBEDDED_ICO: &[u8] = include_bytes!("../punct.ico");
const EMBEDDED_MODEL: &[u8] = include_bytes!("../../ml/export/model.int8.onnx");
const EMBEDDED_TOKENIZER: &[u8] = include_bytes!("../../ml/export/tokenizer.json");
const EMBEDDED_DLL: &[u8] = include_bytes!("../../ml/export/onnxruntime.dll");

fn ensure_onnxruntime_dll() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    // 1. If onnxruntime.dll exists next to current exe or in current dir, use it
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let local_dll = dir.join("onnxruntime.dll");
            if local_dll.exists() {
                return Ok(local_dll);
            }
        }
    }
    let local_dll = std::path::Path::new("onnxruntime.dll");
    if local_dll.exists() {
        return Ok(local_dll.to_path_buf());
    }

    // 2. Otherwise extract to %LOCALAPPDATA%\PunctRu\onnxruntime.dll
    let base_dir = std::env::var("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let target_dir = base_dir.join("PunctRu");
    let _ = std::fs::create_dir_all(&target_dir);
    let target_dll = target_dir.join("onnxruntime.dll");

    let needs_write = match std::fs::metadata(&target_dll) {
        Ok(meta) => meta.len() != EMBEDDED_DLL.len() as u64,
        Err(_) => true,
    };
    if needs_write {
        std::fs::write(&target_dll, EMBEDDED_DLL)?;
    }

    Ok(target_dll)
}

fn load_app_icon(h_instance: HINSTANCE) -> HICON {
    unsafe {
        // 1. Try loading from compiled PE resource (ID 1)
        if let Ok(h) = LoadIconW(h_instance, PCWSTR(1 as *const u16)) {
            if !h.0.is_null() {
                return h;
            }
        }

        // 2. Try loading from punct.ico next to executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(dir) = exe_path.parent() {
                let ico_path = dir.join("punct.ico");
                if !ico_path.exists() {
                    let _ = std::fs::write(&ico_path, EMBEDDED_ICO);
                }
                let wide_path: Vec<u16> = ico_path
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                if let Ok(h) = windows::Win32::UI::WindowsAndMessaging::LoadImageW(
                    None,
                    PCWSTR(wide_path.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::IMAGE_ICON,
                    0,
                    0,
                    windows::Win32::UI::WindowsAndMessaging::LR_LOADFROMFILE
                        | windows::Win32::UI::WindowsAndMessaging::LR_DEFAULTSIZE,
                ) {
                    if !h.0.is_null() {
                        return HICON(h.0);
                    }
                }
            }
        }

        // 3. Fallback: Parse embedded ICO directory and call CreateIconFromResourceEx
        if EMBEDDED_ICO.len() >= 22 {
            let count = u16::from_le_bytes([EMBEDDED_ICO[4], EMBEDDED_ICO[5]]) as usize;
            for i in 0..count {
                let entry_offset = 6 + i * 16;
                if entry_offset + 16 <= EMBEDDED_ICO.len() {
                    let w = EMBEDDED_ICO[entry_offset];
                    let bytes_in_res = u32::from_le_bytes([
                        EMBEDDED_ICO[entry_offset + 8],
                        EMBEDDED_ICO[entry_offset + 9],
                        EMBEDDED_ICO[entry_offset + 10],
                        EMBEDDED_ICO[entry_offset + 11],
                    ]) as usize;
                    let image_offset = u32::from_le_bytes([
                        EMBEDDED_ICO[entry_offset + 12],
                        EMBEDDED_ICO[entry_offset + 13],
                        EMBEDDED_ICO[entry_offset + 14],
                        EMBEDDED_ICO[entry_offset + 15],
                    ]) as usize;

                    if (w == 32 || w == 24 || w == 16) && image_offset + bytes_in_res <= EMBEDDED_ICO.len() {
                        let slice = &EMBEDDED_ICO[image_offset..image_offset + bytes_in_res];
                        if let Ok(hicon) = windows::Win32::UI::WindowsAndMessaging::CreateIconFromResourceEx(
                            slice,
                            windows::Win32::Foundation::BOOL(1),
                            0x00030000,
                            32,
                            32,
                            Default::default(),
                        ) {
                            if !hicon.0.is_null() {
                                return hicon;
                            }
                        }
                    }
                }
            }
        }

        // 4. Default system fallback
        LoadIconW(HINSTANCE(std::ptr::null_mut()), IDI_APPLICATION).unwrap_or(HICON(std::ptr::null_mut()))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Punctuation Tool (Rust Single-File Edition) ===");

    load_settings();

    let dll_path = ensure_onnxruntime_dll().ok();

    // 1. First priority: Load completely from embedded memory (Single Portable EXE)
    let mut engine_loaded = false;
    match engine::PunctEngine::from_memory(EMBEDDED_MODEL, EMBEDDED_TOKENIZER, dll_path.as_deref()) {
        Ok(eng) => {
            println!("Embedded ONNX model & tokenizer loaded from internal memory!");

            // CLI test mode: `punct.exe --test "любой сырой текст"`
            let args: Vec<String> = std::env::args().collect();
            if args.len() > 1 && args[1] == "--test" {
                let sample = if args.len() > 2 {
                    args[2..].join(" ")
                } else {
                    "привет как дела я вчера звонил но ты не ответил перезвони когда сможешь".to_string()
                };
                println!("\n[TEST MODE: Сравнение режимов]");
                println!("Вход:              {}", sample);
                let mut test_engine = eng;
                let formal = test_engine.restore(&sample, false)?;
                let casual = test_engine.restore(&sample, true)?;
                println!("1. Деловой режим (со всеми точками):  {}", formal);
                println!("2. Чат/Мессенджер (без точки в конце): {}", casual);
                println!("\nТест успешно завершён!");
                return Ok(());
            }

            *GLOBAL_ENGINE.lock().unwrap() = Some(eng);
            engine_loaded = true;
        }
        Err(e) => {
            eprintln!("Notice: Embedded engine load failed ({e}), checking external files...");
        }
    }

    // 2. Fallback: External model files if present
    if !engine_loaded {
        let model_path = find_file(&[
            "model.int8.onnx",
            "../ml/export/model.int8.onnx",
            "../../ml/export/model.int8.onnx",
        ]);
        let tokenizer_path = find_file(&[
            "tokenizer.json",
            "../ml/export/tokenizer.json",
            "../../ml/export/tokenizer.json",
        ]);

        if let (Some(mp), Some(tp)) = (model_path.as_ref(), tokenizer_path.as_ref()) {
            match engine::PunctEngine::new(mp, tp, dll_path.as_deref()) {
                Ok(eng) => {
                    *GLOBAL_ENGINE.lock().unwrap() = Some(eng);
                }
                Err(e) => eprintln!("Warning: Failed to initialize external model: {e}"),
            }
        }
    }

    unsafe {
        let h_instance = GetModuleHandleW(PCWSTR::null())?;
        let class_name = w!("PunctAppWindowClass");

        // Single-instance protection: cleanly close any previously running instance
        if let Ok(old_hwnd) = FindWindowW(class_name, PCWSTR::null()) {
            if !old_hwnd.0.is_null() {
                let _ = SendMessageW(old_hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        }

        let icon = load_app_icon(HINSTANCE(h_instance.0));

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: HINSTANCE(h_instance.0),
            lpszClassName: class_name,
            hIcon: icon,
            ..Default::default()
        };
        RegisterClassExW(&wc);

        // Create a hidden message window
        let hwnd = CreateWindowExW(
            Default::default(),
            class_name,
            w!("Punctuation Restoration"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            HWND(std::ptr::null_mut()),
            HMENU(std::ptr::null_mut()),
            HINSTANCE(h_instance.0),
            None,
        )?;

        // Register Global HotKey: Alt + P (or Ctrl + Alt + P as fallback)
        let _hotkey_registered = if RegisterHotKey(hwnd, HOTKEY_ID, MOD_ALT | MOD_NOREPEAT, 0x50).is_ok() {
            println!("Registered global hotkey: Alt + P");
            true
        } else if RegisterHotKey(hwnd, HOTKEY_ID, MOD_ALT | windows::Win32::UI::Input::KeyboardAndMouse::MOD_CONTROL | MOD_NOREPEAT, 0x50).is_ok() {
            println!("Notice: Alt + P is occupied by another app; registered fallback: Ctrl + Alt + P");
            true
        } else {
            let err = windows::Win32::Foundation::GetLastError();
            eprintln!("Failed to register hotkey (Windows error: {err:?}).");
            false
        };

        add_tray_icon(hwnd, icon);
        assistant::init_assistant(hwnd, HINSTANCE(h_instance.0));
        println!("App running in system tray. Press Alt + P on any selected text.");
        println!("Right-click tray icon to Exit.");

        // Pure event-driven message loop: blocks with 0% CPU consumption
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}
