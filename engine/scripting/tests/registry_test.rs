use hezhou_scripting::{CallbackRegistry, CallbackDescriptor, ScriptValue};

#[test]
fn test_registry_sync_callback_with_context() {
    let mut registry = CallbackRegistry::new();
    
    let descriptor = CallbackDescriptor::new_sync("multiply", "Multiply by context", "int -> int");
    
    extern "C" fn multiply_callback(arg: ScriptValue, context: usize) -> ScriptValue {
        let factor = context as i32;
        if let Some(val) = arg.get_int() {
            ScriptValue::from_int(val * factor)
        } else {
            ScriptValue::err("Expected int")
        }
    }
    
    registry.register_sync("multiply".to_string(), multiply_callback, descriptor, 10);
    
    let arg = ScriptValue::from_int(5);
    let result = registry.trigger_sync("multiply", arg).unwrap();
    
    assert_eq!(result.get_int(), Some(50));
}

#[test]
fn test_registry_different_contexts() {
    let mut registry = CallbackRegistry::new();
    
    extern "C" fn multiply(arg: ScriptValue, context: usize) -> ScriptValue {
        ScriptValue::from_int(arg.get_int().unwrap_or(0) * context as i32)
    }
    
    registry.register_sync(
        "double".to_string(),
        multiply,
        CallbackDescriptor::new_sync("double", "x2", "int->int"),
        2
    );
    
    registry.register_sync(
        "triple".to_string(),
        multiply,
        CallbackDescriptor::new_sync("triple", "x3", "int->int"),
        3
    );
    
    let arg = ScriptValue::from_int(10);
    
    assert_eq!(registry.trigger_sync("double", arg).unwrap().get_int(), Some(20));
    assert_eq!(registry.trigger_sync("triple", arg).unwrap().get_int(), Some(30));
}

#[test]
fn test_registry_error_handling() {
    let mut registry = CallbackRegistry::new();
    
    extern "C" fn error_cb(_arg: ScriptValue, _ctx: usize) -> ScriptValue {
        ScriptValue::err("Custom error")
    }
    
    registry.register_sync(
        "error_test".to_string(),
        error_cb,
        CallbackDescriptor::new_sync("error_test", "returns error", ""),
        0
    );
    
    let result = registry.trigger_sync("error_test", ScriptValue::null()).unwrap();
    assert!(result.is_err());
    assert_eq!(result.get_error_message(), Some("Custom error"));
}

#[test]
fn test_registry_unregister() {
    let mut registry = CallbackRegistry::new();
    
    extern "C" fn identity(arg: ScriptValue, _ctx: usize) -> ScriptValue { arg }
    
    registry.register_sync(
        "temp".to_string(),
        identity,
        CallbackDescriptor::new_sync("temp", "identity", ""),
        0
    );
    
    assert!(registry.trigger_sync("temp", ScriptValue::from_int(1)).is_ok());
    
    registry.unregister("temp");
    
    assert!(registry.trigger_sync("temp", ScriptValue::from_int(1)).is_err());
}

#[test]
fn test_registry_not_found() {
    let registry = CallbackRegistry::new();
    
    assert!(registry.trigger_sync("nonexistent", ScriptValue::null()).is_err());
}

#[test]
fn test_registry_list_callbacks() {
    let mut registry = CallbackRegistry::new();
    
    extern "C" fn sync_cb(_arg: ScriptValue, _ctx: usize) -> ScriptValue { ScriptValue::null() }
    extern "C" fn async_cb(_arg: ScriptValue, _ctx: usize, _ptr: usize) {}
    extern "C" fn task_cb(_arg: ScriptValue, _ctx: usize, _p1: usize, _p2: usize) {}
    
    registry.register_sync("s1".to_string(), sync_cb, CallbackDescriptor::new_sync("s1", "", ""), 0);
    registry.register_async("a1".to_string(), async_cb, CallbackDescriptor::new_async("a1", ""), 0);
    registry.register_task("t1".to_string(), task_cb, CallbackDescriptor::new_task("t1", "", true), true, 0);
    
    let list = registry.list_callbacks();
    assert_eq!(list.len(), 3);
}