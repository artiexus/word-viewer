//! word-viewer.exe: the Win32 window. All file parsing lives in the safe library;
//! the unsafe code here only talks to Windows and never touches file contents.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    win::run();
}

/// Non-Windows builds are a command-line converter, used for testing the parsers.
#[cfg(not(windows))]
fn main() {
    let path = std::env::args().nth(1).expect("usage: word-viewer FILE");
    let data = std::fs::read(&path).expect("read failed");
    match word_viewer::to_rtf_named(&data, &path).ok() {
        Some(rtf) => print!("{rtf}"),
        None => std::process::exit(1),
    }
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use std::fs::File;
    use std::io::Read;
    use std::os::windows::ffi::OsStringExt;
    use std::ptr::{null, null_mut};
    use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicPtr, AtomicU32, Ordering};

    type Hwnd = isize;
    type WndProc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;

    #[repr(C)]
    struct WndClass {
        style: u32,
        proc_: WndProc,
        cls_extra: i32,
        wnd_extra: i32,
        instance: isize,
        icon: isize,
        cursor: isize,
        background: isize,
        menu_name: *const u16,
        class_name: *const u16,
    }

    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        pt: [i32; 2],
        private: u32,
    }

    #[repr(C)]
    struct Accel {
        virt: u8,
        key: u16,
        cmd: u16,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct SetTextEx {
        flags: u32,
        codepage: u32,
    }

    #[repr(C)]
    struct OpenFileName {
        size: u32,
        owner: Hwnd,
        instance: isize,
        filter: *const u16,
        custom_filter: *mut u16,
        max_cust_filter: u32,
        filter_index: u32,
        file: *mut u16,
        max_file: u32,
        file_title: *mut u16,
        max_file_title: u32,
        initial_dir: *const u16,
        title: *const u16,
        flags: u32,
        file_offset: u16,
        file_extension: u16,
        def_ext: *const u16,
        cust_data: isize,
        hook: *const c_void,
        template_name: *const u16,
        reserved_ptr: *mut c_void,
        reserved: u32,
        flags_ex: u32,
    }

    #[repr(C)]
    struct WindowPlacement {
        length: u32,
        flags: u32,
        show_cmd: u32,
        min_pos: [i32; 2],
        max_pos: [i32; 2],
        normal: Rect,
    }

    #[repr(C)]
    struct FindReplace {
        size: u32,
        owner: Hwnd,
        instance: isize,
        flags: u32,
        find_what: *mut u16,
        replace_with: *mut u16,
        find_what_len: u16,
        replace_with_len: u16,
        cust_data: isize,
        hook: *const c_void,
        template_name: *const u16,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct CharRange {
        min: i32,
        max: i32,
    }

    #[repr(C)]
    struct FindTextEx {
        chrg: CharRange,
        text: *const u16,
        found: CharRange,
    }

    #[repr(C)]
    struct PrintDlg {
        size: u32,
        owner: Hwnd,
        dev_mode: isize,
        dev_names: isize,
        dc: isize,
        flags: u32,
        from_page: u16,
        to_page: u16,
        min_page: u16,
        max_page: u16,
        copies: u16,
        instance: isize,
        cust_data: isize,
        print_hook: *const c_void,
        setup_hook: *const c_void,
        print_template: *const u16,
        setup_template: *const u16,
        print_template_handle: isize,
        setup_template_handle: isize,
    }

    #[repr(C)]
    struct DocInfo {
        size: i32,
        name: *const u16,
        output: *const u16,
        datatype: *const u16,
        kind: u32,
    }

    #[repr(C)]
    struct FormatRange {
        dc: isize,
        target: isize,
        rc: Rect,
        page: Rect,
        chrg: CharRange,
    }

    #[repr(C)]
    struct TextLengthEx {
        flags: u32,
        codepage: u32,
    }

    #[repr(C)]
    struct MenuInfo {
        size: u32,
        mask: u32,
        style: u32,
        max_height: u32,
        back: isize,
        help_id: u32,
        data: usize,
    }

    #[repr(C)]
    struct MenuItemInfo {
        size: u32,
        mask: u32,
        ftype: u32,
        state: u32,
        id: u32,
        submenu: isize,
        checked: isize,
        unchecked: isize,
        data: usize,
        type_data: *mut u16,
        cch: u32,
        item_bitmap: isize,
    }

    #[repr(C)]
    struct MeasureItem {
        ctl_type: u32,
        ctl_id: u32,
        item_id: u32,
        width: u32,
        height: u32,
        data: usize,
    }

    #[repr(C)]
    struct DrawItem {
        ctl_type: u32,
        ctl_id: u32,
        item_id: u32,
        action: u32,
        state: u32,
        item: isize,
        dc: isize,
        rc: Rect,
        data: usize,
    }

    #[repr(C)]
    struct MenuBarInfo {
        size: u32,
        bar: Rect,
        menu: isize,
        menu_window: Hwnd,
        flags: u32,
    }

    #[repr(C)]
    struct CharFormat {
        size: u32,
        mask: u32,
        effects: u32,
        height: i32,
        offset: i32,
        color: u32,
        charset: u8,
        pitch_and_family: u8,
        face: [u16; 32],
    }

    #[repr(C)]
    struct ChooseColor {
        size: u32,
        owner: Hwnd,
        instance: isize,
        result: u32,
        custom: *mut u32,
        flags: u32,
        cust_data: isize,
        hook: *const c_void,
        template_name: *const u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn SetDefaultDllDirectories(flags: u32) -> i32;
        fn LoadLibraryExW(name: *const u16, file: isize, flags: u32) -> isize;
        fn GetModuleHandleW(name: *const u16) -> isize;
        fn GetProcAddress(module: isize, name: *const u8) -> *const c_void;
        fn GetCurrentThreadId() -> u32;
        fn GetCommandLineW() -> *const u16;
        fn LocalFree(mem: *mut c_void) -> *mut c_void;
        fn GlobalFree(mem: isize) -> isize;
    }

    #[link(name = "user32")]
    extern "system" {
        fn SetProcessDPIAware() -> i32;
        fn GetDpiForSystem() -> u32;
        fn GetDpiForWindow(h: Hwnd) -> u32;
        fn RegisterClassW(wc: *const WndClass) -> u16;
        fn CreateWindowExW(ex: u32, class: *const u16, title: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32,
                           parent: Hwnd, menu: isize, inst: isize, param: *const c_void) -> Hwnd;
        fn DefWindowProcW(h: Hwnd, m: u32, w: usize, l: isize) -> isize;
        fn GetMessageW(msg: *mut Msg, h: Hwnd, min: u32, max: u32) -> i32;
        fn TranslateMessage(msg: *const Msg) -> i32;
        fn DispatchMessageW(msg: *const Msg) -> isize;
        fn IsDialogMessageW(dlg: Hwnd, msg: *const Msg) -> i32;
        fn CreateAcceleratorTableW(acc: *const Accel, n: i32) -> isize;
        fn TranslateAcceleratorW(h: Hwnd, acc: isize, msg: *const Msg) -> i32;
        fn RegisterWindowMessageW(name: *const u16) -> u32;
        fn PostQuitMessage(code: i32);
        fn DestroyWindow(h: Hwnd) -> i32;
        fn MoveWindow(h: Hwnd, x: i32, y: i32, w: i32, h_: i32, repaint: i32) -> i32;
        fn SendMessageW(h: Hwnd, m: u32, w: usize, l: isize) -> isize;
        fn SetWindowTextW(h: Hwnd, s: *const u16) -> i32;
        fn ShowWindow(h: Hwnd, cmd: i32) -> i32;
        fn SetFocus(h: Hwnd) -> Hwnd;
        fn CheckMenuItem(menu: isize, id: u32, check: u32) -> u32;
        fn SetWindowPos(h: Hwnd, after: Hwnd, x: i32, y: i32, w: i32, h_: i32, flags: u32) -> i32;
        fn MessageBeep(kind: u32) -> i32;
        fn GetWindowPlacement(h: Hwnd, wp: *mut WindowPlacement) -> i32;
        fn GetMenu(h: Hwnd) -> isize;
        fn SetMenuInfo(menu: isize, mi: *const MenuInfo) -> i32;
        fn GetMenuItemCount(menu: isize) -> i32;
        fn GetMenuItemInfoW(menu: isize, item: u32, by_position: i32, mi: *mut MenuItemInfo) -> i32;
        fn SetMenu(h: Hwnd, menu: isize) -> i32;
        fn DialogBoxIndirectParamW(inst: isize, template: *const c_void, parent: Hwnd, proc_: unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize, init: isize) -> isize;
        fn EndDialog(dlg: Hwnd, result: isize) -> i32;
        fn GetDlgItem(dlg: Hwnd, id: i32) -> Hwnd;
        fn InvalidateRect(h: Hwnd, r: *const Rect, erase: i32) -> i32;
        fn GetClassNameW(h: Hwnd, name: *mut u16, n: i32) -> i32;
        fn GetClientRect(h: Hwnd, r: *mut Rect) -> i32;
        fn ClientToScreen(h: Hwnd, pt: *mut i32) -> i32;
        fn SetWindowLongPtrW(h: Hwnd, index: i32, value: isize) -> isize;
        fn CallWindowProcW(prev: isize, h: Hwnd, m: u32, w: usize, l: isize) -> isize;
        fn SetWindowsHookExW(id: i32, hook: unsafe extern "system" fn(i32, usize, isize) -> isize, module: isize, thread: u32) -> isize;
        fn CallNextHookEx(hook: isize, code: i32, w: usize, l: isize) -> isize;
        fn GetActiveWindow() -> Hwnd;
        fn DestroyMenu(menu: isize) -> i32;
        fn GetMenuBarInfo(h: Hwnd, object: i32, item: i32, mbi: *mut MenuBarInfo) -> i32;
        fn GetWindowRect(h: Hwnd, r: *mut Rect) -> i32;
        fn GetDC(h: Hwnd) -> isize;
        fn GetWindowDC(h: Hwnd) -> isize;
        fn ReleaseDC(h: Hwnd, dc: isize) -> i32;
        fn FillRect(dc: isize, r: *const Rect, brush: isize) -> i32;
        fn DrawTextW(dc: isize, s: *const u16, n: i32, r: *mut Rect, format: u32) -> i32;
        fn SetWindowPlacement(h: Hwnd, wp: *const WindowPlacement) -> i32;
        fn SystemParametersInfoW(action: u32, uparam: u32, pparam: *mut c_void, winini: u32) -> i32;
        fn LoadCursorW(inst: isize, name: usize) -> isize;
        fn LoadIconW(inst: isize, name: usize) -> isize;
        fn CreateMenu() -> isize;
        fn CreatePopupMenu() -> isize;
        fn AppendMenuW(menu: isize, flags: u32, id: usize, text: *const u16) -> i32;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn CommandLineToArgvW(cmd: *const u16, argc: *mut i32) -> *mut *mut u16;
        fn DragQueryFileW(drop: isize, i: u32, file: *mut u16, n: u32) -> u32;
        fn DragFinish(drop: isize);
    }

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetOpenFileNameW(ofn: *mut OpenFileName) -> i32;
        fn FindTextW(fr: *mut FindReplace) -> Hwnd;
        fn PrintDlgW(pd: *mut PrintDlg) -> i32;
        fn ChooseColorW(cc: *mut ChooseColor) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn GetDeviceCaps(dc: isize, index: i32) -> i32;
        fn StartDocW(dc: isize, di: *const DocInfo) -> i32;
        fn StartPage(dc: isize) -> i32;
        fn EndPage(dc: isize) -> i32;
        fn EndDoc(dc: isize) -> i32;
        fn AbortDoc(dc: isize) -> i32;
        fn DeleteDC(dc: isize) -> i32;
        fn CreateSolidBrush(color: u32) -> isize;
        fn CreateFontIndirectW(lf: *const c_void) -> isize;
        fn DeleteObject(obj: isize) -> i32;
        fn SelectObject(dc: isize, obj: isize) -> isize;
        fn SetTextColor(dc: isize, color: u32) -> u32;
        fn SetBkMode(dc: isize, mode: i32) -> i32;
    }

    const WM_DESTROY: u32 = 0x0002;
    const WM_SIZE: u32 = 0x0005;
    const WM_SETFOCUS: u32 = 0x0007;
    const WM_COMMAND: u32 = 0x0111;
    const WM_COPY: u32 = 0x0301;
    const WM_DROPFILES: u32 = 0x0233;
    const EM_SETSEL: u32 = 0x00B1;
    const EM_SETRECT: u32 = 0x00B3;
    const EM_SCROLLCARET: u32 = 0x00B7;
    const EM_EXGETSEL: u32 = 0x0434;
    const EM_EXLIMITTEXT: u32 = 0x0435;
    const EM_EXSETSEL: u32 = 0x0437;
    const EM_FORMATRANGE: u32 = 0x0439;
    const EM_GETTEXTLENGTHEX: u32 = 0x045F;
    const EM_SETTEXTEX: u32 = 0x0461;
    const EM_FINDTEXTEXW: u32 = 0x047C;
    const EM_GETZOOM: u32 = 0x04E0;
    const EM_SETZOOM: u32 = 0x04E1;
    const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0800;
    const FR_DOWN: u32 = 0x1;
    const FR_FINDNEXT: u32 = 0x8;
    const FR_DIALOGTERM: u32 = 0x40;
    const FR_SEARCH_FLAGS: u32 = 0x1 | 0x2 | 0x4; // down, whole word, match case
    const ID_OPEN: usize = 1;
    const ID_EXIT: usize = 2;
    const ID_PRINT: usize = 3;
    const ID_COPY: usize = 4;
    const ID_SELECT_ALL: usize = 5;
    const ID_FIND: usize = 6;
    const ID_FIND_NEXT: usize = 7;
    const ID_FIND_PREV: usize = 8;
    const ID_ZOOM_IN: usize = 9;
    const ID_ZOOM_OUT: usize = 10;
    const ID_ZOOM_RESET: usize = 11;
    const ID_DARK: usize = 12;
    const ID_PREFS: usize = 13;
    /// Text sizes offered, in points; and weights, as Segoe UI faces.
    const SIZES: [u32; 10] = [9, 10, 11, 12, 13, 14, 16, 18, 20, 24];
    const WEIGHTS: [(&str, &str); 4] = [("Light", "Segoe UI Light"), ("Semilight", "Segoe UI Semilight"), ("Regular", "Segoe UI"), ("Semibold", "Segoe UI Semibold")];
    const DEFAULT_SIZE: u32 = 11;
    const DEFAULT_WEIGHT: u32 = 2;
    /// Dark mode colors (COLORREF, 0x00BBGGRR): Windows Terminal's default
    /// "Campbell" scheme, background #0C0C0C and text #CCCCCC.
    const DARK_BG: u32 = 0x000C_0C0C;
    const DARK_TEXT: u32 = 0x00CC_CCCC;
    const MAX_PATH: usize = 260;
    /// Zoom steps, in percent.
    const ZOOMS: [u32; 13] = [50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400];

    static WIN: AtomicIsize = AtomicIsize::new(0);
    static EDIT: AtomicIsize = AtomicIsize::new(0);
    static FIND_DLG: AtomicIsize = AtomicIsize::new(0);
    static FIND_MSG: AtomicU32 = AtomicU32::new(0);
    /// The find dialog's state, which must outlive the (modeless) dialog.
    static FIND: AtomicPtr<Find> = AtomicPtr::new(null_mut());
    static DARK: AtomicBool = AtomicBool::new(false);
    /// Dark mode follows Windows' app mode until the user picks one.
    static FOLLOW_WINDOWS: AtomicBool = AtomicBool::new(true);
    static TEXT_SIZE: AtomicU32 = AtomicU32::new(DEFAULT_SIZE);
    static TEXT_WEIGHT: AtomicU32 = AtomicU32::new(DEFAULT_WEIGHT);
    static DARK_TEXT_COLOR: AtomicU32 = AtomicU32::new(DARK_TEXT);
    static DARK_BG_COLOR: AtomicU32 = AtomicU32::new(DARK_BG);
    static VIEW_MENU: AtomicIsize = AtomicIsize::new(0);

    struct Find {
        fr: FindReplace,
        text: [u16; 256],
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// Length of a NUL-terminated wide string, looking at most `max` units.
    unsafe fn wlen(p: *const u16, max: usize) -> usize {
        (0..max).position(|i| *p.add(i) == 0).unwrap_or(max)
    }

    fn edit() -> Hwnd {
        EDIT.load(Ordering::Relaxed)
    }

    /// Pixels for `px` at 96 DPI, scaled to the display's DPI.
    fn scaled(px: i32) -> i32 {
        let dpi = unsafe { GetDpiForWindow(WIN.load(Ordering::Relaxed)) }.max(96) as i32;
        px * dpi / 96
    }

    // ---- remembered settings: window placement, zoom and dark mode ----

    /// %LOCALAPPDATA%\word-viewer\window.txt. Only the window's own settings
    /// are kept; never anything about documents.
    fn settings_path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("word-viewer").join("window.txt"))
    }

    struct Settings {
        /// The window's normal (not maximized) rectangle.
        normal: Rect,
        maximized: bool,
        zoom: u32,
        /// Dark or light as the user chose, or None to follow Windows.
        dark: Option<bool>,
        /// Dark mode's text and background colors.
        colors: Option<(u32, u32)>,
        /// Text size in points, and weight (an index into WEIGHTS).
        text: Option<(u32, u32)>,
    }

    fn load_settings() -> Option<Settings> {
        let mut s = String::new();
        File::open(settings_path()?).ok()?.take(256).read_to_string(&mut s).ok()?;
        let v: Vec<i64> = s.split_whitespace().map(|x| x.parse().ok()).collect::<Option<_>>()?;
        let &[l, t, r, b, max, zoom] = v.get(..6)? else { return None };
        let [l, t, r, b] = [l, t, r, b].map(|x| x.clamp(-100_000, 100_000) as i32);
        // later fields were added over time: dark mode (0 follow Windows,
        // 1 dark, 2 light), its colors, then text size and weight
        let dark = match v.get(6) {
            Some(1) => Some(true),
            Some(2) => Some(false),
            _ => None,
        };
        let colors = match v.get(7..9) {
            Some(&[fg, bg]) => Some(((fg & 0xFF_FFFF) as u32, (bg & 0xFF_FFFF) as u32)),
            _ => None,
        };
        let text = match v.get(9..11) {
            Some(&[size, weight]) => Some((size.clamp(6, 72) as u32, weight.clamp(0, WEIGHTS.len() as i64 - 1) as u32)),
            _ => None,
        };
        let ok = r - l >= 200 && b - t >= 150 && r - l <= 32767 && b - t <= 32767;
        ok.then_some(Settings { normal: Rect { left: l, top: t, right: r, bottom: b }, maximized: max != 0, zoom: zoom.clamp(10, 500) as u32, dark, colors, text })
    }

    fn save_settings() {
        let mut wp: WindowPlacement = unsafe { std::mem::zeroed() };
        wp.length = std::mem::size_of::<WindowPlacement>() as u32;
        if unsafe { GetWindowPlacement(WIN.load(Ordering::Relaxed), &mut wp) } == 0 {
            return;
        }
        let n = wp.normal;
        let dark = match (FOLLOW_WINDOWS.load(Ordering::Relaxed), DARK.load(Ordering::Relaxed)) {
            (true, _) => 0,
            (false, true) => 1,
            (false, false) => 2,
        };
        let (fg, bg) = (DARK_TEXT_COLOR.load(Ordering::Relaxed), DARK_BG_COLOR.load(Ordering::Relaxed));
        let (size, weight) = (TEXT_SIZE.load(Ordering::Relaxed), TEXT_WEIGHT.load(Ordering::Relaxed));
        let line = format!("{} {} {} {} {} {} {dark} {fg} {bg} {size} {weight}\n", n.left, n.top, n.right, n.bottom, (wp.show_cmd == 3) as u8, zoom());
        if let Some(path) = settings_path() {
            let _ = path.parent().map(std::fs::create_dir_all);
            let _ = std::fs::write(path, line);
        }
    }

    // ---- showing documents ----

    /// The document as the library rendered it, to show again when the text
    /// preferences change.
    static SHOWN_RTF: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

    /// `rtf` with the text preferences applied: font sizes scaled from the
    /// default 11 pt, and the weight's face in the font table. Reads the RTF
    /// this crate writes, where document text is escaped (\\, \{, \}), so
    /// only real control words are changed.
    fn with_text_prefs(rtf: &str) -> String {
        let (size, weight) = (TEXT_SIZE.load(Ordering::Relaxed), TEXT_WEIGHT.load(Ordering::Relaxed) as usize);
        let face = WEIGHTS.get(weight).map_or("Segoe UI", |w| w.1);
        let mut out = String::with_capacity(rtf.len() + 64);
        let b = rtf.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'\\' {
                match b.get(i + 1) {
                    Some(b'f') if b.get(i + 2) == Some(&b's') && b.get(i + 3).is_some_and(|c| c.is_ascii_digit()) => {
                        let digits = b[i + 3..].iter().take_while(|c| c.is_ascii_digit()).count();
                        let half_points: u32 = rtf[i + 3..i + 3 + digits].parse::<u32>().unwrap_or(22).min(3276);
                        out.push_str(&format!("\\fs{}", (half_points * size + DEFAULT_SIZE / 2) / DEFAULT_SIZE));
                        i += 3 + digits;
                        continue;
                    }
                    Some(&c) => {
                        // an escape (\\ \{ \}) or another control word: copy its first two bytes
                        out.push('\\');
                        out.push(c as char);
                        i += 2;
                        continue;
                    }
                    None => {}
                }
            }
            out.push(b[i] as char); // RTF output is pure ASCII
            i += 1;
        }
        out.replacen("{\\f0 Segoe UI;}", &format!("{{\\f0 {face};}}"), 1)
    }

    fn show_rtf(rtf: &str) {
        *SHOWN_RTF.lock().unwrap_or_else(|e| e.into_inner()) = rtf.to_string();
        let mut buf = with_text_prefs(rtf).into_bytes();
        buf.push(0);
        let st = SetTextEx { flags: 0, codepage: 0 };
        let z = zoom(); // replacing the text resets RichEdit's zoom
        unsafe {
            SendMessageW(edit(), EM_SETTEXTEX, &st as *const _ as usize, buf.as_ptr() as isize);
            SendMessageW(edit(), EM_SETSEL, 0, 0);
            SendMessageW(edit(), EM_SCROLLCARET, 0, 0);
        }
        // new text has the default (automatic) color
        if DARK.load(Ordering::Relaxed) {
            text_color(Some(DARK_TEXT_COLOR.load(Ordering::Relaxed)));
        }
        set_zoom(z);
    }

    /// Show the current document again (after the text preferences change),
    /// keeping the same text at the top of the window.
    fn reshow() {
        unsafe {
            let line = SendMessageW(edit(), 0x00CE, 0, 0); // EM_GETFIRSTVISIBLELINE
            let at = SendMessageW(edit(), 0x00BB, line as usize, 0); // EM_LINEINDEX
            let rtf = SHOWN_RTF.lock().unwrap_or_else(|e| e.into_inner()).clone();
            show_rtf(&rtf);
            let line = SendMessageW(edit(), 0x0436, 0, at); // EM_EXLINEFROMCHAR
            SendMessageW(edit(), 0x00B6, 0, line); // EM_LINESCROLL
        }
    }

    fn read_limited(path: &std::path::Path) -> Option<Vec<u8>> {
        let mut f = File::open(path).ok()?;
        // the library applies each format's own limit; PDFs have the largest
        if f.metadata().ok()?.len() > word_viewer::MAX_PDF as u64 {
            return None;
        }
        let mut data = Vec::new();
        f.by_ref().take(word_viewer::MAX_PDF as u64 + 1).read_to_end(&mut data).ok()?;
        Some(data)
    }

    fn load(path_w: &[u16]) {
        let path = std::path::PathBuf::from(std::ffi::OsString::from_wide(path_w));
        let name: String = path.file_name().map(|n| n.to_string_lossy().chars().take(200).collect()).unwrap_or_default();
        // A parser bug becomes "could not read", never a crash.
        let rtf = read_limited(&path).ok_or(word_viewer::ReadError::Unsupported).and_then(|d| {
            std::panic::catch_unwind(|| word_viewer::to_rtf_named(&d, &name)).unwrap_or(Err(word_viewer::ReadError::Unsupported))
        });
        let title = wide(&format!("{name} - Word Viewer"));
        unsafe { SetWindowTextW(WIN.load(Ordering::Relaxed), title.as_ptr()) };
        match rtf {
            Ok(rtf) => show_rtf(&rtf),
            Err(e) => show_rtf(&format!("{{\\rtf1\\fs22 {}}}", e.message())),
        }
    }

    fn open_dialog() {
        let mut file = [0u16; MAX_PATH];
        let filter: Vec<u16> = "Documents (Word, PDF, RTF, ODT, EPUB, HTML, text)\0*.docx;*.doc;*.pdf;*.rtf;*.odt;*.epub;*.html;*.htm;*.txt;*.md\0All files\0*.*\0\0".encode_utf16().collect();
        let mut ofn: OpenFileName = unsafe { std::mem::zeroed() };
        ofn.size = std::mem::size_of::<OpenFileName>() as u32;
        ofn.owner = WIN.load(Ordering::Relaxed);
        ofn.filter = filter.as_ptr();
        ofn.file = file.as_mut_ptr();
        ofn.max_file = MAX_PATH as u32;
        ofn.flags = 0x1000 | 0x0800; // OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST
        if unsafe { GetOpenFileNameW(&mut ofn) } != 0 {
            let n = file.iter().position(|&c| c == 0).unwrap_or(MAX_PATH);
            load(&file[..n]);
        }
    }

    // ---- zoom ----

    /// The current zoom, in percent (RichEdit also zooms with Ctrl+wheel).
    fn zoom() -> u32 {
        let (mut num, mut den) = (0u32, 0u32);
        unsafe { SendMessageW(edit(), EM_GETZOOM, &mut num as *mut _ as usize, &mut den as *mut _ as isize) };
        if num == 0 || den == 0 {
            100
        } else {
            (num * 100 + den / 2) / den
        }
    }

    fn set_zoom(pct: u32) {
        let pct = pct.clamp(ZOOMS[0], ZOOMS[ZOOMS.len() - 1]);
        let (num, den) = if pct == 100 { (0, 0) } else { (pct as usize, 100) };
        unsafe { SendMessageW(edit(), EM_SETZOOM, num, den) };
    }

    /// Zoom to the next step in `dir` (+1 in, -1 out) from the current zoom.
    fn step_zoom(dir: i32) {
        let z = zoom();
        let next = if dir > 0 { ZOOMS.iter().find(|&&s| s > z) } else { ZOOMS.iter().rev().find(|&&s| s < z) };
        if let Some(&s) = next {
            set_zoom(s);
        }
    }

    // ---- dark mode ----

    /// Set all text to `color`, or back to the automatic (system) color.
    /// Documents never set text colors, so this loses nothing.
    fn text_color(color: Option<u32>) {
        let mut cf: CharFormat = unsafe { std::mem::zeroed() };
        cf.size = std::mem::size_of::<CharFormat>() as u32;
        cf.mask = 0x4000_0000; // CFM_COLOR
        match color {
            Some(c) => cf.color = c,
            None => cf.effects = 0x4000_0000, // CFE_AUTOCOLOR
        }
        unsafe { SendMessageW(edit(), 0x0444, 4, &cf as *const _ as isize) }; // EM_SETCHARFORMAT, SCF_ALL
    }

    /// Function `name` from a System32 DLL, if it has one (older Windows may not).
    unsafe fn optional(dll: &str, name: &[u8]) -> Option<*const c_void> {
        let m = LoadLibraryExW(wide(dll).as_ptr(), 0, LOAD_LIBRARY_SEARCH_SYSTEM32);
        let f = if m == 0 { null() } else { GetProcAddress(m, name.as_ptr()) };
        (!f.is_null()).then_some(f)
    }

    /// Show the current mode: colors, title bar, scroll bar and the menu check.
    fn apply_theme() {
        let dark = DARK.load(Ordering::Relaxed);
        let win = WIN.load(Ordering::Relaxed);
        unsafe {
            // EM_SETBKGNDCOLOR: wParam 1 is the system color
            SendMessageW(edit(), 0x0443, !dark as usize, DARK_BG_COLOR.load(Ordering::Relaxed) as isize);
            text_color(dark.then(|| DARK_TEXT_COLOR.load(Ordering::Relaxed)));
            // a dark title bar (Windows 10 20H1+; 19 on 1809 to 1909)
            if let Some(f) = optional("dwmapi.dll", b"DwmSetWindowAttribute\0") {
                let set: unsafe extern "system" fn(Hwnd, u32, *const c_void, u32) -> i32 = std::mem::transmute(f);
                let on = dark as i32;
                if set(win, 20, &on as *const _ as *const c_void, 4) != 0 {
                    set(win, 19, &on as *const _ as *const c_void, 4);
                }
                // redraw the frame: SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_FRAMECHANGED
                SetWindowPos(win, 0, 0, 0, 0, 0, 0x1 | 0x2 | 0x4 | 0x20);
                // Windows 10 repaints the title bar only when the window's
                // active state changes: flip it and back
                let active = (GetActiveWindow() == win) as usize;
                SendMessageW(win, 0x0086, active ^ 1, 0); // WM_NCACTIVATE
                SendMessageW(win, 0x0086, active, 0);
            }
            // a dark scroll bar
            if let Some(f) = optional("uxtheme.dll", b"SetWindowTheme\0") {
                let set: unsafe extern "system" fn(Hwnd, *const u16, *const u16) -> i32 = std::mem::transmute(f);
                let theme = wide("DarkMode_Explorer");
                set(edit(), if dark { theme.as_ptr() } else { null() }, null());
            }
            // the menus: drawn by the viewer in dark mode (Windows 10 has no dark menus)
            let old_menu = GetMenu(win);
            let bar = build_menu(dark);
            let brush = if dark { CreateSolidBrush(menu_colors().0) } else { 0 };
            if dark {
                let info = MenuInfo { size: std::mem::size_of::<MenuInfo>() as u32, mask: 0x2 | 0x8000_0000, back: brush, ..std::mem::zeroed() }; // MIM_BACKGROUND | MIM_APPLYTOSUBMENUS
                SetMenuInfo(bar, &info);
            }
            CheckMenuItem(VIEW_MENU.load(Ordering::Relaxed), ID_DARK as u32, if dark { 0x8 } else { 0 }); // MF_CHECKED
            SetMenu(win, bar);
            if old_menu != 0 {
                DestroyMenu(old_menu);
            }
            let old_brush = MENU_BRUSH.swap(brush, Ordering::Relaxed);
            if old_brush != 0 {
                DeleteObject(old_brush);
            }
        }
    }

    // ---- dark menus ----

    /// The menus: each title, and its items' text and command ("" is a separator).
    const MENUS: [(&str, &[(&str, usize)]); 3] = [
        ("&File", &[("&Open...\tCtrl+O", ID_OPEN), ("&Print...\tCtrl+P", ID_PRINT), ("", 0), ("E&xit", ID_EXIT)]),
        ("&Edit", &[("&Copy\tCtrl+C", ID_COPY), ("Select &All\tCtrl+A", ID_SELECT_ALL), ("", 0), ("&Find...\tCtrl+F", ID_FIND), ("Find &Next\tF3", ID_FIND_NEXT), ("Find Pre&vious\tShift+F3", ID_FIND_PREV)]),
        ("&View", &[
            ("Zoom &In\tCtrl++", ID_ZOOM_IN), ("Zoom &Out\tCtrl+-", ID_ZOOM_OUT), ("&Actual Size\tCtrl+0", ID_ZOOM_RESET), ("", 0),
            ("&Dark Mode\tCtrl+D", ID_DARK), ("&Text Preferences...", ID_PREFS),
        ]),
    ];

    /// Build the menu bar: ordinary menus, or in dark mode owner-drawn ones
    /// whose item data is an index (from 1) into MENU_TEXT.
    unsafe fn build_menu(dark: bool) -> isize {
        const MF_SEPARATOR: u32 = 0x800;
        const MF_POPUP: u32 = 0x10;
        const MF_OWNERDRAW: u32 = 0x100;
        let mut texts = MENU_TEXT.lock().unwrap_or_else(|e| e.into_inner());
        let fill = texts.is_empty(); // the same items every time: list them once
        let mut add = |text: &str, top: bool| -> *const u16 {
            if fill {
                texts.push(MenuText { text: text.encode_utf16().collect(), top, separator: text.is_empty() });
            }
            let n = texts.iter().position(|t| t.top == top && t.text.iter().copied().eq(text.encode_utf16())).map_or(0, |i| i + 1);
            n as *const u16
        };
        let bar = CreateMenu();
        for (title, items) in MENUS {
            let popup = CreatePopupMenu();
            for &(text, id) in items {
                let sep = if text.is_empty() { MF_SEPARATOR } else { 0 };
                if dark {
                    AppendMenuW(popup, MF_OWNERDRAW | sep, id, add(text, false));
                } else if text.is_empty() {
                    AppendMenuW(popup, sep, 0, null());
                } else {
                    AppendMenuW(popup, 0, id, wide(text).as_ptr());
                }
            }
            if title == "&View" {
                VIEW_MENU.store(popup, Ordering::Relaxed);
            }
            if dark {
                AppendMenuW(bar, MF_POPUP | MF_OWNERDRAW, popup as usize, add(title, true));
            } else {
                AppendMenuW(bar, MF_POPUP, popup as usize, wide(title).as_ptr());
            }
        }
        bar
    }

    /// Owner-drawn items have no text Windows can see, so it asks for their
    /// mnemonics (Alt+F, then O...): the item whose "&" letter is `ch`.
    unsafe fn menu_char(ch: u16, menu: isize) -> isize {
        let ch = char::from_u32(ch as u32).map(|c| c.to_ascii_lowercase());
        for i in 0..GetMenuItemCount(menu).max(0) {
            let mut mi = MenuItemInfo { size: std::mem::size_of::<MenuItemInfo>() as u32, mask: 0x20, ..std::mem::zeroed() }; // MIIM_DATA
            GetMenuItemInfoW(menu, i as u32, 1, &mut mi);
            let Some(t) = menu_text(mi.data) else { continue };
            let key = t.text.iter().position(|&c| c == '&' as u16).and_then(|p| t.text.get(p + 1)).and_then(|&c| char::from_u32(c as u32));
            if key.map(|c| c.to_ascii_lowercase()) == ch {
                return (2 << 16) | i as isize; // MNC_EXECUTE, item index
            }
        }
        0 // MNC_IGNORE
    }

    /// A menu item's text in dark mode, where the menus are owner-drawn.
    #[derive(Clone)]
    struct MenuText {
        text: Vec<u16>,
        top: bool,
        separator: bool,
    }

    static MENU_TEXT: std::sync::Mutex<Vec<MenuText>> = std::sync::Mutex::new(Vec::new());
    static MENU_BRUSH: AtomicIsize = AtomicIsize::new(0);
    static MENU_FONT: AtomicIsize = AtomicIsize::new(0);

    fn menu_text(index: usize) -> Option<MenuText> {
        MENU_TEXT.lock().unwrap_or_else(|e| e.into_inner()).get(index.checked_sub(1)?).cloned()
    }

    /// `a` blended toward `b` by `t`/256, per channel.
    fn mix(a: u32, b: u32, t: u32) -> u32 {
        (0..3).map(|i| ((((a >> (i * 8)) & 0xFF) * (256 - t) + ((b >> (i * 8)) & 0xFF) * t) / 256) << (i * 8)).sum()
    }

    /// Menu background, highlighted item, text, and separator colors, from dark mode's colors.
    fn menu_colors() -> (u32, u32, u32, u32) {
        let (fg, bg) = (DARK_TEXT_COLOR.load(Ordering::Relaxed), DARK_BG_COLOR.load(Ordering::Relaxed));
        (mix(bg, fg, 20), mix(bg, fg, 56), fg, mix(bg, fg, 80))
    }

    /// The menu font: Windows' own, as set in Settings.
    unsafe fn menu_font() -> isize {
        let mut f = MENU_FONT.load(Ordering::Relaxed);
        if f == 0 {
            // NONCLIENTMETRICSW (504 bytes), whose lfMenuFont is at byte 224
            let mut ncm = [0u32; 126];
            ncm[0] = 504;
            if SystemParametersInfoW(0x0029, 504, ncm.as_mut_ptr() as *mut c_void, 0) != 0 { // SPI_GETNONCLIENTMETRICS
                f = CreateFontIndirectW(ncm.as_ptr().add(224 / 4) as *const c_void);
            }
            MENU_FONT.store(f, Ordering::Relaxed);
        }
        f
    }

    unsafe fn fill(dc: isize, r: &Rect, color: u32) {
        let b = CreateSolidBrush(color);
        FillRect(dc, r, b);
        DeleteObject(b);
    }

    /// The label and the shortcut ("&Open...\tCtrl+O").
    fn split_tab(text: &[u16]) -> (&[u16], &[u16]) {
        match text.iter().position(|&c| c == '\t' as u16) {
            Some(i) => (&text[..i], &text[i + 1..]),
            None => (text, &[]),
        }
    }

    const DT_CENTER: u32 = 0x1;
    const DT_RIGHT: u32 = 0x2;
    const DT_VCENTER: u32 = 0x4;
    const DT_SINGLELINE: u32 = 0x20;
    const DT_CALCRECT: u32 = 0x400;
    const DT_HIDEPREFIX: u32 = 0x10_0000;

    /// DrawTextW, for a slice. An empty one is skipped: its pointer is only a
    /// placeholder, which DrawTextW reads even when told the length is 0.
    unsafe fn draw_text(dc: isize, s: &[u16], r: &mut Rect, format: u32) -> i32 {
        if s.is_empty() {
            return 0;
        }
        DrawTextW(dc, s.as_ptr(), s.len() as i32, r, format)
    }

    unsafe fn text_width(dc: isize, s: &[u16]) -> i32 {
        let mut r = Rect::default();
        draw_text(dc, s, &mut r, DT_SINGLELINE | DT_CALCRECT);
        r.right - r.left
    }

    unsafe fn measure_menu(m: &mut MeasureItem) {
        let Some(t) = menu_text(m.data) else { return };
        if t.separator {
            (m.width, m.height) = (0, scaled(7) as u32);
            return;
        }
        let win = WIN.load(Ordering::Relaxed);
        let dc = GetDC(win);
        let old = SelectObject(dc, menu_font());
        let (label, keys) = split_tab(&t.text);
        let (lw, kw) = (text_width(dc, label), text_width(dc, keys));
        let mut probe = Rect::default();
        let h = DrawTextW(dc, wide("Ag").as_ptr(), 2, &mut probe, DT_SINGLELINE | DT_CALCRECT);
        SelectObject(dc, old);
        ReleaseDC(win, dc);
        if t.top {
            (m.width, m.height) = ((lw + scaled(4)) as u32, h as u32);
        } else {
            let gap = if kw > 0 { scaled(32) + kw } else { 0 };
            (m.width, m.height) = ((scaled(28) + lw + gap + scaled(12)) as u32, (h + scaled(8)) as u32);
        }
    }

    unsafe fn draw_menu(d: &DrawItem) {
        let Some(t) = menu_text(d.data) else { return };
        let (back, hot, fg, line) = menu_colors();
        const SELECTED: u32 = 0x1;
        const GRAYED: u32 = 0x2 | 0x4;
        const CHECKED: u32 = 0x8;
        const NOACCEL: u32 = 0x100;
        const HOTLIGHT: u32 = 0x40;
        let r = d.rc;
        if t.separator {
            fill(d.dc, &r, back);
            let mid = (r.top + r.bottom) / 2;
            fill(d.dc, &Rect { left: r.left + scaled(8), top: mid, right: r.right - scaled(8), bottom: mid + 1 }, line);
            return;
        }
        fill(d.dc, &r, if d.state & (SELECTED | HOTLIGHT) != 0 { hot } else { back });
        SetBkMode(d.dc, 1); // TRANSPARENT
        SetTextColor(d.dc, if d.state & GRAYED != 0 { mix(fg, back, 128) } else { fg });
        let old = SelectObject(d.dc, menu_font());
        let prefix = if d.state & NOACCEL != 0 { DT_HIDEPREFIX } else { 0 };
        let (label, keys) = split_tab(&t.text);
        let mut rc = r;
        if t.top {
            draw_text(d.dc, label, &mut rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | prefix);
        } else {
            if d.state & CHECKED != 0 {
                let mut c = Rect { right: r.left + scaled(28), ..r };
                DrawTextW(d.dc, wide("\u{2713}").as_ptr(), 1, &mut c, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            }
            rc.left += scaled(28);
            rc.right -= scaled(12);
            draw_text(d.dc, label, &mut rc, DT_VCENTER | DT_SINGLELINE | prefix);
            draw_text(d.dc, keys, &mut rc, DT_RIGHT | DT_VCENTER | DT_SINGLELINE);
        }
        SelectObject(d.dc, old);
    }

    /// Windows draws a light line under the menu bar, outside what the menu
    /// draws; paint over it in dark mode.
    unsafe fn menu_bar_line(h: Hwnd) {
        let mut mb = MenuBarInfo { size: std::mem::size_of::<MenuBarInfo>() as u32, ..std::mem::zeroed() };
        if GetMenuBarInfo(h, -3, 0, &mut mb) == 0 { // OBJID_MENU
            return;
        }
        let mut w = Rect::default();
        GetWindowRect(h, &mut w);
        let line = Rect { left: mb.bar.left - w.left, top: mb.bar.bottom - w.top, right: mb.bar.right - w.left, bottom: mb.bar.bottom - w.top + 1 };
        let dc = GetWindowDC(h);
        fill(dc, &line, menu_colors().0);
        ReleaseDC(h, dc);
    }

    /// Whether Windows is set to dark mode for apps (Settings > Colors >
    /// "Choose your default app mode"). advapi32 is looked up at run time,
    /// so the exe's imports don't change.
    fn windows_dark() -> bool {
        unsafe {
            let Some(f) = optional("advapi32.dll", b"RegGetValueW\0") else { return false };
            let get: unsafe extern "system" fn(isize, *const u16, *const u16, u32, *mut u32, *mut c_void, *mut u32) -> i32 = std::mem::transmute(f);
            let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
            let (mut light, mut size) = (1u32, 4u32);
            // HKEY_CURRENT_USER, RRF_RT_REG_DWORD
            let found = get(0x8000_0001u32 as i32 as isize, key.as_ptr(), wide("AppsUseLightTheme").as_ptr(), 0x10, null_mut(), &mut light as *mut _ as *mut c_void, &mut size) == 0;
            found && light == 0
        }
    }

    /// Whether the NUL-terminated wide string at `p` is `s` (reading at most s.len() + 1 units).
    unsafe fn wstr_eq(p: *const u16, s: &str) -> bool {
        s.encode_utf16().chain(Some(0)).enumerate().all(|(i, c)| *p.add(i) == c)
    }

    // ---- dark dropdown borders ----

    /// Windows' own window procedure for menu windows (class "#32768").
    static MENU_PROC: AtomicIsize = AtomicIsize::new(0);

    /// A hook on this thread's window creation, to catch dropdown menus as
    /// they open and paint their borders (which owner-drawing doesn't reach).
    unsafe extern "system" fn cbt_hook(code: i32, w: usize, l: isize) -> isize {
        if code == 3 { // HCBT_CREATEWND
            let h = w as Hwnd;
            let mut class = [0u16; 16];
            let n = GetClassNameW(h, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
            if class[..n].iter().copied().eq("#32768".encode_utf16()) {
                let ours = menu_proc as *const () as isize;
                let old = SetWindowLongPtrW(h, -4, ours); // GWLP_WNDPROC
                if old != ours {
                    MENU_PROC.store(old, Ordering::Relaxed);
                }
            }
        }
        CallNextHookEx(0, code, w, l)
    }

    unsafe extern "system" fn menu_proc(h: Hwnd, m: u32, w: usize, l: isize) -> isize {
        let r = CallWindowProcW(MENU_PROC.load(Ordering::Relaxed), h, m, w, l);
        // WM_NCPAINT, or WM_PRINT of the frame (used for the fade-in)
        let print = m == 0x0317 && l & 0x2 != 0; // PRF_NONCLIENT
        if DARK.load(Ordering::Relaxed) && (m == 0x0085 || print) {
            let dc = if print { w as isize } else { GetWindowDC(h) };
            let (mut win, mut client) = (Rect::default(), Rect::default());
            GetWindowRect(h, &mut win);
            GetClientRect(h, &mut client);
            let mut origin = [0i32; 2];
            ClientToScreen(h, origin.as_mut_ptr());
            // the frame: everything between the window's edge and its client area
            let (cl, ct) = (origin[0] - win.left, origin[1] - win.top);
            let (cr, cb) = (cl + client.right, ct + client.bottom);
            let (ww, wh) = (win.right - win.left, win.bottom - win.top);
            // like Windows' own: the menu's background, with a thin outer edge
            let back = menu_colors().0;
            let edge = mix(back, DARK_TEXT_COLOR.load(Ordering::Relaxed), 40);
            for (inset, color) in [(0, edge), (1, back)] {
                for strip in [
                    Rect { left: inset, top: inset, right: ww - inset, bottom: ct },
                    Rect { left: inset, top: cb, right: ww - inset, bottom: wh - inset },
                    Rect { left: inset, top: ct, right: cl, bottom: cb },
                    Rect { left: cr, top: ct, right: ww - inset, bottom: cb },
                ] {
                    fill(dc, &strip, color);
                }
            }
            if !print {
                ReleaseDC(h, dc);
            }
        }
        r
    }

    /// The standard color dialog, starting at `color`; the chosen color, if any.
    fn choose_color(owner: Hwnd, color: u32) -> Option<u32> {
        let mut custom = [0x00FF_FFFFu32; 16];
        custom[0] = DARK_TEXT;
        custom[1] = DARK_BG;
        let mut cc: ChooseColor = unsafe { std::mem::zeroed() };
        cc.size = std::mem::size_of::<ChooseColor>() as u32;
        cc.owner = owner;
        cc.result = color;
        cc.custom = custom.as_mut_ptr();
        cc.flags = 0x1 | 0x2; // CC_RGBINIT | CC_FULLOPEN
        (unsafe { ChooseColorW(&mut cc) } != 0).then_some(cc.result & 0xFF_FFFF)
    }

    // ---- View > Text Preferences ----

    const IDC_SIZE: u16 = 100;
    const IDC_WEIGHT: u16 = 101;
    const IDC_MATCH: u16 = 110;
    const IDC_LIGHT: u16 = 111;
    const IDC_DARK: u16 = 112;
    const IDC_FG_SWATCH: u16 = 120;
    const IDC_FG: u16 = 121;
    const IDC_BG_SWATCH: u16 = 122;
    const IDC_BG: u16 = 123;
    const IDC_RESET: u16 = 124;

    /// The dark mode colors being edited in the dialog (applied on OK), and
    /// brushes for their swatches.
    static EDIT_FG: AtomicU32 = AtomicU32::new(0);
    static EDIT_BG: AtomicU32 = AtomicU32::new(0);
    static SWATCHES: [AtomicIsize; 2] = [AtomicIsize::new(0), AtomicIsize::new(0)];

    /// An in-memory dialog template (DLGTEMPLATE and DLGITEMTEMPLATEs), so the
    /// exe needs no resource script for it.
    struct Template(Vec<u16>);

    impl Template {
        fn u32(&mut self, v: u32) {
            self.0.extend([v as u16, (v >> 16) as u16]);
        }

        fn str(&mut self, s: &str) {
            self.0.extend(s.encode_utf16().chain(Some(0)));
        }

        fn align(&mut self) {
            if self.0.len() % 2 == 1 {
                self.0.push(0);
            }
        }

        fn new(title: &str, items: u16, cx: i16, cy: i16) -> Template {
            let mut t = Template(Vec::new());
            // WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_MODALFRAME | DS_SETFONT | DS_CENTER
            t.u32(0x8000_0000 | 0x00C0_0000 | 0x0008_0000 | 0x80 | 0x40 | 0x800);
            t.u32(0);
            t.0.extend([items, 0, 0, cx as u16, cy as u16, 0, 0]); // count, x, y, cx, cy, no menu, default class
            t.str(title);
            t.0.push(9); // point size
            t.str("Segoe UI");
            t
        }

        /// A control: `class` is a predefined class atom (0x80 button, 0x82 static, 0x85 combo box).
        fn item(&mut self, class: u16, style: u32, rect: [i16; 4], id: u16, text: &str) {
            self.align();
            self.u32(0x4000_0000 | 0x1000_0000 | style); // WS_CHILD | WS_VISIBLE
            self.u32(0);
            self.0.extend(rect.map(|v| v as u16));
            self.0.extend([id, 0xFFFF, class]);
            self.str(text);
            self.0.push(0); // no creation data
        }
    }

    fn prefs_dialog() {
        const BUTTON: u16 = 0x80;
        const STATIC: u16 = 0x82;
        const COMBO: u16 = 0x85;
        const TAB: u32 = 0x0001_0000; // WS_TABSTOP
        const GROUP: u32 = 0x0002_0000; // WS_GROUP
        const NONE: u16 = 0xFFFF;
        let mut t = Template::new("Text Preferences", 20, 230, 188);
        t.item(BUTTON, 7, [7, 6, 216, 50], NONE, "Text"); // BS_GROUPBOX
        t.item(STATIC, 0, [16, 21, 44, 9], NONE, "&Size:");
        t.item(COMBO, 3 | 0x0020_0000 | TAB | GROUP, [62, 19, 50, 140], IDC_SIZE, ""); // CBS_DROPDOWNLIST | WS_VSCROLL
        t.item(STATIC, 0, [116, 21, 40, 9], NONE, "points");
        t.item(STATIC, 0, [16, 38, 44, 9], NONE, "&Weight:");
        t.item(COMBO, 3 | 0x0020_0000 | TAB | GROUP, [62, 36, 80, 100], IDC_WEIGHT, "");
        t.item(BUTTON, 7, [7, 62, 216, 98], NONE, "Dark mode");
        t.item(BUTTON, 9 | TAB | GROUP, [16, 77, 66, 10], IDC_MATCH, "&Match Windows"); // BS_AUTORADIOBUTTON
        t.item(BUTTON, 9, [86, 77, 62, 10], IDC_LIGHT, "Always &light");
        t.item(BUTTON, 9, [152, 77, 66, 10], IDC_DARK, "Always &dark");
        t.item(STATIC, GROUP, [16, 98, 70, 9], NONE, "Text color:");
        t.item(STATIC, 0x1000, [90, 96, 30, 12], IDC_FG_SWATCH, ""); // SS_SUNKEN
        t.item(BUTTON, TAB | GROUP, [126, 95, 54, 14], IDC_FG, "&Change...");
        t.item(STATIC, 0, [16, 118, 70, 9], NONE, "Background color:");
        t.item(STATIC, 0x1000, [90, 116, 30, 12], IDC_BG_SWATCH, "");
        t.item(BUTTON, TAB, [126, 115, 54, 14], IDC_BG, "C&hange...");
        t.item(BUTTON, TAB, [16, 138, 70, 14], IDC_RESET, "&Reset colors");
        t.item(STATIC, 0, [90, 141, 130, 9], NONE, "Ctrl+D also switches dark mode.");
        t.item(BUTTON, 1 | TAB | GROUP, [117, 168, 50, 14], 1, "OK"); // BS_DEFPUSHBUTTON, IDOK
        t.item(BUTTON, TAB, [173, 168, 50, 14], 2, "Cancel"); // IDCANCEL
        // DialogBoxIndirect needs the template DWORD-aligned
        t.align();
        let words: Vec<u32> = t.0.chunks(2).map(|c| c[0] as u32 | (c[1] as u32) << 16).collect();
        unsafe {
            if DialogBoxIndirectParamW(GetModuleHandleW(null()), words.as_ptr() as *const c_void, WIN.load(Ordering::Relaxed), prefs_proc, 0) == 1 {
                reshow();
                apply_theme();
            }
            for s in &SWATCHES {
                let b = s.swap(0, Ordering::Relaxed);
                if b != 0 {
                    DeleteObject(b);
                }
            }
        }
    }

    unsafe fn set_swatch(dlg: Hwnd, i: usize, color: u32) {
        let old = SWATCHES[i].swap(CreateSolidBrush(color), Ordering::Relaxed);
        if old != 0 {
            DeleteObject(old);
        }
        InvalidateRect(GetDlgItem(dlg, [IDC_FG_SWATCH, IDC_BG_SWATCH][i] as i32), null(), 1);
    }

    unsafe extern "system" fn prefs_proc(dlg: Hwnd, m: u32, w: usize, l: isize) -> isize {
        match m {
            0x0110 => { // WM_INITDIALOG
                let item = |id: u16| GetDlgItem(dlg, id as i32);
                let size = TEXT_SIZE.load(Ordering::Relaxed);
                for (i, s) in SIZES.iter().enumerate() {
                    SendMessageW(item(IDC_SIZE), 0x0143, 0, wide(&s.to_string()).as_ptr() as isize); // CB_ADDSTRING
                    if *s == size {
                        SendMessageW(item(IDC_SIZE), 0x014E, i, 0); // CB_SETCURSEL
                    }
                }
                for (name, _) in WEIGHTS {
                    SendMessageW(item(IDC_WEIGHT), 0x0143, 0, wide(name).as_ptr() as isize);
                }
                SendMessageW(item(IDC_WEIGHT), 0x014E, TEXT_WEIGHT.load(Ordering::Relaxed) as usize, 0);
                let mode = match (FOLLOW_WINDOWS.load(Ordering::Relaxed), DARK.load(Ordering::Relaxed)) {
                    (true, _) => IDC_MATCH,
                    (false, false) => IDC_LIGHT,
                    (false, true) => IDC_DARK,
                };
                SendMessageW(item(mode), 0x00F1, 1, 0); // BM_SETCHECK
                EDIT_FG.store(DARK_TEXT_COLOR.load(Ordering::Relaxed), Ordering::Relaxed);
                EDIT_BG.store(DARK_BG_COLOR.load(Ordering::Relaxed), Ordering::Relaxed);
                set_swatch(dlg, 0, EDIT_FG.load(Ordering::Relaxed));
                set_swatch(dlg, 1, EDIT_BG.load(Ordering::Relaxed));
                1
            }
            0x0138 => { // WM_CTLCOLORSTATIC: paint the swatches in their colors
                for (i, id) in [IDC_FG_SWATCH, IDC_BG_SWATCH].into_iter().enumerate() {
                    if l == GetDlgItem(dlg, id as i32) {
                        return SWATCHES[i].load(Ordering::Relaxed);
                    }
                }
                0
            }
            0x0111 => { // WM_COMMAND
                match (w & 0xFFFF) as u16 {
                    IDC_FG | IDC_BG => {
                        let (i, cur) = if (w & 0xFFFF) as u16 == IDC_FG { (0, &EDIT_FG) } else { (1, &EDIT_BG) };
                        if let Some(c) = choose_color(dlg, cur.load(Ordering::Relaxed)) {
                            cur.store(c, Ordering::Relaxed);
                            set_swatch(dlg, i, c);
                        }
                    }
                    IDC_RESET => {
                        EDIT_FG.store(DARK_TEXT, Ordering::Relaxed);
                        EDIT_BG.store(DARK_BG, Ordering::Relaxed);
                        set_swatch(dlg, 0, DARK_TEXT);
                        set_swatch(dlg, 1, DARK_BG);
                    }
                    1 => { // OK: apply everything
                        let pick = |id: u16| SendMessageW(GetDlgItem(dlg, id as i32), 0x0147, 0, 0); // CB_GETCURSEL
                        if let Some(&s) = usize::try_from(pick(IDC_SIZE)).ok().and_then(|i| SIZES.get(i)) {
                            TEXT_SIZE.store(s, Ordering::Relaxed);
                        }
                        if let Ok(i) = usize::try_from(pick(IDC_WEIGHT)) {
                            TEXT_WEIGHT.store(i.min(WEIGHTS.len() - 1) as u32, Ordering::Relaxed);
                        }
                        let checked = |id: u16| SendMessageW(GetDlgItem(dlg, id as i32), 0x00F0, 0, 0) == 1; // BM_GETCHECK
                        FOLLOW_WINDOWS.store(checked(IDC_MATCH), Ordering::Relaxed);
                        DARK.store(if checked(IDC_MATCH) { windows_dark() } else { checked(IDC_DARK) }, Ordering::Relaxed);
                        DARK_TEXT_COLOR.store(EDIT_FG.load(Ordering::Relaxed), Ordering::Relaxed);
                        DARK_BG_COLOR.store(EDIT_BG.load(Ordering::Relaxed), Ordering::Relaxed);
                        EndDialog(dlg, 1);
                    }
                    2 => {
                        EndDialog(dlg, 0); // Cancel
                    }
                    _ => {}
                }
                1
            }
            _ => 0,
        }
    }

    // ---- find ----

    fn find_state() -> &'static mut Find {
        let mut p = FIND.load(Ordering::Relaxed);
        if p.is_null() {
            // allocated once and never freed: the find dialog holds pointers into it
            let f = Box::leak(Box::new(Find { fr: unsafe { std::mem::zeroed() }, text: [0; 256] }));
            f.fr.size = std::mem::size_of::<FindReplace>() as u32;
            f.fr.flags = FR_DOWN;
            f.fr.find_what = f.text.as_mut_ptr();
            f.fr.find_what_len = f.text.len() as u16;
            p = f;
            FIND.store(p, Ordering::Relaxed);
        }
        // SAFETY: only used on the UI thread, one borrow at a time
        unsafe { &mut *p }
    }

    fn find_dialog() {
        let dlg = FIND_DLG.load(Ordering::Relaxed);
        if dlg != 0 {
            unsafe { SetFocus(dlg) };
            return;
        }
        let f = find_state();
        f.fr.owner = WIN.load(Ordering::Relaxed);
        FIND_DLG.store(unsafe { FindTextW(&mut f.fr) }, Ordering::Relaxed);
    }

    /// Find the dialog's text after (or before) the selection, wrapping around.
    fn find_next(down: bool) {
        let f = find_state();
        if f.text[0] == 0 {
            find_dialog();
            return;
        }
        let flags = (f.fr.flags & FR_SEARCH_FLAGS & !FR_DOWN) | if down { FR_DOWN } else { 0 };
        let mut sel = CharRange::default();
        unsafe { SendMessageW(edit(), EM_EXGETSEL, 0, &mut sel as *mut _ as isize) };
        let len = unsafe {
            let tl = TextLengthEx { flags: 0, codepage: 1200 };
            SendMessageW(edit(), EM_GETTEXTLENGTHEX, &tl as *const _ as usize, 0) as i32
        };
        // down: from the selection's end to the end, then from the start;
        // up: from the selection's start back to the start, then from the end
        let tries = if down { [CharRange { min: sel.max, max: -1 }, CharRange { min: 0, max: -1 }] } else { [CharRange { min: sel.min, max: 0 }, CharRange { min: len, max: 0 }] };
        for chrg in tries {
            let mut ft = FindTextEx { chrg, text: f.text.as_ptr(), found: CharRange::default() };
            if unsafe { SendMessageW(edit(), EM_FINDTEXTEXW, flags as usize, &mut ft as *mut _ as isize) } >= 0 {
                unsafe {
                    SendMessageW(edit(), EM_EXSETSEL, 0, &ft.found as *const _ as isize);
                    SendMessageW(edit(), EM_SCROLLCARET, 0, 0);
                }
                return;
            }
        }
        unsafe { MessageBeep(0) };
    }

    // ---- print ----

    fn print() {
        let mut pd: PrintDlg = unsafe { std::mem::zeroed() };
        pd.size = std::mem::size_of::<PrintDlg>() as u32;
        pd.owner = WIN.load(Ordering::Relaxed);
        // PD_RETURNDC | PD_NOSELECTION | PD_NOPAGENUMS | PD_USEDEVMODECOPIESANDCOLLATE
        pd.flags = 0x100 | 0x4 | 0x8 | 0x40000;
        if unsafe { PrintDlgW(&mut pd) } == 0 || pd.dc == 0 {
            return;
        }
        unsafe {
            let dc = pd.dc;
            // device units -> twips; the page and its 1" margins, relative to
            // the printable area's corner
            let caps = |i| GetDeviceCaps(dc, i) as i64;
            let tw = |px: i64, dpi: i64| (px * 1440 / dpi.max(1)) as i32;
            let (dx, dy) = (caps(88), caps(90)); // LOGPIXELSX, LOGPIXELSY
            let (pw, ph) = (tw(caps(110), dx), tw(caps(111), dy)); // PHYSICALWIDTH, PHYSICALHEIGHT
            let (ox, oy) = (tw(caps(112), dx), tw(caps(113), dy)); // PHYSICALOFFSETX, PHYSICALOFFSETY
            let page = Rect { left: 0, top: 0, right: pw, bottom: ph };
            let rc = Rect { left: 1440 - ox, top: 1440 - oy, right: pw - 1440 - ox, bottom: ph - 1440 - oy };
            let mut title = [0u16; 256];
            let n = (SendMessageW(WIN.load(Ordering::Relaxed), 0x000D, title.len() - 1, title.as_mut_ptr() as isize)) as usize; // WM_GETTEXT
            let di = DocInfo { size: std::mem::size_of::<DocInfo>() as i32, name: if n > 0 { title.as_ptr() } else { null() }, output: null(), datatype: null(), kind: 0 };
            let tl = TextLengthEx { flags: 0, codepage: 1200 };
            let len = SendMessageW(edit(), EM_GETTEXTLENGTHEX, &tl as *const _ as usize, 0) as i32;
            let dark = DARK.load(Ordering::Relaxed);
            if dark {
                text_color(None); // print dark mode's light text as black
            }
            if rc.right > rc.left && rc.bottom > rc.top && StartDocW(dc, &di) > 0 {
                let mut fr = FormatRange { dc, target: dc, rc, page, chrg: CharRange { min: 0, max: -1 } };
                let mut ok = true;
                while ok && fr.chrg.min < len {
                    ok = StartPage(dc) > 0;
                    fr.rc = rc; // EM_FORMATRANGE shrinks it to what it used
                    let next = SendMessageW(edit(), EM_FORMATRANGE, 1, &mut fr as *mut _ as isize) as i32;
                    ok &= EndPage(dc) > 0;
                    if next <= fr.chrg.min {
                        break; // no progress: nothing more fits
                    }
                    fr.chrg.min = next;
                }
                SendMessageW(edit(), EM_FORMATRANGE, 0, 0); // free RichEdit's cached printer data
                if ok {
                    EndDoc(dc);
                } else {
                    AbortDoc(dc);
                }
            }
            if dark {
                text_color(Some(DARK_TEXT_COLOR.load(Ordering::Relaxed)));
            }
            DeleteDC(dc);
            GlobalFree(pd.dev_mode);
            GlobalFree(pd.dev_names);
        }
    }

    // ---- the window ----

    unsafe extern "system" fn proc_(h: Hwnd, m: u32, w: usize, l: isize) -> isize {
        match m {
            WM_SIZE => {
                let (cw, ch) = ((l & 0xFFFF) as i32, ((l >> 16) & 0xFFFF) as i32);
                MoveWindow(edit(), 0, 0, cw, ch, 1);
                let r = Rect { left: scaled(48), top: scaled(32), right: cw - scaled(48), bottom: ch - scaled(16) };
                SendMessageW(edit(), EM_SETRECT, 0, &r as *const _ as isize);
                0
            }
            WM_SETFOCUS => {
                SetFocus(edit());
                0
            }
            // dark mode's owner-drawn menus (ODT_MENU)
            0x002C if (*(l as *const MeasureItem)).ctl_type == 1 => { // WM_MEASUREITEM
                measure_menu(&mut *(l as *mut MeasureItem));
                1
            }
            0x002B if (*(l as *const DrawItem)).ctl_type == 1 => { // WM_DRAWITEM
                draw_menu(&*(l as *const DrawItem));
                1
            }
            0x0120 if DARK.load(Ordering::Relaxed) => menu_char(w as u16, l), // WM_MENUCHAR
            // Windows' light/dark app mode changed (Settings > Colors)
            0x001A if FOLLOW_WINDOWS.load(Ordering::Relaxed) && l != 0 && wstr_eq(l as *const u16, "ImmersiveColorSet") => { // WM_SETTINGCHANGE
                let dark = windows_dark();
                if dark != DARK.swap(dark, Ordering::Relaxed) {
                    apply_theme();
                }
                0
            }
            0x0085 | 0x0086 => { // WM_NCPAINT, WM_NCACTIVATE
                let r = DefWindowProcW(h, m, w, l);
                if DARK.load(Ordering::Relaxed) {
                    menu_bar_line(h);
                }
                r
            }
            WM_COMMAND => {
                match w & 0xFFFF {
                    ID_OPEN => open_dialog(),
                    ID_PRINT => print(),
                    ID_EXIT => {
                        DestroyWindow(h);
                    }
                    ID_COPY => {
                        SendMessageW(edit(), WM_COPY, 0, 0);
                    }
                    ID_SELECT_ALL => {
                        SendMessageW(edit(), EM_SETSEL, 0, -1);
                    }
                    ID_FIND => find_dialog(),
                    ID_FIND_NEXT => find_next(true),
                    ID_FIND_PREV => find_next(false),
                    ID_ZOOM_IN => step_zoom(1),
                    ID_ZOOM_OUT => step_zoom(-1),
                    ID_ZOOM_RESET => set_zoom(100),
                    ID_DARK => {
                        FOLLOW_WINDOWS.store(false, Ordering::Relaxed);
                        DARK.fetch_xor(true, Ordering::Relaxed);
                        apply_theme();
                    }
                    ID_PREFS => prefs_dialog(),
                    _ => {}
                }
                0
            }
            WM_DROPFILES => {
                let mut file = [0u16; MAX_PATH];
                let n = DragQueryFileW(w as isize, 0, file.as_mut_ptr(), MAX_PATH as u32) as usize;
                if n > 0 && n < MAX_PATH {
                    load(&file[..n]);
                }
                DragFinish(w as isize);
                0
            }
            WM_DESTROY => {
                save_settings();
                PostQuitMessage(0);
                0
            }
            _ if m != 0 && m == FIND_MSG.load(Ordering::Relaxed) => {
                let flags = find_state().fr.flags;
                if flags & FR_DIALOGTERM != 0 {
                    FIND_DLG.store(0, Ordering::Relaxed);
                } else if flags & FR_FINDNEXT != 0 {
                    find_next(flags & FR_DOWN != 0);
                }
                0
            }
            _ => DefWindowProcW(h, m, w, l),
        }
    }

    pub fn run() {
        unsafe {
            // Only load DLLs from System32, never from the exe's folder (e.g. Downloads).
            SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
            SetProcessDPIAware();
            if LoadLibraryExW(wide("Msftedit.dll").as_ptr(), 0, LOAD_LIBRARY_SEARCH_SYSTEM32) == 0 {
                return;
            }
            FIND_MSG.store(RegisterWindowMessageW(wide("commdlg_FindReplace").as_ptr()), Ordering::Relaxed);
            let inst = GetModuleHandleW(null());
            let class = wide("WordViewer");
            let wc = WndClass {
                style: 0,
                proc_,
                cls_extra: 0,
                wnd_extra: 0,
                instance: inst,
                icon: LoadIconW(inst, 1),      // the embedded icon (assets/word-viewer.rc)
                cursor: LoadCursorW(0, 32512), // IDC_ARROW
                background: 0,
                menu_name: null(),
                class_name: class.as_ptr(),
            };
            RegisterClassW(&wc);

            let bar = build_menu(false); // apply_theme rebuilds it for dark mode

            // Default size: 1000 x 1150 at 100% scaling, fitted to the work area and centered.
            let mut work = Rect::default();
            SystemParametersInfoW(0x0030, 0, &mut work as *mut _ as *mut c_void, 0); // SPI_GETWORKAREA
            let dpi = GetDpiForSystem().max(96) as i32;
            let (ww, wh) = (work.right - work.left, work.bottom - work.top);
            let (w, h) = ((1000 * dpi / 96).min(ww * 9 / 10).max(400), (1150 * dpi / 96).min(wh * 95 / 100).max(300));
            let win = CreateWindowExW(
                0x10, // WS_EX_ACCEPTFILES
                class.as_ptr(), wide("Word Viewer").as_ptr(),
                0x00CF_0000, // WS_OVERLAPPEDWINDOW
                work.left + (ww - w) / 2, work.top + (wh - h) / 2, w, h,
                0, bar, inst, null());
            WIN.store(win, Ordering::Relaxed);
            SetWindowsHookExW(5, cbt_hook, 0, GetCurrentThreadId()); // WH_CBT, this thread only
            let edit = CreateWindowExW(
                0, wide("RICHEDIT50W").as_ptr(), wide("").as_ptr(),
                // child visible vscroll multiline readonly autovscroll nohidesel
                0x4000_0000 | 0x1000_0000 | 0x0020_0000 | 0x4 | 0x800 | 0x40 | 0x100,
                0, 0, 0, 0, win, 0, inst, null());
            EDIT.store(edit, Ordering::Relaxed);
            SendMessageW(edit, EM_EXLIMITTEXT, 0, 0x7FFF_FFFF);

            let saved = load_settings();
            let chosen = saved.as_ref().and_then(|s| s.dark);
            FOLLOW_WINDOWS.store(chosen.is_none(), Ordering::Relaxed);
            DARK.store(chosen.unwrap_or_else(windows_dark), Ordering::Relaxed);
            if let Some(s) = &saved {
                set_zoom(s.zoom);
                if let Some((fg, bg)) = s.colors {
                    DARK_TEXT_COLOR.store(fg, Ordering::Relaxed);
                    DARK_BG_COLOR.store(bg, Ordering::Relaxed);
                }
                if let Some((size, weight)) = s.text {
                    TEXT_SIZE.store(size, Ordering::Relaxed);
                    TEXT_WEIGHT.store(weight, Ordering::Relaxed);
                }
            }
            apply_theme(); // before the window shows, so dark mode doesn't flash white
            let mut argc = 0;
            let argv = CommandLineToArgvW(GetCommandLineW(), &mut argc);
            if !argv.is_null() && argc > 1 {
                let arg = *argv.add(1);
                load(std::slice::from_raw_parts(arg, wlen(arg, 32768)));
            } else {
                show_rtf("{\\rtf1\\fs22 Use File > Open (Ctrl+O), or drag a document onto this window.}");
            }
            if !argv.is_null() {
                LocalFree(argv as *mut c_void);
            }

            match saved {
                // Windows moves a saved position that's now off-screen (say, a
                // monitor was unplugged) back onto one.
                Some(s) => {
                    let wp = WindowPlacement {
                        length: std::mem::size_of::<WindowPlacement>() as u32,
                        flags: 0,
                        show_cmd: if s.maximized { 3 } else { 1 }, // SW_SHOWMAXIMIZED, SW_SHOWNORMAL
                        min_pos: [-1, -1],
                        max_pos: [-1, -1],
                        normal: s.normal,
                    };
                    SetWindowPlacement(win, &wp);
                }
                None => {
                    ShowWindow(win, 10); // SW_SHOWDEFAULT
                }
            }
            SetFocus(edit);

            let (ctrl, shift) = (0x01 | 0x08, 0x01 | 0x04); // FVIRTKEY | FCONTROL, FVIRTKEY | FSHIFT
            let keys = [
                (ctrl, b'O' as u16, ID_OPEN),
                (ctrl, b'P' as u16, ID_PRINT),
                (ctrl, b'F' as u16, ID_FIND),
                (0x01, 0x72, ID_FIND_NEXT),  // F3
                (shift, 0x72, ID_FIND_PREV), // Shift+F3
                (ctrl, 0xBB, ID_ZOOM_IN),    // Ctrl+= / Ctrl++ on the main keyboard
                (ctrl, 0x6B, ID_ZOOM_IN),    // numpad +
                (ctrl, 0xBD, ID_ZOOM_OUT),   // Ctrl+-
                (ctrl, 0x6D, ID_ZOOM_OUT),   // numpad -
                (ctrl, b'0' as u16, ID_ZOOM_RESET),
                (ctrl, 0x60, ID_ZOOM_RESET), // numpad 0
                (ctrl, b'D' as u16, ID_DARK),
            ];
            let accels: Vec<Accel> = keys.iter().map(|&(virt, key, cmd)| Accel { virt, key, cmd: cmd as u16 }).collect();
            let hacc = CreateAcceleratorTableW(accels.as_ptr(), accels.len() as i32);
            let mut msg: Msg = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                let dlg = FIND_DLG.load(Ordering::Relaxed);
                if dlg != 0 && IsDialogMessageW(dlg, &msg) != 0 {
                    continue;
                }
                if TranslateAcceleratorW(win, hacc, &msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn text_prefs_scale_sizes_and_change_only_real_control_words() {
            TEXT_SIZE.store(22, Ordering::Relaxed); // twice the default 11 pt
            TEXT_WEIGHT.store(3, Ordering::Relaxed); // Semibold
            // a heading, then document text that looks like RTF (escaped by the library)
            let rtf = r"{\rtf1\ansi\deff0{\fonttbl{\f0 Segoe UI;}}\fs22 {\b\fs40 Title}\par \\fs22 \{\\fs40\} \fi-360 x}";
            assert_eq!(with_text_prefs(rtf), r"{\rtf1\ansi\deff0{\fonttbl{\f0 Segoe UI Semibold;}}\fs44 {\b\fs80 Title}\par \\fs22 \{\\fs40\} \fi-360 x}");
            TEXT_SIZE.store(DEFAULT_SIZE, Ordering::Relaxed);
            TEXT_WEIGHT.store(DEFAULT_WEIGHT, Ordering::Relaxed);
            assert_eq!(with_text_prefs(rtf), rtf);
        }
    }
}
