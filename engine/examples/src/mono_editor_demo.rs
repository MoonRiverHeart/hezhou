#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod editor;

#[cfg(windows)]
fn redirect_stdout_to_null() {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateFileW(
            lp_file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *mut c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: *mut c_void,
        ) -> *mut c_void;
        fn SetStdHandle(std_handle: u32, handle: *mut c_void) -> i32;
    }
    let name: Vec<u16> = "NUL\0".encode_utf16().collect();
    unsafe {
        let handle = CreateFileW(
            name.as_ptr(),
            0x4000_0000, // GENERIC_WRITE
            0x0000_0001 | 0x0000_0002, // FILE_SHARE_READ | FILE_SHARE_WRITE
            std::ptr::null_mut(),
            3, // OPEN_EXISTING
            0,
            std::ptr::null_mut(),
        );
        if handle as isize != -1 {
            SetStdHandle(0xFFFF_FFF5, handle); // STD_OUTPUT_HANDLE
            SetStdHandle(0xFFFF_FFF4, handle); // STD_ERROR_HANDLE
        }
    }
}

fn startup_trace(msg: &str) {
    let root = std::env::var_os("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("Hezhou").join("Editor"))
        .unwrap_or_else(|| std::env::temp_dir().join("hezhou"));
    let _ = std::fs::create_dir_all(&root);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(root.join("startup.log")) {
        use std::io::Write;
        let _ = writeln!(f, "{}", msg);
    }
}

fn main() {
    // GUI subsystem has no console; point CRT stdout/stderr at NUL so logger
    // println!/eprintln! do not panic.
    startup_trace("main");
    #[cfg(windows)]
    redirect_stdout_to_null();
    startup_trace("stdout redirected");
    editor::run();
    startup_trace("editor returned");
}