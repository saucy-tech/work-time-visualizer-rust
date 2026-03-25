use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Shell::{SHAppBarMessage, ABM_GETTASKBARPOS, APPBARDATA};

// ── String helpers ─────────────────────────────────────────────────────────

/// Encode a Rust &str as a null-terminated UTF-16 Vec for Win32 APIs.
pub fn wide_str(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ── Color ──────────────────────────────────────────────────────────────────

/// RGB color that converts to a Win32 COLORREF (0x00BBGGRR).
#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    #[allow(dead_code)]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse "#RRGGBB" (the leading '#' is optional).
    pub fn from_hex(hex: &str) -> Self {
        let h = hex.trim_start_matches('#');
        let parse = |s: &str| u8::from_str_radix(s, 16).unwrap_or(0);
        if h.len() >= 6 {
            Self {
                r: parse(&h[0..2]),
                g: parse(&h[2..4]),
                b: parse(&h[4..6]),
            }
        } else {
            Self::default()
        }
    }

    /// Win32 COLORREF value (0x00BBGGRR).
    pub fn to_colorref(self) -> u32 {
        self.r as u32 | (self.g as u32) << 8 | (self.b as u32) << 16
    }

    /// Pack as 0xFFRRGGBB (for DIB pixel with full alpha).
    #[allow(dead_code)]
    pub fn to_bgra_pixel(self) -> u32 {
        0xFF00_0000u32 | (self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
    }
}

// ── Taskbar helpers ────────────────────────────────────────────────────────

/// Find the main taskbar window handle ("Shell_TrayWnd").
pub fn find_taskbar() -> Option<HWND> {
    unsafe {
        let class = wide_str("Shell_TrayWnd");
        match FindWindowW(PCWSTR::from_raw(class.as_ptr()), PCWSTR::null()) {
            Ok(h) if h != HWND::default() => Some(h),
            _ => None,
        }
    }
}

/// Return the taskbar's screen rectangle via SHAppBarMessage.
pub fn get_taskbar_rect() -> Option<RECT> {
    let hwnd = find_taskbar()?;
    unsafe {
        let mut abd = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd,
            ..Default::default()
        };
        let r = SHAppBarMessage(ABM_GETTASKBARPOS, &mut abd);
        if r == 0 { None } else { Some(abd.rc) }
    }
}

/// Return the bounding rect of any window in screen coordinates.
#[allow(dead_code)]
pub fn get_window_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() { Some(rect) } else { None }
    }
}

/// Embed `hwnd` as a child of the taskbar — reserved for future use.
#[allow(dead_code)]
/// Converts style from POPUP → CHILD and calls SetParent.
/// Returns true on success.
pub fn embed_in_taskbar(hwnd: HWND, taskbar: HWND) -> bool {
    unsafe {
        // Swap POPUP style for CHILD + CLIPSIBLINGS
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let new_style = (style & !WS_POPUP.0) | WS_CHILD.0 | WS_CLIPSIBLINGS.0;
        SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);

        // Add TOOLWINDOW + NOACTIVATE to extended style
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            (ex | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as i32,
        );

        SetParent(hwnd, taskbar).is_ok()
    }
}

/// DPI for a given window (falls back to 96 if the API is unavailable).
pub fn get_dpi_for_window(hwnd: HWND) -> u32 {
    unsafe {
        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
        if dpi == 0 { 96 } else { dpi }
    }
}

/// Position `hwnd` by moving it (no size change when w/h == 0 means "keep").
pub fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let _ = MoveWindow(hwnd, x, y, w, h, true);
    }
}
