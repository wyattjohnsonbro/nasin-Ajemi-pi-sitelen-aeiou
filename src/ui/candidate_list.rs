use std::{cmp::max, ffi::{CString, OsString}, mem::{self, size_of, ManuallyDrop}};
use log::{trace, debug, error};
use windows::{Win32::{UI::WindowsAndMessaging::{CreateWindowExA, DefWindowProcA, DestroyWindow, GetWindowLongPtrA, LoadCursorW, RegisterClassExA, SetWindowLongPtrA, SetWindowPos, ShowWindow, CS_DROPSHADOW, CS_HREDRAW, CS_IME, CS_VREDRAW, HICON, HWND_TOPMOST, IDC_ARROW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, WINDOW_LONG_PTR_INDEX, WM_PAINT, WNDCLASSEXA, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP}, Foundation::{GetLastError, BOOL, E_FAIL, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM}, Graphics::Gdi::{self, BeginPaint, CreateFontA, EndPaint, GetDC, GetDeviceCaps, GetTextExtentPoint32W, InvalidateRect, ReleaseDC, SelectObject, SetBkMode, SetTextColor, TextOutW, HDC, HFONT, LOGPIXELSY, OUT_TT_PRECIS, PAINTSTRUCT, TRANSPARENT}}, core::{s, PCSTR}};
use windows::core::Result;
use crate::{engine::Suggestion, extend::OsStrExt2, global, ui::Color, CANDI_INDEXES, CANDI_INDEX_SUFFIX, FONT_SIZE};

const WINDOW_CLASS: PCSTR = s!("CANDIDATE_LIST");
// Color scheme
const TEXT_COLOR: Color = Color::gray(0);
const TEXT_HIGHLIGHT_COLOR: Color = Color::gray(0);
const TEXT_INDEX_COLOR: Color = Color::gray(160);
const CLIP_COLOR: Color =  Color::hex(0x0078D7);
const WND_COLOR: Color = Color::gray(250);
const HIGHTLIGHT_COLOR: Color = Color::rgb(232, 232, 255);

// Dark Mode Color scheme
// const TEXT_COLOR: Color = Color::gray(255);
// const TEXT_HIGHLIGHT_COLOR: Color = Color::gray(255);
// const TEXT_INDEX_COLOR: Color = Color::gray(96);
// const CLIP_COLOR: Color =  Color::rgb(200, 0, 0);
// const WND_COLOR: Color = Color::gray(16);
// const HIGHTLIGHT_COLOR: Color = Color::rgb(128, 0, 0);

// Layout
const HORIZONTAL: bool = true;
const CLIP_WIDTH: i32 = 3;
const LABEL_PADDING_TOP: i32 = 2;
const LABEL_PADDING_BOTTOM: i32 = 2;
const LABEL_PADDING_LEFT: i32 = 3;
const LABEL_PADDING_RIGHT: i32 = 4;
const BORDER_WIDTH: i32 = 0;

const POS_OFFSETX: i32 = 2;
const POS_OFFSETY: i32 = 2;

/// To create a window you need to register the window class beforehand.
pub fn setup() -> Result<()> {
    let wcex = WNDCLASSEXA {
        cbSize: size_of::<WNDCLASSEXA>() as u32,
        style: CS_IME | CS_HREDRAW | CS_VREDRAW | CS_DROPSHADOW,
        lpfnWndProc: Some(wind_proc),
        cbClsExtra: 0,
        cbWndExtra: size_of::<Box<PaintArg>>().try_into().unwrap(),
        hInstance: global::dll_module(),
        hIcon: HICON::default(),
        hCursor: unsafe{ LoadCursorW(None, IDC_ARROW)? },
        hbrBackground: Color::hex(0xFFFFFF).into(),
        lpszMenuName: PCSTR::null(),
        lpszClassName: WINDOW_CLASS,
        hIconSm: HICON::default()
    };
    unsafe {
        if RegisterClassExA(&wcex) == 0 {
            error!("Failed to register window class for candidate list");
            return GetLastError();
        }
        debug!("Registered window class for candidate list.");
    }
    Ok(())
}

/// use default handlers for everything but repaint
unsafe extern "system" fn wind_proc(window: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => paint(window),
        _  => DefWindowProcA(window, msg, wparam, lparam),
    }
}


//----------------------------------------------------------------------------
//
//  The implementation
//
//----------------------------------------------------------------------------

#[derive(Default)]
pub struct CandidateList {
    window: HWND,
    font: HFONT,
}

impl CandidateList {
    pub fn create(_parent_window: HWND) -> Result<CandidateList> {
        // WS_EX_TOOLWINDOW:  A floating toolbar that won't appear in taskbar and ALT+TAB.
        // WS_EX_NOACTIVATE:  A window that doesn't take the foreground thus not making parent window lose focus.
        // WS_EX_TOPMOST:     A window that is topmost.
        // WS_POPUP:          A window having no top bar or border.
        // see: https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
        unsafe {
            let window = CreateWindowExA(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST, 
                WINDOW_CLASS, PCSTR::null(),
                WS_POPUP,
                0, 0, 0, 0, None, None, 
                global::dll_module(),
                None);
            if window.0 == 0 {
                error!("CreateWindowExA returned null.");
                return match GetLastError() {
                    Ok(_) => Err(E_FAIL.into()),
                    Err(e) => Err(e)
                };
            }
            let dc: HDC = GetDC(window);
            let pixel_per_inch = GetDeviceCaps(dc, LOGPIXELSY);
            let font_size = FONT_SIZE * pixel_per_inch / 72;
            let font_name = CString::new(global::FONT).unwrap();
            let font_name = PCSTR::from_raw(font_name.as_bytes_with_nul().as_ptr());
            let font = CreateFontA (
                font_size, 0, 0, 0, 0, 0, 0, 0, 0, OUT_TT_PRECIS.0 as u32, 0, 0, 0, font_name);
            if font.is_invalid() {
                error!("CreateFontA failed.");
                return match GetLastError() {
                    Ok(_) => Err(E_FAIL.into()),
                    Err(e) => Err(e)
                };
            }
            ReleaseDC(window, dc);
            Ok(CandidateList{ window, font })
        }
    }

    pub fn locate(&self, x: i32, y: i32) -> Result<()>{
        trace!("locate({x}, {y})");
        unsafe {SetWindowPos(
            self.window, HWND_TOPMOST, 
            x + POS_OFFSETX, y + POS_OFFSETY, 0, 0,
            SWP_NOACTIVATE | SWP_NOSIZE)? };

        Ok(())
    }

    pub fn show(&self, suggs: &Vec<Suggestion>) -> Result<()> {
        unsafe{ 
            let mut indice = Vec::with_capacity(suggs.len());
            let mut candis = Vec::with_capacity(suggs.len());

            let mut row_height: i32 = 0;
            let mut index_width: i32 = 0;
            let mut candi_width: i32 = 0;
            let mut candi_widths = Vec::with_capacity(suggs.len());
                
            let dc: HDC = GetDC(self.window);
            SelectObject(dc, self.font);
            for (index, sugg) in suggs.iter().enumerate() {
                let mut size = SIZE::default();
                let index = format!("{}{}", CANDI_INDEXES[index], CANDI_INDEX_SUFFIX);
                let index = OsString::from(index).wchars();
                GetTextExtentPoint32W(dc, &index, &mut size);
                row_height = max(row_height, size.cy);
                index_width = max(index_width, size.cx);
                indice.push(index);

                let candi = OsString::from(&sugg.output).wchars();
                GetTextExtentPoint32W(dc, &candi, &mut size);
                row_height = max(row_height, size.cy);
                candi_width = max(candi_width, size.cx);
                candi_widths.push(size.cx);
                candis.push(candi);
            }
            ReleaseDC(self.window, dc);
            let label_height = LABEL_PADDING_TOP + row_height + LABEL_PADDING_BOTTOM;
            let mut wnd_height = 0;
            let mut wnd_width = 0;
            if HORIZONTAL {
                wnd_height += label_height;
                wnd_width += CLIP_WIDTH;
                for candi_width in candi_widths.iter() {
                    wnd_width += LABEL_PADDING_LEFT + LABEL_PADDING_RIGHT;
                    wnd_width += index_width;
                    wnd_width += candi_width;
                }
            } else {
                let candi_num: i32 = suggs.len().try_into().unwrap();
                wnd_height += candi_num * label_height;
                wnd_width += CLIP_WIDTH + LABEL_PADDING_LEFT + index_width + candi_width + LABEL_PADDING_RIGHT;
                wnd_width = max(wnd_width, wnd_height * 4 / 5)
            }
            wnd_height += BORDER_WIDTH * 2;
            wnd_width += BORDER_WIDTH * 2;

            let highlight_width = if HORIZONTAL {
                LABEL_PADDING_LEFT + index_width + candi_widths[0] + LABEL_PADDING_RIGHT
            } else {
                wnd_width - CLIP_WIDTH - BORDER_WIDTH * 2
            };

            // passing extra args to WndProc
            let arg = PaintArg {
                wnd_width, wnd_height, highlight_width, label_height,
                index_width,  candi_widths: candi_widths.clone(),
                candis, indice, font: self.font,
            };
            let long_ptr = arg.to_long_ptr();
            SetWindowLongPtrA(self.window, WINDOW_LONG_PTR_INDEX::default(), long_ptr);
            // resize and show
            SetWindowPos(
                self.window, HWND_TOPMOST, 0, 0, wnd_width, wnd_height, SWP_NOACTIVATE | SWP_NOMOVE)?;
            ShowWindow(self.window, SW_SHOWNOACTIVATE);
            // force repaint
            InvalidateRect(self.window, None, BOOL::from(true));
        };
        Ok(())
    }

    pub fn hide(&self) {
        unsafe { 
            ShowWindow(self.window, SW_HIDE); 
        }
    }

    pub fn destroy(&self) -> Result<()> {
        unsafe { DestroyWindow(self.window) }
    }
}

struct PaintArg {
    wnd_width: i32,
    wnd_height: i32,
    highlight_width: i32,
    label_height: i32,
    index_width: i32,
    candi_widths: Vec<i32>,
    font: HFONT,
    indice: Vec<Vec<u16>>,
    candis: Vec<Vec<u16>>,
}
impl PaintArg {
    unsafe fn to_long_ptr(self) -> isize{
        mem::transmute(ManuallyDrop::new(Box::new(self)))
    }
    unsafe fn from_long_ptr(long_ptr: isize) -> Option<Box<PaintArg>>{
        if long_ptr == 0 {
            None
        } else {
            Some(mem::transmute(long_ptr))
        }
    }
}

unsafe fn paint(window: HWND) -> LRESULT{
    // load the extra arg
    let Some(arg) = PaintArg::from_long_ptr(GetWindowLongPtrA(window, WINDOW_LONG_PTR_INDEX::default())) else {
        error!("Args for repaint is not found.");
        return LRESULT::default();
    };
    SetWindowLongPtrA(window, WINDOW_LONG_PTR_INDEX::default(), 0);
    let mut ps = PAINTSTRUCT::default();
    let dc: HDC = BeginPaint(window, &mut ps);
    if dc.is_invalid() {
        error!("BeginPaint failed.");
        return LRESULT::default();
    }
    // window
    FillRect(dc, 0, 0, arg.wnd_width, arg.wnd_height, WND_COLOR);
    // clip
    FillRect(dc, BORDER_WIDTH, BORDER_WIDTH, CLIP_WIDTH, arg.label_height, CLIP_COLOR);
    // highlight
    FillRect(dc, BORDER_WIDTH + CLIP_WIDTH, BORDER_WIDTH, arg.highlight_width, arg.label_height, HIGHTLIGHT_COLOR);
    // highlighted text
    let mut index_x = BORDER_WIDTH + CLIP_WIDTH + LABEL_PADDING_LEFT;
    let mut candi_x = BORDER_WIDTH + index_x + arg.index_width;
    let mut y = BORDER_WIDTH + LABEL_PADDING_TOP;
    SelectObject(dc, arg.font);
    SetBkMode(dc, TRANSPARENT);
    TextOut(dc, index_x, y, &arg.indice[0], TEXT_INDEX_COLOR);
    TextOut(dc, candi_x, y, &arg.candis[0], TEXT_HIGHLIGHT_COLOR);
    // normal text
    for i in 1..arg.candis.len() {
        if HORIZONTAL {
            index_x += arg.index_width + arg.candi_widths[i - 1] + LABEL_PADDING_LEFT + LABEL_PADDING_RIGHT;
            candi_x += arg.index_width + arg.candi_widths[i - 1] + LABEL_PADDING_LEFT + LABEL_PADDING_RIGHT;
        } else {
            y += arg.label_height;
        }
        TextOut(dc, index_x, y, &arg.indice[i], TEXT_INDEX_COLOR);
        TextOut(dc, candi_x, y, &arg.candis[i], TEXT_COLOR);
    }
    ReleaseDC(window, dc);
    EndPaint(window, &mut ps);
    LRESULT::default()
}

#[allow(non_snake_case)]
unsafe fn TextOut(hdc: HDC, x: i32, y: i32, wchars:&[u16], color: Color) {
    SetTextColor(hdc, color);
    TextOutW(hdc, x, y, wchars);
}

#[allow(non_snake_case)]
unsafe fn FillRect(hdc: HDC, x: i32, y: i32, width: i32, height: i32, color: Color) {
    let rect = RECT {
        left: x,
        top: y,
        right: x + width,
        bottom: height,
    };
    Gdi::FillRect(hdc, &rect, color);
}


