use std::io;

#[cfg(windows)]
mod windows_ui {
    use std::ffi::{OsStr, c_void};
    use std::io;
    use std::mem::{size_of, zeroed};
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::ptr::{null, null_mut};
    use std::sync::{Mutex, OnceLock};
    use std::thread;

    use crate::library::{SnapshotEntry, SnapshotLibrary};
    use crate::protocol::ProtocolServer;
    use tabsnap_companion::{
        PortableLayout, StorageMode, activate_storage, load_storage_mode, resolve_storage,
        validate_storage_dir,
    };

    type Hwnd = isize;
    type Hinstance = isize;
    type Hmenu = isize;
    type Hcursor = isize;
    type Hicon = isize;
    type Hbrush = isize;
    type Lresult = isize;
    type Wparam = usize;
    type Lparam = isize;
    type Uint = u32;
    type Dword = u32;
    type Bool = i32;

    const CW_USEDEFAULT: i32 = i32::MIN;
    const WS_OVERLAPPEDWINDOW: Dword = 0x00CF_0000;
    const WS_VISIBLE: Dword = 0x1000_0000;
    const WS_CHILD: Dword = 0x4000_0000;
    const WS_BORDER: Dword = 0x0080_0000;
    const WS_VSCROLL: Dword = 0x0020_0000;
    const WS_TABSTOP: Dword = 0x0001_0000;
    const ES_AUTOHSCROLL: Dword = 0x0080;
    const LBS_NOTIFY: Dword = 0x0001;
    const CBS_DROPDOWNLIST: Dword = 0x0003;

    const WM_DESTROY: Uint = 0x0002;
    const WM_COMMAND: Uint = 0x0111;
    const SW_SHOW: i32 = 5;

    const LB_ADDSTRING: Uint = 0x0180;
    const LB_RESETCONTENT: Uint = 0x0184;
    const LB_GETCURSEL: Uint = 0x0188;
    const LB_ERR: isize = -1;
    const CB_ADDSTRING: Uint = 0x0143;
    const CB_GETCURSEL: Uint = 0x0147;
    const CB_SETCURSEL: Uint = 0x014E;

    const CF_UNICODETEXT: Uint = 13;
    const GMEM_MOVEABLE: Uint = 0x0002;

    const OFN_FILEMUSTEXIST: Dword = 0x0000_1000;
    const OFN_PATHMUSTEXIST: Dword = 0x0000_0800;
    const OFN_OVERWRITEPROMPT: Dword = 0x0000_0002;

    const ID_LIST: usize = 1001;
    const ID_REFRESH: usize = 1002;
    const ID_IMPORT: usize = 1003;
    const ID_EXPORT: usize = 1004;
    const ID_COPY_PATH: usize = 1005;
    const ID_STORAGE_MODE: usize = 1010;
    const ID_CUSTOM_PATH: usize = 1011;
    const ID_APPLY_STORAGE: usize = 1012;
    const ID_START_SERVER: usize = 1020;
    const ID_COPY_PAIRING: usize = 1021;

    #[repr(C)]
    struct WndClassW {
        style: Uint,
        wnd_proc: Option<unsafe extern "system" fn(Hwnd, Uint, Wparam, Lparam) -> Lresult>,
        cls_extra: i32,
        wnd_extra: i32,
        instance: Hinstance,
        icon: Hicon,
        cursor: Hcursor,
        background: Hbrush,
        menu_name: *const u16,
        class_name: *const u16,
    }

    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: Uint,
        w_param: Wparam,
        l_param: Lparam,
        time: Dword,
        point_x: i32,
        point_y: i32,
        private: Dword,
    }

    #[repr(C)]
    struct OpenFileNameW {
        struct_size: Dword,
        owner: Hwnd,
        instance: Hinstance,
        filter: *const u16,
        custom_filter: *mut u16,
        max_custom_filter: Dword,
        filter_index: Dword,
        file: *mut u16,
        max_file: Dword,
        file_title: *mut u16,
        max_file_title: Dword,
        initial_dir: *const u16,
        title: *const u16,
        flags: Dword,
        file_offset: u16,
        file_extension: u16,
        default_extension: *const u16,
        custom_data: isize,
        hook: *mut c_void,
        template_name: *const u16,
        reserved: *mut c_void,
        reserved2: Dword,
        flags_ex: Dword,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn RegisterClassW(class: *const WndClassW) -> u16;
        fn CreateWindowExW(
            ex_style: Dword,
            class_name: *const u16,
            window_name: *const u16,
            style: Dword,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Hwnd,
            menu: Hmenu,
            instance: Hinstance,
            param: *mut c_void,
        ) -> Hwnd;
        fn DefWindowProcW(hwnd: Hwnd, message: Uint, w_param: Wparam, l_param: Lparam) -> Lresult;
        fn ShowWindow(hwnd: Hwnd, command: i32) -> Bool;
        fn UpdateWindow(hwnd: Hwnd) -> Bool;
        fn GetMessageW(message: *mut Msg, hwnd: Hwnd, min: Uint, max: Uint) -> Bool;
        fn TranslateMessage(message: *const Msg) -> Bool;
        fn DispatchMessageW(message: *const Msg) -> Lresult;
        fn PostQuitMessage(code: i32);
        fn SendMessageW(hwnd: Hwnd, message: Uint, w_param: Wparam, l_param: Lparam) -> Lresult;
        fn SetWindowTextW(hwnd: Hwnd, text: *const u16) -> Bool;
        fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
        fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max: i32) -> i32;
        fn MessageBoxW(hwnd: Hwnd, text: *const u16, caption: *const u16, kind: Uint) -> i32;
        fn OpenClipboard(hwnd: Hwnd) -> Bool;
        fn EmptyClipboard() -> Bool;
        fn SetClipboardData(format: Uint, memory: isize) -> isize;
        fn CloseClipboard() -> Bool;
        fn LoadCursorW(instance: Hinstance, cursor_name: *const u16) -> Hcursor;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
        fn GlobalAlloc(flags: Uint, bytes: usize) -> isize;
        fn GlobalLock(memory: isize) -> *mut c_void;
        fn GlobalUnlock(memory: isize) -> Bool;
        fn GlobalFree(memory: isize) -> isize;
    }

    #[link(name = "comdlg32")]
    unsafe extern "system" {
        fn GetOpenFileNameW(open_file_name: *mut OpenFileNameW) -> Bool;
        fn GetSaveFileNameW(open_file_name: *mut OpenFileNameW) -> Bool;
    }

    struct UiState {
        layout: PortableLayout,
        library: SnapshotLibrary,
        entries: Vec<SnapshotEntry>,
        list: Hwnd,
        storage_mode: Hwnd,
        custom_path: Hwnd,
        privacy_status: Hwnd,
        pairing_status: Hwnd,
        pairing_code: Option<String>,
    }

    static STATE: OnceLock<Mutex<UiState>> = OnceLock::new();

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn child(
        parent: Hwnd,
        class: &str,
        text: &str,
        style: Dword,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: usize,
    ) -> io::Result<Hwnd> {
        let class = wide(class);
        let text = wide(text);
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                text.as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                x,
                y,
                width,
                height,
                parent,
                id as Hmenu,
                GetModuleHandleW(null()),
                null_mut(),
            )
        };
        if hwnd == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(hwnd)
        }
    }

    fn set_text(hwnd: Hwnd, value: &str) {
        let value = wide(value);
        unsafe {
            SetWindowTextW(hwnd, value.as_ptr());
        }
    }

    fn get_text(hwnd: Hwnd) -> String {
        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length <= 0 {
            return String::new();
        }
        let mut buffer = vec![0_u16; length as usize + 1];
        let read = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        String::from_utf16_lossy(&buffer[..read.max(0) as usize])
    }

    fn message(parent: Hwnd, text: &str, title: &str) {
        let text = wide(text);
        let title = wide(title);
        unsafe {
            MessageBoxW(parent, text.as_ptr(), title.as_ptr(), 0);
        }
    }

    fn refresh_library() -> io::Result<()> {
        let mut state = STATE.get().expect("UI state initialized").lock().unwrap();
        state.entries = state.library.list()?;
        unsafe {
            SendMessageW(state.list, LB_RESETCONTENT, 0, 0);
        }
        for entry in &state.entries {
            let line = wide(&format!("{}    {} bytes", entry.file_name, entry.size));
            unsafe {
                SendMessageW(state.list, LB_ADDSTRING, 0, line.as_ptr() as Lparam);
            }
        }
        Ok(())
    }

    fn selected_entry() -> Option<SnapshotEntry> {
        let state = STATE.get()?.lock().ok()?;
        let index = unsafe { SendMessageW(state.list, LB_GETCURSEL, 0, 0) };
        if index == LB_ERR || index < 0 {
            return None;
        }
        state.entries.get(index as usize).cloned()
    }

    fn choose_file(parent: Hwnd, save: bool) -> Option<PathBuf> {
        let mut buffer = vec![0_u16; 32_768];
        let filter = wide("TabSnap snapshots (*.tabsnap)\0*.tabsnap\0All files (*.*)\0*.*\0");
        let title = wide(if save {
            "Export encrypted snapshot"
        } else {
            "Import encrypted snapshot"
        });
        let extension = wide("tabsnap");
        let mut dialog: OpenFileNameW = unsafe { zeroed() };
        dialog.struct_size = size_of::<OpenFileNameW>() as Dword;
        dialog.owner = parent;
        dialog.filter = filter.as_ptr();
        dialog.filter_index = 1;
        dialog.file = buffer.as_mut_ptr();
        dialog.max_file = buffer.len() as Dword;
        dialog.title = title.as_ptr();
        dialog.default_extension = extension.as_ptr();
        dialog.flags = OFN_PATHMUSTEXIST
            | if save {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST
            };

        let ok = unsafe {
            if save {
                GetSaveFileNameW(&mut dialog)
            } else {
                GetOpenFileNameW(&mut dialog)
            }
        };
        if ok == 0 {
            return None;
        }
        let end = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        Some(PathBuf::from(String::from_utf16_lossy(&buffer[..end])))
    }

    fn copy_clipboard(parent: Hwnd, value: &str) -> io::Result<()> {
        let wide = wide(value);
        let bytes = wide.len() * size_of::<u16>();
        unsafe {
            if OpenClipboard(parent) == 0 {
                return Err(io::Error::last_os_error());
            }
            if EmptyClipboard() == 0 {
                CloseClipboard();
                return Err(io::Error::last_os_error());
            }
            let memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if memory == 0 {
                CloseClipboard();
                return Err(io::Error::last_os_error());
            }
            let target = GlobalLock(memory) as *mut u8;
            if target.is_null() {
                GlobalFree(memory);
                CloseClipboard();
                return Err(io::Error::last_os_error());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, target, bytes);
            GlobalUnlock(memory);
            if SetClipboardData(CF_UNICODETEXT, memory) == 0 {
                GlobalFree(memory);
                CloseClipboard();
                return Err(io::Error::last_os_error());
            }
            CloseClipboard();
        }
        Ok(())
    }

    fn apply_storage(parent: Hwnd) -> io::Result<()> {
        let (layout, combo, custom) = {
            let state = STATE.get().expect("UI state initialized").lock().unwrap();
            (state.layout.clone(), state.storage_mode, state.custom_path)
        };
        let selected = unsafe { SendMessageW(combo, CB_GETCURSEL, 0, 0) };
        let mode = match selected {
            0 => StorageMode::Portable,
            1 => StorageMode::Local,
            2 => {
                let path = get_text(custom).trim().to_owned();
                if path.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Custom storage path is empty.",
                    ));
                }
                StorageMode::Custom(PathBuf::from(path))
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Select a storage mode.",
                ));
            }
        };

        let storage = activate_storage(&layout, mode)?;
        let mut state = STATE.get().expect("UI state initialized").lock().unwrap();
        state.library = SnapshotLibrary::new(storage.snapshots_dir.clone());
        set_text(
            state.privacy_status,
            &format!(
                "Storage: {} | {} | drive hint: {}",
                storage.mode.name(),
                storage.snapshots_dir.display(),
                storage.drive_hint.name()
            ),
        );
        drop(state);
        refresh_library()?;
        message(parent, "Storage updated and write-tested.", "TabSnap");
        Ok(())
    }

    fn start_server(parent: Hwnd) -> io::Result<()> {
        let library = {
            let state = STATE.get().expect("UI state initialized").lock().unwrap();
            if state.pairing_code.is_some() {
                message(
                    parent,
                    "The companion protocol is already running.",
                    "TabSnap",
                );
                return Ok(());
            }
            state.library.clone()
        };
        validate_storage_dir(library.root())?;
        let server = ProtocolServer::bind(library)?;
        let endpoint = server.endpoint()?;
        let pairing = server.pairing_code()?;
        {
            let mut state = STATE.get().expect("UI state initialized").lock().unwrap();
            state.pairing_code = Some(pairing.clone());
            set_text(
                state.pairing_status,
                &format!(
                    "Listening: {endpoint}\r\nPairing code: {pairing}\r\nLoopback only. Token stays in memory. Close TabSnap to stop."
                ),
            );
        }
        thread::spawn(move || {
            let _ = server.serve_forever();
        });
        Ok(())
    }

    fn import_snapshot(parent: Hwnd) -> io::Result<()> {
        let Some(path) = choose_file(parent, false) else {
            return Ok(());
        };
        let entry = {
            let state = STATE.get().expect("UI state initialized").lock().unwrap();
            state.library.import_file(&path)?
        };
        refresh_library()?;
        message(
            parent,
            &format!("Imported {} ({} bytes).", entry.file_name, entry.size),
            "TabSnap",
        );
        Ok(())
    }

    fn export_snapshot(parent: Hwnd) -> io::Result<()> {
        let Some(entry) = selected_entry() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Select a snapshot first.",
            ));
        };
        let Some(chosen) = choose_file(parent, true) else {
            return Ok(());
        };
        let destination = chosen.parent().unwrap_or_else(|| Path::new("."));
        let exported = {
            let state = STATE.get().expect("UI state initialized").lock().unwrap();
            state.library.export_file(&entry.file_name, destination)?
        };
        message(
            parent,
            &format!("Exported to {}", exported.display()),
            "TabSnap",
        );
        Ok(())
    }

    unsafe extern "system" fn window_proc(
        hwnd: Hwnd,
        message_id: Uint,
        w_param: Wparam,
        l_param: Lparam,
    ) -> Lresult {
        match message_id {
            WM_COMMAND => {
                let id = w_param & 0xffff;
                let result = match id {
                    ID_REFRESH => refresh_library(),
                    ID_IMPORT => import_snapshot(hwnd),
                    ID_EXPORT => export_snapshot(hwnd),
                    ID_COPY_PATH => selected_entry()
                        .ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidInput, "Select a snapshot first.")
                        })
                        .and_then(|entry| copy_clipboard(hwnd, &entry.path.display().to_string())),
                    ID_APPLY_STORAGE => apply_storage(hwnd),
                    ID_START_SERVER => start_server(hwnd),
                    ID_COPY_PAIRING => {
                        let pairing = STATE
                            .get()
                            .and_then(|state| state.lock().ok())
                            .and_then(|state| state.pairing_code.clone());
                        pairing
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::NotFound,
                                    "Start the companion protocol first.",
                                )
                            })
                            .and_then(|value| copy_clipboard(hwnd, &value))
                    }
                    _ => Ok(()),
                };
                if let Err(error) = result {
                    message(hwnd, &error.to_string(), "TabSnap error");
                }
                0
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, message_id, w_param, l_param) },
        }
    }

    pub fn run() -> io::Result<()> {
        let layout = PortableLayout::discover()?;
        let mode = load_storage_mode(&layout)?;
        let storage = resolve_storage(&layout, &mode)?;
        validate_storage_dir(&storage.snapshots_dir)?;

        let instance = unsafe { GetModuleHandleW(null()) };
        let class_name = wide("TabSnapCompanionWindow");
        let cursor = unsafe { LoadCursorW(0, 32512usize as *const u16) };
        let class = WndClassW {
            style: 0,
            wnd_proc: Some(window_proc),
            cls_extra: 0,
            wnd_extra: 0,
            instance,
            icon: 0,
            cursor,
            background: 6,
            menu_name: null(),
            class_name: class_name.as_ptr(),
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(io::Error::last_os_error());
        }

        let title = wide("TabSnap Companion");
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                820,
                620,
                0,
                0,
                instance,
                null_mut(),
            )
        };
        if hwnd == 0 {
            return Err(io::Error::last_os_error());
        }

        child(
            hwnd,
            "STATIC",
            "Encrypted snapshot library",
            0,
            20,
            18,
            300,
            22,
            0,
        )?;
        let list = child(
            hwnd,
            "LISTBOX",
            "",
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY | WS_TABSTOP,
            20,
            45,
            760,
            235,
            ID_LIST,
        )?;
        child(
            hwnd, "BUTTON", "Refresh", WS_TABSTOP, 20, 292, 90, 30, ID_REFRESH,
        )?;
        child(
            hwnd, "BUTTON", "Import", WS_TABSTOP, 120, 292, 90, 30, ID_IMPORT,
        )?;
        child(
            hwnd, "BUTTON", "Export", WS_TABSTOP, 220, 292, 90, 30, ID_EXPORT,
        )?;
        child(
            hwnd,
            "BUTTON",
            "Copy path",
            WS_TABSTOP,
            320,
            292,
            100,
            30,
            ID_COPY_PATH,
        )?;

        child(hwnd, "STATIC", "Storage", 0, 20, 340, 100, 22, 0)?;
        let storage_mode = child(
            hwnd,
            "COMBOBOX",
            "",
            CBS_DROPDOWNLIST | WS_TABSTOP,
            20,
            365,
            150,
            120,
            ID_STORAGE_MODE,
        )?;
        for item in ["Portable", "Local", "Custom"] {
            let item = wide(item);
            unsafe {
                SendMessageW(storage_mode, CB_ADDSTRING, 0, item.as_ptr() as Lparam);
            }
        }
        let selected = match mode {
            StorageMode::Portable => 0,
            StorageMode::Local => 1,
            StorageMode::Custom(_) => 2,
        };
        unsafe {
            SendMessageW(storage_mode, CB_SETCURSEL, selected, 0);
        }
        let custom_value = match &mode {
            StorageMode::Custom(path) => path.display().to_string(),
            _ => String::new(),
        };
        let custom_path = child(
            hwnd,
            "EDIT",
            &custom_value,
            WS_BORDER | ES_AUTOHSCROLL | WS_TABSTOP,
            185,
            365,
            475,
            28,
            ID_CUSTOM_PATH,
        )?;
        child(
            hwnd,
            "BUTTON",
            "Apply",
            WS_TABSTOP,
            675,
            365,
            105,
            28,
            ID_APPLY_STORAGE,
        )?;
        let privacy_status = child(
            hwnd,
            "STATIC",
            &format!(
                "Storage: {} | {} | drive hint: {}",
                storage.mode.name(),
                storage.snapshots_dir.display(),
                storage.drive_hint.name()
            ),
            0,
            20,
            404,
            760,
            42,
            0,
        )?;

        child(hwnd, "STATIC", "Local bridge", 0, 20, 460, 100, 22, 0)?;
        child(
            hwnd,
            "BUTTON",
            "Start protocol",
            WS_TABSTOP,
            20,
            486,
            125,
            30,
            ID_START_SERVER,
        )?;
        child(
            hwnd,
            "BUTTON",
            "Copy pairing code",
            WS_TABSTOP,
            155,
            486,
            145,
            30,
            ID_COPY_PAIRING,
        )?;
        let pairing_status = child(
            hwnd,
            "STATIC",
            "Stopped. No listener. No network connection.\r\nPasswords and decrypted browser data never enter this companion.",
            0,
            320,
            480,
            460,
            70,
            0,
        )?;

        STATE
            .set(Mutex::new(UiState {
                layout,
                library: SnapshotLibrary::new(storage.snapshots_dir),
                entries: Vec::new(),
                list,
                storage_mode,
                custom_path,
                privacy_status,
                pairing_status,
                pairing_code: None,
            }))
            .map_err(|_| io::Error::other("UI state was already initialized."))?;

        refresh_library()?;
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
        }

        let mut message: Msg = unsafe { zeroed() };
        loop {
            let result = unsafe { GetMessageW(&mut message, 0, 0, 0) };
            if result == -1 {
                return Err(io::Error::last_os_error());
            }
            if result == 0 {
                break;
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
pub fn run() -> io::Result<()> {
    windows_ui::run()
}

#[cfg(not(windows))]
pub fn run() -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "The native companion UI is available on Windows only.",
    ))
}
