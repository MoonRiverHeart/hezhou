pub type SetGamePreviewExtentFn = extern "C" fn(u32, u32);
pub type SetCameraParamsFn = extern "C" fn(f32, f32, f32, f32, f32);
pub type SetRendererGameStateFn = extern "C" fn(i32);
pub type GetRendererGameStateFn = extern "C" fn() -> i32;
pub type SetEntityTransformFn = extern "C" fn(f32, f32, f32, f32, f32, f32, f32, f32, f32, f32);
pub type SetEntityAngleFn = extern "C" fn(f32);
pub type GetEntityAngleFn = extern "C" fn() -> f32;