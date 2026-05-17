use crate::error::ScriptError;
use crate::script_manager::ScriptManager;
use crate::value_bridge::ScriptValue;

pub struct MonoUIExecutor {
    manager: ScriptManager,
    assembly_name: String,
    dll_path: String,
}

impl MonoUIExecutor {
    pub fn new(dll_path: &str) -> Result<Self, ScriptError> {
        let mut manager = ScriptManager::new()?;
        let assembly_name = manager.load_script(dll_path)?;

        Ok(Self {
            manager,
            assembly_name,
            dll_path: dll_path.to_string(),
        })
    }

    pub fn call_static_void(
        &self,
        class_name: &str,
        method_name: &str,
        args: &[ScriptValue],
    ) -> Result<(), ScriptError> {
        let result = self.manager.execute(
            &self.assembly_name,
            "",
            class_name,
            method_name,
            args.first().and_then(|a| {
                if a.type_tag == 2 {
                    Some(a.float_value)
                } else {
                    None
                }
            }),
            args.len() as i32,
        )?;

        Ok(())
    }

    pub fn call_static_with_args(
        &self,
        class_name: &str,
        method_name: &str,
        int_args: &[i64],
        float_args: &[f32],
    ) -> Result<ScriptValue, ScriptError> {
        let arg_float = float_args.first().copied();
        let param_count = int_args.len() as i32 + float_args.len() as i32;

        self.manager.execute(
            &self.assembly_name,
            "",
            class_name,
            method_name,
            arg_float,
            param_count,
        )
    }

    pub fn reload(&mut self) -> Result<(), ScriptError> {
        let new_name = self.manager.reload(&self.dll_path)?;
        self.assembly_name = new_name;
        Ok(())
    }
}
