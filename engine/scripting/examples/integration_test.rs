use hezhou_scripting::*;
use std::ffi::CString;

fn main() {
    println!("=== Rust <-> C# Scripting Integration Test ===\n");
    
    std::env::set_var("MONO_PATH", "C:/Program Files/Mono/lib/mono/4.5");
    
    test_callback_context();
    test_error_propagation();
    test_multiple_callbacks();
    
    println!("\n=== All tests passed ===");
}

fn test_callback_context() {
    println!("[Test 1] Callback with context parameter");
    
    let manager_ptr = scripting_init();
    
    extern "C" fn multiply_by_context(arg: ScriptValue, context: usize) -> ScriptValue {
        let factor = context as i32;
        if let Some(val) = arg.get_int() {
            println!("  [Rust Callback] arg={}, context={}, result={}", val, factor, val * factor);
            ScriptValue::from_int(val * factor)
        } else {
            ScriptValue::err("Expected int")
        }
    }
    
    let name = CString::new("multiply").unwrap();
    let desc = CString::new("Multiply by context value").unwrap();
    let sig = CString::new("int -> int").unwrap();
    
    scripting_register_sync_callback(
        manager_ptr,
        name.as_ptr(),
        multiply_by_context,
        desc.as_ptr(),
        sig.as_ptr(),
        10,
    );
    
    let trigger_name = CString::new("multiply").unwrap();
    let arg = ScriptValue::from_int(5);
    let result = scripting_trigger_sync(manager_ptr, trigger_name.as_ptr(), arg);
    
    assert!(result.is_ok(), "Result should be ok");
    assert_eq!(result.get_int(), Some(50), "5 * 10 = 50");
    println!("  [PASS] Context parameter works: 5 * 10 = 50\n");
    
    scripting_shutdown(manager_ptr);
}

fn test_error_propagation() {
    println!("[Test 2] Error propagation");
    
    let manager_ptr = scripting_init();
    
    extern "C" fn error_callback(_arg: ScriptValue, _context: usize) -> ScriptValue {
        println!("  [Rust Callback] Returning error");
        ScriptValue::err("Test error from callback")
    }
    
    let name = CString::new("error_test").unwrap();
    let desc = CString::new("Returns error").unwrap();
    let sig = CString::new("any -> error").unwrap();
    
    scripting_register_sync_callback(
        manager_ptr,
        name.as_ptr(),
        error_callback,
        desc.as_ptr(),
        sig.as_ptr(),
        0,
    );
    
    let trigger_name = CString::new("error_test").unwrap();
    let result = scripting_trigger_sync(manager_ptr, trigger_name.as_ptr(), ScriptValue::null());
    
    assert!(result.is_err(), "Result should be error");
    let err_msg = result.get_error_message();
    assert_eq!(err_msg, Some("Test error from callback"));
    println!("  [PASS] Error propagation works: '{}'\n", err_msg.unwrap_or(""));
    
    scripting_shutdown(manager_ptr);
}

fn test_multiple_callbacks() {
    println!("[Test 3] Multiple callbacks with different contexts");
    
    let manager_ptr = scripting_init();
    
    extern "C" fn multiply(arg: ScriptValue, context: usize) -> ScriptValue {
        let factor = context as i32;
        ScriptValue::from_int(arg.get_int().unwrap_or(0) * factor)
    }
    
    let name1 = CString::new("double").unwrap();
    let name2 = CString::new("triple").unwrap();
    let name3 = CString::new("quadruple").unwrap();
    let desc = CString::new("").unwrap();
    let sig = CString::new("int -> int").unwrap();
    
    scripting_register_sync_callback(manager_ptr, name1.as_ptr(), multiply, desc.as_ptr(), sig.as_ptr(), 2);
    scripting_register_sync_callback(manager_ptr, name2.as_ptr(), multiply, desc.as_ptr(), sig.as_ptr(), 3);
    scripting_register_sync_callback(manager_ptr, name3.as_ptr(), multiply, desc.as_ptr(), sig.as_ptr(), 4);
    
    let arg = ScriptValue::from_int(100);
    
    let trigger1 = CString::new("double").unwrap();
    let trigger2 = CString::new("triple").unwrap();
    let trigger3 = CString::new("quadruple").unwrap();
    
    let r1 = scripting_trigger_sync(manager_ptr, trigger1.as_ptr(), arg);
    let r2 = scripting_trigger_sync(manager_ptr, trigger2.as_ptr(), arg);
    let r3 = scripting_trigger_sync(manager_ptr, trigger3.as_ptr(), arg);
    
    assert_eq!(r1.get_int(), Some(200), "100 * 2 = 200");
    assert_eq!(r2.get_int(), Some(300), "100 * 3 = 300");
    assert_eq!(r3.get_int(), Some(400), "100 * 4 = 400");
    
    println!("  double: 100 * 2 = {}", r1.get_int().unwrap());
    println!("  triple: 100 * 3 = {}", r2.get_int().unwrap());
    println!("  quadruple: 100 * 4 = {}", r3.get_int().unwrap());
    println!("  [PASS] Multiple callbacks work\n");
    
    scripting_shutdown(manager_ptr);
}