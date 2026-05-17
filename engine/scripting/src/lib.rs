pub mod callback_registry;
pub mod callback_types;
pub mod error;
pub mod ffi;
pub mod script_manager_lite;
pub mod value_bridge;

#[cfg(feature = "mono")]
pub mod script_manager;

#[cfg(feature = "mono")]
pub mod mono_executor;

#[cfg(feature = "mono")]
pub mod mono_ui_executor;

#[cfg(feature = "mono")]
pub use mono_ui_executor::MonoUIExecutor;

#[cfg(feature = "native-aot")]
pub mod native_aot_executor;

pub mod executor;

pub use callback_registry::*;
pub use callback_types::*;
pub use error::*;
pub use executor::*;
pub use ffi::*;
pub use value_bridge::*;

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
