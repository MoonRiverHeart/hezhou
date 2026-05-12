use hezhou_scripting::callback_registry::CallbackRegistry;
use hezhou_scripting::callback_types::CallbackDescriptor;
use hezhou_scripting::value_bridge::ScriptValue;

#[test]
fn test_sync_callback_with_context() {
    let mut registry = CallbackRegistry::new();
    
    let descriptor = CallbackDescriptor::new_sync("test_add", "Add numbers", "int -> int");
    
    extern "C" fn test_callback(arg: ScriptValue, context: usize) -> ScriptValue {
        let multiplier = context as i32;
        if let Some(val) = arg.get_int() {
            ScriptValue::from_int(val * multiplier)
        } else {
            ScriptValue::err("Expected int")
        }
    }
    
    registry.register_sync("test_add".to_string(), test_callback, descriptor, 10);
    
    let arg = ScriptValue::from_int(5);
    let result = registry.trigger_sync("test_add", arg).unwrap();
    
    assert_eq!(result.get_int(), Some(50));
}

#[test]
fn test_callback_not_found() {
    let registry = CallbackRegistry::new();
    
    let arg = ScriptValue::from_int(5);
    let result = registry.trigger_sync("nonexistent", arg);
    
    assert!(result.is_err());
}

#[test]
fn test_unregister_callback() {
    let mut registry = CallbackRegistry::new();
    
    let descriptor = CallbackDescriptor::new_sync("temp", "Temporary", "int -> int");
    
    extern "C" fn temp_callback(arg: ScriptValue, _context: usize) -> ScriptValue {
        arg
    }
    
    registry.register_sync("temp".to_string(), temp_callback, descriptor, 0);
    
    let arg = ScriptValue::from_int(5);
    let result = registry.trigger_sync("temp", arg);
    assert!(result.is_ok());
    
    registry.unregister("temp");
    
    let result2 = registry.trigger_sync("temp", arg);
    assert!(result2.is_err());
}

#[test]
fn test_list_callbacks() {
    let mut registry = CallbackRegistry::new();
    
    let sync_desc = CallbackDescriptor::new_sync("sync1", "Sync callback", "int -> int");
    let async_desc = CallbackDescriptor::new_async("async1", "Async callback");
    let task_desc = CallbackDescriptor::new_task("task1", "Task callback", true);
    
    extern "C" fn sync_cb(_arg: ScriptValue, _ctx: usize) -> ScriptValue { ScriptValue::null() }
    extern "C" fn async_cb(_arg: ScriptValue, _ctx: usize, _ptr: usize) {}
    extern "C" fn task_cb(_arg: ScriptValue, _ctx: usize, _p1: usize, _p2: usize) {}
    
    registry.register_sync("sync1".to_string(), sync_cb, sync_desc, 0);
    registry.register_async("async1".to_string(), async_cb, async_desc, 0);
    registry.register_task("task1".to_string(), task_cb, task_desc, true, 0);
    
    let list = registry.list_callbacks();
    assert_eq!(list.len(), 3);
}