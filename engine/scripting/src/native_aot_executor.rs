use crate::error::ScriptError;
use crate::executor::ScriptExecutor;
use crate::value_bridge::ScriptValue;
use libloading::Library;
use parking_lot::Mutex;
use std::sync::LazyLock;

type InitializeFn = extern "C" fn();
type GetRotationSpeedFn = extern "C" fn() -> f32;
type SetRotationSpeedFn = extern "C" fn(f32);
type ResetRotationFn = extern "C" fn();
type TriggerRotationCallbackFn = extern "C" fn(f32) -> f32;

static ROTATION_CALLBACK: LazyLock<Mutex<Option<extern "C" fn(f32) -> f32>>> = 
    LazyLock::new(|| Mutex::new(None));

#[unsafe(no_mangle)]
pub extern "C" fn register_rotation_callback(callback: extern "C" fn(f32) -> f32) {
    let mut cb = ROTATION_CALLBACK.lock();
    *cb = Some(callback);
}

pub struct NativeAotExecutor {
    library: Option<Library>,
    dll_path: String,
    initialize: Option<InitializeFn>,
    get_rotation_speed: Option<GetRotationSpeedFn>,
    set_rotation_speed: Option<SetRotationSpeedFn>,
    reset_rotation: Option<ResetRotationFn>,
}

impl NativeAotExecutor {
    pub fn new() -> Self {
        Self {
            library: None,
            dll_path: String::new(),
            initialize: None,
            get_rotation_speed: None,
            set_rotation_speed: None,
            reset_rotation: None,
        }
    }
}

impl ScriptExecutor for NativeAotExecutor {
    fn load(&mut self, path: &str) -> Result<(), ScriptError> {
        unsafe {
            let library = Library::new(path)
                .map_err(|e| ScriptError::LoadFailed(format!("Failed to load DLL: {}", e)))?;
            
            let initialize: InitializeFn = *library
                .get(b"csharp_initialize")
                .map_err(|e| ScriptError::SymbolNotFound(format!("csharp_initialize: {}", e)))?;
            
            let get_rotation_speed: GetRotationSpeedFn = *library
                .get(b"csharp_get_rotation_speed")
                .map_err(|e| ScriptError::SymbolNotFound(format!("csharp_get_rotation_speed: {}", e)))?;
            
            let set_rotation_speed: SetRotationSpeedFn = *library
                .get(b"csharp_set_rotation_speed")
                .map_err(|e| ScriptError::SymbolNotFound(format!("csharp_set_rotation_speed: {}", e)))?;
            
            let reset_rotation: ResetRotationFn = *library
                .get(b"csharp_reset_rotation")
                .map_err(|e| ScriptError::SymbolNotFound(format!("csharp_reset_rotation: {}", e)))?;
            
            initialize();
            
            self.library = Some(library);
            self.dll_path = path.to_string();
            self.initialize = Some(initialize);
            self.get_rotation_speed = Some(get_rotation_speed);
            self.set_rotation_speed = Some(set_rotation_speed);
            self.reset_rotation = Some(reset_rotation);
            
            Ok(())
        }
    }
    
    fn reload(&mut self) -> Result<(), ScriptError> {
        Err(ScriptError::UnsupportedOperation("NativeAOT does not support hot reload. Use Mono for development.".to_string()))
    }
    
    fn call(&self, method: &str, args: ScriptValue) -> Result<ScriptValue, ScriptError> {
        match method {
            "GetRotationSpeed" => {
                let speed = self.get_rotation_speed
                    .ok_or(ScriptError::NotInitialized)?();
                Ok(ScriptValue::from_float(speed))
            }
            "SetRotationSpeed" => {
                let speed = if args.type_tag == 2 {
                    args.float_value
                } else if args.type_tag == 1 {
                    args.int_value as f32
                } else {
                    return Err(ScriptError::InvalidArgument);
                };
                self.set_rotation_speed
                    .ok_or(ScriptError::NotInitialized)?(speed);
                Ok(ScriptValue::null())
            }
            _ => Err(ScriptError::MethodNotFound(method.to_string())),
        }
    }
    
    fn unload(&mut self) {
        self.library = None;
        self.dll_path.clear();
        self.initialize = None;
        self.get_rotation_speed = None;
        self.set_rotation_speed = None;
        self.reset_rotation = None;
    }
    
    fn get_rotation_speed(&self) -> Result<f32, ScriptError> {
        self.get_rotation_speed
            .ok_or(ScriptError::NotInitialized)?()
            .pipe(|speed| Ok(speed))
    }
    
    fn set_rotation_speed(&mut self, speed: f32) -> Result<(), ScriptError> {
        self.set_rotation_speed
            .ok_or(ScriptError::NotInitialized)?(speed);
        Ok(())
    }
}

trait Pipe<T> {
    fn pipe<U, F: FnOnce(T) -> U>(self, f: F) -> U;
}

impl<T> Pipe<T> for T {
    fn pipe<U, F: FnOnce(T) -> U>(self, f: F) -> U {
        f(self)
    }
}