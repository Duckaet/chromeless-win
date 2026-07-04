use std::ffi::c_void;
use tao::platform::windows::WindowExtWindows;
use tao::window::Window;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

pub fn capture_window_png(window: &Window) -> Option<Vec<u8>> {
    unsafe {
        let hwnd = HWND(window.hwnd() as *mut c_void);

        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect).is_err() {
            return None;
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return None;
        }

        let hdc_screen = GetDC(Some(hwnd));
        if hdc_screen.0.is_null() {
            return None;
        }

        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        if hdc_mem.0.is_null() {
            ReleaseDC(Some(hwnd), hdc_screen);
            return None;
        }

        let hbmp = CreateCompatibleBitmap(hdc_screen, width, height);
        ReleaseDC(Some(hwnd), hdc_screen);

        if hbmp.0.is_null() {
            let _ = DeleteDC(hdc_mem);
            return None;
        }

        let old = SelectObject(hdc_mem, HGDIOBJ(hbmp.0));
        let pw_flags = PRINT_WINDOW_FLAGS(0x00000002);
        let _ = PrintWindow(hwnd, hdc_mem, pw_flags);
        SelectObject(hdc_mem, old);

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0u32,
                ..Default::default()
            },
            bmiColors: [RGBQUAD::default(); 1],
        };

        let pixel_count = (width * height) as usize;
        let mut pixels: Vec<u8> = vec![0u8; pixel_count * 4];

        let scan_lines = GetDIBits(
            hdc_mem,
            hbmp,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        let _ = DeleteObject(HGDIOBJ(hbmp.0));
        let _ = DeleteDC(hdc_mem);

        if scan_lines == 0 {
            return None;
        }

        let mut rgb = Vec::with_capacity(pixel_count * 3);
        for chunk in pixels.chunks_exact(4) {
            rgb.push(chunk[2]);
            rgb.push(chunk[1]);
            rgb.push(chunk[0]);
        }

        let img = image::RgbImage::from_raw(width as u32, height as u32, rgb)?;
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).ok()?;
        Some(buf.into_inner())
    }
}
