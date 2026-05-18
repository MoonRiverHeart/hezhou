use std::ffi::{c_void, c_char};

pub type WidgetTreeHandle = *mut c_void;

pub type GetButtonIdFn = extern "C" fn() -> u64;
pub type SetButtonIdFn = extern "C" fn(u64);
pub type SetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type SetOnClickThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type RegisterUpdateThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterResizeThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterGlobalClickThunkPtrFn = extern "C" fn(*const c_void);
pub type TriggerResizeFn = extern "C" fn(f32, f32);
pub type GetScreenSizeFn = extern "C" fn(*mut f32, *mut f32);

pub type CreateButtonFn = extern "C" fn(WidgetTreeHandle, f32, f32, f32, f32, *const c_char) -> u64;
pub type CreateLabelFn = extern "C" fn(WidgetTreeHandle, f32, f32, f32, f32, *const c_char) -> u64;
pub type CreatePanelFn = extern "C" fn(WidgetTreeHandle, f32, f32, f32, f32) -> u64;
pub type CreateVStackFn = extern "C" fn(WidgetTreeHandle, f32) -> u64;
pub type CreateVStackInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32) -> u64;
pub type CreateHStackFn = extern "C" fn(WidgetTreeHandle, f32) -> u64;
pub type CreateHStackInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32) -> u64;
pub type CreateButtonInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, *const c_char) -> u64;
pub type CreateLabelInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, *const c_char) -> u64;
pub type CreatePanelInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, f32, f32, f32, f32) -> u64;
pub type GetRootIdFn = extern "C" fn(WidgetTreeHandle) -> u64;
pub type SetWidgetLayoutFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32);
pub type SetPositionFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type SetSizeFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type RemoveWidgetFn = extern "C" fn(WidgetTreeHandle, u64);
pub type CreateTextEditFn = extern "C" fn(WidgetTreeHandle, f32, f32) -> u64;
pub type CreateTextEditInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32) -> u64;
pub type TextEditSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type TextEditInsertCharFn = extern "C" fn(WidgetTreeHandle, u64, c_char);
pub type TextEditDeleteCharFn = extern "C" fn(WidgetTreeHandle, u64);
pub type TextEditGetTextLenFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type TextEditGetTextFn = extern "C" fn(WidgetTreeHandle, u64, *mut c_char, usize);
pub type TriggerHotReloadFn = extern "C" fn();

#[repr(C)]
pub struct FfiContext {
    pub ui_get_primary_button_id: GetButtonIdFn,
    pub ui_set_primary_button_id: SetButtonIdFn,
    pub ui_widget_set_text: SetTextFn,
    pub ui_button_set_on_click_thunk_ptr: SetOnClickThunkPtrFn,
    pub ui_register_update_thunk_ptr: RegisterUpdateThunkPtrFn,
    pub ui_register_resize_thunk_ptr: RegisterResizeThunkPtrFn,
    pub ui_register_global_click_thunk_ptr: RegisterGlobalClickThunkPtrFn,
    pub ui_trigger_resize: TriggerResizeFn,
    pub ui_get_screen_size: GetScreenSizeFn,
    pub ui_create_button: CreateButtonFn,
    pub ui_create_label: CreateLabelFn,
    pub ui_create_panel: CreatePanelFn,
    pub ui_create_vstack: CreateVStackFn,
    pub ui_create_vstack_in_parent: CreateVStackInParentFn,
    pub ui_create_hstack: CreateHStackFn,
    pub ui_create_hstack_in_parent: CreateHStackInParentFn,
    pub ui_create_button_in_parent: CreateButtonInParentFn,
    pub ui_create_label_in_parent: CreateLabelInParentFn,
    pub ui_create_panel_in_parent: CreatePanelInParentFn,
    pub ui_get_root_id: GetRootIdFn,
    pub ui_set_widget_layout: SetWidgetLayoutFn,
    pub ui_widget_set_position: SetPositionFn,
    pub ui_widget_set_size: SetSizeFn,
    pub ui_remove_widget: RemoveWidgetFn,
    pub ui_create_text_edit: CreateTextEditFn,
    pub ui_create_text_edit_in_parent: CreateTextEditInParentFn,
    pub ui_text_edit_set_text: TextEditSetTextFn,
    pub ui_text_edit_insert_char: TextEditInsertCharFn,
    pub ui_text_edit_delete_char: TextEditDeleteCharFn,
    pub ui_text_edit_get_text_len: TextEditGetTextLenFn,
    pub ui_text_edit_get_text: TextEditGetTextFn,
    pub ui_trigger_hot_reload: TriggerHotReloadFn,
    pub widget_tree_ptr: WidgetTreeHandle,
}

static mut FFI_CONTEXT: Option<Box<FfiContext>> = None;

pub fn set_ffi_context(ctx: FfiContext) {
    unsafe {
        let boxed = Box::new(ctx);
        FFI_CONTEXT = Some(boxed);
    }
}

pub fn get_ffi_context_ptr() -> *const FfiContext {
    unsafe { 
        match &FFI_CONTEXT {
            Some(boxed) => boxed.as_ref() as *const FfiContext,
            None => std::ptr::null(),
        }
    }
}