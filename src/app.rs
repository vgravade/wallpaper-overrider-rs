//! Native Win32 GUI for selecting, previewing, and applying wallpaper policy values.
//!
//! The UI is intentionally implemented without a framework to keep the release binary small.
//! Most helpers below exist to make the Win32 ownership and DPI rules explicit.

use image::{imageops::FilterType, DynamicImage, ImageReader, Limits, RgbaImage};
use std::{
    ffi::{OsStr, OsString},
    mem::{size_of, zeroed},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use windows_sys::Win32::{
    Foundation::{
        GetLastError, ERROR_CLASS_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM,
    },
    Graphics::{
        Dwm::DwmSetWindowAttribute,
        Gdi::{
            BeginPaint, CreateFontIndirectW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint,
            FillRect, FrameRect, GetStockObject, InvalidateRect, SelectObject, SetBkColor,
            SetBkMode, SetTextColor, StretchDIBits, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER,
            BI_RGB, COLOR_WINDOW, DEFAULT_GUI_FONT, DIB_RGB_COLORS, HBRUSH, HDC, HFONT,
            PAINTSTRUCT, RGBQUAD, SRCCOPY, TRANSPARENT,
        },
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Controls::{
            Dialogs::{
                GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY, OFN_NOCHANGEDIR,
                OFN_PATHMUSTEXIST, OPENFILENAMEW,
            },
            DRAWITEMSTRUCT, TOOLTIPS_CLASSW, TTF_IDISHWND, TTF_SUBCLASS, TTM_ADDTOOLW,
            TTM_SETTOOLINFOW, TTS_ALWAYSTIP, TTS_NOPREFIX, TTTOOLINFOW,
        },
        HiDpi::{GetDpiForSystem, GetDpiForWindow, SystemParametersInfoForDpi},
        Input::KeyboardAndMouse::EnableWindow,
        Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP},
        WindowsAndMessaging::{
            AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
            GetClientRect, GetMessageW, GetSystemMetrics, GetWindowLongPtrW, GetWindowTextLengthW,
            GetWindowTextW, LoadCursorW, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW,
            SendMessageW, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow,
            TranslateMessage, CBN_SELCHANGE, CBS_DROPDOWNLIST, CB_ADDSTRING, CB_GETCURSEL,
            CB_SETCURSEL, CREATESTRUCTW, GWLP_USERDATA, HICON, HMENU, ICON_BIG, ICON_SMALL,
            IDC_ARROW, MSG, NONCLIENTMETRICSW, SM_CXSCREEN, SM_CYSCREEN, SPI_GETNONCLIENTMETRICS,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SW_SHOW, WINDOW_EX_STYLE, WM_APP, WM_COMMAND,
            WM_CREATE, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED, WM_DRAWITEM,
            WM_DROPFILES, WM_ERASEBKGND, WM_NCCREATE, WM_PAINT, WM_SETFONT, WM_SETICON,
            WM_SETTINGCHANGE, WM_SIZE, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN,
            WS_CLIPSIBLINGS, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP, WS_SYSMENU, WS_TABSTOP,
            WS_VISIBLE,
        },
    },
};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

use crate::{elevation, i18n::Language, registry, wallpaper_style::WallpaperStyle};

const PREVIEW_W: u32 = 316;
const PREVIEW_H: u32 = 198;
const PREVIEW_WORK_SCALE: u32 = 2;
const PREVIEW_MAX_DECODE_DIMENSION: u32 = 32_768;
const PREVIEW_MAX_DECODE_ALLOC: u64 = 128 * 1024 * 1024;

const WINDOW_W: i32 = 540;
const WINDOW_H: i32 = 514;
const PREVIEW_X: i32 = 24;
const PREVIEW_Y: i32 = 22;
const PREVIEW_VIEW_W: i32 = 492;
const PREVIEW_VIEW_H: i32 = 318;
const ROW_IMAGE_Y: i32 = 360;
const ROW_STYLE_Y: i32 = 402;
const LABEL_X: i32 = 24;
const LABEL_W: i32 = 72;
const FIELD_X: i32 = 110;
const FIELD_H: i32 = 26;
const PATH_W: i32 = 292;
const BROWSE_X: i32 = 410;
const BUTTON_W: i32 = 106;
const BUTTON_H: i32 = 30;
const STYLE_W: i32 = 180;
const STATUS_X: i32 = 24;
const STATUS_Y: i32 = 434;
const STATUS_H: i32 = 18;
const CONTENT_RIGHT_MARGIN: i32 = 24;
const CONTROL_GAP: i32 = 8;
const ACTION_Y: i32 = STATUS_Y + STATUS_H + CONTROL_GAP;
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
const DWMWCP_ROUND: u32 = 2;

const SS_NOPREFIX: u32 = 0x0000_0080;
const SS_ENDELLIPSIS: u32 = 0x0000_4000;
const BS_OWNERDRAW: u32 = 0x0000_000b;
const ODS_SELECTED: u32 = 0x0001;
const ODS_DISABLED: u32 = 0x0004;
const ODS_HOTLIGHT: u32 = 0x0040;
const DRAG_QUERY_FILE_COUNT: u32 = 0xffff_ffff;

const ID_BROWSE: isize = 1001;
const ID_STYLE: isize = 1002;
const ID_APPLY: isize = 1003;
const ID_CLOSE: isize = 1004;
const IDI_APP_ICON: u16 = 1;
const WM_APPLY_DONE: u32 = WM_APP + 1;

struct PreviewBitmap {
    width: i32,
    height: i32,
    bgra: Vec<u8>,
}

// GDI brushes returned by CreateSolidBrush must be released with DeleteObject.
struct OwnedBrush(HBRUSH);

impl OwnedBrush {
    fn solid(color: u32) -> Option<Self> {
        let brush = unsafe { CreateSolidBrush(color) };
        (!brush.is_null()).then_some(Self(brush))
    }

    fn get(&self) -> HBRUSH {
        self.0
    }
}

impl Drop for OwnedBrush {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0);
        }
    }
}

struct ApplyResult {
    path: PathBuf,
    style: WallpaperStyle,
    result: Result<(), String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Palette {
    window_bg: u32,
    preview_bezel: u32,
    preview_screen: u32,
    preview_empty_bg: u32,
    preview_empty_text: u32,
    preview_edge: u32,
    preview_highlight: u32,
    preview_empty_grid: u32,
    label_text: u32,
    status_text: u32,
    button_bg: u32,
    button_hover_bg: u32,
    button_pressed_bg: u32,
    button_border: u32,
    button_text: u32,
    button_disabled_bg: u32,
    button_disabled_text: u32,
    accent_bg: u32,
    accent_hover_bg: u32,
    accent_pressed_bg: u32,
    accent_text: u32,
    path_bg: u32,
    path_border: u32,
    path_icon_bg: u32,
    path_icon_text: u32,
    path_text: u32,
}

impl UiTheme {
    fn detect() -> Self {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(personalize) =
            hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
        else {
            return Self::Light;
        };
        match personalize.get_value::<u32, _>("AppsUseLightTheme") {
            Ok(0) => Self::Dark,
            _ => Self::Light,
        }
    }

    fn palette(self) -> Palette {
        match self {
            Self::Light => Palette {
                window_bg: rgb(246, 248, 251),
                preview_bezel: rgb(28, 31, 36),
                preview_screen: rgb(9, 11, 15),
                preview_empty_bg: rgb(232, 236, 243),
                preview_empty_text: rgb(77, 85, 99),
                preview_edge: rgb(148, 163, 184),
                preview_highlight: rgb(255, 255, 255),
                preview_empty_grid: rgb(219, 225, 235),
                label_text: rgb(31, 41, 55),
                status_text: rgb(75, 85, 99),
                button_bg: rgb(255, 255, 255),
                button_hover_bg: rgb(242, 246, 252),
                button_pressed_bg: rgb(225, 232, 242),
                button_border: rgb(203, 213, 225),
                button_text: rgb(31, 41, 55),
                button_disabled_bg: rgb(229, 234, 242),
                button_disabled_text: rgb(139, 148, 163),
                accent_bg: rgb(0, 95, 184),
                accent_hover_bg: rgb(0, 103, 192),
                accent_pressed_bg: rgb(0, 80, 158),
                accent_text: rgb(255, 255, 255),
                path_bg: rgb(255, 255, 255),
                path_border: rgb(203, 213, 225),
                path_icon_bg: rgb(219, 234, 254),
                path_icon_text: rgb(0, 95, 184),
                path_text: rgb(31, 41, 55),
            },
            Self::Dark => Palette {
                window_bg: rgb(30, 32, 36),
                preview_bezel: rgb(12, 14, 18),
                preview_screen: rgb(4, 6, 10),
                preview_empty_bg: rgb(42, 45, 52),
                preview_empty_text: rgb(190, 198, 211),
                preview_edge: rgb(78, 86, 100),
                preview_highlight: rgb(64, 70, 82),
                preview_empty_grid: rgb(50, 55, 64),
                label_text: rgb(238, 242, 247),
                status_text: rgb(198, 205, 217),
                button_bg: rgb(45, 49, 56),
                button_hover_bg: rgb(55, 60, 69),
                button_pressed_bg: rgb(38, 42, 49),
                button_border: rgb(76, 84, 97),
                button_text: rgb(241, 245, 249),
                button_disabled_bg: rgb(39, 42, 48),
                button_disabled_text: rgb(118, 127, 142),
                accent_bg: rgb(96, 165, 250),
                accent_hover_bg: rgb(125, 181, 252),
                accent_pressed_bg: rgb(69, 142, 230),
                accent_text: rgb(5, 10, 20),
                path_bg: rgb(45, 49, 56),
                path_border: rgb(76, 84, 97),
                path_icon_bg: rgb(30, 64, 111),
                path_icon_text: rgb(191, 219, 254),
                path_text: rgb(241, 245, 249),
            },
        }
    }
}

struct NativeApp {
    lang: Language,
    theme: UiTheme,
    dpi: u32,
    hwnd: HWND,
    preview_hwnd: HWND,
    image_label_hwnd: HWND,
    path_hwnd: HWND,
    browse_hwnd: HWND,
    style_label_hwnd: HWND,
    style_hwnd: HWND,
    status_hwnd: HWND,
    apply_hwnd: HWND,
    close_hwnd: HWND,
    path_tooltip_hwnd: HWND,
    path_tooltip_text: Vec<u16>,
    ui_font: HFONT,
    ui_font_owned: bool,
    wallpaper_path: Option<PathBuf>,
    style: WallpaperStyle,
    applied_wallpaper_path: Option<PathBuf>,
    applied_style: WallpaperStyle,
    preview: Option<PreviewBitmap>,
    apply_in_progress: bool,
    apply_tx: Sender<ApplyResult>,
    apply_rx: Receiver<ApplyResult>,
    window_bg_brush: OwnedBrush,
}

pub fn run(lang: Language) -> anyhow::Result<()> {
    let hinstance = unsafe { GetModuleHandleW(null()) };
    anyhow::ensure!(!hinstance.is_null(), "GetModuleHandleW failed");

    let class_name = wide("WallpaperOverriderWindow");
    let preview_class_name = wide("WallpaperOverriderPreview");
    let path_class_name = wide("WallpaperOverriderPath");
    register_class(
        &class_name,
        hinstance,
        Some(window_proc),
        (COLOR_WINDOW + 1) as HBRUSH,
    )?;
    register_class(
        &preview_class_name,
        hinstance,
        Some(preview_proc),
        (COLOR_WINDOW + 1) as HBRUSH,
    )?;
    register_class(
        &path_class_name,
        hinstance,
        Some(path_proc),
        (COLOR_WINDOW + 1) as HBRUSH,
    )?;

    // Registry reads are best-effort: a missing or unreadable policy should not
    // prevent the picker from opening with an empty selection.
    let (wallpaper_str, style) = registry::get_current_wallpaper().unwrap_or((None, None));
    let wallpaper_path = wallpaper_str.map(PathBuf::from).filter(|p| p.is_file());
    let style = style.unwrap_or_default();
    let preview = wallpaper_path
        .as_deref()
        .and_then(|path| build_preview_bitmap(path, style).ok());

    let initial_dpi = unsafe { GetDpiForSystem().max(96) };
    let (apply_tx, apply_rx) = mpsc::channel();
    let theme = UiTheme::detect();
    let window_bg_brush = OwnedBrush::solid(theme.palette().window_bg)
        .ok_or_else(|| anyhow::anyhow!("CreateSolidBrush failed"))?;
    let app = Box::new(NativeApp {
        lang,
        theme,
        dpi: initial_dpi,
        hwnd: null_mut(),
        preview_hwnd: null_mut(),
        image_label_hwnd: null_mut(),
        path_hwnd: null_mut(),
        browse_hwnd: null_mut(),
        style_label_hwnd: null_mut(),
        style_hwnd: null_mut(),
        status_hwnd: null_mut(),
        apply_hwnd: null_mut(),
        close_hwnd: null_mut(),
        path_tooltip_hwnd: null_mut(),
        path_tooltip_text: Vec::new(),
        ui_font: null_mut(),
        ui_font_owned: false,
        wallpaper_path: wallpaper_path.clone(),
        style,
        applied_wallpaper_path: wallpaper_path,
        applied_style: style,
        preview,
        apply_in_progress: false,
        apply_tx,
        apply_rx,
        window_bg_brush,
    });

    let style_flags = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let ex_style: WINDOW_EX_STYLE = 0;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: scale(WINDOW_W, initial_dpi),
        bottom: scale(WINDOW_H, initial_dpi),
    };
    unsafe {
        AdjustWindowRectEx(&mut rect, style_flags, 0, ex_style);
    }
    let window_w = rect.right - rect.left;
    let window_h = rect.bottom - rect.top;
    let x = (unsafe { GetSystemMetrics(SM_CXSCREEN) } - window_w) / 2;
    let y = (unsafe { GetSystemMetrics(SM_CYSCREEN) } - window_h) / 2;

    let title = wide(lang.app_title());
    // The boxed state is handed to Win32 and reclaimed on WM_DESTROY.
    let app_ptr = Box::into_raw(app);
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            title.as_ptr(),
            style_flags | WS_CLIPCHILDREN,
            x,
            y,
            window_w,
            window_h,
            null_mut(),
            null_mut(),
            hinstance,
            app_ptr.cast(),
        )
    };
    if hwnd.is_null() {
        unsafe {
            drop(Box::from_raw(app_ptr));
        }
        anyhow::bail!("CreateWindowExW failed: {}", last_error());
    }

    unsafe {
        let app = &mut *app_ptr;
        app.dpi = GetDpiForWindow(hwnd).max(96);
        enable_modern_window_chrome(hwnd, app.theme);
        set_window_icon(hwnd, hinstance);
        DragAcceptFiles(hwnd, 1);
        resize_window_for_dpi(hwnd, app.dpi);
        layout_controls(app);
        InvalidateRect(app.preview_hwnd, null(), 1);
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
    }

    let mut msg: MSG = unsafe { zeroed() };
    loop {
        let ret = unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) };
        if ret == 0 {
            break;
        }
        anyhow::ensure!(ret != -1, "GetMessageW failed: {}", last_error());
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}

impl NativeApp {
    fn palette(&self) -> Palette {
        self.theme.palette()
    }

    fn refresh_theme(&mut self) {
        let theme = UiTheme::detect();
        if self.theme == theme {
            return;
        }
        self.theme = theme;
        if let Some(brush) = OwnedBrush::solid(self.palette().window_bg) {
            self.window_bg_brush = brush;
        }
        enable_modern_window_chrome(self.hwnd, self.theme);
        unsafe {
            InvalidateRect(self.hwnd, null(), 1);
            InvalidateRect(self.preview_hwnd, null(), 1);
            InvalidateRect(self.browse_hwnd, null(), 1);
            InvalidateRect(self.apply_hwnd, null(), 1);
            InvalidateRect(self.close_hwnd, null(), 1);
        }
    }

    fn scale(&self, value: i32) -> i32 {
        scale(value, self.layout_dpi())
    }

    fn layout_dpi(&self) -> u32 {
        let mut rect: RECT = unsafe { zeroed() };
        let has_client =
            unsafe { !self.hwnd.is_null() && GetClientRect(self.hwnd, &mut rect) != 0 };
        if !has_client {
            return self.dpi.max(96);
        }

        let client_w = rect.right - rect.left;
        let client_h = rect.bottom - rect.top;
        if client_w <= 0 || client_h <= 0 {
            return self.dpi.max(96);
        }

        // When Windows sends a resized client area during DPI transitions, derive
        // the layout DPI from the actual client size so controls stay visible.
        layout_dpi_for_client(client_w, client_h, self.dpi)
    }

    fn can_apply(&self) -> bool {
        self.wallpaper_path.is_some()
            && !self.apply_in_progress
            && (self.wallpaper_path != self.applied_wallpaper_path
                || self.style != self.applied_style)
    }

    fn refresh_apply_enabled(&self) {
        unsafe {
            EnableWindow(self.apply_hwnd, self.can_apply() as i32);
            InvalidateRect(self.apply_hwnd, null(), 1);
        }
    }

    fn refresh_path_text(&mut self) {
        let display = self
            .wallpaper_path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| self.lang.empty_path().to_owned());
        set_window_text(self.path_hwnd, &display);
        self.update_path_tooltip(&display);
        unsafe {
            InvalidateRect(self.path_hwnd, null(), 1);
        }
    }

    fn path_display_name(&self) -> String {
        self.wallpaper_path
            .as_deref()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.lang.empty_path().to_owned())
    }

    fn update_path_tooltip(&mut self, text: &str) {
        self.path_tooltip_text = wide(text);
        if self.path_tooltip_hwnd.is_null() {
            return;
        }

        let mut tool = tooltip_info(
            self.hwnd,
            self.path_hwnd,
            self.path_tooltip_text.as_mut_ptr(),
        );
        unsafe {
            SendMessageW(
                self.path_tooltip_hwnd,
                TTM_SETTOOLINFOW,
                0,
                (&mut tool as *mut TTTOOLINFOW) as LPARAM,
            );
        }
    }

    fn set_status(&self, text: &str) {
        set_window_text(self.status_hwnd, text);
    }

    fn browse(&mut self) {
        if let Some(path) = open_image_dialog(self.hwnd, self.lang) {
            self.select_wallpaper_path(path);
        }
    }

    fn select_wallpaper_path(&mut self, path: PathBuf) {
        self.wallpaper_path = Some(path);
        self.rebuild_preview();
        self.refresh_path_text();
        self.set_status("");
        self.refresh_apply_enabled();
        unsafe {
            InvalidateRect(self.preview_hwnd, null(), 1);
        }
    }

    fn handle_drop(&mut self, drop: HDROP) {
        let path = first_dropped_file(drop);
        unsafe {
            DragFinish(drop);
        }
        if let Some(path) = path.filter(|path| is_supported_image_path(path)) {
            self.select_wallpaper_path(path);
        }
    }

    fn set_style_from_combo(&mut self) {
        let index = unsafe { SendMessageW(self.style_hwnd, CB_GETCURSEL, 0, 0) };
        if index < 0 {
            return;
        }

        let Some(style) = WallpaperStyle::all().get(index as usize).copied() else {
            return;
        };
        if self.style == style {
            return;
        }

        self.style = style;
        self.rebuild_preview();
        self.refresh_apply_enabled();
        unsafe {
            InvalidateRect(self.preview_hwnd, null(), 1);
        }
    }

    fn rebuild_preview(&mut self) {
        self.preview = self
            .wallpaper_path
            .as_deref()
            .and_then(|path| build_preview_bitmap(path, self.style).ok());
    }

    fn apply(&mut self) {
        let Some(path) = self.wallpaper_path.clone() else {
            self.set_status(self.lang.no_wallpaper_selected());
            return;
        };
        if !path.is_file() {
            self.set_status(self.lang.file_no_longer_exists());
            return;
        }
        if self.apply_in_progress {
            return;
        }
        if !self.can_apply() {
            self.set_status(self.lang.no_changes_to_apply());
            return;
        }

        self.apply_in_progress = true;
        self.refresh_apply_enabled();
        self.set_status(self.lang.applying_wallpaper());

        let hwnd = self.hwnd as isize;
        let tx = self.apply_tx.clone();
        let lang = self.lang;
        let style = self.style;
        thread::spawn(move || {
            let result = apply_wallpaper(&path, style, lang);
            let message = ApplyResult {
                path,
                style,
                result,
            };
            if tx.send(message).is_ok() {
                unsafe {
                    // Marshal completion back to the UI thread before touching HWND state.
                    let _ = PostMessageW(hwnd as HWND, WM_APPLY_DONE, 0, 0);
                }
            };
        });
    }

    fn handle_apply_result(&mut self, result: ApplyResult) {
        self.apply_in_progress = false;
        match result.result {
            Ok(()) => {
                self.applied_wallpaper_path = Some(result.path);
                self.applied_style = result.style;
                self.set_status(self.lang.wallpaper_applied());
            }
            Err(err) => self.set_status(&err),
        }
        self.refresh_apply_enabled();
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: Windows calls this procedure with message-specific pointer payloads.
    unsafe {
        if msg == WM_NCCREATE {
            // WM_NCCREATE is the first reliable point where lpCreateParams is available.
            // Store the app pointer in GWLP_USERDATA so later messages can find it.
            let createstruct = lparam as *const CREATESTRUCTW;
            let app = (*createstruct).lpCreateParams as *mut NativeApp;
            (*app).hwnd = hwnd;
            (*app).dpi = GetDpiForWindow(hwnd).max(96);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as isize);
        }

        let app = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativeApp;
        if app.is_null() {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let app = &mut *app;

        match msg {
            WM_CREATE => {
                app.dpi = GetDpiForWindow(hwnd).max(96);
                enable_modern_window_chrome(hwnd, app.theme);
                match create_controls(app) {
                    Ok(()) => 0,
                    Err(_) => -1,
                }
            }
            WM_ERASEBKGND => {
                paint_window_background(hwnd, wparam as HDC, app);
                1
            }
            WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
                style_text_control(wparam as HDC, lparam as HWND, app)
            }
            WM_DRAWITEM => draw_button(lparam as *const DRAWITEMSTRUCT, app),
            WM_COMMAND => {
                let id = loword(wparam) as isize;
                let notification = hiword(wparam) as u32;
                match id {
                    ID_BROWSE => app.browse(),
                    ID_STYLE if notification == CBN_SELCHANGE => app.set_style_from_combo(),
                    ID_APPLY => app.apply(),
                    ID_CLOSE => {
                        DestroyWindow(hwnd);
                    }
                    _ => {}
                }
                0
            }
            WM_DPICHANGED => {
                app.dpi = hiword(wparam) as u32;
                if lparam != 0 {
                    let rect = &*(lparam as *const RECT);
                    SetWindowPos(
                        hwnd,
                        null_mut(),
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                update_ui_font(app);
                layout_controls(app);
                InvalidateRect(app.preview_hwnd, null(), 1);
                0
            }
            WM_SIZE => {
                layout_controls(app);
                InvalidateRect(app.preview_hwnd, null(), 1);
                0
            }
            WM_APPLY_DONE => {
                while let Ok(result) = app.apply_rx.try_recv() {
                    app.handle_apply_result(result);
                }
                0
            }
            WM_DROPFILES => {
                app.handle_drop(wparam as HDROP);
                0
            }
            WM_SETTINGCHANGE => {
                app.refresh_theme();
                0
            }
            WM_DESTROY => {
                let app_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativeApp;
                if !app_ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    if (*app_ptr).ui_font_owned && !(*app_ptr).ui_font.is_null() {
                        DeleteObject((*app_ptr).ui_font);
                    }
                    drop(Box::from_raw(app_ptr));
                }
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe extern "system" fn preview_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: Windows calls this procedure with message-specific pointer payloads.
    unsafe {
        if msg == WM_NCCREATE {
            let createstruct = lparam as *const CREATESTRUCTW;
            let app = (*createstruct).lpCreateParams as *mut NativeApp;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as isize);
        }

        match msg {
            WM_PAINT => {
                let app = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativeApp;
                let mut ps: PAINTSTRUCT = zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                if !app.is_null() {
                    paint_preview(hwnd, hdc, &*app);
                }
                EndPaint(hwnd, &ps);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe extern "system" fn path_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: Windows calls this procedure with message-specific pointer payloads.
    unsafe {
        if msg == WM_NCCREATE {
            let createstruct = lparam as *const CREATESTRUCTW;
            let app = (*createstruct).lpCreateParams as *mut NativeApp;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as isize);
        }

        match msg {
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                let app = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativeApp;
                let mut ps: PAINTSTRUCT = zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                if !app.is_null() {
                    paint_path_pill(hwnd, hdc, &*app);
                }
                EndPaint(hwnd, &ps);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn create_controls(app: &mut NativeApp) -> anyhow::Result<()> {
    app.preview_hwnd = create_child(
        app.hwnd,
        "WallpaperOverriderPreview",
        "",
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        0,
        (app as *mut NativeApp).cast(),
    )?;

    app.image_label_hwnd = create_child(
        app.hwnd,
        "STATIC",
        app.lang.choose_picture(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        0,
        null_mut(),
    )?;

    app.path_hwnd = create_child(
        app.hwnd,
        "WallpaperOverriderPath",
        "",
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        0,
        (app as *mut NativeApp).cast(),
    )?;
    app.path_tooltip_hwnd = create_path_tooltip(app)?;

    app.browse_hwnd = create_child(
        app.hwnd,
        "BUTTON",
        app.lang.browse_button(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_TABSTOP | BS_OWNERDRAW,
        ID_BROWSE,
        null_mut(),
    )?;

    app.style_label_hwnd = create_child(
        app.hwnd,
        "STATIC",
        app.lang.choose_fit(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        0,
        null_mut(),
    )?;

    app.style_hwnd = create_child(
        app.hwnd,
        "COMBOBOX",
        "",
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_TABSTOP | CBS_DROPDOWNLIST as u32,
        ID_STYLE,
        null_mut(),
    )?;
    for &style in WallpaperStyle::all() {
        let label = wide(app.lang.wallpaper_style(style));
        unsafe {
            SendMessageW(app.style_hwnd, CB_ADDSTRING, 0, label.as_ptr() as LPARAM);
        }
    }
    let selected = WallpaperStyle::all()
        .iter()
        .position(|style| *style == app.style)
        .unwrap_or(0);
    unsafe {
        SendMessageW(app.style_hwnd, CB_SETCURSEL, selected, 0);
    }

    app.status_hwnd = create_child(
        app.hwnd,
        "STATIC",
        "",
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | SS_NOPREFIX | SS_ENDELLIPSIS,
        0,
        null_mut(),
    )?;

    app.apply_hwnd = create_child(
        app.hwnd,
        "BUTTON",
        app.lang.apply_button(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_TABSTOP | BS_OWNERDRAW,
        ID_APPLY,
        null_mut(),
    )?;

    app.close_hwnd = create_child(
        app.hwnd,
        "BUTTON",
        app.lang.close_button(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_TABSTOP | BS_OWNERDRAW,
        ID_CLOSE,
        null_mut(),
    )?;

    update_ui_font(app);
    app.refresh_path_text();
    app.refresh_apply_enabled();
    layout_controls(app);
    Ok(())
}

fn update_ui_font(app: &mut NativeApp) {
    let old_font = app.ui_font;
    let old_owned = app.ui_font_owned;
    let (font, owned) = create_message_font(app.dpi);
    app.ui_font = font;
    app.ui_font_owned = owned;
    apply_control_font(app, font);

    if old_owned && !old_font.is_null() {
        unsafe {
            DeleteObject(old_font);
        }
    }
}

fn apply_control_font(app: &NativeApp, font: HFONT) {
    for hwnd in [
        app.image_label_hwnd,
        app.path_hwnd,
        app.browse_hwnd,
        app.style_label_hwnd,
        app.style_hwnd,
        app.status_hwnd,
        app.apply_hwnd,
        app.close_hwnd,
    ] {
        unsafe {
            SendMessageW(hwnd, WM_SETFONT, font as WPARAM, 1);
        }
    }
}

fn create_path_tooltip(app: &mut NativeApp) -> anyhow::Result<HWND> {
    app.path_tooltip_text = wide(app.lang.empty_path());
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            TOOLTIPS_CLASSW,
            null(),
            WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
            0,
            0,
            0,
            0,
            app.hwnd,
            null_mut(),
            GetModuleHandleW(null()),
            null_mut(),
        )
    };
    anyhow::ensure!(!hwnd.is_null(), "tooltip CreateWindowExW failed");

    let mut tool = tooltip_info(app.hwnd, app.path_hwnd, app.path_tooltip_text.as_mut_ptr());
    unsafe {
        SendMessageW(
            hwnd,
            TTM_ADDTOOLW,
            0,
            (&mut tool as *mut TTTOOLINFOW) as LPARAM,
        );
    }

    Ok(hwnd)
}

fn tooltip_info(owner: HWND, target: HWND, text: *mut u16) -> TTTOOLINFOW {
    TTTOOLINFOW {
        cbSize: size_of::<TTTOOLINFOW>() as u32,
        uFlags: TTF_SUBCLASS | TTF_IDISHWND,
        hwnd: owner,
        uId: target as usize,
        rect: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        hinst: null_mut(),
        lpszText: text,
        lParam: 0,
        lpReserved: null_mut(),
    }
}

fn create_message_font(dpi: u32) -> (HFONT, bool) {
    let mut metrics: NONCLIENTMETRICSW = unsafe { zeroed() };
    metrics.cbSize = size_of::<NONCLIENTMETRICSW>() as u32;

    // Match the system message font at the window DPI instead of relying on
    // DEFAULT_GUI_FONT, which is only a fallback for older or failing systems.
    let ok = unsafe {
        SystemParametersInfoForDpi(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            (&mut metrics as *mut NONCLIENTMETRICSW).cast(),
            0,
            dpi.max(96),
        ) != 0
    };
    if ok {
        let font = unsafe { CreateFontIndirectW(&metrics.lfMessageFont) };
        if !font.is_null() {
            return (font, true);
        }
    }

    (unsafe { GetStockObject(DEFAULT_GUI_FONT) } as HFONT, false)
}

fn layout_controls(app: &NativeApp) {
    let dpi = app.layout_dpi();
    let action_layout = action_row_layout(logical_client_width(app, dpi));
    let scaled = |value| scale(value, dpi);

    move_window(
        app.preview_hwnd,
        scaled(PREVIEW_X),
        scaled(PREVIEW_Y),
        scaled(PREVIEW_VIEW_W),
        scaled(PREVIEW_VIEW_H),
    );
    move_window(
        app.image_label_hwnd,
        scaled(LABEL_X),
        scaled(ROW_IMAGE_Y + 4),
        scaled(LABEL_W),
        scaled(FIELD_H),
    );
    move_window(
        app.path_hwnd,
        scaled(FIELD_X),
        scaled(ROW_IMAGE_Y),
        scaled(PATH_W),
        scaled(FIELD_H),
    );
    move_window(
        app.browse_hwnd,
        scaled(BROWSE_X),
        scaled(ROW_IMAGE_Y - 1),
        scaled(BUTTON_W),
        scaled(BUTTON_H),
    );
    move_window(
        app.style_label_hwnd,
        scaled(LABEL_X),
        scaled(ROW_STYLE_Y + 4),
        scaled(LABEL_W),
        scaled(FIELD_H),
    );
    move_window(
        app.style_hwnd,
        scaled(FIELD_X),
        scaled(ROW_STYLE_Y),
        scaled(STYLE_W),
        scaled(160),
    );
    move_window(
        app.status_hwnd,
        scaled(action_layout.status_x),
        scaled(action_layout.status_y),
        scaled(action_layout.status_w),
        scaled(action_layout.status_h),
    );
    move_window(
        app.apply_hwnd,
        scaled(action_layout.apply_x),
        scaled(action_layout.action_y),
        scaled(BUTTON_W),
        scaled(BUTTON_H),
    );
    move_window(
        app.close_hwnd,
        scaled(action_layout.close_x),
        scaled(action_layout.action_y),
        scaled(BUTTON_W),
        scaled(BUTTON_H),
    );
}

fn paint_window_background(hwnd: HWND, hdc: HDC, app: &NativeApp) {
    let mut rect: RECT = unsafe { zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut rect);
        FillRect(hdc, &rect, app.window_bg_brush.get());
    }
}

fn paint_path_pill(hwnd: HWND, hdc: HDC, app: &NativeApp) {
    let mut rect: RECT = unsafe { zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut rect);
    }
    let palette = app.palette();
    let (Some(bg), Some(border), Some(icon_bg)) = (
        OwnedBrush::solid(palette.path_bg),
        OwnedBrush::solid(palette.path_border),
        OwnedBrush::solid(palette.path_icon_bg),
    ) else {
        return;
    };

    unsafe {
        FillRect(hdc, &rect, bg.get());
        FrameRect(hdc, &rect, border.get());
        SetBkMode(hdc, TRANSPARENT as i32);
    }

    let icon_size = app.scale(18);
    let icon = RECT {
        left: rect.left + app.scale(8),
        top: rect.top + ((rect.bottom - rect.top - icon_size) / 2),
        right: rect.left + app.scale(8) + icon_size,
        bottom: rect.top + ((rect.bottom - rect.top - icon_size) / 2) + icon_size,
    };
    unsafe {
        FillRect(hdc, &icon, icon_bg.get());
        SetTextColor(hdc, palette.path_icon_text);
    }
    let mut icon_text_rect = icon;
    let icon_text = wide("IMG");
    let previous_font = unsafe { SelectObject(hdc, app.ui_font) };
    unsafe {
        DrawTextW(
            hdc,
            icon_text.as_ptr(),
            -1,
            &mut icon_text_rect,
            windows_sys::Win32::Graphics::Gdi::DT_CENTER
                | windows_sys::Win32::Graphics::Gdi::DT_VCENTER
                | windows_sys::Win32::Graphics::Gdi::DT_SINGLELINE,
        );
        SetTextColor(hdc, palette.path_text);
    }

    let mut text_rect = RECT {
        left: icon.right + app.scale(8),
        top: rect.top,
        right: rect.right - app.scale(10),
        bottom: rect.bottom,
    };
    let display = app.path_display_name();
    let text = wide(&display);
    unsafe {
        DrawTextW(
            hdc,
            text.as_ptr(),
            -1,
            &mut text_rect,
            windows_sys::Win32::Graphics::Gdi::DT_LEFT
                | windows_sys::Win32::Graphics::Gdi::DT_VCENTER
                | windows_sys::Win32::Graphics::Gdi::DT_SINGLELINE
                | windows_sys::Win32::Graphics::Gdi::DT_END_ELLIPSIS,
        );
        if !previous_font.is_null() {
            SelectObject(hdc, previous_font);
        }
    }
}

fn style_text_control(hdc: HDC, control: HWND, app: &NativeApp) -> LRESULT {
    let palette = app.palette();
    unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);
        SetBkColor(hdc, palette.window_bg);
        let text_color = if control == app.status_hwnd {
            palette.status_text
        } else {
            palette.label_text
        };
        SetTextColor(hdc, text_color);
    }

    app.window_bg_brush.get() as LRESULT
}

fn draw_button(item: *const DRAWITEMSTRUCT, app: &NativeApp) -> LRESULT {
    if item.is_null() {
        return 0;
    }

    let item = unsafe { &*item };
    let palette = app.palette();
    let is_accent = item.CtlID as isize == ID_APPLY;
    let disabled = item.itemState & ODS_DISABLED != 0;
    let pressed = item.itemState & ODS_SELECTED != 0;
    let hot = item.itemState & ODS_HOTLIGHT != 0;

    let (bg, text) = if disabled {
        (palette.button_disabled_bg, palette.button_disabled_text)
    } else if is_accent {
        let bg = if pressed {
            palette.accent_pressed_bg
        } else if hot {
            palette.accent_hover_bg
        } else {
            palette.accent_bg
        };
        (bg, palette.accent_text)
    } else {
        let bg = if pressed {
            palette.button_pressed_bg
        } else if hot {
            palette.button_hover_bg
        } else {
            palette.button_bg
        };
        (bg, palette.button_text)
    };

    let (Some(bg_brush), Some(border_brush)) = (
        OwnedBrush::solid(bg),
        OwnedBrush::solid(palette.button_border),
    ) else {
        return 1;
    };

    let mut rect = item.rcItem;
    unsafe {
        FillRect(item.hDC, &rect, bg_brush.get());
        FrameRect(item.hDC, &rect, border_brush.get());
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, text);
    }

    rect.left += 10;
    rect.right -= 10;
    if pressed {
        rect.left += 1;
        rect.top += 1;
    }

    let label = button_text(item.hwndItem);
    let label = wide(&label);
    let previous_font = unsafe { SelectObject(item.hDC, app.ui_font) };
    unsafe {
        DrawTextW(
            item.hDC,
            label.as_ptr(),
            -1,
            &mut rect,
            windows_sys::Win32::Graphics::Gdi::DT_CENTER
                | windows_sys::Win32::Graphics::Gdi::DT_VCENTER
                | windows_sys::Win32::Graphics::Gdi::DT_SINGLELINE,
        );
        if !previous_font.is_null() {
            SelectObject(item.hDC, previous_font);
        }
    }

    1
}

fn button_text(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }

    let mut buffer = vec![0u16; len as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..copied.max(0) as usize])
}

fn enable_modern_window_chrome(hwnd: HWND, theme: UiTheme) {
    let dark_mode: i32 = (theme == UiTheme::Dark) as i32;
    let corner_preference: i32 = DWMWCP_ROUND as i32;

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark_mode as *const i32).cast(),
            size_of::<i32>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corner_preference as *const i32).cast(),
            size_of::<i32>() as u32,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionRowLayout {
    status_x: i32,
    status_y: i32,
    status_w: i32,
    status_h: i32,
    apply_x: i32,
    close_x: i32,
    action_y: i32,
}

fn action_row_layout(client_w: i32) -> ActionRowLayout {
    // Keep the status line full-width and move actions from the right edge; this
    // preserves margins even if Windows reports a narrower-than-designed client.
    let content_right = (client_w - CONTENT_RIGHT_MARGIN).max(STATUS_X + 1);
    let close_x = (content_right - BUTTON_W).max(STATUS_X);
    let apply_x = (close_x - CONTROL_GAP - BUTTON_W).max(STATUS_X);

    ActionRowLayout {
        status_x: STATUS_X,
        status_y: STATUS_Y,
        status_w: (content_right - STATUS_X).max(1),
        status_h: STATUS_H,
        apply_x,
        close_x,
        action_y: ACTION_Y,
    }
}

fn logical_client_width(app: &NativeApp, dpi: u32) -> i32 {
    let mut rect: RECT = unsafe { zeroed() };
    let has_client = unsafe { !app.hwnd.is_null() && GetClientRect(app.hwnd, &mut rect) != 0 };
    if has_client && rect.right > rect.left {
        return unscale(rect.right - rect.left, dpi).max(1);
    }

    WINDOW_W
}

fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        SetWindowPos(hwnd, null_mut(), x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

fn resize_window_for_dpi(hwnd: HWND, dpi: u32) {
    let style_flags = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let ex_style: WINDOW_EX_STYLE = 0;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: scale(WINDOW_W, dpi),
        bottom: scale(WINDOW_H, dpi),
    };
    unsafe {
        AdjustWindowRectEx(&mut rect, style_flags, 0, ex_style);
        SetWindowPos(
            hwnd,
            null_mut(),
            0,
            0,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

fn paint_preview(hwnd: HWND, hdc: HDC, app: &NativeApp) {
    let mut rect: RECT = unsafe { zeroed() };
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let palette = app.palette();
    let (Some(bezel), Some(black), Some(face), Some(edge), Some(highlight)) = (
        OwnedBrush::solid(palette.preview_bezel),
        OwnedBrush::solid(palette.preview_screen),
        OwnedBrush::solid(palette.preview_empty_bg),
        OwnedBrush::solid(palette.preview_edge),
        OwnedBrush::solid(palette.preview_highlight),
    ) else {
        return;
    };

    unsafe {
        FillRect(hdc, &rect, bezel.get());
        FrameRect(hdc, &rect, edge.get());
    }
    let screen = RECT {
        left: app.scale(8),
        top: app.scale(10),
        right: width - app.scale(8),
        bottom: height - app.scale(10),
    };
    unsafe {
        FillRect(hdc, &screen, black.get());
        FrameRect(hdc, &screen, highlight.get());
    }

    if let Some(preview) = &app.preview {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: preview.width,
                biHeight: -preview.height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };
        unsafe {
            StretchDIBits(
                hdc,
                screen.left,
                screen.top,
                screen.right - screen.left,
                screen.bottom - screen.top,
                0,
                0,
                preview.width,
                preview.height,
                preview.bgra.as_ptr().cast(),
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
    } else {
        unsafe {
            FillRect(hdc, &screen, face.get());
            fill_checkerboard(
                hdc,
                screen,
                palette.preview_empty_bg,
                palette.preview_empty_grid,
                app.scale(16).max(4),
            );
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, palette.preview_empty_text);
        }
        let mut text_rect = screen;
        let text = wide(app.lang.empty_preview_title());
        let previous_font = unsafe { SelectObject(hdc, app.ui_font) };
        unsafe {
            DrawTextW(
                hdc,
                text.as_ptr(),
                -1,
                &mut text_rect,
                windows_sys::Win32::Graphics::Gdi::DT_CENTER
                    | windows_sys::Win32::Graphics::Gdi::DT_VCENTER
                    | windows_sys::Win32::Graphics::Gdi::DT_SINGLELINE,
            );
            if !previous_font.is_null() {
                SelectObject(hdc, previous_font);
            }
        }
    }
}

fn apply_wallpaper(path: &Path, style: WallpaperStyle, lang: Language) -> Result<(), String> {
    // Prefer the non-elevated HKCU write. If policy permissions block it, use the
    // elevated broker path that writes the same values under HKEY_USERS\<SID>.
    if let Ok(()) = registry::set_wallpaper_for_current_user(path, style) {
        let _ = registry::refresh_wallpaper_session(path);
        return Ok(());
    }

    if elevation::is_elevated() {
        let sid = elevation::current_user_sid().map_err(|e| lang.failed_resolve_sid(e))?;
        registry::set_wallpaper_for_sid(&sid, path, style)
            .map_err(|e| lang.failed_to_apply(e))
            .map(|()| {
                let _ = registry::refresh_wallpaper_session(path);
            })
    } else {
        let sid = elevation::current_user_sid().map_err(|e| lang.failed_resolve_sid(e))?;
        let broker_args = vec![
            OsString::from("--target-sid"),
            OsString::from(sid),
            OsString::from("--wallpaper"),
            path.as_os_str().to_owned(),
            OsString::from("--style"),
            OsString::from(style.code()),
        ];

        match elevation::run_elevated_with_args(&broker_args) {
            Ok(0) => {
                let _ = registry::refresh_wallpaper_session(path);
                Ok(())
            }
            Ok(code) => Err(lang.elevated_broker_failed(code)),
            Err(e) => Err(lang.elevation_failed(e)),
        }
    }
}

fn fill_checkerboard(hdc: HDC, rect: RECT, base: u32, alternate: u32, size: i32) {
    let (Some(base_brush), Some(alternate_brush)) =
        (OwnedBrush::solid(base), OwnedBrush::solid(alternate))
    else {
        return;
    };

    unsafe {
        FillRect(hdc, &rect, base_brush.get());
    }

    let size = size.max(1);
    let mut y = rect.top;
    let mut row = 0;
    while y < rect.bottom {
        let mut x = rect.left;
        let mut col = 0;
        while x < rect.right {
            if (row + col) % 2 == 0 {
                let tile = RECT {
                    left: x,
                    top: y,
                    right: (x + size).min(rect.right),
                    bottom: (y + size).min(rect.bottom),
                };
                unsafe {
                    FillRect(hdc, &tile, alternate_brush.get());
                }
            }
            x += size;
            col += 1;
        }
        y += size;
        row += 1;
    }
}

fn build_preview_bitmap(path: &Path, style: WallpaperStyle) -> anyhow::Result<PreviewBitmap> {
    let work = load_preview_work_image(path)?;
    let rgba = render_preview(&work, style, PREVIEW_W, PREVIEW_H);
    let mut bgra = Vec::with_capacity(rgba.len());
    for px in rgba.chunks_exact(4) {
        // StretchDIBits with a 32-bit BI_RGB DIB expects bytes in BGRA order.
        bgra.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
    }

    Ok(PreviewBitmap {
        width: PREVIEW_W as i32,
        height: PREVIEW_H as i32,
        bgra,
    })
}

fn open_image_dialog(owner: HWND, lang: Language) -> Option<PathBuf> {
    let mut file_buffer = [0u16; 4096];
    let mut filter = wide(&format!(
        "{}\0*.jpg;*.jpeg;*.png;*.bmp\0All files\0*.*\0",
        lang.images_filter()
    ));
    filter.push(0);

    let mut ofn: OPENFILENAMEW = unsafe { zeroed() };
    ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = file_buffer.as_mut_ptr();
    ofn.nMaxFile = file_buffer.len() as u32;
    ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY | OFN_NOCHANGEDIR;

    let ok = unsafe { GetOpenFileNameW(&mut ofn) != 0 };
    if !ok {
        return None;
    }

    let len = file_buffer.iter().position(|ch| *ch == 0).unwrap_or(0);
    if len == 0 {
        return None;
    }

    Some(PathBuf::from(String::from_utf16_lossy(&file_buffer[..len])))
}

fn first_dropped_file(drop: HDROP) -> Option<PathBuf> {
    let count = unsafe { DragQueryFileW(drop, DRAG_QUERY_FILE_COUNT, null_mut(), 0) };
    if count == 0 {
        return None;
    }

    let len = unsafe { DragQueryFileW(drop, 0, null_mut(), 0) };
    if len == 0 {
        return None;
    }

    let mut buffer = vec![0u16; len as usize + 1];
    let copied = unsafe { DragQueryFileW(drop, 0, buffer.as_mut_ptr(), buffer.len() as u32) };
    if copied == 0 {
        return None;
    }

    Some(PathBuf::from(String::from_utf16_lossy(
        &buffer[..copied as usize],
    )))
}

fn is_supported_image_path(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(OsStr::to_str)
            .map(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "bmp"
                )
            })
            .unwrap_or(false)
}

fn load_preview_work_image(path: &Path) -> anyhow::Result<DynamicImage> {
    let mut reader = ImageReader::open(path)?;
    reader.limits(preview_decode_limits());
    let img = reader.with_guessed_format()?.decode()?;
    // Preview rendering never needs the original full-resolution bitmap.
    Ok(downscale_for_preview(
        img,
        PREVIEW_W * PREVIEW_WORK_SCALE,
        PREVIEW_H * PREVIEW_WORK_SCALE,
    ))
}

fn preview_decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(PREVIEW_MAX_DECODE_DIMENSION);
    limits.max_image_height = Some(PREVIEW_MAX_DECODE_DIMENSION);
    limits.max_alloc = Some(PREVIEW_MAX_DECODE_ALLOC);
    limits
}

fn downscale_for_preview(img: DynamicImage, max_w: u32, max_h: u32) -> DynamicImage {
    let (iw, ih) = (img.width(), img.height());
    if iw <= max_w && ih <= max_h {
        return img;
    }

    img.resize(max_w, max_h, FilterType::Triangle)
}

fn render_preview(img: &DynamicImage, style: WallpaperStyle, width: u32, height: u32) -> Vec<u8> {
    let bg_pixel = image::Rgba([20u8, 20, 20, 255]);
    let mut canvas = RgbaImage::new(width, height);
    for p in canvas.pixels_mut() {
        *p = bg_pixel;
    }

    match style {
        WallpaperStyle::Stretch | WallpaperStyle::Span => {
            let resized = img.resize_exact(width, height, FilterType::Triangle);
            image::imageops::overlay(&mut canvas, &resized.to_rgba8(), 0, 0);
        }
        WallpaperStyle::Center => {
            let rgba = img.to_rgba8();
            let (iw, ih) = rgba.dimensions();
            let x = (width as i64 - iw as i64) / 2;
            let y = (height as i64 - ih as i64) / 2;
            image::imageops::overlay(&mut canvas, &rgba, x, y);
        }
        WallpaperStyle::Fit => {
            let resized = img.resize(width, height, FilterType::Triangle);
            let rgba = resized.to_rgba8();
            let (rw, rh) = rgba.dimensions();
            let x = (width as i64 - rw as i64) / 2;
            let y = (height as i64 - rh as i64) / 2;
            image::imageops::overlay(&mut canvas, &rgba, x, y);
        }
        WallpaperStyle::Fill => {
            let resized = img.resize_to_fill(width, height, FilterType::Triangle);
            image::imageops::overlay(&mut canvas, &resized.to_rgba8(), 0, 0);
        }
        WallpaperStyle::Tile => {
            let rgba = img.to_rgba8();
            let (iw, ih) = rgba.dimensions();
            if iw > 0 && ih > 0 {
                let mut ty: i64 = 0;
                while ty < height as i64 {
                    let mut tx: i64 = 0;
                    while tx < width as i64 {
                        image::imageops::overlay(&mut canvas, &rgba, tx, ty);
                        tx += iw as i64;
                    }
                    ty += ih as i64;
                }
            }
        }
    }

    canvas.into_raw()
}

fn create_child(
    parent: HWND,
    class: &str,
    text: &str,
    style: u32,
    id: isize,
    param: *mut std::ffi::c_void,
) -> anyhow::Result<HWND> {
    create_child_ex(0, parent, class, text, style, id, param)
}

fn create_child_ex(
    ex_style: u32,
    parent: HWND,
    class: &str,
    text: &str,
    style: u32,
    id: isize,
    param: *mut std::ffi::c_void,
) -> anyhow::Result<HWND> {
    let class = wide(class);
    let text = wide(text);
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class.as_ptr(),
            text.as_ptr(),
            style,
            0,
            0,
            1,
            1,
            parent,
            id as HMENU,
            GetModuleHandleW(null()),
            param,
        )
    };
    anyhow::ensure!(
        !hwnd.is_null(),
        "CreateWindowExW child failed: {}",
        last_error()
    );
    Ok(hwnd)
}

fn register_class(
    class_name: &[u16],
    hinstance: HINSTANCE,
    proc: windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC,
    background: HBRUSH,
) -> anyhow::Result<()> {
    let wc = WNDCLASSW {
        style: 0,
        lpfnWndProc: proc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: app_icon(hinstance),
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
        hbrBackground: background,
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
    };
    let atom = unsafe { RegisterClassW(&wc) };
    if atom == 0 {
        let error = unsafe { GetLastError() };
        anyhow::ensure!(
            error == ERROR_CLASS_ALREADY_EXISTS,
            "RegisterClassW failed: GetLastError={error}"
        );
    }
    Ok(())
}

fn set_window_icon(hwnd: HWND, hinstance: HINSTANCE) {
    let icon = app_icon(hinstance);
    if icon.is_null() {
        return;
    }

    unsafe {
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as WPARAM, icon as LPARAM);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as WPARAM, icon as LPARAM);
    }
}

fn app_icon(hinstance: HINSTANCE) -> HICON {
    unsafe { LoadIconW(hinstance, int_resource(IDI_APP_ICON)) }
}

fn int_resource(id: u16) -> *const u16 {
    id as usize as *const u16
}

fn set_window_text(hwnd: HWND, text: &str) {
    let text = wide(text);
    unsafe {
        SetWindowTextW(hwnd, text.as_ptr());
    }
}

fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn loword(value: usize) -> u16 {
    (value & 0xffff) as u16
}

fn hiword(value: usize) -> u16 {
    ((value >> 16) & 0xffff) as u16
}

fn scale(value: i32, dpi: u32) -> i32 {
    ((value as i64 * dpi as i64 + 48) / 96) as i32
}

fn unscale(value: i32, dpi: u32) -> i32 {
    ((value as i64 * 96 + dpi.max(1) as i64 / 2) / dpi.max(1) as i64) as i32
}

fn layout_dpi_for_client(client_w: i32, client_h: i32, window_dpi: u32) -> u32 {
    if client_w <= 0 || client_h <= 0 {
        return window_dpi.max(96);
    }

    let width_dpi = ((client_w as i64 * 96 + WINDOW_W as i64 / 2) / WINDOW_W as i64) as u32;
    let height_dpi = ((client_h as i64 * 96 + WINDOW_H as i64 / 2) / WINDOW_H as i64) as u32;
    let client_dpi = width_dpi.min(height_dpi).max(1);

    client_dpi.min(window_dpi.max(96)).max(72)
}

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}

fn last_error() -> String {
    format!("GetLastError={}", unsafe { GetLastError() })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BG: [u8; 4] = [20, 20, 20, 255];
    const RED: [u8; 4] = [220, 0, 0, 255];
    const BLUE: [u8; 4] = [0, 30, 220, 255];

    fn solid_image(width: u32, height: u32, rgba: [u8; 4]) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(width, height, image::Rgba(rgba)))
    }

    fn pixel(buf: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * width + x) * 4) as usize;
        [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
    }

    #[test]
    fn center_keeps_image_size_and_uses_background() {
        let img = solid_image(2, 2, RED);

        let preview = render_preview(&img, WallpaperStyle::Center, 4, 4);

        assert_eq!(pixel(&preview, 4, 0, 0), BG);
        assert_eq!(pixel(&preview, 4, 1, 1), RED);
        assert_eq!(pixel(&preview, 4, 2, 2), RED);
        assert_eq!(pixel(&preview, 4, 3, 3), BG);
    }

    #[test]
    fn tile_repeats_the_source_image() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                image::Rgba(RED)
            } else {
                image::Rgba(BLUE)
            }
        }));

        let preview = render_preview(&img, WallpaperStyle::Tile, 5, 2);

        assert_eq!(pixel(&preview, 5, 0, 0), RED);
        assert_eq!(pixel(&preview, 5, 1, 0), BLUE);
        assert_eq!(pixel(&preview, 5, 2, 0), RED);
        assert_eq!(pixel(&preview, 5, 3, 1), BLUE);
        assert_eq!(pixel(&preview, 5, 4, 1), RED);
    }

    #[test]
    fn fit_preserves_aspect_ratio_with_letterbox() {
        let img = solid_image(4, 2, RED);

        let preview = render_preview(&img, WallpaperStyle::Fit, 4, 4);

        assert_eq!(pixel(&preview, 4, 0, 0), BG);
        assert_eq!(pixel(&preview, 4, 0, 1), RED);
        assert_eq!(pixel(&preview, 4, 3, 2), RED);
        assert_eq!(pixel(&preview, 4, 3, 3), BG);
    }

    #[test]
    fn stretch_fills_the_entire_preview() {
        let img = solid_image(1, 1, RED);

        let preview = render_preview(&img, WallpaperStyle::Stretch, 3, 2);

        for y in 0..2 {
            for x in 0..3 {
                assert_eq!(pixel(&preview, 3, x, y), RED);
            }
        }
    }

    #[test]
    fn layout_dpi_does_not_overflow_when_client_stays_logical() {
        assert_eq!(layout_dpi_for_client(WINDOW_W, WINDOW_H, 120), 96);
    }

    #[test]
    fn layout_dpi_follows_a_real_dpi_resized_client() {
        assert_eq!(
            layout_dpi_for_client(scale(WINDOW_W, 144), scale(WINDOW_H, 144), 144),
            144
        );
    }

    #[test]
    fn layout_dpi_can_shrink_to_keep_controls_visible() {
        assert_eq!(
            layout_dpi_for_client(scale(WINDOW_W, 80), scale(WINDOW_H, 80), 96),
            80
        );
    }

    #[test]
    fn action_row_gives_status_the_full_content_width() {
        let layout = action_row_layout(WINDOW_W);

        assert_eq!(layout.status_x, STATUS_X);
        assert_eq!(layout.status_w, WINDOW_W - CONTENT_RIGHT_MARGIN - STATUS_X);
        assert_eq!(layout.apply_x, 296);
        assert_eq!(layout.close_x, 410);
    }

    #[test]
    fn action_row_keeps_status_above_buttons() {
        let layout = action_row_layout(WINDOW_W);

        assert_eq!(
            layout.action_y - (layout.status_y + layout.status_h),
            CONTROL_GAP
        );
    }

    #[test]
    fn action_row_keeps_consistent_outer_margins() {
        let layout = action_row_layout(WINDOW_W);

        assert_eq!(layout.status_x, STATUS_X);
        assert_eq!(
            WINDOW_W - (layout.status_x + layout.status_w),
            CONTENT_RIGHT_MARGIN
        );
        assert_eq!(WINDOW_W - (layout.close_x + BUTTON_W), CONTENT_RIGHT_MARGIN);
        assert_eq!(
            WINDOW_H - (layout.action_y + BUTTON_H),
            CONTENT_RIGHT_MARGIN
        );
    }

    #[test]
    fn unscale_round_trips_scaled_layout_values() {
        assert_eq!(unscale(scale(WINDOW_W, 144), 144), WINDOW_W);
        assert_eq!(unscale(scale(WINDOW_H, 120), 120), WINDOW_H);
    }

    #[test]
    fn preview_decode_limits_are_bounded() {
        let limits = preview_decode_limits();

        assert_eq!(limits.max_image_width, Some(PREVIEW_MAX_DECODE_DIMENSION));
        assert_eq!(limits.max_image_height, Some(PREVIEW_MAX_DECODE_DIMENSION));
        assert_eq!(limits.max_alloc, Some(PREVIEW_MAX_DECODE_ALLOC));
    }
}
