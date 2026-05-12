fn main() {
    csbindgen::Builder::default()
        .input_extern_file("src/lib.rs")
        .csharp_dll_name("callback_demo")
        .generate_csharp_file("../csharp/CallbackNative.cs")
        .unwrap();
}