use std::ffi::{c_void, c_char};

pub type WidgetTreeHandle = *mut c_void;

pub type GetButtonIdFn = extern "C" fn() -> u64;
pub type SetButtonIdFn = extern "C" fn(u64);
pub type SetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type SetOnClickThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type RegisterUpdateThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterResizeThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterGlobalClickThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterKeyThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterMouseMoveThunkPtrFn = extern "C" fn(*const c_void);
pub type TriggerResizeFn = extern "C" fn(f32, f32);
pub type GetScreenSizeFn = extern "C" fn(*mut f32, *mut f32);
pub type SetContentScaleFn = extern "C" fn(f32);
pub type GetContentScaleFn = extern "C" fn() -> f32;

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
pub type CreatePreviewWindowFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, u64) -> u64;
pub type SetPreviewTextureFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type GetRootIdFn = extern "C" fn(WidgetTreeHandle) -> u64;
pub type SetWidgetLayoutFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32);
pub type SetPositionFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type SetSizeFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type SetWidgetLayerFn = extern "C" fn(WidgetTreeHandle, u64, u32);
pub type GetWidgetLayerFn = extern "C" fn(WidgetTreeHandle, u64) -> u32;
pub type RemoveWidgetFn = extern "C" fn(WidgetTreeHandle, u64);
pub type CreateTextEditFn = extern "C" fn(WidgetTreeHandle, f32, f32) -> u64;
pub type CreateTextEditInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32) -> u64;
pub type TextEditSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type TextEditInsertCharFn = extern "C" fn(WidgetTreeHandle, u64, c_char);
pub type TextEditDeleteCharFn = extern "C" fn(WidgetTreeHandle, u64);
pub type TextEditGetTextLenFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type TextEditGetTextFn = extern "C" fn(WidgetTreeHandle, u64, *mut c_char, usize);
pub type TextEditShowLineNumbersFn = extern "C" fn(WidgetTreeHandle, u64, bool);
pub type TriggerHotReloadFn = extern "C" fn();
pub type SetGamePreviewExtentFn = extern "C" fn(u32, u32);
pub type SetCameraParamsFn = extern "C" fn(f32, f32, f32, f32, f32);
pub type IsPreviewWindowSelectedFn = extern "C" fn(WidgetTreeHandle, u64) -> bool;
pub type SetPreviewWindowSelectedFn = extern "C" fn(WidgetTreeHandle, u64, bool);
pub type SetPreviewWindowEditModeFn = extern "C" fn(WidgetTreeHandle, u64, bool);

pub type CreateListFn = extern "C" fn(WidgetTreeHandle, f32, u32) -> u64;
pub type CreateListInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, u32) -> u64;
pub type CreateListItemFn = extern "C" fn(WidgetTreeHandle, *const c_char) -> u64;
pub type CreateListItemInParentFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, u32) -> u64;
pub type ListItemSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type ListItemSetFontSizeFn = extern "C" fn(WidgetTreeHandle, u64, f32);

pub type CreateDropdownFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32) -> u64;
pub type DropdownSetOptionsFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, usize);
pub type DropdownSetSelectedFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type DropdownGetSelectedFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type DropdownSetOnSelectThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

pub type CreateInputFieldFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32) -> u64;
pub type InputFieldSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type InputFieldGetTextFn = extern "C" fn(WidgetTreeHandle, u64, *mut c_char, usize) -> usize;
pub type InputFieldSetOnChangeThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type InputFieldSetPlaceholderFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);

pub type CreateTabWidgetFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32) -> u64;
pub type TabWidgetAddTabFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, u64, bool) -> u32;
pub type TabWidgetSetActiveFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type TabWidgetGetActiveFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type TabWidgetRemoveTabFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type TabWidgetSetOnSelectThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type TabWidgetSetOnCloseThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type TabWidgetGetTabCountFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;

pub type CreateTreeViewFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32) -> u64;
pub type TreeViewAddNodeFn = extern "C" fn(WidgetTreeHandle, u64, u64, *const c_char, u64, bool) -> u64;
pub type TreeViewRemoveNodeFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type TreeViewSetSelectedFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type TreeViewGetSelectedFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type TreeViewExpandNodeFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type TreeViewCollapseNodeFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type TreeViewSetOnSelectThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type TreeNodeSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type TreeNodeGetUserDataFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type TreeViewClearSelectionFn = extern "C" fn(WidgetTreeHandle, u64);

pub type CreatePopupMenuFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type PopupMenuAddItemFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, *const c_char, usize);
pub type PopupMenuAddSeparatorFn = extern "C" fn(WidgetTreeHandle, u64);
pub type PopupMenuShowFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type PopupMenuHideFn = extern "C" fn(WidgetTreeHandle, u64);
pub type PopupMenuIsVisibleFn = extern "C" fn(WidgetTreeHandle, u64) -> bool;
pub type PopupMenuSetOnClickThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

pub type CreateGridViewFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, f32) -> u64;
pub type GridViewAddItemFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, u64) -> u32;
pub type GridViewRemoveItemFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type GridViewSetSelectedFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type GridViewGetSelectedFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type GridViewGetSelectedUserDataFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type GridViewClearFn = extern "C" fn(WidgetTreeHandle, u64);
pub type GridViewItemCountFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type GridViewSetOnClickThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

pub type CreateDialogFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, f32, f32) -> u64;
pub type DialogSetContentFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type DialogAddButtonFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, i32);
pub type DialogShowFn = extern "C" fn(WidgetTreeHandle, u64);
pub type DialogHideFn = extern "C" fn(WidgetTreeHandle, u64);
pub type DialogIsVisibleFn = extern "C" fn(WidgetTreeHandle, u64) -> bool;
pub type DialogGetResultFn = extern "C" fn(WidgetTreeHandle, u64) -> i32;
pub type DialogSetOnResultThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

pub type CreateFileBrowserFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, *const c_char) -> u64;
pub type FileBrowserSetPathFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type FileBrowserSetFilterFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type FileBrowserNavigateUpFn = extern "C" fn(WidgetTreeHandle, u64);
pub type FileBrowserRefreshFn = extern "C" fn(WidgetTreeHandle, u64);
pub type FileBrowserGetSelectedPathFn = extern "C" fn(WidgetTreeHandle, u64, *mut c_char, usize) -> bool;
pub type FileBrowserGetCurrentPathFn = extern "C" fn(WidgetTreeHandle, u64, *mut c_char, usize) -> bool;
pub type FileBrowserSetOnSelectThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type FileBrowserSetOnDoubleClickThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

pub type SceneCreateFn = extern "C" fn() -> *mut c_void;
pub type SceneDestroyFn = extern "C" fn(*mut c_void);
pub type SceneCreateCubeFn = extern "C" fn(*mut c_void) -> u64;
pub type SceneAttachScriptFn = extern "C" fn(*mut c_void, u64, *const c_char, *const c_char);
pub type SceneSetGameStateFn = extern "C" fn(*mut c_void, i32);
pub type SceneGetGameStateFn = extern "C" fn(*mut c_void) -> i32;
pub type ScenePickEntityFn = extern "C" fn(*mut c_void, f32, f32, f32, f32, f32, f32) -> u64;
pub type SceneSelectEntityFn = extern "C" fn(*mut c_void, u64);
pub type SceneUpdateFn = extern "C" fn(*mut c_void, f32);

pub type SetRendererGameStateFn = extern "C" fn(i32);
pub type GetRendererGameStateFn = extern "C" fn() -> i32;

pub type SetEntityTransformFn = extern "C" fn(f32, f32, f32, f32, f32, f32, f32, f32, f32, f32);
pub type SetEntityAngleFn = extern "C" fn(f32);
pub type GetEntityAngleFn = extern "C" fn() -> f32;

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

pub type DfxLogFn = extern "C" fn(*mut c_void, u8, *const c_char, *const c_char, *const c_char, u32);
pub type DfxTraceBeginFn = extern "C" fn(*mut c_void, *const c_char, *const c_char);
pub type DfxTraceEndFn = extern "C" fn(*mut c_void, *const c_char, *const c_char);
pub type SetStatusTextFn = extern "C" fn(*const c_char);
pub type OnHotReloadCompleteFn = extern "C" fn();

#[repr(C)]
pub struct FfiContext {
    pub ui_get_primary_button_id: GetButtonIdFn,
    pub ui_set_primary_button_id: SetButtonIdFn,
    pub ui_widget_set_text: SetTextFn,
    pub ui_button_set_on_click_thunk_ptr: SetOnClickThunkPtrFn,
    pub ui_register_update_thunk_ptr: RegisterUpdateThunkPtrFn,
    pub ui_register_resize_thunk_ptr: RegisterResizeThunkPtrFn,
    pub ui_register_global_click_thunk_ptr: RegisterGlobalClickThunkPtrFn,
    pub ui_register_key_thunk_ptr: RegisterKeyThunkPtrFn,
    pub ui_register_mouse_move_thunk_ptr: RegisterMouseMoveThunkPtrFn,
    pub ui_trigger_resize: TriggerResizeFn,
    pub ui_get_screen_size: GetScreenSizeFn,
    pub ui_set_content_scale: SetContentScaleFn,
    pub ui_get_content_scale: GetContentScaleFn,
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
    pub ui_create_preview_window: CreatePreviewWindowFn,
    pub ui_set_preview_texture: SetPreviewTextureFn,
    pub ui_get_root_id: GetRootIdFn,
    pub ui_set_widget_layout: SetWidgetLayoutFn,
    pub ui_widget_set_position: SetPositionFn,
    pub ui_widget_set_size: SetSizeFn,
    pub ui_widget_set_layer: SetWidgetLayerFn,
    pub ui_widget_get_layer: GetWidgetLayerFn,
    pub ui_remove_widget: RemoveWidgetFn,
    pub ui_create_text_edit: CreateTextEditFn,
    pub ui_create_text_edit_in_parent: CreateTextEditInParentFn,
    pub ui_text_edit_set_text: TextEditSetTextFn,
    pub ui_text_edit_insert_char: TextEditInsertCharFn,
    pub ui_text_edit_delete_char: TextEditDeleteCharFn,
    pub ui_text_edit_get_text_len: TextEditGetTextLenFn,
    pub ui_text_edit_get_text: TextEditGetTextFn,
    pub ui_text_edit_show_line_numbers: TextEditShowLineNumbersFn,
    pub ui_trigger_hot_reload: TriggerHotReloadFn,
    pub ui_set_game_preview_extent: SetGamePreviewExtentFn,
    pub ui_set_camera_params: SetCameraParamsFn,
    pub ui_is_preview_window_selected: IsPreviewWindowSelectedFn,
    pub ui_set_preview_window_selected: SetPreviewWindowSelectedFn,
    pub ui_set_preview_window_edit_mode: SetPreviewWindowEditModeFn,
    pub ui_create_list: CreateListFn,
    pub ui_create_list_in_parent: CreateListInParentFn,
    pub ui_create_list_item: CreateListItemFn,
    pub ui_create_list_item_in_parent: CreateListItemInParentFn,
    pub ui_list_item_set_text: ListItemSetTextFn,
    pub ui_list_item_set_font_size: ListItemSetFontSizeFn,
    pub ui_create_dropdown: CreateDropdownFn,
    pub ui_dropdown_set_options: DropdownSetOptionsFn,
    pub ui_dropdown_set_selected: DropdownSetSelectedFn,
    pub ui_dropdown_get_selected: DropdownGetSelectedFn,
    pub ui_dropdown_set_on_select_thunk_ptr: DropdownSetOnSelectThunkPtrFn,
    pub ui_create_input_field: CreateInputFieldFn,
    pub ui_input_field_set_text: InputFieldSetTextFn,
    pub ui_input_field_get_text: InputFieldGetTextFn,
    pub ui_input_field_set_on_change_thunk_ptr: InputFieldSetOnChangeThunkPtrFn,
    pub ui_input_field_set_placeholder: InputFieldSetPlaceholderFn,
    pub ui_create_tab_widget: CreateTabWidgetFn,
    pub ui_tab_widget_add_tab: TabWidgetAddTabFn,
    pub ui_tab_widget_set_active: TabWidgetSetActiveFn,
    pub ui_tab_widget_get_active: TabWidgetGetActiveFn,
    pub ui_tab_widget_remove_tab: TabWidgetRemoveTabFn,
    pub ui_tab_widget_set_on_select_thunk_ptr: TabWidgetSetOnSelectThunkPtrFn,
    pub ui_tab_widget_set_on_close_thunk_ptr: TabWidgetSetOnCloseThunkPtrFn,
    pub ui_tab_widget_get_tab_count: TabWidgetGetTabCountFn,
    pub ui_create_tree_view: CreateTreeViewFn,
    pub ui_tree_view_add_node: TreeViewAddNodeFn,
    pub ui_tree_view_remove_node: TreeViewRemoveNodeFn,
    pub ui_tree_view_set_selected: TreeViewSetSelectedFn,
    pub ui_tree_view_get_selected: TreeViewGetSelectedFn,
    pub ui_tree_view_expand_node: TreeViewExpandNodeFn,
    pub ui_tree_view_collapse_node: TreeViewCollapseNodeFn,
    pub ui_tree_view_set_on_select_thunk_ptr: TreeViewSetOnSelectThunkPtrFn,
    pub ui_tree_node_set_text: TreeNodeSetTextFn,
    pub ui_tree_node_get_user_data: TreeNodeGetUserDataFn,
    pub ui_tree_view_clear_selection: TreeViewClearSelectionFn,
    pub ui_create_popup_menu: CreatePopupMenuFn,
    pub ui_popup_menu_add_item: PopupMenuAddItemFn,
    pub ui_popup_menu_add_separator: PopupMenuAddSeparatorFn,
    pub ui_popup_menu_show: PopupMenuShowFn,
    pub ui_popup_menu_hide: PopupMenuHideFn,
    pub ui_popup_menu_is_visible: PopupMenuIsVisibleFn,
    pub ui_popup_menu_set_on_click_thunk_ptr: PopupMenuSetOnClickThunkPtrFn,
    pub ui_create_grid_view: CreateGridViewFn,
    pub ui_grid_view_add_item: GridViewAddItemFn,
    pub ui_grid_view_remove_item: GridViewRemoveItemFn,
    pub ui_grid_view_set_selected: GridViewSetSelectedFn,
    pub ui_grid_view_get_selected: GridViewGetSelectedFn,
    pub ui_grid_view_get_selected_user_data: GridViewGetSelectedUserDataFn,
    pub ui_grid_view_clear: GridViewClearFn,
    pub ui_grid_view_item_count: GridViewItemCountFn,
    pub ui_grid_view_set_on_click_thunk_ptr: GridViewSetOnClickThunkPtrFn,
    pub ui_create_dialog: CreateDialogFn,
    pub ui_dialog_set_content: DialogSetContentFn,
    pub ui_dialog_add_button: DialogAddButtonFn,
    pub ui_dialog_show: DialogShowFn,
    pub ui_dialog_hide: DialogHideFn,
    pub ui_dialog_is_visible: DialogIsVisibleFn,
    pub ui_dialog_get_result: DialogGetResultFn,
    pub ui_dialog_set_on_result_thunk_ptr: DialogSetOnResultThunkPtrFn,
    pub ui_create_file_browser: CreateFileBrowserFn,
    pub ui_file_browser_set_path: FileBrowserSetPathFn,
    pub ui_file_browser_set_filter: FileBrowserSetFilterFn,
    pub ui_file_browser_navigate_up: FileBrowserNavigateUpFn,
    pub ui_file_browser_refresh: FileBrowserRefreshFn,
    pub ui_file_browser_get_selected_path: FileBrowserGetSelectedPathFn,
    pub ui_file_browser_get_current_path: FileBrowserGetCurrentPathFn,
    pub ui_file_browser_set_on_select_thunk_ptr: FileBrowserSetOnSelectThunkPtrFn,
    pub ui_file_browser_set_on_double_click_thunk_ptr: FileBrowserSetOnDoubleClickThunkPtrFn,
    pub scene_create: SceneCreateFn,
    pub scene_destroy: SceneDestroyFn,
    pub scene_create_cube: SceneCreateCubeFn,
    pub scene_attach_script: SceneAttachScriptFn,
    pub scene_set_game_state: SceneSetGameStateFn,
    pub scene_get_game_state: SceneGetGameStateFn,
    pub scene_pick_entity: ScenePickEntityFn,
    pub scene_select_entity: SceneSelectEntityFn,
    pub scene_update: SceneUpdateFn,
    pub set_renderer_game_state: SetRendererGameStateFn,
    pub get_renderer_game_state: GetRendererGameStateFn,
    pub set_entity_transform: SetEntityTransformFn,
    pub set_entity_angle: SetEntityAngleFn,
    pub get_entity_angle: GetEntityAngleFn,
    pub scene_get_entity_position: SceneGetEntityPositionFn,
    pub scene_get_entity_rotation: SceneGetEntityRotationFn,
    pub scene_get_entity_scale: SceneGetEntityScaleFn,
    pub scene_set_entity_position: SceneSetEntityPositionFn,
    pub scene_set_entity_scale: SceneSetEntityScaleFn,
    pub scene_rotate_entity: SceneRotateEntityFn,
    pub scene_set_entity_name: SceneSetEntityNameFn,
    pub scene_get_entity_name: SceneGetEntityNameFn,
    pub set_selected_entity: SetSelectedEntityFn,
    pub scene_attach_script_binding: SceneAttachScriptBindingFn,
    pub scene_remove_script_binding: SceneRemoveScriptBindingFn,
    pub scene_get_script_binding_count: SceneGetScriptBindingCountFn,
    pub scene_get_script_binding_info: SceneGetScriptBindingInfoFn,
    pub scene_set_script_binding_enabled: SceneSetScriptBindingEnabledFn,
    pub scene_create_entity: SceneCreateEntityFn,
    pub scene_get_entity_count: SceneGetEntityCountFn,
    pub scene_get_entity_id: SceneGetEntityIdFn,
    pub scene_remove_entity: SceneRemoveEntityFn,
    pub widget_tree_ptr: WidgetTreeHandle,
    pub dfx_handle: *mut c_void,
    pub dfx_log: DfxLogFn,
    pub dfx_trace_begin: DfxTraceBeginFn,
    pub dfx_trace_end: DfxTraceEndFn,
    pub set_status_text: SetStatusTextFn,
    pub on_hot_reload_complete: OnHotReloadCompleteFn,
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