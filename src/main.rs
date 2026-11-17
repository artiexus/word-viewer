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
    use std::sync::atomic::{AtomicIsize, AtomicPtr, AtomicU32, Ordering};

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

    #[link(name = "kernel32")]
    extern "system" {
        fn SetDefaultDllDirectories(flags: u32) -> i32;
        fn LoadLibraryExW(name: *const u16, file: isize, flags: u32) -> isize;
        fn GetModuleHandleW(name: *const u16) -> isize;
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
        fn MessageBeep(kind: u32) -> i32;
        fn GetWindowPlacement(h: Hwnd, wp: *mut WindowPlacement) -> i32;
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
    const MAX_PATH: usize = 260;
    /// Zoom steps, in percent.
    const ZOOMS: [u32; 13] = [50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400];

    static WIN: AtomicIsize = AtomicIsize::new(0);
    static EDIT: AtomicIsize = AtomicIsize::new(0);
    static FIND_DLG: AtomicIsize = AtomicIsize::new(0);
    static FIND_MSG: AtomicU32 = AtomicU32::new(0);
    /// The find dialog's state, which must outlive the (modeless) dialog.
    static FIND: AtomicPtr<Find> = AtomicPtr::new(null_mut());

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

    // ---- remembered settings: window placement and zoom ----

    /// %LOCALAPPDATA%\word-viewer\window.txt. Only the window's own settings
    /// are kept; never anything about documents.
    fn settings_path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("word-viewer").join("window.txt"))
    }

    /// Saved normal-window rectangle, whether it was maximized, and zoom percent.
    fn load_settings() -> Option<(Rect, bool, u32)> {
        let mut s = String::new();
        File::open(settings_path()?).ok()?.take(256).read_to_string(&mut s).ok()?;
        let v: Vec<i32> = s.split_whitespace().map(|x| x.parse().ok()).collect::<Option<_>>()?;
        let [l, t, r, b, max, zoom] = v[..] else { return None };
        let ok = r - l >= 200 && b - t >= 150 && r - l <= 32767 && b - t <= 32767;
        ok.then_some((Rect { left: l, top: t, right: r, bottom: b }, max != 0, zoom.clamp(10, 500) as u32))
    }

    fn save_settings() {
        let mut wp: WindowPlacement = unsafe { std::mem::zeroed() };
        wp.length = std::mem::size_of::<WindowPlacement>() as u32;
        if unsafe { GetWindowPlacement(WIN.load(Ordering::Relaxed), &mut wp) } == 0 {
            return;
        }
        let n = wp.normal;
        let line = format!("{} {} {} {} {} {}\n", n.left, n.top, n.right, n.bottom, (wp.show_cmd == 3) as u8, zoom());
        if let Some(path) = settings_path() {
            let _ = path.parent().map(std::fs::create_dir_all);
            let _ = std::fs::write(path, line);
        }
    }

    // ---- showing documents ----

    fn show_rtf(rtf: &str) {
        let mut buf = rtf.as_bytes().to_vec(); // RTF output is pure ASCII
        buf.push(0);
        let st = SetTextEx { flags: 0, codepage: 0 };
        let z = zoom(); // replacing the text resets RichEdit's zoom
        unsafe {
            SendMessageW(edit(), EM_SETTEXTEX, &st as *const _ as usize, buf.as_ptr() as isize);
            SendMessageW(edit(), EM_SETSEL, 0, 0);
            SendMessageW(edit(), EM_SCROLLCARET, 0, 0);
        }
        set_zoom(z);
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

    unsafe fn menu(items: &[(&str, usize)]) -> isize {
        let m = CreatePopupMenu();
        for &(text, id) in items {
            if text.is_empty() {
                AppendMenuW(m, 0x0800, 0, null()); // MF_SEPARATOR
            } else {
                AppendMenuW(m, 0, id, wide(text).as_ptr());
            }
        }
        m
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

            let bar = CreateMenu();
            for (title, items) in [
                ("&File", &[("&Open...\tCtrl+O", ID_OPEN), ("&Print...\tCtrl+P", ID_PRINT), ("", 0), ("E&xit", ID_EXIT)][..]),
                ("&Edit", &[("&Copy\tCtrl+C", ID_COPY), ("Select &All\tCtrl+A", ID_SELECT_ALL), ("", 0), ("&Find...\tCtrl+F", ID_FIND), ("Find &Next\tF3", ID_FIND_NEXT), ("Find Pre&vious\tShift+F3", ID_FIND_PREV)][..]),
                ("&View", &[("Zoom &In\tCtrl++", ID_ZOOM_IN), ("Zoom &Out\tCtrl+-", ID_ZOOM_OUT), ("&Actual Size\tCtrl+0", ID_ZOOM_RESET)][..]),
            ] {
                AppendMenuW(bar, 0x0010, menu(items) as usize, wide(title).as_ptr()); // MF_POPUP
            }

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
            let edit = CreateWindowExW(
                0, wide("RICHEDIT50W").as_ptr(), wide("").as_ptr(),
                // child visible vscroll multiline readonly autovscroll nohidesel
                0x4000_0000 | 0x1000_0000 | 0x0020_0000 | 0x4 | 0x800 | 0x40 | 0x100,
                0, 0, 0, 0, win, 0, inst, null());
            EDIT.store(edit, Ordering::Relaxed);
            SendMessageW(edit, EM_EXLIMITTEXT, 0, 0x7FFF_FFFF);

            let saved = load_settings();
            if let Some((_, _, zoom)) = saved {
                set_zoom(zoom);
            }
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
                Some((normal, max, _)) => {
                    let wp = WindowPlacement {
                        length: std::mem::size_of::<WindowPlacement>() as u32,
                        flags: 0,
                        show_cmd: if max { 3 } else { 1 }, // SW_SHOWMAXIMIZED, SW_SHOWNORMAL
                        min_pos: [-1, -1],
                        max_pos: [-1, -1],
                        normal,
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
}
