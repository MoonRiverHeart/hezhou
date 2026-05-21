use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle, SetStatusTextFn, OnHotReloadCompleteFn}};
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
    dfx_info!("Demo", "Log file: {}", log_path);
    if screenshot_mode {
        dfx_info!("Demo", "Screenshot mode: delay={}s", screenshot_delay);
    }
    
    dfx_trace_begin!("Startup", "editor");
    dfx_info!("Demo", "[1] 创建编辑器窗口 (1280x720)...");
    
    dfx_trace_begin!("Window", "create");
    let mut renderer = UIVulkanRenderer::new(1280, 720, "Hezhou Game Editor")
        .expect("Failed to create renderer");
    unsafe { RENDERER = Some(&mut renderer as *mut UIVulkanRenderer); }
    dfx_trace_end!("Window", "create");
    dfx_info!("Demo", "窗口创建成功!");

    dfx_info!("Demo", "[2] 设置UI Root Panel...");
    dfx_trace_begin!("UI", "setup");
    renderer.setup_ui_for_script();
    ui_ffi::ui_set_screen_size(1280.0, 720.0);
    let content_scale = renderer.get_content_scale();
    ui_ffi::ui_set_content_scale(content_scale);
    dfx_trace_end!("UI", "setup");
    dfx_info!("Demo", "Content scale: {} (DPI: {})", content_scale, content_scale * 96.0);

    dfx_info!("Demo", "[3] 编译C#编辑器脚本...");
    dfx_trace_begin!("Script", "compile");
    hot_reload::compile_editor_script();
    dfx_trace_end!("Script", "compile");

    dfx_info!("Demo", "[4] 设置FFI Context...");
    let widget_tree_handle: WidgetTreeHandle = renderer.get_widget_tree_handle() as WidgetTreeHandle;
    
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
        widget_tree_ptr: widget_tree_handle,
        dfx_handle: dfx_for_csharp as *mut std::ffi::c_void,
        dfx_log: unsafe { std::mem::transmute(hezhou_dfx::dfx_log as *const std::ffi::c_void) },
        dfx_trace_begin: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_begin as *const std::ffi::c_void) },
        dfx_trace_end: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_end as *const std::ffi::c_void) },
        set_status_text: ffi_impl::set_status_text,
        on_hot_reload_complete: ffi_impl::on_hot_reload_complete_placeholder,
        ui_debug_print_widget_tree: unsafe { std::mem::transmute(ui_ffi::ui_debug_print_widget_tree as *const std::ffi::c_void) },
        ui_widget_set_flex_expand: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_flex_expand as *const std::ffi::c_void) },
        ui_widget_set_cross_axis_fill: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_cross_axis_fill as *const std::ffi::c_void) },
    };
    hezhou_scripting::ffi_context::set_ffi_context(ffi_ctx);
    let ffi_ptr = hezhou_scripting::ffi_context::get_ffi_context_ptr();
    unsafe { FFI_PTR = Some(ffi_ptr); }
    dfx_info!("Demo", "FfiContext已设置, ptr={:?}", ffi_ptr);

    dfx_info!("Demo", "[5] 加载Mono DLL...");
    dfx_trace_begin!("Mono", "load");
    let dll_path = "scripts/bin/Mono/EditorScript.dll";
    let executor = MonoUIExecutor::new(dll_path)
        .expect("Failed to load Mono DLL");
    dfx_trace_end!("Mono", "load");
    
    unsafe {
        EXECUTOR = Some(executor);
    }
    dfx_info!("Demo", "加载成功!");

    dfx_info!("Demo", "[6] 调用EditorScript.Initialize...");
    dfx_trace_begin!("Mono", "initialize");
    unsafe {
        if let Some(ref executor) = EXECUTOR {
            executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr as usize)
                .expect("Initialize failed");
        }
    }
    dfx_trace_end!("Mono", "initialize");
    dfx_trace_end!("Startup", "editor");
    dfx_info!("Demo", "编辑器UI创建成功!");

    let screenshot_path = if screenshot_mode {
        args.iter().position(|a| a == "--output")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| format!("screenshots/mono_editor_demo.png"))
    } else { String::new() };
    
    if screenshot_mode {
        std::fs::create_dir_all("screenshots").ok();
    }
    
    dfx_info!("Demo", "[7] 开始主循环...");
    dfx_info!("Demo", "Trace will be saved to traces/trace_latest.json on exit");

    let mut frame_count = 0u64;
    let start_time = Instant::now();
    let mut screenshot_taken = false;
    
    loop {
        frame_count += 1;
        dfx_trace_begin!("Frame", "render");
        
        renderer.process_events();
        
        if screenshot_mode && !screenshot_taken {
            let elapsed = start_time.elapsed().as_secs_f32();
            if elapsed >= screenshot_delay {
                dfx_trace_begin!("DrawFrame", "render");
                match renderer.draw_frame() {
                    Ok(_) => {
                        dfx_trace_end!("DrawFrame", "render");
                        dfx_trace_end!("Frame", "render");
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
                
                if !running {
                    dfx_trace_end!("Frame", "render");
                    break;
                }
            }
            Err(e) => {
                dfx_trace_end!("DrawFrame", "render");
                dfx_error!("Demo", "{}", e);
                break;
            }
        }

        dfx_trace_end!("Frame", "render");
        
        if frame_count % 300 == 0 {
            std::fs::create_dir_all("traces").ok();
            let trace_path = "traces/trace_latest.json";
            if let Err(e) = dfx.lock().get_trace_analyzer().lock().save_to_file(trace_path) {
                dfx_error!("Demo", "Failed to save trace: {}", e);
            }
            dfx.lock().get_trace_analyzer().lock().clear();
        }
        
        std::thread::sleep(Duration::from_millis(16));
    }

    dfx_info!("Demo", "[8] 清理资源...");
    
    unsafe {
        if let Some(ref mut executor) = EXECUTOR {
            dfx_info!("Demo", "先卸载Mono assembly...");
            executor.shutdown();
        }
        EXECUTOR = None;
    }
    
    dfx_info!("Demo", "清理UI widgets...");
    ui_ffi::ui_clear_widget_tree(widget_tree_handle as ui_ffi::WidgetTreeHandle);
    dfx_info!("Demo", "UI widgets清理完成");
    
    dfx_info!("Demo", "保存trace...");
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
            Ok(_) => dfx_info!("Demo", "Trace saved to {}", trace_path),
            Err(e) => dfx_error!("Demo", "Failed to save trace: {}", e),
        }
    }
    dfx_info!("Demo", "Trace保存完成");
    
    dfx_info!("Demo", "=== Editor Closed ===");
    std::process::exit(0);
}