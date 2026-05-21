mod types;
mod ui_system;
mod thunk;
mod widget_basic;
mod widget_complex;
mod widget_new;
mod scene;
mod renderer;

pub use types::*;
pub use ui_system::*;
pub use thunk::*;
pub use widget_basic::*;
pub use widget_complex::*;
pub use widget_new::*;
pub use scene::*;
pub use renderer::*;

use std::ffi::c_void;

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
    pub ui_debug_print_widget_tree: DebugPrintWidgetTreeFn,
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