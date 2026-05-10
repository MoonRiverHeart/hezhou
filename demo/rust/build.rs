fn main() {
    #[cfg(windows)]
    {
        // 指定 Windows SDK 库路径
        println!("cargo:rustc-link-search=native=C:\\Program Files (x86)\\Windows Kits\\10\\Lib\\10.0.26100.0\\um\\x64");
        
        // 链接系统库
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=bcrypt");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=oleaut32");
    }
    
    // csbindgen::Builder::default()
    //     .input_extern_file("src/lib.rs")
    //     .csharp_dll_name("csharptorust_lib")
    //     .generate_csharp_file("../csharp/CsharpCaller/NativeMethods.cs")
    //     .unwrap();
}