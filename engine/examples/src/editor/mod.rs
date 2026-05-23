use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle, EventDispatcherHandle, SetStatusTextFn, OnHotReloadCompleteFn}};
use hezhou_ui::ffi as ui_ffi;
use hezhou_dfx::*;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};


pub mod ffi_impl;
pub mod scene_ffi_impl;
pub mod hot_reload;
pub mod dfx_init;

static mut EXECUTOR: Option<MonoUIExecutor> = None;
pub(crate) static HOT_RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);
static mut RENDERER: Option<*mut UIVulkanRenderer> = None;
pub(crate) static mut SCENE: Option<*mut hezhou_core::Scene> = None;
pub(crate) static mut FFI_PTR: Option<*const FfiContext> = None;
pub(crate) static mut STATUS_TEXT_CALLBACK: Option<SetStatusTextFn> = None;
pub(crate) static mut HOT_RELOAD_COMPLETE_CALLBACK: Option<OnHotReloadCompleteFn> = None;

#[derive(Clone, Debug)]
pub(crate) struct SavedScriptBinding {
    pub script_path: String,
    pub class_name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct SavedEntityBinding {
    pub entity_id: u64,
    pub bindings: Vec<SavedScriptBinding>,
}

pub(crate) static mut SAVED_BINDINGS: Vec<SavedEntityBinding> = Vec::new();

pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let screenshot_mode = args.iter().any(|a| a == "--screenshot");
    let screenshot_delay = if screenshot_mode { 
        args.iter().position(|a| a == "--delay")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(3.0)
    } else { 0.0 };
    
    let (dfx, log_path) = dfx_init::setup_dfx();
    
    dfx_info!("Demo", "=== Hezhou Game Editor ===");
    dfx_info!("Demo", "Log: {}", log_path);
    if screenshot_mode {
        dfx_info!("Demo", "Screenshot mode: delay={}s", screenshot_delay);
    }
    
    dfx_trace_begin!("Startup", "editor");
    
    dfx_trace_begin!("Window", "create");
    let mut renderer = UIVulkanRenderer::new(1280, 720, "Hezhou Game Editor")
        .expect("Failed to create renderer");
    unsafe { RENDERER = Some(&mut renderer as *mut UIVulkanRenderer); }
    dfx_trace_end!("Window", "create");

    dfx_trace_begin!("UI", "setup");
    renderer.setup_ui_for_script();
    ui_ffi::ui_set_screen_size(1280.0, 720.0);
    let content_scale = renderer.get_content_scale();
    ui_ffi::ui_set_content_scale(content_scale);
    dfx_trace_end!("UI", "setup");

    dfx_trace_begin!("Script", "compile");
    hot_reload::compile_editor_script();
    dfx_trace_end!("Script", "compile");

    let widget_tree_handle: WidgetTreeHandle = renderer.get_widget_tree_handle() as WidgetTreeHandle;
    let event_dispatcher_handle = renderer.get_event_dispatcher_handle() as *mut std::ffi::c_void;
    
    let dfx_for_csharp = hezhou_dfx::dfx_create();
    hezhou_dfx::dfx_set_log_level(dfx_for_csharp, 2);
    
    let log_path_c = std::ffi::CString::new(log_path.clone()).unwrap();
    hezhou_dfx::dfx_enable_file_output(dfx_for_csharp, log_path_c.as_ptr());
    
    let ffi_ctx = FfiContext {
        ui_get_primary_button_id: ui_ffi::ui_get_primary_button_id,
        ui_set_primary_button_id: ui_ffi::ui_set_primary_button_id,
        ui_widget_set_text: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_text as *const std::ffi::c_void) },
        ui_button_set_on_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_button_set_on_click_thunk_ptr as *const std::ffi::c_void) },
        ui_register_update_thunk_ptr: ui_ffi::ui_register_update_thunk_ptr,
        ui_register_resize_thunk_ptr: ui_ffi::ui_register_resize_thunk_ptr,
        ui_register_global_click_thunk_ptr: ui_ffi::ui_register_global_click_thunk_ptr,
        ui_register_key_thunk_ptr: ui_ffi::ui_register_key_thunk_ptr,
        ui_register_mouse_move_thunk_ptr: ui_ffi::ui_register_mouse_move_thunk_ptr,
        ui_register_mouse_wheel_thunk_ptr: ui_ffi::ui_register_mouse_wheel_thunk_ptr,
        ui_register_tree_node_right_click_thunk_ptr: ui_ffi::ui_register_tree_node_right_click_thunk_ptr,
        ui_trigger_resize: ui_ffi::ui_trigger_resize,
        ui_get_screen_size: ui_ffi::ui_get_screen_size,
        ui_set_content_scale: ui_ffi::ui_set_content_scale,
        ui_get_content_scale: ui_ffi::ui_get_content_scale,
        ui_create_button: unsafe { std::mem::transmute(ui_ffi::ui_create_button as *const std::ffi::c_void) },
        ui_create_label: unsafe { std::mem::transmute(ui_ffi::ui_create_label as *const std::ffi::c_void) },
        ui_create_panel: unsafe { std::mem::transmute(ui_ffi::ui_create_panel as *const std::ffi::c_void) },
        ui_create_vstack: unsafe { std::mem::transmute(ui_ffi::ui_create_vstack as *const std::ffi::c_void) },
        ui_create_vstack_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_vstack_in_parent as *const std::ffi::c_void) },
        ui_create_hstack: unsafe { std::mem::transmute(ui_ffi::ui_create_hstack as *const std::ffi::c_void) },
        ui_create_hstack_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_hstack_in_parent as *const std::ffi::c_void) },
        ui_create_button_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_button_in_parent as *const std::ffi::c_void) },
        ui_create_label_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_label_in_parent as *const std::ffi::c_void) },
        ui_create_panel_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_panel_in_parent as *const std::ffi::c_void) },
        ui_create_preview_window: unsafe { std::mem::transmute(ui_ffi::ui_create_preview_window as *const std::ffi::c_void) },
        ui_set_preview_texture: unsafe { std::mem::transmute(ui_ffi::ui_set_preview_texture as *const std::ffi::c_void) },
        ui_get_root_id: unsafe { std::mem::transmute(ui_ffi::ui_get_root_id as *const std::ffi::c_void) },
        ui_set_widget_layout: unsafe { std::mem::transmute(ui_ffi::ui_set_widget_layout as *const std::ffi::c_void) },
        ui_widget_set_position: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_position as *const std::ffi::c_void) },
        ui_widget_set_size: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_size as *const std::ffi::c_void) },
        ui_widget_set_layer: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_layer as *const std::ffi::c_void) },
        ui_widget_get_layer: unsafe { std::mem::transmute(ui_ffi::ui_widget_get_layer as *const std::ffi::c_void) },
        ui_remove_widget: unsafe { std::mem::transmute(ui_ffi::ui_remove_widget as *const std::ffi::c_void) },
        ui_create_text_edit: unsafe { std::mem::transmute(ui_ffi::ui_create_text_edit as *const std::ffi::c_void) },
        ui_create_text_edit_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_text_edit_in_parent as *const std::ffi::c_void) },
        ui_text_edit_set_text: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_set_text as *const std::ffi::c_void) },
        ui_text_edit_insert_char: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_insert_char as *const std::ffi::c_void) },
        ui_text_edit_delete_char: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_delete_char as *const std::ffi::c_void) },
        ui_text_edit_get_text_len: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_get_text_len as *const std::ffi::c_void) },
        ui_text_edit_get_text: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_get_text as *const std::ffi::c_void) },
        ui_text_edit_show_line_numbers: unsafe { std::mem::transmute(ui_ffi::ui_set_text_edit_show_line_numbers as *const std::ffi::c_void) },
        ui_trigger_hot_reload: ffi_impl::trigger_hot_reload,
        ui_set_game_preview_extent: ffi_impl::set_game_preview_extent,
        ui_set_camera_params: ffi_impl::set_camera_params,
        ui_is_preview_window_selected: unsafe { std::mem::transmute(ui_ffi::ui_is_preview_window_selected as *const std::ffi::c_void) },
        ui_set_preview_window_selected: unsafe { std::mem::transmute(ui_ffi::ui_set_preview_window_selected as *const std::ffi::c_void) },
        ui_set_preview_window_edit_mode: unsafe { std::mem::transmute(ui_ffi::ui_set_preview_window_edit_mode as *const std::ffi::c_void) },
        ui_create_list: unsafe { std::mem::transmute(ui_ffi::ui_create_list as *const std::ffi::c_void) },
        ui_create_list_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_list_in_parent as *const std::ffi::c_void) },
        ui_create_list_item: unsafe { std::mem::transmute(ui_ffi::ui_create_list_item as *const std::ffi::c_void) },
        ui_create_list_item_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_list_item_in_parent as *const std::ffi::c_void) },
        ui_list_item_set_text: unsafe { std::mem::transmute(ui_ffi::ui_list_item_set_text as *const std::ffi::c_void) },
        ui_list_item_set_font_size: unsafe { std::mem::transmute(ui_ffi::ui_list_item_set_font_size as *const std::ffi::c_void) },
        ui_create_dropdown: unsafe { std::mem::transmute(ui_ffi::ui_create_dropdown as *const std::ffi::c_void) },
        ui_dropdown_set_options: unsafe { std::mem::transmute(ui_ffi::ui_dropdown_set_options as *const std::ffi::c_void) },
        ui_dropdown_set_selected: unsafe { std::mem::transmute(ui_ffi::ui_dropdown_set_selected as *const std::ffi::c_void) },
        ui_dropdown_get_selected: unsafe { std::mem::transmute(ui_ffi::ui_dropdown_get_selected as *const std::ffi::c_void) },
        ui_dropdown_set_on_select_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_dropdown_set_on_select_thunk_ptr as *const std::ffi::c_void) },
        ui_create_input_field: unsafe { std::mem::transmute(ui_ffi::ui_create_input_field as *const std::ffi::c_void) },
        ui_input_field_set_text: unsafe { std::mem::transmute(ui_ffi::ui_input_field_set_text as *const std::ffi::c_void) },
        ui_input_field_get_text: unsafe { std::mem::transmute(ui_ffi::ui_input_field_get_text as *const std::ffi::c_void) },
        ui_input_field_set_on_change_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_input_field_set_on_change_thunk_ptr as *const std::ffi::c_void) },
        ui_input_field_set_placeholder: unsafe { std::mem::transmute(ui_ffi::ui_input_field_set_placeholder as *const std::ffi::c_void) },
        ui_create_checkbox: unsafe { std::mem::transmute(ui_ffi::ui_create_checkbox as *const std::ffi::c_void) },
        ui_create_checkbox_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_checkbox_in_parent as *const std::ffi::c_void) },
        ui_checkbox_set_checked: unsafe { std::mem::transmute(ui_ffi::ui_checkbox_set_checked as *const std::ffi::c_void) },
        ui_checkbox_get_checked: unsafe { std::mem::transmute(ui_ffi::ui_checkbox_get_checked as *const std::ffi::c_void) },
        ui_checkbox_set_on_change_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_checkbox_set_on_change_thunk_ptr as *const std::ffi::c_void) },
        ui_checkbox_set_text: unsafe { std::mem::transmute(ui_ffi::ui_checkbox_set_text as *const std::ffi::c_void) },
        ui_create_slider: unsafe { std::mem::transmute(ui_ffi::ui_create_slider as *const std::ffi::c_void) },
        ui_create_slider_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_slider_in_parent as *const std::ffi::c_void) },
        ui_slider_set_value: unsafe { std::mem::transmute(ui_ffi::ui_slider_set_value as *const std::ffi::c_void) },
        ui_slider_get_value: unsafe { std::mem::transmute(ui_ffi::ui_slider_get_value as *const std::ffi::c_void) },
        ui_slider_set_range: unsafe { std::mem::transmute(ui_ffi::ui_slider_set_range as *const std::ffi::c_void) },
        ui_slider_set_on_change_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_slider_set_on_change_thunk_ptr as *const std::ffi::c_void) },
        ui_slider_set_step: unsafe { std::mem::transmute(ui_ffi::ui_slider_set_step as *const std::ffi::c_void) },
        ui_create_scroll_view: unsafe { std::mem::transmute(ui_ffi::ui_create_scroll_view as *const std::ffi::c_void) },
        ui_scroll_view_set_scroll_offset: unsafe { std::mem::transmute(ui_ffi::ui_scroll_view_set_scroll_offset as *const std::ffi::c_void) },
        ui_scroll_view_get_scroll_offset: unsafe { std::mem::transmute(ui_ffi::ui_scroll_view_get_scroll_offset as *const std::ffi::c_void) },
        ui_scroll_view_set_show_scrollbars: unsafe { std::mem::transmute(ui_ffi::ui_scroll_view_set_show_scrollbars as *const std::ffi::c_void) },
        ui_scroll_view_set_on_scroll_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_scroll_view_set_on_scroll_thunk_ptr as *const std::ffi::c_void) },
        ui_create_split_view: unsafe { std::mem::transmute(ui_ffi::ui_create_split_view as *const std::ffi::c_void) },
        ui_split_view_set_split_ratio: unsafe { std::mem::transmute(ui_ffi::ui_split_view_set_split_ratio as *const std::ffi::c_void) },
        ui_split_view_get_split_ratio: unsafe { std::mem::transmute(ui_ffi::ui_split_view_get_split_ratio as *const std::ffi::c_void) },
        ui_split_view_set_min_ratio: unsafe { std::mem::transmute(ui_ffi::ui_split_view_set_min_ratio as *const std::ffi::c_void) },
        ui_split_view_set_max_ratio: unsafe { std::mem::transmute(ui_ffi::ui_split_view_set_max_ratio as *const std::ffi::c_void) },
        ui_split_view_set_on_ratio_change_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_split_view_set_on_ratio_change_thunk_ptr as *const std::ffi::c_void) },
        ui_create_tab_widget: unsafe { std::mem::transmute(ui_ffi::ui_create_tab_widget as *const std::ffi::c_void) },
        ui_tab_widget_add_tab: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_add_tab as *const std::ffi::c_void) },
        ui_tab_widget_set_active: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_set_active as *const std::ffi::c_void) },
        ui_tab_widget_get_active: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_get_active as *const std::ffi::c_void) },
        ui_tab_widget_remove_tab: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_remove_tab as *const std::ffi::c_void) },
        ui_tab_widget_set_on_select_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_set_on_select_thunk_ptr as *const std::ffi::c_void) },
        ui_tab_widget_set_on_close_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_set_on_close_thunk_ptr as *const std::ffi::c_void) },
        ui_tab_widget_get_tab_count: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_get_tab_count as *const std::ffi::c_void) },
        ui_create_tree_view: unsafe { std::mem::transmute(ui_ffi::ui_create_tree_view as *const std::ffi::c_void) },
        ui_tree_view_add_node: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_add_node as *const std::ffi::c_void) },
        ui_tree_view_remove_node: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_remove_node as *const std::ffi::c_void) },
        ui_tree_view_set_selected: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_set_selected as *const std::ffi::c_void) },
        ui_tree_view_get_selected: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_get_selected as *const std::ffi::c_void) },
        ui_tree_view_expand_node: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_expand_node as *const std::ffi::c_void) },
        ui_tree_view_collapse_node: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_collapse_node as *const std::ffi::c_void) },
ui_tree_view_set_on_select_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_set_on_select_thunk_ptr as *const std::ffi::c_void) },
            ui_tree_view_set_on_toggle_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_set_on_toggle_thunk_ptr as *const std::ffi::c_void) },
            ui_tree_view_is_node_expanded: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_is_node_expanded as *const std::ffi::c_void) },
            ui_tree_node_set_text: unsafe { std::mem::transmute(ui_ffi::ui_tree_node_set_text as *const std::ffi::c_void) },
        ui_tree_node_get_user_data: unsafe { std::mem::transmute(ui_ffi::ui_tree_node_get_user_data as *const std::ffi::c_void) },
        ui_tree_view_clear_selection: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_clear_selection as *const std::ffi::c_void) },
        ui_create_popup_menu: unsafe { std::mem::transmute(ui_ffi::ui_create_popup_menu as *const std::ffi::c_void) },
        ui_popup_menu_add_item: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_add_item as *const std::ffi::c_void) },
        ui_popup_menu_add_separator: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_add_separator as *const std::ffi::c_void) },
        ui_popup_menu_show: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_show as *const std::ffi::c_void) },
        ui_popup_menu_hide: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_hide as *const std::ffi::c_void) },
        ui_popup_menu_is_visible: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_is_visible as *const std::ffi::c_void) },
        ui_popup_menu_set_on_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_set_on_click_thunk_ptr as *const std::ffi::c_void) },
        ui_create_grid_view: unsafe { std::mem::transmute(ui_ffi::ui_create_grid_view as *const std::ffi::c_void) },
        ui_grid_view_add_item: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_add_item as *const std::ffi::c_void) },
        ui_grid_view_remove_item: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_remove_item as *const std::ffi::c_void) },
        ui_grid_view_set_selected: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_set_selected as *const std::ffi::c_void) },
        ui_grid_view_get_selected: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_get_selected as *const std::ffi::c_void) },
        ui_grid_view_get_selected_user_data: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_get_selected_user_data as *const std::ffi::c_void) },
        ui_grid_view_clear: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_clear as *const std::ffi::c_void) },
        ui_grid_view_item_count: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_item_count as *const std::ffi::c_void) },
        ui_grid_view_set_on_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_set_on_click_thunk_ptr as *const std::ffi::c_void) },
        ui_create_dialog: unsafe { std::mem::transmute(ui_ffi::ui_create_dialog as *const std::ffi::c_void) },
        ui_dialog_set_content: unsafe { std::mem::transmute(ui_ffi::ui_dialog_set_content as *const std::ffi::c_void) },
        ui_dialog_add_button: unsafe { std::mem::transmute(ui_ffi::ui_dialog_add_button as *const std::ffi::c_void) },
        ui_dialog_show: unsafe { std::mem::transmute(ui_ffi::ui_dialog_show as *const std::ffi::c_void) },
        ui_dialog_hide: unsafe { std::mem::transmute(ui_ffi::ui_dialog_hide as *const std::ffi::c_void) },
        ui_dialog_is_visible: unsafe { std::mem::transmute(ui_ffi::ui_dialog_is_visible as *const std::ffi::c_void) },
        ui_dialog_get_result: unsafe { std::mem::transmute(ui_ffi::ui_dialog_get_result as *const std::ffi::c_void) },
        ui_dialog_set_on_result_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_dialog_set_on_result_thunk_ptr as *const std::ffi::c_void) },
        ui_create_file_browser: unsafe { std::mem::transmute(ui_ffi::ui_create_file_browser as *const std::ffi::c_void) },
        ui_file_browser_set_path: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_set_path as *const std::ffi::c_void) },
        ui_file_browser_set_filter: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_set_filter as *const std::ffi::c_void) },
        ui_file_browser_navigate_up: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_navigate_up as *const std::ffi::c_void) },
        ui_file_browser_refresh: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_refresh as *const std::ffi::c_void) },
        ui_file_browser_get_selected_path: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_get_selected_path as *const std::ffi::c_void) },
        ui_file_browser_get_current_path: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_get_current_path as *const std::ffi::c_void) },
        ui_file_browser_set_on_select_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_set_on_select_thunk_ptr as *const std::ffi::c_void) },
        ui_file_browser_set_on_double_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_set_on_double_click_thunk_ptr as *const std::ffi::c_void) },
        scene_create: scene_ffi_impl::scene_create_editor,
        scene_destroy: scene_ffi_impl::scene_destroy_editor,
        scene_create_cube: scene_ffi_impl::scene_create_cube_editor,
            scene_create_plane: scene_ffi_impl::scene_create_plane_editor,
            scene_create_directional_light: scene_ffi_impl::scene_create_directional_light_editor,
        scene_attach_script: scene_ffi_impl::scene_attach_script_editor,
        scene_set_game_state: scene_ffi_impl::scene_set_game_state_editor,
        scene_get_game_state: scene_ffi_impl::scene_get_game_state_editor,
        scene_pick_entity: scene_ffi_impl::scene_pick_entity_editor,
        scene_select_entity: scene_ffi_impl::scene_select_entity_editor,
        scene_update: scene_ffi_impl::scene_update_editor,
        set_renderer_game_state: ffi_impl::set_renderer_game_state,
        get_renderer_game_state: ffi_impl::get_renderer_game_state,
        set_entity_transform: ffi_impl::set_entity_transform,
        set_entity_angle: ffi_impl::set_entity_angle,
        get_entity_angle: ffi_impl::get_entity_angle,
        scene_get_entity_position: scene_ffi_impl::scene_get_entity_position_editor,
        scene_get_entity_rotation: scene_ffi_impl::scene_get_entity_rotation_editor,
        scene_get_entity_scale: scene_ffi_impl::scene_get_entity_scale_editor,
        scene_set_entity_position: scene_ffi_impl::scene_set_entity_position_editor,
        scene_set_entity_scale: scene_ffi_impl::scene_set_entity_scale_editor,
        scene_rotate_entity: scene_ffi_impl::scene_rotate_entity_editor,
        scene_set_entity_name: scene_ffi_impl::scene_set_entity_name_editor,
        scene_get_entity_name: scene_ffi_impl::scene_get_entity_name_editor,
        set_selected_entity: ffi_impl::set_selected_entity,
        scene_attach_script_binding: scene_ffi_impl::scene_attach_script_binding_editor,
        scene_remove_script_binding: scene_ffi_impl::scene_remove_script_binding_editor,
        scene_get_script_binding_count: scene_ffi_impl::scene_get_script_binding_count_editor,
        scene_get_script_binding_info: scene_ffi_impl::scene_get_script_binding_info_editor,
        scene_set_script_binding_enabled: scene_ffi_impl::scene_set_script_binding_enabled_editor,
        scene_create_entity: scene_ffi_impl::scene_create_entity_editor,
        scene_get_entity_count: scene_ffi_impl::scene_get_entity_count_editor,
        scene_get_entity_id: scene_ffi_impl::scene_get_entity_id_editor,
        scene_remove_entity: scene_ffi_impl::scene_remove_entity_editor,
        scene_set_parent: scene_ffi_impl::scene_set_parent_editor,
        scene_get_parent: scene_ffi_impl::scene_get_parent_editor,
        scene_get_child_count: scene_ffi_impl::scene_get_child_count_editor,
        scene_get_child_id: scene_ffi_impl::scene_get_child_id_editor,
        ui_entity_get_property_count: scene_ffi_impl::ui_entity_get_property_count_editor,
        ui_entity_get_property_name: scene_ffi_impl::ui_entity_get_property_name_editor,
        ui_entity_get_property_type: scene_ffi_impl::ui_entity_get_property_type_editor,
        ui_entity_get_property_category: scene_ffi_impl::ui_entity_get_property_category_editor,
        ui_entity_get_property_read_only: scene_ffi_impl::ui_entity_get_property_read_only_editor,
        ui_entity_get_property_value_float3: scene_ffi_impl::ui_entity_get_property_value_float3_editor,
        ui_entity_set_property_value_float3: scene_ffi_impl::ui_entity_set_property_value_float3_editor,
        ui_entity_get_property_value_string: scene_ffi_impl::ui_entity_get_property_value_string_editor,
        ui_entity_set_property_value_string: scene_ffi_impl::ui_entity_set_property_value_string_editor,
        ui_entity_get_property_value_float: scene_ffi_impl::ui_entity_get_property_value_float_editor,
        ui_entity_set_property_value_float: scene_ffi_impl::ui_entity_set_property_value_float_editor,
        widget_tree_ptr: widget_tree_handle,
        dfx_handle: dfx_for_csharp as *mut std::ffi::c_void,
        dfx_log: unsafe { std::mem::transmute(hezhou_dfx::dfx_log as *const std::ffi::c_void) },
        dfx_trace_begin: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_begin as *const std::ffi::c_void) },
        dfx_trace_end: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_end as *const std::ffi::c_void) },
        dfx_set_counter: unsafe { std::mem::transmute(hezhou_dfx::dfx_set_counter as *const std::ffi::c_void) },
        dfx_perf_begin_frame: unsafe { std::mem::transmute(hezhou_dfx::dfx_perf_begin_frame as *const std::ffi::c_void) },
        dfx_perf_end_frame: unsafe { std::mem::transmute(hezhou_dfx::dfx_perf_end_frame as *const std::ffi::c_void) },
        set_status_text: ffi_impl::set_status_text,
        on_hot_reload_complete: ffi_impl::on_hot_reload_complete_placeholder,
        ui_debug_print_widget_tree: unsafe { std::mem::transmute(ui_ffi::ui_debug_print_widget_tree as *const std::ffi::c_void) },
        ui_widget_set_flex_expand: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_flex_expand as *const std::ffi::c_void) },
        ui_widget_set_cross_axis_fill: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_cross_axis_fill as *const std::ffi::c_void) },
        ui_widget_set_background_color: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_background_color as *const std::ffi::c_void) },
        event_dispatcher_ptr: event_dispatcher_handle,
        ui_simulate_click_at: unsafe { std::mem::transmute(ui_ffi::ui_simulate_click_at as *const std::ffi::c_void) },
        ui_widget_get_type: unsafe { std::mem::transmute(ui_ffi::ui_widget_get_type as *const std::ffi::c_void) },
        ui_widget_get_layout: unsafe { std::mem::transmute(ui_ffi::ui_widget_get_layout as *const std::ffi::c_void) },
        ui_widget_get_parent: unsafe { std::mem::transmute(ui_ffi::ui_widget_get_parent as *const std::ffi::c_void) },
        ui_widget_get_child_count: unsafe { std::mem::transmute(ui_ffi::ui_widget_get_child_count as *const std::ffi::c_void) },
        ui_widget_get_child_id: unsafe { std::mem::transmute(ui_ffi::ui_widget_get_child_id as *const std::ffi::c_void) },
        ui_widget_get_total_count: unsafe { std::mem::transmute(ui_ffi::ui_widget_get_total_count as *const std::ffi::c_void) },
        ui_debug_dump_tree_to_buffer: unsafe { std::mem::transmute(ui_ffi::ui_debug_dump_tree_to_buffer as *const std::ffi::c_void) },
        capture_screenshot_to_file: ffi_impl::capture_screenshot_to_file,
        asset_library_get_category_count: hezhou_core::asset_library_get_category_count,
        asset_library_get_category_name: hezhou_core::asset_library_get_category_name,
        asset_library_get_asset_count: hezhou_core::asset_library_get_asset_count,
        asset_library_get_asset_info: hezhou_core::asset_library_get_asset_info,
        asset_library_create_entity_from_template: unsafe { std::mem::transmute(hezhou_core::asset_library_create_entity_from_template as *const std::ffi::c_void) },
        asset_library_create_mesh_entity: scene_ffi_impl::asset_library_create_mesh_entity_editor,
        project_create_new: hezhou_core::project_create_new,
        project_load: hezhou_core::project_load,
        project_save: hezhou_core::project_save,
        project_get_name: hezhou_core::project_get_name,
        project_get_path: hezhou_core::project_get_path,
        project_get_entity_count: hezhou_core::project_get_entity_count,
        project_is_loaded: hezhou_core::project_is_loaded,
        project_sync_to_scene: unsafe { std::mem::transmute(hezhou_core::project_sync_to_scene as *const std::ffi::c_void) },
        project_sync_from_scene: unsafe { std::mem::transmute(hezhou_core::project_sync_from_scene as *const std::ffi::c_void) },
        project_get_settings: hezhou_core::project_get_settings,
        project_set_settings: hezhou_core::project_set_settings,
        project_get_entity_info: hezhou_core::project_get_entity_info,
        project_add_entity: hezhou_core::project_add_entity,
        project_remove_entity: hezhou_core::project_remove_entity,
    };
    hezhou_scripting::ffi_context::set_ffi_context(ffi_ctx);
    let ffi_ptr = hezhou_scripting::ffi_context::get_ffi_context_ptr();
    unsafe { FFI_PTR = Some(ffi_ptr); }

    dfx_trace_begin!("Mono", "load");
    let dll_path = "scripts/bin/Mono/EditorScript.dll";
    let executor = MonoUIExecutor::new(dll_path)
        .expect("Failed to load Mono DLL");
    dfx_trace_end!("Mono", "load");
    
    unsafe {
        EXECUTOR = Some(executor);
    }

    dfx_trace_begin!("Mono", "initialize");
    unsafe {
        if let Some(ref executor) = EXECUTOR {
            executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr as usize)
                .expect("Initialize failed");
        }
    }
    dfx_trace_end!("Mono", "initialize");
    dfx_trace_end!("Startup", "editor");
    
    // Connect Scene to renderer for multi-entity rendering
    unsafe {
        if let Some(scene_ptr) = SCENE {
            renderer.set_scene(scene_ptr);
        }
    }

    let screenshot_path = if screenshot_mode {
        args.iter().position(|a| a == "--output")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| format!("screenshots/mono_editor_demo.png"))
    } else { String::new() };
    
    if screenshot_mode {
        std::fs::create_dir_all("screenshots").ok();
    }
    
    dfx_info!("Demo", "Editor started");
    dfx_info!("Demo", "Trace → traces/trace_latest.json");

    let mut frame_count = 0u64;
    let start_time = Instant::now();
    let mut screenshot_taken = false;
    
    loop {
        frame_count += 1;
        dfx_trace_begin!("Frame", "render");
        dfx.lock().get_perf_monitor().lock().begin_frame();
        
        dfx_trace_begin!("ProcessEvents", "ui");
        renderer.process_events();
        dfx_trace_end!("ProcessEvents", "ui");
        
        if screenshot_mode && !screenshot_taken {
            let elapsed = start_time.elapsed().as_secs_f32();
            if elapsed >= screenshot_delay {
                dfx_trace_begin!("DrawFrame", "render");
                match renderer.draw_frame() {
                    Ok(_) => {
                        dfx_trace_end!("DrawFrame", "render");
                        dfx_trace_end!("Frame", "render");
                        dfx.lock().get_perf_monitor().lock().end_frame();
                        dfx_info!("Screenshot", "Taking screenshot after {:.1}s...", elapsed);
                        if let Err(e) = renderer.capture_screenshot(&screenshot_path) {
                            dfx_error!("Screenshot", "Failed: {}", e);
                        } else {
                            dfx_info!("Screenshot", "Saved: {}", screenshot_path);
                        }
                        screenshot_taken = true;
                        break;
                    }
                    Err(e) => {
                        dfx_trace_end!("DrawFrame", "render");
                        dfx_trace_end!("Frame", "render");
                        dfx.lock().get_perf_monitor().lock().end_frame();
                        dfx_error!("Demo", "Draw error: {}", e);
                        break;
                    }
                }
            }
        }
        
        if HOT_RELOAD_REQUESTED.load(Ordering::SeqCst) {
            HOT_RELOAD_REQUESTED.store(false, Ordering::SeqCst);
            if hot_reload::handle_hot_reload(widget_tree_handle) {
                continue;
            }
        }

        dfx_trace_begin!("DrawFrame", "render");
        match renderer.draw_frame() {
            Ok(running) => {
                dfx_trace_end!("DrawFrame", "render");
                dfx_trace_end!("Frame", "render");
                
                // End perf frame and record counter points
                {
                    let dfx_guard = dfx.lock();
                    let perf_monitor = dfx_guard.get_perf_monitor();
                    let mut perf = perf_monitor.lock();
                    perf.end_frame();
                    let snapshot_opt = perf.get_latest_snapshot();
                    drop(perf); // Release perf lock before acquiring trace lock
                    
                    if let Some(snapshot) = snapshot_opt {
                        let trace_analyzer = dfx_guard.get_trace_analyzer();
                        let mut trace = trace_analyzer.lock();
                        trace.set_counter("FPS", "perf", snapshot.fps as i64);
                        trace.set_counter("CPU%", "perf", snapshot.cpu_usage_percent as i64);
                        trace.set_counter("MemUsedMB", "memory", snapshot.memory_used_mb as i64);
                        trace.set_counter("MemAvailMB", "memory", snapshot.memory_available_mb as i64);
                        trace.set_counter("FrameMs", "perf", snapshot.frame_time_ms as i64);
                    }
                }
                
                if !running {
                    break;
                }
            }
            Err(e) => {
                dfx_trace_end!("DrawFrame", "render");
                dfx_trace_end!("Frame", "render");
                dfx.lock().get_perf_monitor().lock().end_frame();
                dfx_error!("Demo", "{}", e);
                break;
            }
        }
        
        if frame_count % 300 == 0 {
            std::fs::create_dir_all("traces").ok();
            let trace_path = "traces/trace_latest.json";
            if let Err(e) = dfx.lock().get_trace_analyzer().lock().save_to_file(trace_path) {
                dfx_error!("Demo", "Failed to save trace: {}", e);
            }
            dfx.lock().get_trace_analyzer().lock().clear();
        }
        
        }

    dfx_info!("Demo", "Shutting down...");
    
    unsafe {
        if let Some(ref mut executor) = EXECUTOR {
            executor.shutdown();
        }
        EXECUTOR = None;
    }
    
    ui_ffi::ui_clear_widget_tree(widget_tree_handle as ui_ffi::WidgetTreeHandle);
    
    dfx_info!("Demo", "Saving trace...");
    std::fs::create_dir_all("traces").ok();
    let trace_path = format!("traces/trace_{}.json", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    {
        let dfx_guard = dfx.lock();
        let trace_analyzer_arc = dfx_guard.get_trace_analyzer();
        let trace_analyzer = trace_analyzer_arc.lock();
        let result = trace_analyzer.save_to_file(&trace_path);
        drop(trace_analyzer);
        drop(dfx_guard);
        match result {
            Ok(_) => dfx_info!("Demo", "Trace saved: {}", trace_path),
            Err(e) => dfx_error!("Demo", "Failed to save trace: {}", e),
        }
    }
    
    dfx_info!("Demo", "=== Editor Closed ===");
    std::process::exit(0);
}