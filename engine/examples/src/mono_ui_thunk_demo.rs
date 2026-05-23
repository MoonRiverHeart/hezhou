use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle}};
use hezhou_ui::ffi as ui_ffi;
use hezhou_dfx::*;
use std::time::{Duration, Instant};

pub extern "C" fn trigger_hot_reload() {}
pub extern "C" fn set_game_preview_extent(_width: u32, _height: u32) {}
pub extern "C" fn set_camera_params(_yaw: f32, _pitch: f32, _x: f32, _y: f32, _z: f32) {}
pub extern "C" fn register_key_stub(_ptr: *const std::ffi::c_void) {}
pub extern "C" fn register_mouse_move_stub(_ptr: *const std::ffi::c_void) {}
pub extern "C" fn set_preview_window_edit_mode_stub(_handle: WidgetTreeHandle, _id: u64, _mode: bool) {}
pub extern "C" fn scene_create_stub() -> *mut std::ffi::c_void { std::ptr::null_mut() }
pub extern "C" fn scene_destroy_stub(_scene: *mut std::ffi::c_void) {}
pub extern "C" fn scene_create_cube_stub(_scene: *mut std::ffi::c_void) -> u64 { 0 }
pub extern "C" fn scene_create_plane_stub(_scene: *mut std::ffi::c_void) -> u64 { 0 }
pub extern "C" fn scene_create_directional_light_stub(_scene: *mut std::ffi::c_void) -> u64 { 0 }
pub extern "C" fn scene_attach_script_stub(_scene: *mut std::ffi::c_void, _id: u64, _path: *const i8, _class: *const i8) {}
pub extern "C" fn scene_set_game_state_stub(_scene: *mut std::ffi::c_void, _state: i32) {}
pub extern "C" fn scene_get_game_state_stub(_scene: *mut std::ffi::c_void) -> i32 { 0 }
pub extern "C" fn scene_pick_entity_stub(_scene: *mut std::ffi::c_void, _ox: f32, _oy: f32, _oz: f32, _dx: f32, _dy: f32, _dz: f32) -> u64 { 0 }
pub extern "C" fn scene_select_entity_stub(_scene: *mut std::ffi::c_void, _id: u64) {}
pub extern "C" fn scene_update_stub(_scene: *mut std::ffi::c_void, _dt: f32) {}
pub extern "C" fn set_renderer_game_state_stub(_state: i32) {}
pub extern "C" fn get_renderer_game_state_stub() -> i32 { 0 }
pub extern "C" fn set_entity_transform_stub(_px: f32, _py: f32, _pz: f32, _rx: f32, _ry: f32, _rz: f32, _rw: f32, _sx: f32, _sy: f32, _sz: f32) {}
pub extern "C" fn set_entity_angle_stub(_angle: f32) {}
pub extern "C" fn get_entity_angle_stub() -> f32 { 0.0 }
pub extern "C" fn scene_get_entity_position_stub(_scene: *mut std::ffi::c_void, _id: u64, _x: *mut f32, _y: *mut f32, _z: *mut f32) {}
pub extern "C" fn scene_get_entity_rotation_stub(_scene: *mut std::ffi::c_void, _id: u64, _x: *mut f32, _y: *mut f32, _z: *mut f32, _w: *mut f32) {}
pub extern "C" fn scene_get_entity_scale_stub(_scene: *mut std::ffi::c_void, _id: u64, _x: *mut f32, _y: *mut f32, _z: *mut f32) {}

pub extern "C" fn scene_set_entity_position_stub(_scene: *mut std::ffi::c_void, _id: u64, _x: f32, _y: f32, _z: f32) {}

pub extern "C" fn scene_set_entity_scale_stub(_scene: *mut std::ffi::c_void, _id: u64, _x: f32, _y: f32, _z: f32) {}

pub extern "C" fn scene_rotate_entity_stub(_scene: *mut std::ffi::c_void, _id: u64, _angle: f32) {}

pub extern "C" fn scene_set_entity_name_stub(_scene: *mut std::ffi::c_void, _id: u64, _name: *const i8) {}

pub extern "C" fn scene_get_entity_name_stub(_scene: *mut std::ffi::c_void, _id: u64, _buffer: *mut i8, _size: usize) -> usize { 0 }

pub extern "C" fn set_selected_entity_stub(_id: u64, _selected: bool) {}
pub extern "C" fn scene_attach_script_binding_stub(_scene: *mut std::ffi::c_void, _id: u64, _path: *const i8, _class: *const i8) {}
pub extern "C" fn scene_remove_script_binding_stub(_scene: *mut std::ffi::c_void, _id: u64, _index: usize) {}
pub extern "C" fn scene_get_script_binding_count_stub(_scene: *mut std::ffi::c_void, _id: u64) -> usize { 0 }
pub extern "C" fn scene_get_script_binding_info_stub(_scene: *mut std::ffi::c_void, _id: u64, _index: usize, _path: *mut i8, _path_size: usize, _class: *mut i8, _class_size: usize, _enabled: *mut bool) -> bool { false }
pub extern "C" fn scene_set_script_binding_enabled_stub(_scene: *mut std::ffi::c_void, _id: u64, _index: usize, _enabled: bool) {}
pub extern "C" fn scene_create_entity_stub(_scene: *mut std::ffi::c_void) -> u64 { 0 }
pub extern "C" fn scene_remove_entity_stub(_scene: *mut std::ffi::c_void, _entity_id: u64) { }
pub extern "C" fn scene_get_entity_count_stub(_scene: *mut std::ffi::c_void) -> u64 { 0 }
pub extern "C" fn scene_get_entity_id_stub(_scene: *mut std::ffi::c_void, _index: u64) -> u64 { 0 }
pub extern "C" fn scene_set_parent_stub(_scene: *mut std::ffi::c_void, _entity_id: u64, _parent_id: u64) {}
pub extern "C" fn scene_get_parent_stub(_scene: *mut std::ffi::c_void, _entity_id: u64) -> u64 { 0 }
pub extern "C" fn scene_get_child_count_stub(_scene: *mut std::ffi::c_void, _parent_id: u64) -> usize { 0 }
pub extern "C" fn scene_get_child_id_stub(_scene: *mut std::ffi::c_void, _parent_id: u64, _index: usize) -> u64 { 0 }
pub extern "C" fn ui_create_slider_stub(_handle: *mut std::ffi::c_void, _parent_id: u64, _w: f32, _h: f32) -> u64 { 0 }
pub extern "C" fn ui_create_slider_in_parent_stub(_handle: *mut std::ffi::c_void, _parent_id: u64, _x: f32, _y: f32, _w: f32, _h: f32, _min: f32, _max: f32, _val: f32) -> u64 { 0 }
pub extern "C" fn ui_slider_set_value_stub(_handle: *mut std::ffi::c_void, _id: u64, _val: f32) {}
pub extern "C" fn ui_slider_get_value_stub(_handle: *mut std::ffi::c_void, _id: u64) -> f32 { 0.0 }
pub extern "C" fn ui_slider_set_range_stub(_handle: *mut std::ffi::c_void, _id: u64, _min: f32, _max: f32) {}
pub extern "C" fn ui_slider_set_on_change_thunk_ptr_stub(_handle: *mut std::ffi::c_void, _id: u64, _ptr: *const std::ffi::c_void) {}
pub extern "C" fn ui_slider_set_step_stub(_handle: *mut std::ffi::c_void, _id: u64, _step: f32) {}
pub extern "C" fn ui_create_scroll_view_stub(_handle: *mut std::ffi::c_void, _parent_id: u64, _x: f32, _y: f32, _w: f32, _h: f32) -> u64 { 0 }
pub extern "C" fn ui_scroll_view_set_scroll_offset_stub(_handle: *mut std::ffi::c_void, _id: u64, _offset: f32) {}
pub extern "C" fn ui_scroll_view_get_scroll_offset_stub(_handle: *mut std::ffi::c_void, _id: u64) -> f32 { 0.0 }
pub extern "C" fn ui_scroll_view_set_show_scrollbars_stub(_handle: *mut std::ffi::c_void, _id: u64, _v: u32, _h: u32) {}
pub extern "C" fn ui_scroll_view_set_on_scroll_thunk_ptr_stub(_handle: *mut std::ffi::c_void, _id: u64, _ptr: *const std::ffi::c_void) {}
pub extern "C" fn ui_create_split_view_stub(_handle: *mut std::ffi::c_void, _parent_id: u64, _x: f32, _y: f32, _w: f32, _h: f32, _orientation: u32) -> u64 { 0 }
pub extern "C" fn ui_split_view_set_split_ratio_stub(_handle: *mut std::ffi::c_void, _id: u64, _ratio: f32) {}
pub extern "C" fn ui_split_view_get_split_ratio_stub(_handle: *mut std::ffi::c_void, _id: u64) -> f32 { 0.5 }
pub extern "C" fn ui_split_view_set_min_ratio_stub(_handle: *mut std::ffi::c_void, _id: u64, _min: f32) {}
pub extern "C" fn ui_split_view_set_max_ratio_stub(_handle: *mut std::ffi::c_void, _id: u64, _max: f32) {}
pub extern "C" fn ui_split_view_set_on_ratio_change_thunk_ptr_stub(_handle: *mut std::ffi::c_void, _id: u64, _ptr: *const std::ffi::c_void) {}
// Property reflection stubs
pub extern "C" fn ui_entity_get_property_count_stub() -> u32 { 0 }
pub extern "C" fn ui_entity_get_property_name_stub(_index: u32) -> *const std::ffi::c_char { std::ptr::null() }
pub extern "C" fn ui_entity_get_property_type_stub(_index: u32) -> u32 { 0 }
pub extern "C" fn ui_entity_get_property_category_stub(_index: u32) -> *const std::ffi::c_char { std::ptr::null() }
pub extern "C" fn ui_entity_get_property_read_only_stub(_index: u32) -> bool { true }
pub extern "C" fn ui_entity_get_property_value_float3_stub(_scene: *mut std::ffi::c_void, _id: u64, _name: *const std::ffi::c_char, _x: *mut f32, _y: *mut f32, _z: *mut f32) -> bool { false }
pub extern "C" fn ui_entity_set_property_value_float3_stub(_scene: *mut std::ffi::c_void, _id: u64, _name: *const std::ffi::c_char, _x: f32, _y: f32, _z: f32) -> bool { false }
pub extern "C" fn ui_entity_get_property_value_string_stub(_scene: *mut std::ffi::c_void, _id: u64, _name: *const std::ffi::c_char, _buf: *mut std::ffi::c_char, _len: u32) -> u32 { 0 }
pub extern "C" fn ui_entity_set_property_value_string_stub(_scene: *mut std::ffi::c_void, _id: u64, _name: *const std::ffi::c_char, _val: *const std::ffi::c_char) -> bool { false }
pub extern "C" fn ui_entity_get_property_value_float_stub(_scene: *mut std::ffi::c_void, _id: u64, _name: *const std::ffi::c_char, _out: *mut f32) -> bool { false }
pub extern "C" fn ui_entity_set_property_value_float_stub(_scene: *mut std::ffi::c_void, _id: u64, _name: *const std::ffi::c_char, _val: f32) -> bool { false }
pub extern "C" fn set_status_text_stub(_text: *const i8) {}
pub extern "C" fn on_hot_reload_complete_stub() {}
pub extern "C" fn dfx_set_counter_stub(_system: *mut std::ffi::c_void, _name: *const i8, _category: *const i8, _value: i64) {}
pub extern "C" fn dfx_perf_begin_frame_stub(_system: *mut std::ffi::c_void) {}
pub extern "C" fn dfx_perf_end_frame_stub(_system: *mut std::ffi::c_void) {}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let screenshot_mode = args.iter().any(|a| a == "--screenshot");
    let screenshot_delay = if screenshot_mode { 
        args.iter().position(|a| a == "--delay")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(2.0)
    } else { 0.0 };
    let screenshot_path = if screenshot_mode {
        args.iter().position(|a| a == "--output")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| "screenshots/mono_ui_thunk_demo.png".to_string())
    } else { String::new() };
    
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Debug);
    
    dfx_info!("Demo", "=== Thunk + Mono JIT UI Demo ===");
    if screenshot_mode {
        std::fs::create_dir_all("screenshots").ok();
        dfx_info!("Demo", "[Screenshot mode] delay={}s, output={}", screenshot_delay, screenshot_path);
    }
    dfx_info!("Demo", "[1] 初始化 Vulkan UI Renderer...");
    
    let mut renderer = UIVulkanRenderer::new(800, 600, "C# UI Demo")
        .expect("Failed to create renderer");
    dfx_info!("Demo", "Renderer初始化成功!");

    dfx_info!("Demo", "[2] 设置UI Root Panel...");
    renderer.setup_ui_for_script();
    dfx_info!("Demo", "Root Panel设置完成!");

    dfx_info!("Demo", "[3] 编译C#脚本...");
    compile_csharp_script();

    dfx_info!("Demo", "[4] 设置FFI Context...");
    let widget_tree_handle: WidgetTreeHandle = renderer.get_widget_tree_handle() as WidgetTreeHandle;
    
    let ffi_ctx = FfiContext {
        ui_get_primary_button_id: ui_ffi::ui_get_primary_button_id,
        ui_set_primary_button_id: ui_ffi::ui_set_primary_button_id,
        ui_widget_set_text: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_text as *const std::ffi::c_void) },
        ui_button_set_on_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_button_set_on_click_thunk_ptr as *const std::ffi::c_void) },
        ui_register_update_thunk_ptr: ui_ffi::ui_register_update_thunk_ptr,
        ui_register_resize_thunk_ptr: ui_ffi::ui_register_resize_thunk_ptr,
        ui_register_global_click_thunk_ptr: ui_ffi::ui_register_global_click_thunk_ptr,
        ui_register_key_thunk_ptr: register_key_stub,
        ui_register_mouse_move_thunk_ptr: register_mouse_move_stub,
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
        ui_trigger_hot_reload: trigger_hot_reload,
        ui_set_game_preview_extent: set_game_preview_extent,
        ui_set_camera_params: set_camera_params,
        ui_is_preview_window_selected: unsafe { std::mem::transmute(ui_ffi::ui_is_preview_window_selected as *const std::ffi::c_void) },
        ui_set_preview_window_selected: unsafe { std::mem::transmute(ui_ffi::ui_set_preview_window_selected as *const std::ffi::c_void) },
        ui_set_preview_window_edit_mode: unsafe { std::mem::transmute(set_preview_window_edit_mode_stub as *const std::ffi::c_void) },
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
        ui_create_slider: unsafe { std::mem::transmute(ui_create_slider_stub as *const std::ffi::c_void) },
        ui_create_slider_in_parent: unsafe { std::mem::transmute(ui_create_slider_in_parent_stub as *const std::ffi::c_void) },
        ui_slider_set_value: unsafe { std::mem::transmute(ui_slider_set_value_stub as *const std::ffi::c_void) },
        ui_slider_get_value: unsafe { std::mem::transmute(ui_slider_get_value_stub as *const std::ffi::c_void) },
        ui_slider_set_range: unsafe { std::mem::transmute(ui_slider_set_range_stub as *const std::ffi::c_void) },
        ui_slider_set_on_change_thunk_ptr: unsafe { std::mem::transmute(ui_slider_set_on_change_thunk_ptr_stub as *const std::ffi::c_void) },
        ui_slider_set_step: unsafe { std::mem::transmute(ui_slider_set_step_stub as *const std::ffi::c_void) },
        ui_create_scroll_view: unsafe { std::mem::transmute(ui_create_scroll_view_stub as *const std::ffi::c_void) },
        ui_scroll_view_set_scroll_offset: unsafe { std::mem::transmute(ui_scroll_view_set_scroll_offset_stub as *const std::ffi::c_void) },
        ui_scroll_view_get_scroll_offset: unsafe { std::mem::transmute(ui_scroll_view_get_scroll_offset_stub as *const std::ffi::c_void) },
        ui_scroll_view_set_show_scrollbars: unsafe { std::mem::transmute(ui_scroll_view_set_show_scrollbars_stub as *const std::ffi::c_void) },
        ui_scroll_view_set_on_scroll_thunk_ptr: unsafe { std::mem::transmute(ui_scroll_view_set_on_scroll_thunk_ptr_stub as *const std::ffi::c_void) },
        ui_create_split_view: unsafe { std::mem::transmute(ui_create_split_view_stub as *const std::ffi::c_void) },
        ui_split_view_set_split_ratio: unsafe { std::mem::transmute(ui_split_view_set_split_ratio_stub as *const std::ffi::c_void) },
        ui_split_view_get_split_ratio: unsafe { std::mem::transmute(ui_split_view_get_split_ratio_stub as *const std::ffi::c_void) },
        ui_split_view_set_min_ratio: unsafe { std::mem::transmute(ui_split_view_set_min_ratio_stub as *const std::ffi::c_void) },
        ui_split_view_set_max_ratio: unsafe { std::mem::transmute(ui_split_view_set_max_ratio_stub as *const std::ffi::c_void) },
        ui_split_view_set_on_ratio_change_thunk_ptr: unsafe { std::mem::transmute(ui_split_view_set_on_ratio_change_thunk_ptr_stub as *const std::ffi::c_void) },
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
        scene_create: scene_create_stub,
        scene_destroy: scene_destroy_stub,
        scene_create_cube: scene_create_cube_stub,
        scene_create_plane: scene_create_plane_stub,
        scene_create_directional_light: scene_create_directional_light_stub,
        scene_attach_script: scene_attach_script_stub,
        scene_set_game_state: scene_set_game_state_stub,
        scene_get_game_state: scene_get_game_state_stub,
        scene_pick_entity: scene_pick_entity_stub,
        scene_select_entity: scene_select_entity_stub,
        scene_update: scene_update_stub,
        set_renderer_game_state: set_renderer_game_state_stub,
        get_renderer_game_state: get_renderer_game_state_stub,
        set_entity_transform: set_entity_transform_stub,
        set_entity_angle: set_entity_angle_stub,
        get_entity_angle: get_entity_angle_stub,
        scene_get_entity_position: scene_get_entity_position_stub,
        scene_get_entity_rotation: scene_get_entity_rotation_stub,
        scene_get_entity_scale: scene_get_entity_scale_stub,
        scene_set_entity_position: scene_set_entity_position_stub,
        scene_set_entity_scale: scene_set_entity_scale_stub,
        scene_rotate_entity: scene_rotate_entity_stub,
        scene_set_entity_name: scene_set_entity_name_stub,
        scene_get_entity_name: scene_get_entity_name_stub,
        set_selected_entity: set_selected_entity_stub,
        scene_attach_script_binding: scene_attach_script_binding_stub,
        scene_remove_script_binding: scene_remove_script_binding_stub,
        scene_get_script_binding_count: scene_get_script_binding_count_stub,
        scene_get_script_binding_info: scene_get_script_binding_info_stub,
        scene_set_script_binding_enabled: scene_set_script_binding_enabled_stub,
        scene_create_entity: scene_create_entity_stub,
        scene_get_entity_count: scene_get_entity_count_stub,
        scene_get_entity_id: scene_get_entity_id_stub,
        scene_remove_entity: scene_remove_entity_stub,
        scene_set_parent: scene_set_parent_stub,
        scene_get_parent: scene_get_parent_stub,
        scene_get_child_count: scene_get_child_count_stub,
        scene_get_child_id: scene_get_child_id_stub,
        ui_entity_get_property_count: ui_entity_get_property_count_stub,
        ui_entity_get_property_name: ui_entity_get_property_name_stub,
        ui_entity_get_property_type: ui_entity_get_property_type_stub,
        ui_entity_get_property_category: ui_entity_get_property_category_stub,
        ui_entity_get_property_read_only: ui_entity_get_property_read_only_stub,
        ui_entity_get_property_value_float3: ui_entity_get_property_value_float3_stub,
        ui_entity_set_property_value_float3: ui_entity_set_property_value_float3_stub,
        ui_entity_get_property_value_string: ui_entity_get_property_value_string_stub,
        ui_entity_set_property_value_string: ui_entity_set_property_value_string_stub,
        ui_entity_get_property_value_float: ui_entity_get_property_value_float_stub,
        ui_entity_set_property_value_float: ui_entity_set_property_value_float_stub,
        widget_tree_ptr: widget_tree_handle,
        dfx_handle: std::ptr::null_mut(),
        dfx_log: unsafe { std::mem::transmute(hezhou_dfx::dfx_log as *const std::ffi::c_void) },
        dfx_trace_begin: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_begin as *const std::ffi::c_void) },
        dfx_trace_end: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_end as *const std::ffi::c_void) },
        dfx_set_counter: dfx_set_counter_stub,
        dfx_perf_begin_frame: dfx_perf_begin_frame_stub,
        dfx_perf_end_frame: dfx_perf_end_frame_stub,
        set_status_text: set_status_text_stub,
        on_hot_reload_complete: on_hot_reload_complete_stub,
        ui_debug_print_widget_tree: unsafe { std::mem::transmute(ui_ffi::ui_debug_print_widget_tree as *const std::ffi::c_void) },
        ui_widget_set_flex_expand: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_flex_expand as *const std::ffi::c_void) },
        ui_widget_set_cross_axis_fill: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_cross_axis_fill as *const std::ffi::c_void) },
        ui_widget_set_background_color: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_background_color as *const std::ffi::c_void) },
        // Asset library stubs (not used in this demo)
        asset_library_get_category_count: stub_asset_library_get_category_count,
        asset_library_get_category_name: stub_asset_library_get_category_name,
        asset_library_get_asset_count: stub_asset_library_get_asset_count,
        asset_library_get_asset_info: stub_asset_library_get_asset_info,
        asset_library_create_entity_from_template: stub_asset_library_create_entity_from_template,
        asset_library_create_mesh_entity: stub_asset_library_create_mesh_entity,
        // Project stubs (not used in this demo)
        project_create_new: stub_project_create_new,
        project_load: stub_project_load,
        project_save: stub_project_save,
        project_get_name: stub_project_get_name,
        project_get_path: stub_project_get_path,
        project_get_entity_count: stub_project_get_entity_count,
        project_is_loaded: stub_project_is_loaded,
        project_sync_to_scene: stub_project_sync_to_scene,
        project_sync_from_scene: stub_project_sync_from_scene,
        project_get_settings: stub_project_get_settings,
        project_set_settings: stub_project_set_settings,
        project_get_entity_info: stub_project_get_entity_info,
        project_add_entity: stub_project_add_entity,
        project_remove_entity: stub_project_remove_entity,
    };
    hezhou_scripting::ffi_context::set_ffi_context(ffi_ctx);
    let ffi_ptr = hezhou_scripting::ffi_context::get_ffi_context_ptr();
    dfx_info!("Demo", "FfiContext已设置, ptr={:?}", ffi_ptr);

    dfx_info!("Demo", "[5] 加载Mono DLL...");
    let dll_path = "scripts/bin/Mono/TestScript.dll";
    let executor = MonoUIExecutor::new(dll_path)
        .expect("Failed to load Mono DLL");
    dfx_info!("Demo", "加载成功!");

    dfx_info!("Demo", "[6] 调用Initialize...");
    executor.call_static_with_ptr("TestScript", "Initialize", ffi_ptr as usize)
        .expect("Initialize failed");
    dfx_info!("Demo", "Initialize调用成功!");

    dfx_info!("Demo", "[7] 开始主循环...");
    
    let start_time = Instant::now();
    let mut screenshot_taken = false;

    loop {
        renderer.process_events();
        
        if screenshot_mode && !screenshot_taken {
            let elapsed = start_time.elapsed().as_secs_f32();
            if elapsed >= screenshot_delay {
                dfx_info!("Screenshot", "Taking screenshot...");
                if let Err(e) = renderer.capture_screenshot(&screenshot_path) {
                    dfx_error!("Screenshot", "Failed: {}", e);
                } else {
                    dfx_info!("Screenshot", "Saved: {}", screenshot_path);
                }
                screenshot_taken = true;
                break;
            }
        }

        match renderer.draw_frame() {
            Ok(running) => {
                if !running {
                    break;
                }
            }
            Err(e) => {
                dfx_error!("Demo", "{}", e);
                break;
            }
        }

        std::thread::sleep(Duration::from_millis(16));
    }

    dfx_info!("Demo", "[8] 清理资源...");
    
    if screenshot_mode {
        dfx_info!("Demo", "Screenshot mode - skipping cleanup");
    } else {
        renderer.cleanup();
    }
    dfx_info!("Demo", "=== Demo Complete ===");
}

// Asset library stubs (not used in this demo)
#[unsafe(no_mangle)]
extern "C" fn stub_asset_library_get_category_count() -> usize { 0 }
#[unsafe(no_mangle)]
extern "C" fn stub_asset_library_get_category_name(_: usize, _: *mut std::ffi::c_char, _: usize) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_asset_library_get_asset_count(_: usize) -> usize { 0 }
#[unsafe(no_mangle)]
extern "C" fn stub_asset_library_get_asset_info(_: usize, _: usize, _: *mut u64, _: *mut std::ffi::c_char, _: usize, _: *mut u32, _: *mut std::ffi::c_char, _: usize) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_asset_library_create_entity_from_template(_: *mut std::ffi::c_void, _: u64) -> u64 { 0 }
#[unsafe(no_mangle)]
extern "C" fn stub_asset_library_create_mesh_entity(_: *mut std::ffi::c_void, _: u32) -> u64 { 0 }

// Project stubs (not used in this demo)
#[unsafe(no_mangle)]
extern "C" fn stub_project_create_new(_: *const std::ffi::c_char, _: *const std::ffi::c_char) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_load(_: *const std::ffi::c_char) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_save() -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_get_name(_: *mut std::ffi::c_char, _: usize) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_get_path(_: *mut std::ffi::c_char, _: usize) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_get_entity_count() -> usize { 0 }
#[unsafe(no_mangle)]
extern "C" fn stub_project_is_loaded() -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_sync_to_scene(_: *mut std::ffi::c_void) {}
#[unsafe(no_mangle)]
extern "C" fn stub_project_sync_from_scene(_: *const std::ffi::c_void) {}
#[unsafe(no_mangle)]
extern "C" fn stub_project_get_settings(_: *mut u32, _: *mut u32, _: *mut u32) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_set_settings(_: u32, _: u32, _: u32) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_get_entity_info(_: u64, _: *mut std::ffi::c_char, _: usize, _: *mut f32, _: *mut f32, _: *mut f32, _: *mut std::ffi::c_char, _: usize) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_add_entity(_: u64, _: *const std::ffi::c_char, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32, _: *const std::ffi::c_char) -> bool { false }
#[unsafe(no_mangle)]
extern "C" fn stub_project_remove_entity(_: u64) -> bool { false }

fn compile_csharp_script() {
    use std::process::Command;
    
    let result = Command::new("mcs")
        .args([
            "-target:library",
            "-out:scripts/bin/Mono/TestScript.dll",
            "scripts/TestScript.cs",
            "scripts/UI.cs",
        ])
        .output();
    
    match result {
        Ok(output) => {
            if output.status.success() {
                dfx_info!("Demo", "TestScript.dll编译成功");
            } else {
                dfx_error!("Demo", "编译失败: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(_) => {
            dfx_info!("Demo", "mcs not found");
        }
    }
}