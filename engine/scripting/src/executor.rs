use crate::error::ScriptError;
use crate::value_bridge::ScriptValue;

pub trait ScriptExecutor {
    fn load(&mut self, path: &str) -> Result<(), ScriptError>;
    fn reload(&mut self) -> Result<(), ScriptError>;
    fn call(&self, method: &str, args: ScriptValue) -> Result<ScriptValue, ScriptError>;
    fn unload(&mut self);

    fn get_rotation_speed(&self) -> Result<f32, ScriptError>;
    fn set_rotation_speed(&mut self, speed: f32) -> Result<(), ScriptError>;
}

#[cfg(all(feature = "mono", not(feature = "native-aot")))]
pub use crate::mono_executor::MonoExecutor;

#[cfg(feature = "native-aot")]
pub use crate::native_aot_executor::NativeAotExecutor;

#[cfg(all(feature = "mono", not(feature = "native-aot")))]
pub type DefaultExecutor = MonoExecutor;

#[cfg(feature = "native-aot")]
pub type DefaultExecutor = NativeAotExecutor;
