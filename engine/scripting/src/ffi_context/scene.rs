use std::ffi::{c_void, c_char};

pub type SceneCreateFn = extern "C" fn() -> *mut c_void;
pub type SceneDestroyFn = extern "C" fn(*mut c_void);
pub type SceneCreateCubeFn = extern "C" fn(*mut c_void) -> u64;
pub type SceneCreatePlaneFn = extern "C" fn(*mut c_void) -> u64;
pub type SceneCreateCornellBoxFn = extern "C" fn(*mut c_void) -> u64;
pub type SceneCreateDirectionalLightFn = extern "C" fn(*mut c_void) -> u64;
pub type SceneAttachScriptFn = extern "C" fn(*mut c_void, u64, *const c_char, *const c_char);
pub type SceneSetGameStateFn = extern "C" fn(*mut c_void, i32);
pub type SceneGetGameStateFn = extern "C" fn(*mut c_void) -> i32;
pub type ScenePickEntityFn = extern "C" fn(*mut c_void, f32, f32, f32, f32, f32, f32) -> u64;
pub type SceneSelectEntityFn = extern "C" fn(*mut c_void, u64);
pub type SceneUpdateFn = extern "C" fn(*mut c_void, f32);

pub type SceneGetEntityPositionFn = extern "C" fn(*mut c_void, u64, *mut f32, *mut f32, *mut f32);
pub type SceneGetEntityRotationFn = extern "C" fn(*mut c_void, u64, *mut f32, *mut f32, *mut f32, *mut f32);
pub type SceneGetEntityScaleFn = extern "C" fn(*mut c_void, u64, *mut f32, *mut f32, *mut f32);
pub type SceneSetEntityPositionFn = extern "C" fn(*mut c_void, u64, f32, f32, f32);
pub type SceneSetEntityScaleFn = extern "C" fn(*mut c_void, u64, f32, f32, f32);
pub type SceneRotateEntityFn = extern "C" fn(*mut c_void, u64, f32);
pub type SceneSetEntityNameFn = extern "C" fn(*mut c_void, u64, *const c_char);
pub type SceneGetEntityNameFn = extern "C" fn(*mut c_void, u64, *mut c_char, usize) -> usize;

pub type SetSelectedEntityFn = extern "C" fn(u64, bool);

pub type SceneAttachScriptBindingFn = extern "C" fn(*mut c_void, u64, *const c_char, *const c_char);
pub type SceneRemoveScriptBindingFn = extern "C" fn(*mut c_void, u64, usize);
pub type SceneGetScriptBindingCountFn = extern "C" fn(*mut c_void, u64) -> usize;
pub type SceneGetScriptBindingInfoFn = extern "C" fn(*mut c_void, u64, usize, *mut c_char, usize, *mut c_char, usize, *mut bool) -> bool;
pub type SceneSetScriptBindingEnabledFn = extern "C" fn(*mut c_void, u64, usize, bool);
pub type SceneCreateEntityFn = extern "C" fn(*mut c_void) -> u64;
pub type SceneGetEntityCountFn = extern "C" fn(*mut c_void) -> u64;
pub type SceneGetEntityIdFn = extern "C" fn(*mut c_void, u64) -> u64;
pub type SceneRemoveEntityFn = extern "C" fn(*mut c_void, u64);
pub type SceneSetParentFn = extern "C" fn(*mut c_void, u64, u64);
pub type SceneGetParentFn = extern "C" fn(*mut c_void, u64) -> u64;
pub type SceneGetChildCountFn = extern "C" fn(*mut c_void, u64) -> usize;
pub type SceneGetChildIdFn = extern "C" fn(*mut c_void, u64, usize) -> u64;