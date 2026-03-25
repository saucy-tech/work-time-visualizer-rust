use std::ffi::c_void;
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    GetDpiForSystem, SetProcessDpiAwarenessContext,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::{self, Config};
use crate::native_interop::{
    get_taskbar_rect, move_window, wide_str, Color,
};
use crate::theme::is_dark_mode;
use crate::time_calc::{calculate_progress, TimeProgress};

// ── Layout constants (at 96 DPI — scaled at runtime) ──────────────────────

const BASE_W: i32 = 216; // window width
const BASE_H: i32 = 44;  // window height (designed to fit taskbar)

const PADDING: i32 = 4;
const LABEL_W: i32 = 24; // "Day" / "Wk " text area
const LABEL_GAP: i32 = 4;
const BLOCK_W: i32 = 13;
const BLOCK_H: i32 = 10;
const BLOCK_GAP: i32 = 2;
const BLOCK_RADIUS: i32 = 2; // corner ellipse half-width/height
const ROW_H: i32 = 14; // height of each progress row
const ROW_GAP: i32 = 10; // gap between the two rows

const PCT_GAP: i32 = 4;
const PCT_W: i32 = 28;

// Blocks start x:  PADDING + LABEL_W + LABEL_GAP
const BLOCKS_X: i32 = PADDING + LABEL_W + LABEL_GAP;
// % text x:       BLOCKS_X + num_blocks * (BLOCK_W + BLOCK_GAP) + PCT_GAP - BLOCK_GAP
//  (subtract last BLOCK_GAP so there's no trailing gap after the last block)

// Context-menu command IDs
const IDM_SETTINGS: u32 = 1001;
const IDM_EXIT: u32 = 1002;

// WM_TIMER id
const TIMER_REFRESH: usize = 1;
const REFRESH_MS: u32 = 15_000; // redraw every 15 s

// ── App state ──────────────────────────────────────────────────────────────

struct AppState {
    config: Config,
    progress: TimeProgress,
    dark_mode: bool,
    /// Scaled window width
    win_w: i32,
    /// Scaled window height
    win_h: i32,
    /// DPI scale factor (dpi / 96.0)
    scale: f32,
}

impl AppState {
    fn new(config: Config, scale: f32, win_w: i32, win_h: i32) -> Self {
        let progress  = calculate_progress(&config);
        let dark_mode = is_dark_mode();
        Self { config, progress, dark_mode, win_w, win_h, scale }
    }

    fn refresh(&mut self) {
        self.progress  = calculate_progress(&self.config);
        self.dark_mode = is_dark_mode();
    }

    fn s(&self, v: i32) -> i32 {
        (v as f32 * self.scale) as i32
    }
}

// ── Entry point ────────────────────────────────────────────────────────────

pub fn run() {
    unsafe {
        // Per-monitor DPI awareness
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let hinstance = GetModuleHandleW(PCWSTR::null())
            .expect("GetModuleHandleW failed");

        // Single-instance guard
        let mutex_name = wide_str("WorkTimeVisualizer_SingleInstance");
        let _mutex = CreateMutexW(None, true, PCWSTR::from_raw(mutex_name.as_ptr()))
            .expect("CreateMutexW failed");
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return; // Another instance is running
        }

        // Register window class
        let class_name = wide_str("WorkTimeVisualizerClass");
        let wc = WNDCLASSEXW {
            cbSize:        std::mem::size_of::<WNDCLASSEXW>() as u32,
            style:         CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc:   Some(wnd_proc),
            hInstance:     hinstance.into(),
            hbrBackground: HBRUSH(std::ptr::null_mut()), // we paint the bg ourselves in WM_ERASEBKGND
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
            hCursor:       LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassExW(&wc);

        // Determine DPI and window size BEFORE creating the window so there
        // is no flash at (0,0) with the wrong size.
        let sys_dpi = GetDpiForSystem();
        let scale   = sys_dpi as f32 / 96.0;
        let win_w   = (BASE_W as f32 * scale) as i32;
        let win_h   = (BASE_H as f32 * scale) as i32;

        // Calculate position (above the taskbar, near the clock)
        let (start_x, start_y) = calc_widget_position(win_w, win_h);

        // Create at the correct position/size and immediately visible.
        // WS_EX_TOPMOST in the extended style keeps it above normal windows.
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::from_raw(wide_str("Work Time Visualizer").as_ptr()),
            WS_POPUP | WS_VISIBLE,
            start_x, start_y, win_w, win_h,
            None, None,
            hinstance,
            None,
        ).expect("CreateWindowExW failed");

        let config = config::load();
        let state  = Box::new(AppState::new(config, scale, win_w, win_h));

        // Store state so WM_PAINT can access it
        let ptr = Box::into_raw(state) as isize;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr);

        // Start refresh timer
        SetTimer(hwnd, TIMER_REFRESH, REFRESH_MS, None);

        // Message loop
        let mut msg = MSG::default();
        loop {
            match GetMessageW(&mut msg, HWND::default(), 0, 0) {
                BOOL(v) if v <= 0 => break,
                _ => {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }
}

// ── Window procedure ───────────────────────────────────────────────────────

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // ── Paint ──────────────────────────────────────────────────────────
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            if let Some(state) = get_state(hwnd) {
                render(hdc, state);
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        // ── Erase background (suppress flicker) ───────────────────────────
        WM_ERASEBKGND => LRESULT(1),

        // ── Timer: refresh data + redraw ──────────────────────────────────
        WM_TIMER => {
            if wparam.0 == TIMER_REFRESH {
                if let Some(state) = get_state(hwnd) {
                    state.refresh();
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            LRESULT(0)
        }

        // ── Right-click: context menu ──────────────────────────────────────
        WM_RBUTTONUP => {
            show_context_menu(hwnd);
            LRESULT(0)
        }

        // ── Context-menu commands ─────────────────────────────────────────
        WM_COMMAND => {
            let cmd = (wparam.0 & 0xFFFF) as u32;
            match cmd {
                IDM_SETTINGS => {
                    let path = config::config_path();
                    // Ensure the file exists before opening
                    if !path.exists() {
                        let _ = config::save(&Config::default());
                    }
                    let path_w = wide_str(path.to_str().unwrap_or(""));
                    let verb   = wide_str("open");
                    ShellExecuteW(
                        hwnd,
                        PCWSTR::from_raw(verb.as_ptr()),
                        PCWSTR::from_raw(path_w.as_ptr()),
                        PCWSTR::null(),
                        PCWSTR::null(),
                        SW_SHOWNORMAL,
                    );
                }
                IDM_EXIT => {
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }

        // ── Cleanup ───────────────────────────────────────────────────────
        WM_DESTROY => {
            let _ = KillTimer(hwnd, TIMER_REFRESH);
            let ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if ptr != 0 {
                drop(Box::from_raw(ptr as *mut AppState));
            }
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ── State access ───────────────────────────────────────────────────────────

unsafe fn get_state(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if ptr == 0 { None } else { Some(&mut *(ptr as *mut AppState)) }
}

// ── Rendering ─────────────────────────────────────────────────────────────

unsafe fn render(hdc: HDC, state: &AppState) {
    let w = state.win_w;
    let h = state.win_h;

    // ── Double buffer ──────────────────────────────────────────────────────
    let mem_dc  = CreateCompatibleDC(hdc);
    let hbmp    = CreateCompatibleBitmap(hdc, w, h);
    let old_obj = SelectObject(mem_dc, hbmp);

    // ── Background ────────────────────────────────────────────────────────
    let bg = Color::from_hex(&state.config.colors.background);
    let bg_brush = CreateSolidBrush(COLORREF(bg.to_colorref()));
    let full = RECT { left: 0, top: 0, right: w, bottom: h };
    FillRect(mem_dc, &full, bg_brush);
    let _ = DeleteObject(bg_brush);

    // ── Two rows ──────────────────────────────────────────────────────────
    let s = |v: i32| state.s(v);

    let row1_y = s(PADDING);
    let row2_y = s(PADDING + ROW_H + ROW_GAP);

    draw_row(
        mem_dc, state, "Day",
        state.progress.daily_pct,
        &state.config.colors.day_filled,
        &state.config.colors.day_empty,
        row1_y,
    );
    draw_row(
        mem_dc, state, "Wk ",
        state.progress.weekly_pct,
        &state.config.colors.week_filled,
        &state.config.colors.week_empty,
        row2_y,
    );

    // ── Blit to screen ────────────────────────────────────────────────────
    BitBlt(hdc, 0, 0, w, h, mem_dc, 0, 0, SRCCOPY).ok();

    // ── Cleanup ───────────────────────────────────────────────────────────
    SelectObject(mem_dc, old_obj);
    let _ = DeleteObject(hbmp);
    let _ = DeleteDC(mem_dc);
}

unsafe fn draw_row(
    dc: HDC,
    state: &AppState,
    label: &str,
    pct: f64,
    filled_hex: &str,
    empty_hex: &str,
    row_y: i32,
) {
    let s         = |v: i32| state.s(v);
    let num_blocks = state.config.display.blocks as i32;

    // ── Colors ─────────────────────────────────────────────────────────────
    let col_filled  = if state.progress.is_weekend {
        Color::from_hex(&state.config.colors.weekend_filled)
    } else {
        Color::from_hex(filled_hex)
    };
    let col_empty = Color::from_hex(empty_hex);
    let col_text  = Color::from_hex(&state.config.colors.text_color);

    // ── Label ──────────────────────────────────────────────────────────────
    {
        let label_rect = RECT {
            left:   s(PADDING),
            top:    row_y,
            right:  s(PADDING + LABEL_W),
            bottom: row_y + s(ROW_H),
        };
        let font = create_font(dc, state.s(9));
        let old_font = SelectObject(dc, font);
        SetTextColor(dc, COLORREF(col_text.to_colorref()));
        SetBkMode(dc, TRANSPARENT);
        let mut lw: Vec<u16> = label.encode_utf16().collect();
        let mut lr = label_rect;
        DrawTextW(dc, &mut lw, &mut lr, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        SelectObject(dc, old_font);
        let _ = DeleteObject(font);
    }

    // ── Blocks ─────────────────────────────────────────────────────────────
    let filled_count = (pct * num_blocks as f64).floor() as i32;
    // partial fill fraction for the next block (0.0 – 1.0)
    let partial_frac = (pct * num_blocks as f64).fract();

    let block_w = s(BLOCK_W);
    let block_h = s(BLOCK_H);
    let block_gap = s(BLOCK_GAP);
    let radius   = s(BLOCK_RADIUS);
    let blocks_x = s(BLOCKS_X);
    let block_y  = row_y + (s(ROW_H) - block_h) / 2;

    for i in 0..num_blocks {
        let bx = blocks_x + i * (block_w + block_gap);

        let color = if i < filled_count {
            col_filled
        } else if i == filled_count && partial_frac > 0.01 {
            // Partial block: blend filled and empty
            blend_colors(col_filled, col_empty, partial_frac as f32)
        } else {
            col_empty
        };

        let brush = CreateSolidBrush(COLORREF(color.to_colorref()));
        let rgn   = CreateRoundRectRgn(
            bx, block_y,
            bx + block_w, block_y + block_h,
            radius * 2, radius * 2, // Win32 uses diameter, not radius
        );
        let _ = FillRgn(dc, rgn, brush);
        let _ = DeleteObject(rgn);
        let _ = DeleteObject(brush);
    }

    // ── Percentage / weekend label ─────────────────────────────────────────
    if state.config.display.show_percentage {
        let pct_text = if state.progress.is_weekend {
            state.config.display.weekend_label.clone()
        } else {
            format!("{:.0}%", pct * 100.0)
        };

        let pct_x = blocks_x + num_blocks * (block_w + block_gap) - block_gap + s(PCT_GAP);
        let pct_rect = RECT {
            left:   pct_x,
            top:    row_y,
            right:  pct_x + s(PCT_W),
            bottom: row_y + s(ROW_H),
        };

        let font = create_font(dc, state.s(9));
        let old_font = SelectObject(dc, font);
        SetTextColor(dc, COLORREF(col_text.to_colorref()));
        SetBkMode(dc, TRANSPARENT);
        let mut tw: Vec<u16> = pct_text.encode_utf16().collect();
        let mut tr = pct_rect;
        DrawTextW(dc, &mut tw, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        SelectObject(dc, old_font);
        let _ = DeleteObject(font);
    }
}

/// Simple linear blend between two colors (t=1.0 → a, t=0.0 → b).
fn blend_colors(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: (a.r as f32 * t + b.r as f32 * (1.0 - t)) as u8,
        g: (a.g as f32 * t + b.g as f32 * (1.0 - t)) as u8,
        b: (a.b as f32 * t + b.b as f32 * (1.0 - t)) as u8,
    }
}

/// Create a Segoe UI font at the given pixel height.
unsafe fn create_font(_dc: HDC, height_px: i32) -> HGDIOBJ {
    let face = wide_str("Segoe UI");
    CreateFontW(
        -height_px, // negative = character height in pixels
        0, 0, 0,
        FW_NORMAL.0 as i32,
        0, 0, 0,
        ANSI_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        (FF_SWISS.0 | VARIABLE_PITCH.0) as u32,
        PCWSTR::from_raw(face.as_ptr()),
    ).into()
}

// ── Context menu ───────────────────────────────────────────────────────────

unsafe fn show_context_menu(hwnd: HWND) {
    let menu = CreatePopupMenu().unwrap();
    AppendMenuW(menu, MF_STRING, IDM_SETTINGS as usize, PCWSTR::from_raw(wide_str("Settings (open config)").as_ptr())).ok();
    AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()).ok();
    AppendMenuW(menu, MF_STRING, IDM_EXIT as usize, PCWSTR::from_raw(wide_str("Exit").as_ptr())).ok();

    let mut cursor = POINT::default();
    let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut cursor);

    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(
        menu,
        TPM_RIGHTALIGN | TPM_BOTTOMALIGN | TPM_LEFTBUTTON,
        cursor.x, cursor.y,
        0, hwnd, None,
    );
    DestroyMenu(menu).ok();
}

// ── Positioning ────────────────────────────────────────────────────────────

/// Calculate the (x, y) screen position for the widget.
///
/// We place it just ABOVE the taskbar in the work area, flush to the right
/// edge of the screen. This avoids the Windows 11 taskbar covering TOPMOST
/// windows and ensures the widget is always visible.
unsafe fn calc_widget_position(w: i32, h: i32) -> (i32, i32) {
    let screen_w = GetSystemMetrics(SM_CXSCREEN);

    // Work area = desktop rectangle excluding the taskbar.
    // work_area.bottom is the pixel row where the taskbar begins (bottom taskbar).
    let mut work_area = RECT::default();
    SystemParametersInfoW(
        SPI_GETWORKAREA, 0,
        Some(&mut work_area as *mut RECT as *mut c_void),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    ).ok();

    // Sit 4 px above the taskbar, 8 px from the right edge
    let x = (screen_w - w - 8).max(0);
    let y = (work_area.bottom - h - 4).max(0);
    (x, y)
}

/// Reposition an already-created window (called on taskbar resize / reconnect).
#[allow(dead_code)]
unsafe fn reposition(hwnd: HWND, w: i32, h: i32) {
    let (x, y) = calc_widget_position(w, h);
    move_window(hwnd, x, y, w, h);
}
