use std::ffi::{c_void, c_char};

pub type AssetLibraryGetCategoryCountFn = extern "C" fn() -> usize;
pub type AssetLibraryGetCategoryNameFn = extern "C" fn(usize, *mut c_char, usize) -> bool;
pub type AssetLibraryGetAssetCountFn = extern "C" fn(usize) -> usize;
pub type AssetLibraryGetAssetInfoFn = extern "C" fn(usize, usize, *mut u64, *mut c_char, usize, *mut u32, *mut c_char, usize) -> bool;
pub type AssetLibraryCreateEntityFromTemplateFn = extern "C" fn(*mut c_void, u64) -> u64;

pub type ProjectCreateNewFn = extern "C" fn(*const c_char, *const c_char) -> bool;
pub type ProjectLoadFn = extern "C" fn(*const c_char) -> bool;
pub type ProjectSaveFn = extern "C" fn() -> bool;
pub type ProjectGetNameFn = extern "C" fn(*mut c_char, usize) -> bool;
pub type ProjectGetPathFn = extern "C" fn(*mut c_char, usize) -> bool;
pub type ProjectGetEntityCountFn = extern "C" fn() -> usize;
pub type ProjectIsLoadedFn = extern "C" fn() -> bool;
pub type ProjectSyncToSceneFn = extern "C" fn(*mut c_void);
pub type ProjectSyncFromSceneFn = extern "C" fn(*const c_void);
pub type ProjectGetSettingsFn = extern "C" fn(*mut u32, *mut u32, *mut u32) -> bool;
pub type ProjectSetSettingsFn = extern "C" fn(u32, u32, u32) -> bool;
pub type ProjectGetEntityInfoFn = extern "C" fn(u64, *mut c_char, usize, *mut f32, *mut f32, *mut f32, *mut c_char, usize) -> bool;
pub type ProjectAddEntityFn = extern "C" fn(u64, *const c_char, f32, f32, f32, f32, f32, f32, f32, f32, f32, *const c_char) -> bool;
pub type ProjectRemoveEntityFn = extern "C" fn(u64) -> bool;