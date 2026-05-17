use hezhou_scripting::*;
use std::ffi::CString;

#[test]
fn test_ffi_init_and_shutdown() {
    let manager_ptr = scripting_init();
    assert!(
        !manager_ptr.is_null(),
        "scripting_init should return non-null pointer"
    );

    scripting_shutdown(manager_ptr);
}

#[test]
fn test_ffi_register_and_trigger_sync_callback() {
    let manager_ptr = scripting_init();

    extern "C" fn test_multiply(arg: ScriptValue, context: usize) -> ScriptValue {
        let multiplier = context as i32;
        if let Some(val) = arg.get_int() {
            ScriptValue::from_int(val * multiplier)
        } else {
            ScriptValue::err("Expected int")
        }
    }

    let name = CString::new("multiply").unwrap();
    let desc = CString::new("Multiply by 10").unwrap();
    let sig = CString::new("int -> int").unwrap();

    scripting_register_sync_callback(
        manager_ptr,
        name.as_ptr(),
        test_multiply,
        desc.as_ptr(),
        sig.as_ptr(),
        10,
    );

    let trigger_name = CString::new("multiply").unwrap();
    let arg = ScriptValue::from_int(5);
    let result = scripting_trigger_sync(manager_ptr, trigger_name.as_ptr(), arg);

    assert!(result.is_ok(), "Result should be ok");
    assert_eq!(result.get_int(), Some(50), "5 * 10 should be 50");

    scripting_shutdown(manager_ptr);
}

#[test]
fn test_ffi_error_propagation() {
    let manager_ptr = scripting_init();

    extern "C" fn error_callback(_arg: ScriptValue, _context: usize) -> ScriptValue {
        ScriptValue::err("Test error message")
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
    let arg = ScriptValue::null();
    let result = scripting_trigger_sync(manager_ptr, trigger_name.as_ptr(), arg);

    assert!(result.is_err(), "Result should be error");
    assert_eq!(result.get_error_message(), Some("Test error message"));

    scripting_shutdown(manager_ptr);
}

#[test]
fn test_ffi_unregister_callback() {
    let manager_ptr = scripting_init();

    extern "C" fn temp_callback(arg: ScriptValue, _context: usize) -> ScriptValue {
        arg
    }

    let name = CString::new("temp").unwrap();
    let desc = CString::new("Temporary").unwrap();
    let sig = CString::new("any -> any").unwrap();

    scripting_register_sync_callback(
        manager_ptr,
        name.as_ptr(),
        temp_callback,
        desc.as_ptr(),
        sig.as_ptr(),
        0,
    );

    let trigger_name = CString::new("temp").unwrap();
    let arg = ScriptValue::from_int(42);
    let result = scripting_trigger_sync(manager_ptr, trigger_name.as_ptr(), arg);
    assert!(result.is_ok());

    scripting_unregister_callback(manager_ptr, name.as_ptr());

    let result2 = scripting_trigger_sync(manager_ptr, trigger_name.as_ptr(), arg);
    assert!(
        result2.is_err(),
        "Callback should not exist after unregister"
    );

    scripting_shutdown(manager_ptr);
}

#[test]
fn test_context_varies_between_calls() {
    let manager_ptr = scripting_init();

    extern "C" fn multiply_by_context(arg: ScriptValue, context: usize) -> ScriptValue {
        let factor = context as i32;
        if let Some(val) = arg.get_int() {
            ScriptValue::from_int(val * factor)
        } else {
            ScriptValue::err("Expected int")
        }
    }

    let name1 = CString::new("double").unwrap();
    let desc1 = CString::new("Multiply by 2").unwrap();
    let sig1 = CString::new("int -> int").unwrap();

    scripting_register_sync_callback(
        manager_ptr,
        name1.as_ptr(),
        multiply_by_context,
        desc1.as_ptr(),
        sig1.as_ptr(),
        2,
    );

    let name2 = CString::new("triple").unwrap();
    let desc2 = CString::new("Multiply by 3").unwrap();
    let sig2 = CString::new("int -> int").unwrap();

    scripting_register_sync_callback(
        manager_ptr,
        name2.as_ptr(),
        multiply_by_context,
        desc2.as_ptr(),
        sig2.as_ptr(),
        3,
    );

    let trigger1 = CString::new("double").unwrap();
    let trigger2 = CString::new("triple").unwrap();
    let arg = ScriptValue::from_int(10);

    let result1 = scripting_trigger_sync(manager_ptr, trigger1.as_ptr(), arg);
    assert_eq!(result1.get_int(), Some(20), "10 * 2 = 20");

    let result2 = scripting_trigger_sync(manager_ptr, trigger2.as_ptr(), arg);
    assert_eq!(result2.get_int(), Some(30), "10 * 3 = 30");

    scripting_shutdown(manager_ptr);
}
