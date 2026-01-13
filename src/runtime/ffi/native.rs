#![allow(unsafe_code)]

use crate::runtime::value_ptr::ValueMutPtr;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::path::PathBuf;

#[cfg(target_family = "unix")]
mod platform {
    use super::*;
    use std::os::raw::{c_char, c_int, c_void};
    use std::ptr;

    #[cfg_attr(target_os = "linux", link(name = "dl"))]
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    const RTLD_LOCAL: c_int = 0;
    const RTLD_NOW: c_int = 2;

    pub(super) unsafe fn load_library(path: &CStr) -> *mut c_void {
        let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
        if handle.is_null() {
            ptr::null_mut()
        } else {
            handle
        }
    }

    pub(super) unsafe fn resolve_symbol(handle: *mut c_void, symbol: &CStr) -> *mut c_void {
        unsafe { dlsym(handle, symbol.as_ptr()) }
    }
}

#[cfg(target_family = "windows")]
mod platform {
    use super::*;
    use std::iter;
    use std::os::raw::c_void;
    use std::os::windows::ffi::OsStrExt;

    type HMODULE = *mut c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(lpFileName: *const u16) -> HMODULE;
        fn GetProcAddress(hModule: HMODULE, lpProcName: *const c_char) -> *mut c_void;
    }

    pub(super) unsafe fn load_library(path: &CStr) -> *mut c_void {
        let wide: Vec<u16> = std::path::Path::new(&path.to_string_lossy().to_string())
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect();
        let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
        if handle.is_null() {
            std::ptr::null_mut()
        } else {
            handle as *mut c_void
        }
    }

    pub(super) unsafe fn resolve_symbol(handle: *mut c_void, symbol: &CStr) -> *mut c_void {
        unsafe { GetProcAddress(handle as HMODULE, symbol.as_ptr()) }
    }
}

#[cfg(not(any(target_family = "unix", target_family = "windows")))]
mod platform {
    use super::*;

    pub(super) unsafe fn load_library(_path: &CStr) -> *mut c_void {
        std::ptr::null_mut()
    }

    pub(super) unsafe fn resolve_symbol(_handle: *mut c_void, _symbol: &CStr) -> *mut c_void {
        std::ptr::null_mut()
    }
}

unsafe fn path_slice<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    if ptr.is_null() || len == 0 {
        return None;
    }
    unsafe { Some(std::slice::from_raw_parts(ptr, len)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chic_rt_host_load_library(path_ptr: *const u8, len: u32) -> *mut c_void {
    let Some(bytes) = (unsafe { path_slice(path_ptr, len as usize) }) else {
        return std::ptr::null_mut();
    };
    let path_cstr = match CString::new(bytes) {
        Ok(cstr) => cstr,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { platform::load_library(&path_cstr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chic_rt_host_resolve_symbol(
    handle: *mut c_void,
    symbol_ptr: *const u8,
    len: u32,
) -> *mut c_void {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let Some(bytes) = (unsafe { path_slice(symbol_ptr, len as usize) }) else {
        return std::ptr::null_mut();
    };
    let symbol = match CString::new(bytes) {
        Ok(cstr) => cstr,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { platform::resolve_symbol(handle, &symbol) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chic_rt_host_ffi_warn(ptr: *const u8, len: u32) {
    let Some(bytes) = (unsafe { path_slice(ptr, len as usize) }) else {
        return;
    };
    if let Ok(text) = std::str::from_utf8(bytes) {
        eprintln!("{text}");
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chic_rt_host_ffi_panic(ptr: *const u8, len: u32) {
    let Some(bytes) = (unsafe { path_slice(ptr, len as usize) }) else {
        panic!("ffi: missing required binding");
    };
    if let Ok(text) = std::str::from_utf8(bytes) {
        panic!("{text}");
    } else {
        panic!("ffi: missing required binding");
    }
}

fn host_path(kind: &str) -> Option<PathBuf> {
    match kind {
        "exe" => std::env::current_exe().ok(),
        "cwd" => std::env::current_dir().ok(),
        _ => None,
    }
}

unsafe fn store_path(path: PathBuf, out_ptr: *mut ValueMutPtr) -> u32 {
    let Some(str_path) = path.to_str() else {
        return 0;
    };
    let mut owned = str_path.as_bytes().to_vec();
    let len = owned.len();
    let ptr = owned.as_mut_ptr();
    std::mem::forget(owned);
    if out_ptr.is_null() {
        return 0;
    }
    unsafe {
        (*out_ptr).ptr = ptr;
        (*out_ptr).size = len;
        (*out_ptr).align = 1;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chic_rt_host_current_exe(out_path: *mut ValueMutPtr) -> u32 {
    let Some(path) = host_path("exe") else {
        return 0;
    };
    unsafe { store_path(path, out_path) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chic_rt_host_current_dir(out_path: *mut ValueMutPtr) -> u32 {
    let Some(path) = host_path("cwd") else {
        return 0;
    };
    unsafe { store_path(path, out_path) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chic_rt_host_free_path(path: ValueMutPtr) {
    if path.ptr.is_null() || path.size == 0 {
        return;
    }
    unsafe {
        let _ = Vec::from_raw_parts(path.ptr, path.size, path.size);
    }
}
