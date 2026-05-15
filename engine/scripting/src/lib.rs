pub mod value_bridge;
pub mod error;
pub mod callback_types;
pub mod callback_registry;
pub mod script_manager_lite;

#[cfg(feature = "mono")]
pub mod script_manager;

#[cfg(feature = "mono")]
pub mod mono_executor;

#[cfg(feature = "native-aot")]
pub mod native_aot_executor;

pub mod executor;

pub use value_bridge::*;
pub use error::*;
pub use callback_types::*;
pub use callback_registry::*;
pub use executor::*;

#[cfg(feature = "native-aot")]
pub use native_aot_executor::register_rotation_callback;

#[cfg(feature = "native-aot")]
use parking_lot::Mutex;
#[cfg(feature = "native-aot")]
use std::sync::LazyLock;

#[cfg(feature = "native-aot")]
static ROTATION_CALLBACK: LazyLock<Mutex<Option<extern "C" fn(f32) -> f32>>> = 
    LazyLock::new(|| Mutex::new(None));

#[cfg(feature = "native-aot")]
#[unsafe(no_mangle)]
pub extern "C" fn trigger_rotation_callback(delta_time: f32) -> f32 {
    let cb = ROTATION_CALLBACK.lock();
    if let Some(callback) = *cb {
        callback(delta_time)
    } else {
        90.0 * delta_time
    }
}