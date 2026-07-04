use tao::platform::windows::WindowExtWindows;
use tao::window::Window;

use std::ffi::c_void;

pub fn set_rounded_corners(window: &Window) {
    unsafe {
        use windows::Win32::Graphics::Dwm::{
            DWM_WINDOW_CORNER_PREFERENCE, DWMWA_WINDOW_CORNER_PREFERENCE, DwmSetWindowAttribute,
        };
        let hwnd = windows::Win32::Foundation::HWND(window.hwnd() as *mut c_void);
        let preference = DWM_WINDOW_CORNER_PREFERENCE(2);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
}

pub fn set_system_backdrop(window: &Window) {
    unsafe {
        use windows::Win32::Graphics::Dwm::{
            DWM_SYSTEMBACKDROP_TYPE, DWMWA_SYSTEMBACKDROP_TYPE, DwmSetWindowAttribute,
        };
        let hwnd = windows::Win32::Foundation::HWND(window.hwnd() as *mut c_void);
        let backdrop = DWM_SYSTEMBACKDROP_TYPE(2);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        );
    }
}
