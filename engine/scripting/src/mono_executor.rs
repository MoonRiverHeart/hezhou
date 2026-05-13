use crate::error::ScriptError;
use crate::executor::ScriptExecutor;
use crate::value_bridge::ScriptValue;
use crate::script_manager::ScriptManager;

pub struct MonoExecutor {
    manager: ScriptManager,
    assembly_name: String,
    dll_path: String,
    namespace: String,
    class_name: String,
}

impl MonoExecutor {
    pub fn new(namespace: &str, class_name: &str) -> Result<Self, ScriptError> {
        let manager = ScriptManager::new()?;
        Ok(Self {
            manager,
            assembly_name: String::new(),
            dll_path: String::new(),
            namespace: namespace.to_string(),
            class_name: class_name.to_string(),
        })
    }
}

impl ScriptExecutor for MonoExecutor {
    fn load(&mut self, path: &str) -> Result<(), ScriptError> {
        self.dll_path = path.to_string();
        self.assembly_name = self.manager.load_script(path)?;
        Ok(())
    }
    
    fn reload(&mut self) -> Result<(), ScriptError> {
        if self.dll_path.is_empty() {
            return Err(ScriptError::NotInitialized);
        }
        self.manager.reload(&self.dll_path)?;
        Ok(())
    }
    
    fn call(&self, method: &str, args: ScriptValue) -> Result<ScriptValue, ScriptError> {
        let (arg1, arg2) = match args.type_tag {
            1 => (args.int_value, 0),  // Int
            2 => (args.float_value as i32, 0),  // Float
            _ => (0, 0),
        };
        
        self.manager.execute(
            &self.assembly_name,
            &self.namespace,
            &self.class_name,
            method,
            (arg1, arg2),
        )
    }
    
    fn unload(&mut self) {
        if !self.assembly_name.is_empty() {
            self.manager.unload(&self.assembly_name);
            self.assembly_name.clear();
        }
    }
    
    fn get_rotation_speed(&self) -> Result<f32, ScriptError> {
        let result = self.call("GetRotationSpeed", ScriptValue::from_int(0))?;
        if result.type_tag == 2 {
            Ok(result.float_value)
        } else if result.type_tag == 1 {
            Ok(result.int_value as f32)
        } else {
            Ok(90.0)
        }
    }
    
    fn set_rotation_speed(&mut self, speed: f32) -> Result<(), ScriptError> {
        self.call("SetRotationSpeed", ScriptValue::from_float(speed))?;
        Ok(())
    }
}