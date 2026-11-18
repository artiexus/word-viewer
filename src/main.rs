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

    #[repr(C)]
    struct LogFont {
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u8,
        underline: u8,
        strike_out: u8,
        charset: u8,
        out_precision: u8,
        clip_precision: u8,
        quality: u8,
        pitch_and_family: u8,
        face: [u16; 32],
    }

    #[repr(C)]
    struct PaintStruct {
        dc: isize,
        erase: i32,
        paint: Rect,
        restore: i32,
        inc_update: i32,
        reserved: [u8; 32],
    }

    #[repr(C)]
    struct ParaFormat2 {
        size: u32,
        mask: u32,
        numbering: u16,
        effects: u16,
        start_indent: i32,
        right_indent: i32,
        offset: i32,
        alignment: u16,
        tab_count: i16,
        tabs: [i32; 32],
        space_before: i32,
        space_after: i32,
        line_spacing: i32,
        style: i16,
        line_spacing_rule: u8,
        outline_level: u8,
        shading_weight: u16,
        shading_style: u16,
        numbering_start: u16,
        numbering_style: u16,
        numbering_tab: u16,
        border_space: u16,
        border_width: u16,
        borders: u16,
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
        fn GetWindowTextW(h: Hwnd, s: *mut u16, n: i32) -> i32;
        fn GetSysColor(index: i32) -> u32;
        fn EnableWindow(h: Hwnd, enable: i32) -> i32;
        fn GetWindowLongPtrW(h: Hwnd, index: i32) -> isize;
        fn RedrawWindow(h: Hwnd, r: *const Rect, region: isize, flags: u32) -> i32;
        fn BeginPaint(h: Hwnd, ps: *mut PaintStruct) -> isize;
        fn EndPaint(h: Hwnd, ps: *const PaintStruct) -> i32;
        fn MapWindowPoints(from: Hwnd, to: Hwnd, pts: *mut i32, n: u32) -> i32;
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
        fn CreatePen(style: i32, width: i32, color: u32) -> isize;
        fn GetStockObject(i: i32) -> isize;
        fn RoundRect(dc: isize, l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> i32;
        fn GetObjectW(h: isize, n: i32, out: *mut c_void) -> i32;
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
    /// Text sizes offered, in points.
    const SIZES: [u32; 10] = [9, 10, 11, 12, 13, 14, 16, 18, 20, 24];
    const WEIGHTS: [&str; 4] = ["Light", "Semilight", "Regular", "Semibold"];
    /// Typefaces offered (all come with Windows), and each one's face for
    /// each weight, if it has one.
    const FONTS: [(&str, [Option<&str>; 4]); 4] = [
        ("Segoe UI", [Some("Segoe UI Light"), Some("Segoe UI Semilight"), Some("Segoe UI"), Some("Segoe UI Semibold")]),
        ("Calibri", [Some("Calibri Light"), None, Some("Calibri"), None]),
        ("Georgia", [None, None, Some("Georgia"), None]),
        ("Sitka Text", [None, None, Some("Sitka Text"), None]),
    ];
    /// Line spacing, in twentieths of a line.
    const SPACINGS: [(&str, u32); 4] = [("Single", 20), ("1.15", 23), ("1.3", 26), ("1.5", 30)];
    /// The longest a line may get, in average characters (0: the window's width).
    const LENGTHS: [(&str, u32); 4] = [("Full", 0), ("Wide", 100), ("Medium", 80), ("Narrow", 65)];
    const MODES: [&str; 3] = ["Match Windows", "Light", "Dark"];
    const DEFAULT_SIZE: u32 = 11;
    const DEFAULT_WEIGHT: u32 = 2;
    /// Dark mode colors (COLORREF, 0x00BBGGRR): Windows Terminal's default
    /// "Campbell" scheme, background #0C0C0C and text #CCCCCC.
    const DARK_BG: u32 = 0x000C_0C0C;
    const DARK_TEXT: u32 = 0x00CC_CCCC;
    /// Windows' own color, which light mode uses unless a scheme is chosen.
    const AUTO: u32 = u32::MAX;

    const fn rgb(r: u32, g: u32, b: u32) -> u32 {
        r | g << 8 | b << 16
    }

    /// Color schemes for each mode: name, text and background.
    const LIGHT_SCHEMES: [(&str, u32, u32); 4] = [
        ("White", AUTO, AUTO),
        ("Paper", rgb(0x26, 0x26, 0x26), rgb(0xFA, 0xF7, 0xF0)),
        ("Sepia", rgb(0x5B, 0x46, 0x36), rgb(0xF4, 0xEC, 0xD8)),
        ("Gray", rgb(0x1E, 0x1E, 0x1E), rgb(0xE3, 0xE3, 0xE3)),
    ];
    const DARK_SCHEMES: [(&str, u32, u32); 4] = [
        ("Black", DARK_TEXT, DARK_BG),
        ("Charcoal", rgb(0xD4, 0xD4, 0xD4), rgb(0x1E, 0x1E, 0x1E)),
        ("Midnight", rgb(0xC9, 0xD1, 0xD9), rgb(0x0D, 0x11, 0x17)),
        ("Warm", rgb(0xE3, 0xD5, 0xBC), rgb(0x1F, 0x1B, 0x16)),
    ];
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
    static LIGHT_TEXT_COLOR: AtomicU32 = AtomicU32::new(AUTO);
    static LIGHT_BG_COLOR: AtomicU32 = AtomicU32::new(AUTO);
    /// Indexes into FONTS, SPACINGS and LENGTHS.
    static TEXT_FONT: AtomicU32 = AtomicU32::new(0);
    static LINE_SPACING: AtomicU32 = AtomicU32::new(0);
    static LINE_LENGTH: AtomicU32 = AtomicU32::new(0);
    static VIEW_MENU: AtomicIsize = AtomicIsize::new(0);

    /// Everything View > Text Preferences sets.
    #[derive(Clone, Copy, PartialEq)]
    struct Prefs {
        font: usize,
        size: u32,
        weight: usize,
        spacing: usize,
        length: usize,
        /// 0 match Windows, 1 always light, 2 always dark
        mode: usize,
        /// Each mode's text and background colors.
        light: (u32, u32),
        dark: (u32, u32),
    }

    impl Prefs {
        const DEFAULT: Prefs = Prefs { font: 0, size: DEFAULT_SIZE, weight: DEFAULT_WEIGHT as usize, spacing: 0, length: 0, mode: 0, light: (AUTO, AUTO), dark: (DARK_TEXT, DARK_BG) };

        fn current() -> Prefs {
            let get = |a: &AtomicU32| a.load(Ordering::Relaxed);
            Prefs {
                font: (get(&TEXT_FONT) as usize).min(FONTS.len() - 1),
                size: get(&TEXT_SIZE),
                weight: (get(&TEXT_WEIGHT) as usize).min(WEIGHTS.len() - 1),
                spacing: (get(&LINE_SPACING) as usize).min(SPACINGS.len() - 1),
                length: (get(&LINE_LENGTH) as usize).min(LENGTHS.len() - 1),
                mode: match (FOLLOW_WINDOWS.load(Ordering::Relaxed), DARK.load(Ordering::Relaxed)) {
                    (true, _) => 0,
                    (false, false) => 1,
                    (false, true) => 2,
                },
                light: (get(&LIGHT_TEXT_COLOR), get(&LIGHT_BG_COLOR)),
                dark: (get(&DARK_TEXT_COLOR), get(&DARK_BG_COLOR)),
            }
        }

        fn apply(self) {
            let set = |a: &AtomicU32, v: u32| a.store(v, Ordering::Relaxed);
            set(&TEXT_FONT, self.font as u32);
            set(&TEXT_SIZE, self.size);
            set(&TEXT_WEIGHT, self.weight as u32);
            set(&LINE_SPACING, self.spacing as u32);
            set(&LINE_LENGTH, self.length as u32);
            set(&LIGHT_TEXT_COLOR, self.light.0);
            set(&LIGHT_BG_COLOR, self.light.1);
            set(&DARK_TEXT_COLOR, self.dark.0);
            set(&DARK_BG_COLOR, self.dark.1);
            FOLLOW_WINDOWS.store(self.mode == 0, Ordering::Relaxed);
            DARK.store(self.dark(), Ordering::Relaxed);
        }

        fn dark(&self) -> bool {
            match self.mode {
                0 => windows_dark(),
                m => m == 2,
            }
        }

        /// The weight shown: the chosen one if the font has it, else Regular.
        fn shown_weight(&self) -> usize {
            if FONTS[self.font].1[self.weight].is_some() { self.weight } else { DEFAULT_WEIGHT as usize }
        }

        fn face(&self) -> &'static str {
            FONTS[self.font].1[self.shown_weight()].unwrap_or(FONTS[self.font].0)
        }
    }

    /// The current mode's text and background colors (either may be AUTO).
    fn colors() -> (u32, u32) {
        let get = |a: &AtomicU32| a.load(Ordering::Relaxed);
        if DARK.load(Ordering::Relaxed) {
            (get(&DARK_TEXT_COLOR), get(&DARK_BG_COLOR))
        } else {
            (get(&LIGHT_TEXT_COLOR), get(&LIGHT_BG_COLOR))
        }
    }

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
        /// Font, line spacing and line length (indexes), and light mode's
        /// text and background colors.
        more: Option<(u32, u32, u32, u32, u32)>,
    }

    fn load_settings() -> Option<Settings> {
        let mut s = String::new();
        File::open(settings_path()?).ok()?.take(512).read_to_string(&mut s).ok()?;
        let v: Vec<i64> = s.split_whitespace().map(|x| x.parse().ok()).collect::<Option<_>>()?;
        let &[l, t, r, b, max, zoom] = v.get(..6)? else { return None };
        let [l, t, r, b] = [l, t, r, b].map(|x| x.clamp(-100_000, 100_000) as i32);
        // later fields were added over time: dark mode (0 follow Windows,
        // 1 dark, 2 light), its colors, text size and weight, then font, line
        // spacing, line length and light mode's colors (-1: Windows' own)
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
        let index = |v: i64, n: usize| v.clamp(0, n as i64 - 1) as u32;
        let color = |c: i64| if c < 0 { AUTO } else { (c & 0xFF_FFFF) as u32 };
        let more = match v.get(11..16) {
            Some(&[font, spacing, length, fg, bg]) => Some((index(font, FONTS.len()), index(spacing, SPACINGS.len()), index(length, LENGTHS.len()), color(fg), color(bg))),
            _ => None,
        };
        let ok = r - l >= 200 && b - t >= 150 && r - l <= 32767 && b - t <= 32767;
        ok.then_some(Settings { normal: Rect { left: l, top: t, right: r, bottom: b }, maximized: max != 0, zoom: zoom.clamp(10, 500) as u32, dark, colors, text, more })
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
        let p = Prefs::current();
        let color = |c: u32| if c == AUTO { -1 } else { c as i64 };
        let (lfg, lbg) = (color(p.light.0), color(p.light.1));
        let line = format!(
            "{} {} {} {} {} {} {dark} {fg} {bg} {size} {weight} {} {} {} {lfg} {lbg}\n",
            n.left, n.top, n.right, n.bottom, (wp.show_cmd == 3) as u8, zoom(), p.font, p.spacing, p.length
        );
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
        let p = Prefs::current();
        with_text_prefs_for(rtf, p.size, p.face())
    }

    fn with_text_prefs_for(rtf: &str, size: u32, face: &str) -> String {
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
            let spacing = LINE_SPACING.load(Ordering::Relaxed) as usize;
            if spacing > 0 {
                line_spacing(edit(), SPACINGS[spacing.min(SPACINGS.len() - 1)].1);
            }
            SendMessageW(edit(), EM_SETSEL, 0, 0);
            SendMessageW(edit(), EM_SCROLLCARET, 0, 0);
        }
        // new text has the default (automatic) color
        let (fg, _) = colors();
        if fg != AUTO {
            text_color(edit(), Some(fg));
        }
        set_zoom(z);
    }

    /// Set every paragraph's line spacing, in twentieths of a line.
    fn line_spacing(h: Hwnd, twentieths: u32) {
        unsafe {
            let mut sel = CharRange::default();
            SendMessageW(h, EM_EXGETSEL, 0, &mut sel as *mut _ as isize);
            SendMessageW(h, EM_SETSEL, 0, -1);
            let mut pf: ParaFormat2 = std::mem::zeroed();
            pf.size = std::mem::size_of::<ParaFormat2>() as u32;
            pf.mask = 0x100; // PFM_LINESPACING
            pf.line_spacing = twentieths as i32;
            pf.line_spacing_rule = 5; // in twentieths of a line
            SendMessageW(h, 0x0447, 0, &pf as *const _ as isize); // EM_SETPARAFORMAT
            SendMessageW(h, EM_EXSETSEL, 0, &sel as *const _ as isize);
        }
    }

    /// Average width of a character of running text in `face`, per 1000 of the font's size.
    unsafe fn char_width(face: &str) -> i64 {
        let mut lf: LogFont = std::mem::zeroed();
        lf.height = -1000;
        set_face(&mut lf, face);
        let font = CreateFontIndirectW(&lf as *const _ as *const c_void);
        let win = WIN.load(Ordering::Relaxed);
        let dc = GetDC(win);
        let old = SelectObject(dc, font);
        let sample: Vec<u16> = "Each paragraph is shown in the font, size and weight you choose, so long reads stay easy.".encode_utf16().collect();
        let w = text_width(dc, &sample) as i64;
        SelectObject(dc, old);
        ReleaseDC(win, dc);
        DeleteObject(font);
        w / sample.len() as i64
    }

    /// Place the text in the window: 48 px margins, and centered in a
    /// narrower column when the line length is limited.
    fn layout() {
        unsafe {
            let win = WIN.load(Ordering::Relaxed);
            let mut c = Rect::default();
            GetClientRect(win, &mut c);
            let (mut left, mut right) = (scaled(48), c.right - scaled(48));
            let chars = LENGTHS.get(LINE_LENGTH.load(Ordering::Relaxed) as usize).map_or(0, |l| l.1);
            if chars > 0 {
                // characters x their width at the text size, zoom and display DPI
                let dpi = GetDpiForWindow(win).max(96) as i64;
                let size = TEXT_SIZE.load(Ordering::Relaxed) as i64;
                let width = chars as i64 * char_width(Prefs::current().face()) * size * dpi * zoom() as i64 / (1000 * 72 * 100);
                let spare = (right - left) as i64 - width;
                if spare > 0 {
                    left += (spare / 2) as i32;
                    right -= (spare - spare / 2) as i32;
                }
            }
            let r = Rect { left, top: scaled(32), right, bottom: c.bottom - scaled(16) };
            SendMessageW(edit(), EM_SETRECT, 0, &r as *const _ as isize);
        }
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
        layout(); // a limited line length is in characters, so it grows with the zoom
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
    fn text_color(h: Hwnd, color: Option<u32>) {
        let mut cf: CharFormat = unsafe { std::mem::zeroed() };
        cf.size = std::mem::size_of::<CharFormat>() as u32;
        cf.mask = 0x4000_0000; // CFM_COLOR
        match color {
            Some(c) => cf.color = c,
            None => cf.effects = 0x4000_0000, // CFE_AUTOCOLOR
        }
        unsafe { SendMessageW(h, 0x0444, 4, &cf as *const _ as isize) }; // EM_SETCHARFORMAT, SCF_ALL
    }

    /// Show a RichEdit's text and background in these colors (AUTO: Windows' own).
    fn set_colors(h: Hwnd, (fg, bg): (u32, u32)) {
        // EM_SETBKGNDCOLOR: wParam 1 is the system color
        unsafe { SendMessageW(h, 0x0443, (bg == AUTO) as usize, if bg == AUTO { 0 } else { bg as isize }) };
        text_color(h, (fg != AUTO).then_some(fg));
    }

    /// A color, or for AUTO the system color `index` (5 window, 8 window text).
    fn resolve(color: u32, index: i32) -> u32 {
        if color == AUTO { unsafe { GetSysColor(index) } } else { color }
    }

    /// The window's title bar in dark or light mode.
    unsafe fn dark_title(h: Hwnd, dark: bool) {
        // Windows 10 20H1+; 19 on 1809 to 1909
        if let Some(f) = optional("dwmapi.dll", b"DwmSetWindowAttribute\0") {
            let set: unsafe extern "system" fn(Hwnd, u32, *const c_void, u32) -> i32 = std::mem::transmute(f);
            let on = dark as i32;
            if set(h, 20, &on as *const _ as *const c_void, 4) != 0 {
                set(h, 19, &on as *const _ as *const c_void, 4);
            }
            // redraw the frame: SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_FRAMECHANGED
            SetWindowPos(h, 0, 0, 0, 0, 0, 0x1 | 0x2 | 0x4 | 0x20);
            // Windows 10 repaints the title bar only when the window's
            // active state changes: flip it and back
            let active = (GetActiveWindow() == h) as usize;
            SendMessageW(h, 0x0086, active ^ 1, 0); // WM_NCACTIVATE
            SendMessageW(h, 0x0086, active, 0);
        }
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
            set_colors(edit(), colors());
            dark_title(win, dark);
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
        reg_dword(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "AppsUseLightTheme") == Some(0)
    }

    /// A DWORD value under HKEY_CURRENT_USER.
    fn reg_dword(key: &str, value: &str) -> Option<u32> {
        unsafe {
            let f = optional("advapi32.dll", b"RegGetValueW\0")?;
            let get: unsafe extern "system" fn(isize, *const u16, *const u16, u32, *mut u32, *mut c_void, *mut u32) -> i32 = std::mem::transmute(f);
            let (mut v, mut size) = (0u32, 4u32);
            // HKEY_CURRENT_USER, RRF_RT_REG_DWORD
            let found = get(0x8000_0001u32 as i32 as isize, wide(key).as_ptr(), wide(value).as_ptr(), 0x10, null_mut(), &mut v as *mut _ as *mut c_void, &mut size) == 0;
            found.then_some(v)
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

    /// The dialog's rows of choices: each row's first control id, and its
    /// number of choices (ids follow on).
    const G_FONT: u16 = 200;
    const G_WEIGHT: u16 = 220;
    const G_SPACING: u16 = 230;
    const G_LENGTH: u16 = 240;
    const G_MODE: u16 = 250;
    const G_LIGHT: u16 = 260;
    const G_DARK: u16 = 270;
    const GROUPS: [(u16, usize); 7] = [
        (G_FONT, FONTS.len()), (G_WEIGHT, WEIGHTS.len()), (G_SPACING, SPACINGS.len()), (G_LENGTH, LENGTHS.len()),
        (G_MODE, MODES.len()), (G_LIGHT, LIGHT_SCHEMES.len()), (G_DARK, DARK_SCHEMES.len()),
    ];
    const IDC_SMALLER: u16 = 210;
    const IDC_SIZE: u16 = 211;
    const IDC_LARGER: u16 = 212;
    const IDC_LIGHT_FG: u16 = 266;
    const IDC_LIGHT_BG: u16 = 267;
    const IDC_DARK_FG: u16 = 276;
    const IDC_DARK_BG: u16 = 277;
    const IDC_RESTORE: u16 = 280;
    const IDC_HEAD_TEXT: u16 = 290;
    const IDC_HEAD_LOOK: u16 = 291;
    const IDC_HEAD_PREVIEW: u16 = 292;
    const IDC_HINT: u16 = 293;
    const IDC_PREVIEW: u16 = 300;
    const WS_TABSTOP: isize = 0x0001_0000;

    /// The preview: a heading and two paragraphs, as the library writes them.
    const SAMPLE: &str = concat!(
        "{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Segoe UI;}}\\fs22\n",
        "\\pard\\sa160 {\\b\\fs32 The Lighthouse Keeper}\\par\n",
        "\\pard\\sa160 Every evening at dusk, Mara climbed the ninety-two steps to light the lamp. ",
        "The town below never thanked her, but the ships did: each one blinked twice as it passed.\\par\n",
        "\\pard\\sa160 She kept a log of every vessel, with {\\b bold} names for the ones that came home ",
        "and {\\i italic} notes about the weather.\\par\n}"
    );

    /// A button's row and place in it, if it's one of the choices.
    fn choice(id: u16) -> Option<(u16, usize)> {
        GROUPS.iter().find(|g| (g.0..g.0 + g.1 as u16).contains(&id)).map(|g| (g.0, (id - g.0) as usize))
    }

    impl Prefs {
        /// The selected choice in a row (None: colors that match no scheme).
        fn selected(&self, group: u16) -> Option<usize> {
            let scheme = |s: &[(&str, u32, u32)], c: (u32, u32)| s.iter().position(|x| (x.1, x.2) == c);
            match group {
                G_FONT => Some(self.font),
                G_WEIGHT => Some(self.shown_weight()),
                G_SPACING => Some(self.spacing),
                G_LENGTH => Some(self.length),
                G_MODE => Some(self.mode),
                G_LIGHT => scheme(&LIGHT_SCHEMES, self.light),
                _ => scheme(&DARK_SCHEMES, self.dark),
            }
        }

        fn choose(&mut self, group: u16, i: usize) {
            match group {
                G_FONT => self.font = i,
                G_WEIGHT => self.weight = i,
                G_SPACING => self.spacing = i,
                G_LENGTH => self.length = i,
                G_MODE => self.mode = i,
                G_LIGHT => self.light = (LIGHT_SCHEMES[i].1, LIGHT_SCHEMES[i].2),
                _ => self.dark = (DARK_SCHEMES[i].1, DARK_SCHEMES[i].2),
            }
        }
    }

    /// The dialog's colors.
    #[derive(Clone, Copy)]
    struct Palette {
        back: u32,
        chip: u32,
        edge: u32,
        text: u32,
        dim: u32,
        accent: u32,
        on_accent: u32,
    }

    impl Palette {
        const NONE: Palette = Palette { back: 0, chip: 0, edge: 0, text: 0, dim: 0, accent: 0, on_accent: 0 };

        /// Light, or dark from dark mode's colors (as the dark menus are);
        /// selections in Windows' accent color.
        fn new(dark: bool, (fg, bg): (u32, u32)) -> Palette {
            let accent = reg_dword(r"Software\Microsoft\Windows\DWM", "AccentColor").map_or(rgb(0x00, 0x67, 0xC0), |c| c & 0xFF_FFFF);
            // black or white, whichever reads better on `c`
            let on = |c: u32| if (c & 0xFF) * 299 + ((c >> 8) & 0xFF) * 587 + ((c >> 16) & 0xFF) * 114 > 150_000 { 0 } else { 0xFF_FFFF };
            if dark {
                let back = mix(bg, fg, 20);
                let accent = mix(accent, 0xFF_FFFF, 100); // lighter, as Windows does on dark
                Palette { back, chip: mix(back, fg, 22), edge: mix(back, fg, 56), text: fg, dim: mix(fg, back, 110), accent, on_accent: on(accent) }
            } else {
                Palette { back: rgb(0xF3, 0xF3, 0xF3), chip: 0xFF_FFFF, edge: rgb(0xD0, 0xD0, 0xD0), text: rgb(0x1B, 0x1B, 0x1B), dim: rgb(0x60, 0x60, 0x60), accent, on_accent: on(accent) }
            }
        }
    }

    thread_local! {
        /// The choices being made in the dialog, applied on OK.
        static PENDING: std::cell::Cell<Prefs> = const { std::cell::Cell::new(Prefs::DEFAULT) };
        /// Whether the preview shows dark mode: the mode chosen, or whose colors were last changed.
        static PREVIEW_DARK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        static PALETTE: std::cell::Cell<Palette> = const { std::cell::Cell::new(Palette::NONE) };
        /// Whether the dialog is dark now, its background brush and its headings' font.
        static DLG_DARK: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
        static DLG_BRUSH: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
        static HEAD_FONT: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
    }

    /// An in-memory dialog template (DLGTEMPLATE and DLGITEMTEMPLATEs), so the
    /// exe needs no resource script for it.
    struct Template {
        words: Vec<u16>,
        count: u16,
    }

    impl Template {
        fn u32(&mut self, v: u32) {
            self.words.extend([v as u16, (v >> 16) as u16]);
        }

        fn str(&mut self, s: &str) {
            self.words.extend(s.encode_utf16().chain(Some(0)));
        }

        fn align(&mut self) {
            if self.words.len() % 2 == 1 {
                self.words.push(0);
            }
        }

        fn new(title: &str, cx: i16, cy: i16) -> Template {
            let mut t = Template { words: Vec::new(), count: 0 };
            // WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN | DS_MODALFRAME | DS_SETFONT | DS_CENTER
            t.u32(0x8000_0000 | 0x00C0_0000 | 0x0008_0000 | 0x0200_0000 | 0x80 | 0x40 | 0x800);
            t.u32(0);
            t.words.extend([0, 0, 0, cx as u16, cy as u16, 0, 0]); // count (set by finish), x, y, cx, cy, no menu, default class
            t.str(title);
            t.words.push(9); // point size
            t.str("Segoe UI");
            t
        }

        fn control(&mut self, class: &[u16], style: u32, rect: [i16; 4], id: u16, text: &str) {
            self.align();
            self.u32(0x4000_0000 | 0x1000_0000 | style); // WS_CHILD | WS_VISIBLE
            self.u32(0);
            self.words.extend(rect.map(|v| v as u16));
            self.words.push(id);
            self.words.extend(class);
            self.str(text);
            self.words.push(0); // no creation data
            self.count += 1;
        }

        /// A control of a predefined class (0x80 button, 0x82 static).
        fn item(&mut self, class: u16, style: u32, rect: [i16; 4], id: u16, text: &str) {
            self.control(&[0xFFFF, class], style, rect, id, text);
        }

        fn named(&mut self, class: &str, style: u32, rect: [i16; 4], id: u16, text: &str) {
            let class: Vec<u16> = class.encode_utf16().chain(Some(0)).collect();
            self.control(&class, style, rect, id, text);
        }

        /// The template, DWORD-aligned as DialogBoxIndirect needs.
        fn finish(mut self) -> Vec<u32> {
            self.words[4] = self.count;
            self.align();
            self.words.chunks(2).map(|c| c[0] as u32 | (c[1] as u32) << 16).collect()
        }
    }

    const BUTTON: u16 = 0x80;
    const STATIC: u16 = 0x82;
    /// Buttons the dialog draws itself (BS_OWNERDRAW), which say when they
    /// get the focus (BS_NOTIFY).
    const CHIP: u32 = 0xB | 0x4000;
    const TAB: u32 = 0x0001_0000; // WS_TABSTOP
    const GROUP: u32 = 0x0002_0000; // WS_GROUP
    const NONE: u16 = 0xFFFF;
    /// A static's text, vertically centered (SS_CENTERIMAGE).
    const LABEL: u32 = 0x200;

    /// A label, then a row of choices `w` dialog units wide.
    fn choice_row(t: &mut Template, y: i16, label: &str, first: u16, names: &[&str], w: i16) {
        t.item(STATIC, LABEL | GROUP, [10, y, 60, 14], NONE, label);
        for (i, name) in names.iter().enumerate() {
            let group = if i == 0 { GROUP } else { 0 };
            t.item(BUTTON, CHIP | group, [72 + i as i16 * (w + 3), y, w, 14], first + i as u16, name);
        }
    }

    fn prefs_dialog() {
        let mut t = Template::new("Text Preferences", 462, 252);
        // left: the choices
        t.item(STATIC, 0, [10, 8, 240, 10], IDC_HEAD_TEXT, "Text");
        choice_row(&mut t, 22, "&Font", G_FONT, &FONTS.map(|f| f.0), 42);
        t.item(STATIC, LABEL | GROUP, [10, 40, 60, 14], NONE, "Si&ze");
        t.item(BUTTON, CHIP | GROUP | TAB, [72, 40, 22, 14], IDC_SMALLER, "\u{2212}");
        t.item(STATIC, LABEL | 1, [96, 40, 40, 14], IDC_SIZE, ""); // SS_CENTER
        t.item(BUTTON, CHIP | TAB, [138, 40, 22, 14], IDC_LARGER, "+");
        choice_row(&mut t, 58, "&Weight", G_WEIGHT, &WEIGHTS, 42);
        choice_row(&mut t, 76, "Line &spacing", G_SPACING, &SPACINGS.map(|s| s.0), 42);
        choice_row(&mut t, 94, "Line &length", G_LENGTH, &LENGTHS.map(|s| s.0), 42);
        t.item(STATIC, GROUP, [10, 116, 240, 10], IDC_HEAD_LOOK, "Appearance");
        choice_row(&mut t, 130, "&Mode", G_MODE, &MODES, 58);
        choice_row(&mut t, 148, "Li&ght colors", G_LIGHT, &LIGHT_SCHEMES.map(|s| s.0), 42);
        t.item(BUTTON, CHIP | GROUP | TAB, [72, 166, 87, 14], IDC_LIGHT_FG, "Text color\u{2026}");
        t.item(BUTTON, CHIP | TAB, [162, 166, 87, 14], IDC_LIGHT_BG, "Background\u{2026}");
        choice_row(&mut t, 184, "Dar&k colors", G_DARK, &DARK_SCHEMES.map(|s| s.0), 42);
        t.item(BUTTON, CHIP | GROUP | TAB, [72, 202, 87, 14], IDC_DARK_FG, "Text color\u{2026}");
        t.item(BUTTON, CHIP | TAB, [162, 202, 87, 14], IDC_DARK_BG, "Background\u{2026}");
        // right: the preview, read-only and out of the tab order
        t.item(STATIC, GROUP, [268, 8, 184, 10], IDC_HEAD_PREVIEW, "Preview");
        t.named("RICHEDIT50W", 0x4 | 0x800, [270, 26, 178, 186], IDC_PREVIEW, ""); // ES_MULTILINE | ES_READONLY
        // the bottom row
        t.item(BUTTON, CHIP | GROUP | TAB, [10, 230, 80, 14], IDC_RESTORE, "&Restore defaults");
        t.item(STATIC, LABEL, [98, 230, 200, 14], IDC_HINT, "Ctrl+D switches between light and dark.");
        t.item(BUTTON, CHIP | GROUP | TAB, [330, 230, 58, 14], 1, "OK"); // IDOK
        t.item(BUTTON, CHIP | TAB, [394, 230, 58, 14], 2, "Cancel"); // IDCANCEL
        let words = t.finish();
        PENDING.set(Prefs::current());
        DLG_DARK.set(None);
        unsafe {
            if DialogBoxIndirectParamW(GetModuleHandleW(null()), words.as_ptr() as *const c_void, WIN.load(Ordering::Relaxed), prefs_proc, 0) == 1 {
                PENDING.get().apply();
                reshow();
                apply_theme();
                layout();
            }
            for res in [&DLG_BRUSH, &HEAD_FONT] {
                let h = res.replace(0);
                if h != 0 {
                    DeleteObject(h);
                }
            }
        }
    }

    /// Copy `face` into a LOGFONT.
    fn set_face(lf: &mut LogFont, face: &str) {
        lf.face = [0; 32];
        for (d, c) in lf.face.iter_mut().zip(face.encode_utf16().take(31)) {
            *d = c;
        }
    }

    /// `base` with another face or weight (0 keeps it); the caller deletes it.
    unsafe fn font_like(base: isize, face: Option<&str>, weight: i32) -> isize {
        let mut lf: LogFont = std::mem::zeroed();
        if GetObjectW(base, std::mem::size_of::<LogFont>() as i32, &mut lf as *mut _ as *mut c_void) == 0 {
            return 0;
        }
        if let Some(f) = face {
            set_face(&mut lf, f);
        }
        if weight != 0 {
            lf.weight = weight;
        }
        CreateFontIndirectW(&lf as *const _ as *const c_void)
    }

    /// A rounded rectangle, filled and outlined (inside `r`) if colors are given.
    unsafe fn round_rect(dc: isize, r: &Rect, fill: Option<u32>, edge: Option<(u32, i32)>, radius: i32) {
        let brush = fill.map_or(0, |c| CreateSolidBrush(c));
        let pen = edge.map_or(0, |(c, w)| CreatePen(6, w, c)); // PS_INSIDEFRAME
        let old_brush = SelectObject(dc, if brush != 0 { brush } else { GetStockObject(5) }); // NULL_BRUSH
        let old_pen = SelectObject(dc, if pen != 0 { pen } else { GetStockObject(8) }); // NULL_PEN
        RoundRect(dc, r.left, r.top, r.right, r.bottom, radius, radius);
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        for h in [brush, pen] {
            if h != 0 {
                DeleteObject(h);
            }
        }
    }

    /// A control's rectangle in the dialog's client coordinates.
    unsafe fn control_rect(dlg: Hwnd, id: u16) -> Rect {
        let mut r = Rect::default();
        GetWindowRect(GetDlgItem(dlg, id as i32), &mut r);
        MapWindowPoints(0, dlg, &mut r as *mut Rect as *mut i32, 2);
        r
    }

    /// Show the pending choices: the dialog's colors, which choice in each
    /// row is selected, the size, and the preview.
    unsafe fn sync(dlg: Hwnd) {
        let p = PENDING.get();
        let dark = p.dark();
        PALETTE.set(Palette::new(dark, p.dark));
        let old = DLG_BRUSH.replace(CreateSolidBrush(PALETTE.get().back));
        if old != 0 {
            DeleteObject(old);
        }
        if DLG_DARK.replace(Some(dark)) != Some(dark) {
            dark_title(dlg, dark);
        }
        // each row's selected choice is its tab stop (so Tab goes to it, and
        // the arrow keys move along the row); weights the font lacks are off
        for (first, n) in GROUPS {
            let sel = p.selected(first).unwrap_or(0);
            for i in 0..n {
                let h = GetDlgItem(dlg, (first + i as u16) as i32);
                let style = GetWindowLongPtrW(h, -16); // GWL_STYLE
                SetWindowLongPtrW(h, -16, if i == sel { style | WS_TABSTOP } else { style & !WS_TABSTOP });
                if first == G_WEIGHT {
                    EnableWindow(h, FONTS[p.font].1.get(i).is_some_and(|f| f.is_some()) as i32);
                }
            }
        }
        SetWindowTextW(GetDlgItem(dlg, IDC_SIZE as i32), wide(&format!("{} pt", p.size)).as_ptr());
        let preview_dark = PREVIEW_DARK.get();
        SetWindowTextW(GetDlgItem(dlg, IDC_HEAD_PREVIEW as i32), wide(if preview_dark { "Preview: dark mode" } else { "Preview: light mode" }).as_ptr());
        let preview = GetDlgItem(dlg, IDC_PREVIEW as i32);
        let mut rtf = with_text_prefs_for(SAMPLE, p.size, p.face()).into_bytes();
        rtf.push(0);
        let st = SetTextEx { flags: 0, codepage: 0 };
        SendMessageW(preview, EM_SETTEXTEX, &st as *const _ as usize, rtf.as_ptr() as isize);
        if p.spacing > 0 {
            line_spacing(preview, SPACINGS[p.spacing].1);
        }
        set_colors(preview, if preview_dark { p.dark } else { p.light });
        RedrawWindow(dlg, null(), 0, 0x1 | 0x4 | 0x80); // RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN
    }

    /// Draw one of the dialog's buttons: a rounded chip, filled with the
    /// accent color when selected; color schemes in their own colors.
    unsafe fn draw_chip(dlg: Hwnd, d: &DrawItem) {
        const PRESSED: u32 = 0x1;
        const DISABLED: u32 = 0x4;
        const FOCUS: u32 = 0x10;
        const NOACCEL: u32 = 0x100;
        const NOFOCUSRECT: u32 = 0x200;
        let (pal, p, id, dc, r) = (PALETTE.get(), PENDING.get(), d.ctl_id as u16, d.dc, d.rc);
        fill(dc, &r, pal.back);
        let pressed = d.state & PRESSED != 0;
        let inset = scaled(2);
        let chip = Rect { left: r.left + inset, top: r.top + inset, right: r.right - inset, bottom: r.bottom - inset };
        let radius = scaled(8);
        let plain = if pressed { mix(pal.chip, pal.text, 36) } else { pal.chip };
        let mut ink = if d.state & DISABLED != 0 { mix(pal.text, pal.back, 170) } else { pal.text };
        let mut face = None;
        let mut dot = None;
        match choice(id) {
            Some((g @ (G_LIGHT | G_DARK), i)) => {
                let (_, fg, bg) = if g == G_LIGHT { LIGHT_SCHEMES[i] } else { DARK_SCHEMES[i] };
                let edge = if p.selected(g) == Some(i) { (pal.accent, scaled(2)) } else { (pal.edge, 1) };
                round_rect(dc, &chip, Some(resolve(bg, 5)), Some(edge), radius);
                ink = resolve(fg, 8);
            }
            Some((g, i)) if p.selected(g) == Some(i) => {
                round_rect(dc, &chip, Some(pal.accent), None, radius);
                ink = pal.on_accent;
                face = if g == G_FONT { Some(FONTS[i].0) } else if g == G_WEIGHT { FONTS[p.font].1[i] } else { None };
            }
            Some((g, i)) => {
                round_rect(dc, &chip, Some(plain), Some((pal.edge, 1)), radius);
                face = if g == G_FONT { Some(FONTS[i].0) } else if g == G_WEIGHT { FONTS[p.font].1[i] } else { None };
            }
            None if id == 1 => {
                // OK
                round_rect(dc, &chip, Some(if pressed { mix(pal.accent, pal.back, 60) } else { pal.accent }), None, radius);
                ink = pal.on_accent;
            }
            None => {
                round_rect(dc, &chip, Some(plain), Some((pal.edge, 1)), radius);
                dot = match id {
                    IDC_LIGHT_FG => Some(resolve(p.light.0, 8)),
                    IDC_LIGHT_BG => Some(resolve(p.light.1, 5)),
                    IDC_DARK_FG => Some(p.dark.0),
                    IDC_DARK_BG => Some(p.dark.1),
                    _ => None,
                };
            }
        }
        let mut text = [0u16; 64];
        let n = GetWindowTextW(d.item, text.as_mut_ptr(), text.len() as i32).clamp(0, 63) as usize;
        let base = SendMessageW(dlg, 0x0031, 0, 0); // WM_GETFONT
        let own = face.map_or(0, |f| font_like(base, Some(f), 0));
        let old = SelectObject(dc, if own != 0 { own } else { base });
        SetBkMode(dc, 1); // TRANSPARENT
        SetTextColor(dc, ink);
        let mut rc = chip;
        if let Some(c) = dot {
            // the color, as a swatch before the label
            let s = scaled(12);
            let (x, y) = (chip.left + scaled(8), (chip.top + chip.bottom - s) / 2);
            round_rect(dc, &Rect { left: x, top: y, right: x + s, bottom: y + s }, Some(c), Some((pal.edge, 1)), scaled(5));
            rc.left = x + s;
        }
        let prefix = if d.state & NOACCEL != 0 { DT_HIDEPREFIX } else { 0 };
        draw_text(dc, &text[..n], &mut rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | prefix);
        SelectObject(dc, old);
        if own != 0 {
            DeleteObject(own);
        }
        if d.state & FOCUS != 0 && d.state & NOFOCUSRECT == 0 {
            round_rect(dc, &r, None, Some((pal.text, 1)), radius + inset);
        }
    }

    /// The preview's card: its background with rounded corners around the text.
    unsafe fn paint_card(dlg: Hwnd) {
        let mut ps: PaintStruct = std::mem::zeroed();
        let dc = BeginPaint(dlg, &mut ps);
        let p = PENDING.get();
        let (_, bg) = if PREVIEW_DARK.get() { p.dark } else { p.light };
        let r = control_rect(dlg, IDC_PREVIEW);
        let pad = scaled(6);
        let card = Rect { left: r.left - pad, top: r.top - pad, right: r.right + pad, bottom: r.bottom + pad };
        round_rect(dc, &card, Some(resolve(bg, 5)), Some((PALETTE.get().edge, 1)), scaled(14));
        EndPaint(dlg, &ps);
    }

    unsafe extern "system" fn prefs_proc(dlg: Hwnd, m: u32, w: usize, l: isize) -> isize {
        let item = |id: u16| GetDlgItem(dlg, id as i32);
        match m {
            0x0110 => { // WM_INITDIALOG
                let head = font_like(SendMessageW(dlg, 0x0031, 0, 0), None, 600); // semibold
                HEAD_FONT.set(head);
                for id in [IDC_HEAD_TEXT, IDC_HEAD_LOOK, IDC_HEAD_PREVIEW] {
                    SendMessageW(item(id), 0x0030, head as usize, 0); // WM_SETFONT
                }
                // the preview's text inset like a page's
                let mut r = Rect::default();
                GetClientRect(item(IDC_PREVIEW), &mut r);
                let pad = scaled(8);
                let inner = Rect { left: pad, top: pad, right: r.right - pad, bottom: r.bottom };
                SendMessageW(item(IDC_PREVIEW), EM_SETRECT, 0, &inner as *const _ as isize);
                PREVIEW_DARK.set(PENDING.get().dark());
                sync(dlg);
                1 // focus the first tab stop: the selected font
            }
            0x0135 | 0x0136 | 0x0138 => { // WM_CTLCOLORBTN, WM_CTLCOLORDLG, WM_CTLCOLORSTATIC
                let pal = PALETTE.get();
                SetTextColor(w as isize, if l == item(IDC_HINT) { pal.dim } else { pal.text });
                SetBkMode(w as isize, 1); // TRANSPARENT
                DLG_BRUSH.get()
            }
            0x002B => { // WM_DRAWITEM
                draw_chip(dlg, &*(l as *const DrawItem));
                1
            }
            0x000F => { // WM_PAINT
                paint_card(dlg);
                1
            }
            // Windows' light/dark app mode changed, while "Match Windows" is chosen
            0x001A if PENDING.get().mode == 0 && l != 0 && wstr_eq(l as *const u16, "ImmersiveColorSet") => { // WM_SETTINGCHANGE
                PREVIEW_DARK.set(windows_dark());
                sync(dlg);
                0
            }
            WM_COMMAND => {
                let (id, code) = ((w & 0xFFFF) as u16, ((w >> 16) & 0xFFFF) as u16);
                if id == IDC_PREVIEW {
                    // the preview is only to look at (disabling it would gray
                    // its background): pass the focus on, without a caret
                    if code == 0x0100 { // EN_SETFOCUS
                        SendMessageW(dlg, 0x0028, 0, 0); // WM_NEXTDLGCTL
                    }
                    return 0;
                }
                let click = code == 0 || code == 5; // BN_CLICKED, BN_DOUBLECLICKED
                let (old, old_preview) = (PENDING.get(), PREVIEW_DARK.get());
                let mut p = old;
                match (id, choice(id)) {
                    // a choice is chosen by clicking it, or (except color
                    // schemes) by moving to it with the arrow keys: BN_SETFOCUS
                    (_, Some((g, i))) if click || (code == 6 && g != G_LIGHT && g != G_DARK) => {
                        p.choose(g, i);
                        match g {
                            G_MODE => PREVIEW_DARK.set(p.dark()),
                            G_LIGHT => PREVIEW_DARK.set(false),
                            G_DARK => PREVIEW_DARK.set(true),
                            _ => {}
                        }
                    }
                    (IDC_SMALLER | IDC_LARGER, _) if click => {
                        let next = if id == IDC_LARGER { SIZES.iter().find(|&&s| s > p.size) } else { SIZES.iter().rev().find(|&&s| s < p.size) };
                        match next {
                            Some(&s) => p.size = s,
                            None => {
                                MessageBeep(0);
                            }
                        }
                    }
                    (IDC_LIGHT_FG | IDC_LIGHT_BG | IDC_DARK_FG | IDC_DARK_BG, _) if click => {
                        let dark = id == IDC_DARK_FG || id == IDC_DARK_BG;
                        let colors = if dark { &mut p.dark } else { &mut p.light };
                        let (slot, system) = if id == IDC_LIGHT_FG || id == IDC_DARK_FG { (&mut colors.0, 8) } else { (&mut colors.1, 5) };
                        if let Some(c) = choose_color(dlg, resolve(*slot, system)) {
                            *slot = c;
                        }
                        PREVIEW_DARK.set(dark);
                    }
                    (IDC_RESTORE, _) if click => {
                        p = Prefs::DEFAULT;
                        PREVIEW_DARK.set(p.dark());
                    }
                    (1 | 2, _) if click => {
                        EndDialog(dlg, (id == 1) as isize); // OK, Cancel
                        return 1;
                    }
                    _ => return 0,
                }
                PENDING.set(p);
                if p != old || PREVIEW_DARK.get() != old_preview {
                    sync(dlg);
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
            // print in black, not dark mode's light text (or a scheme's color)
            let (fg, _) = colors();
            if fg != AUTO {
                text_color(edit(), None);
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
            if fg != AUTO {
                text_color(edit(), Some(fg));
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
                layout();
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
                if let Some((font, spacing, length, fg, bg)) = s.more {
                    TEXT_FONT.store(font, Ordering::Relaxed);
                    LINE_SPACING.store(spacing, Ordering::Relaxed);
                    LINE_LENGTH.store(length, Ordering::Relaxed);
                    LIGHT_TEXT_COLOR.store(fg, Ordering::Relaxed);
                    LIGHT_BG_COLOR.store(bg, Ordering::Relaxed);
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
                    // Ctrl+wheel zooms the text: a limited line length grows with it
                    if msg.message == 0x020A && msg.wparam & 0x8 != 0 && LINE_LENGTH.load(Ordering::Relaxed) != 0 { // WM_MOUSEWHEEL, MK_CONTROL
                        layout();
                    }
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
