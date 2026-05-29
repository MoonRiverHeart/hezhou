//! FfiBridge — C# FFI实现EngineBridge trait
//!
//! 需要编辑器+Mono JIT运行时，通过FfiContext函数指针调用C#封装。
//! 能力边界: 全部tool可用(读取+写入+截图+创建实体+脚本+UI widget)
//! 依赖: hezhou-scripting crate的FfiContext(270+函数指针)
//!
//! FfiContext是#[repr(C)] flat struct，每个字段都是extern "C" fn指针。
//! 所有scene_*函数的第一个参数是scene_ptr (*mut c_void)。
//! project_*和asset_library_*函数使用全局状态，不需要额外指针。

use crate::bridge::*;
use anyhow::{Result, bail};
use std::ffi::{c_void, c_char, CString};
use std::sync::Arc;

/// FfiBridge — 持有FfiContext引用 + scene_ptr
pub struct FfiBridge {
    ctx: Arc<hezhou_scripting::FfiContext>,
    scene_ptr: *mut c_void,
}

// scene_ptr是*mut c_void，与RustBridge的*mut Scene模式一致
unsafe impl Send for FfiBridge {}
unsafe impl Sync for FfiBridge {}

impl FfiBridge {
    /// 从FfiContext创建FfiBridge
    /// # Safety: scene_ptr必须指向有效Scene，生命周期内不被释放
    pub unsafe fn from_ffi_context(
        ctx: Arc<hezhou_scripting::FfiContext>,
        scene_ptr: *mut c_void,
    ) -> Self {
        Self { ctx, scene_ptr }
    }
}

// C字符串读取: scene_get_entity_name返回usize(写入字节数)
fn read_entity_name(ctx: &hezhou_scripting::FfiContext, scene: *mut c_void, entity_id: u64) -> String {
    let max_len = 256;
    let mut buf = vec![0u8; max_len];
    let written = (ctx.scene_get_entity_name)(scene, entity_id, buf.as_mut_ptr() as *mut c_char, max_len);
    if written > 0 {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(written);
        String::from_utf8_lossy(&buf[..end.min(max_len)]).to_string()
    } else {
        format!("Entity_{}", entity_id)
    }
}

// C字符串读取: project_get_name等返回bool
fn read_bool_c_string(write_fn: impl Fn(*mut c_char, usize) -> bool, max_len: usize) -> String {
    let mut buf = vec![0u8; max_len];
    let success = write_fn(buf.as_mut_ptr() as *mut c_char, max_len);
    if success {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(max_len);
        String::from_utf8_lossy(&buf[..end]).to_string()
    } else {
        String::new()
    }
}

/// Quaternion → Y轴旋转角度(度数)
fn quaternion_to_yaw_degrees(qx: f32, qy: f32, qz: f32, qw: f32) -> f32 {
    let sin_yaw = 2.0 * (qw * qy + qx * qz);
    let cos_yaw = 1.0 - 2.0 * (qy * qy + qz * qz);
    sin_yaw.atan2(cos_yaw).to_degrees()
}

impl EngineBridge for FfiBridge {
    fn get_scene_tree(&self) -> Result<Vec<SceneTreeNode>> {
        let scene = self.scene_ptr;
        let entity_count = (self.ctx.scene_get_entity_count)(scene) as usize;
        let mut tree = Vec::new();
        for i in 0..entity_count {
            let entity_id = (self.ctx.scene_get_entity_id)(scene, i as u64);
            let name = read_entity_name(&self.ctx, scene, entity_id);
            let child_count = (self.ctx.scene_get_child_count)(scene, entity_id);
            let children = (0..child_count).map(|j| {
                let child_id = (self.ctx.scene_get_child_id)(scene, entity_id, j);
                SceneTreeNode {
                    entity_id: SchemaU64(child_id),
                    name: read_entity_name(&self.ctx, scene, child_id),
                    children: Vec::new(),
                }
            }).collect();
            tree.push(SceneTreeNode { entity_id: SchemaU64(entity_id), name, children });
        }
        Ok(tree)
    }

    fn list_entities(&self) -> Result<Vec<EntityInfo>> {
        let scene = self.scene_ptr;
        let count = (self.ctx.scene_get_entity_count)(scene) as usize;
        let mut result = Vec::new();
        for i in 0..count {
            let eid = (self.ctx.scene_get_entity_id)(scene, i as u64);
            result.push(self.get_entity_info(eid)?);
        }
        Ok(result)
    }

    fn get_entity_info(&self, entity_id: u64) -> Result<EntityInfo> {
        let scene = self.scene_ptr;
        let name = read_entity_name(&self.ctx, scene, entity_id);
        let mut x=0.0f32; let mut y=0.0f32; let mut z=0.0f32;
        (self.ctx.scene_get_entity_position)(scene, entity_id, &mut x, &mut y, &mut z);
        let mut sx=1.0f32; let mut sy=1.0f32; let mut sz=1.0f32;
        (self.ctx.scene_get_entity_scale)(scene, entity_id, &mut sx, &mut sy, &mut sz);
        let mut qx=0.0f32; let mut qy=0.0f32; let mut qz=0.0f32; let mut qw=1.0f32;
        (self.ctx.scene_get_entity_rotation)(scene, entity_id, &mut qx, &mut qy, &mut qz, &mut qw);
        let et = if name.contains("Cube") {"Cube"} else if name.contains("Plane") {"Plane"} else if name.contains("Light") {"Light"} else {"Empty"};
        Ok(EntityInfo { id: SchemaU64(entity_id), name, position: [SchemaF32(x),SchemaF32(y),SchemaF32(z)], scale: [SchemaF32(sx),SchemaF32(sy),SchemaF32(sz)], rotation_y_degrees: SchemaF32(quaternion_to_yaw_degrees(qx,qy,qz,qw)), entity_type: et.to_string() })
    }

    fn get_project_info(&self) -> Result<ProjectInfo> {
        Ok(ProjectInfo {
            name: read_bool_c_string(|b,l| (self.ctx.project_get_name)(b,l), 256),
            path: read_bool_c_string(|b,l| (self.ctx.project_get_path)(b,l), 512),
            is_loaded: (self.ctx.project_is_loaded)(),
            entity_count: SchemaUsize((self.ctx.project_get_entity_count)()),
        })
    }

    fn list_asset_categories(&self) -> Result<Vec<AssetCategoryInfo>> {
        let count = (self.ctx.asset_library_get_category_count)();
        let mut result = Vec::new();
        for i in 0..count {
            result.push(AssetCategoryInfo {
                index: SchemaUsize(i),
                name: read_bool_c_string(|b,l| (self.ctx.asset_library_get_category_name)(i,b,l), 256),
                asset_count: SchemaUsize((self.ctx.asset_library_get_asset_count)(i)),
            });
        }
        Ok(result)
    }

    fn list_assets(&self, category_index: usize) -> Result<Vec<AssetInfo>> {
        let count = (self.ctx.asset_library_get_asset_count)(category_index);
        let mut result = Vec::new();
        for j in 0..count {
            let mut asset_id: u64 = 0; let mut asset_type: u32 = 0;
            let mut name_buf = vec![0u8; 256]; let mut desc_buf = vec![0u8; 256];
            (self.ctx.asset_library_get_asset_info)(category_index, j, &mut asset_id, name_buf.as_mut_ptr() as *mut c_char, 256, &mut asset_type, desc_buf.as_mut_ptr() as *mut c_char, 256);
            let ne = name_buf.iter().position(|&b|b==0).unwrap_or(256);
            let de = desc_buf.iter().position(|&b|b==0).unwrap_or(256);
            result.push(AssetInfo { id: SchemaU64(asset_id), name: String::from_utf8_lossy(&name_buf[..ne]).to_string(), asset_type: match asset_type { 0=>"EntityTemplate",1=>"Texture",2=>"Material",3=>"Script",_=>"Unknown" }.to_string(), description: String::from_utf8_lossy(&desc_buf[..de]).to_string() });
        }
        Ok(result)
    }

    fn set_entity_position(&self, entity_id: u64, position: [f32; 3]) -> Result<()> {
        (self.ctx.scene_set_entity_position)(self.scene_ptr, entity_id, position[0], position[1], position[2]);
        Ok(())
    }

    fn set_entity_scale(&self, entity_id: u64, scale: [f32; 3]) -> Result<()> {
        (self.ctx.scene_set_entity_scale)(self.scene_ptr, entity_id, scale[0], scale[1], scale[2]);
        Ok(())
    }

    fn rotate_entity(&self, entity_id: u64, angle_degrees: f32) -> Result<()> {
        (self.ctx.scene_rotate_entity)(self.scene_ptr, entity_id, angle_degrees);
        Ok(())
    }

    fn set_game_state(&self, state: u32) -> Result<()> {
        (self.ctx.scene_set_game_state)(self.scene_ptr, state as i32);
        Ok(())
    }

    fn project_save(&self) -> Result<()> {
        if !(self.ctx.project_save)() { bail!("Project save failed"); }
        Ok(())
    }

    fn capture_screenshot(&self, path: &str) -> Result<String> {
        let c_path = CString::new(path).map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;
        if (self.ctx.capture_screenshot_to_file)(c_path.as_ptr()) == 0 { bail!("Screenshot failed"); }
        Ok(path.to_string())
    }

    fn set_camera(&self, _yaw: f32, _pitch: f32, _distance: f32, _target: [f32; 3]) -> Result<()> {
        bail!("set_camera FFI only supports 5 params — needs FFI expansion for target.z")
    }

    fn create_entity(&self, entity_type: &str, name: Option<&str>, position: Option<[f32; 3]>) -> Result<CreateEntityResult> {
        let scene = self.scene_ptr;
        let entity_id = match entity_type {
            "Cube" => (self.ctx.scene_create_cube)(scene),
            "Plane" => (self.ctx.scene_create_plane)(scene),
            "CornellBox" => (self.ctx.scene_create_cornell_box)(scene),
            "Light" => (self.ctx.scene_create_directional_light)(scene),
            "Empty" => (self.ctx.scene_create_entity)(scene),
            _ => bail!("Unknown entity type: {}", entity_type),
        };
        if let Some(n) = name {
            let c_name = CString::new(n).map_err(|e| anyhow::anyhow!("Invalid name: {}", e))?;
            (self.ctx.scene_set_entity_name)(scene, entity_id, c_name.as_ptr());
        }
        if let Some(pos) = position {
            (self.ctx.scene_set_entity_position)(scene, entity_id, pos[0], pos[1], pos[2]);
        }
        Ok(CreateEntityResult { entity_id: SchemaU64(entity_id), name: name.map(String::from).unwrap_or_else(|| format!("Entity_{}", entity_id)), entity_type: entity_type.to_string() })
    }

    fn remove_entity(&self, entity_id: u64) -> Result<()> {
        (self.ctx.scene_remove_entity)(self.scene_ptr, entity_id);
        Ok(())
    }

    fn create_widget(&self, widget_type: &str, parent_id: u64, params: serde_json::Value) -> Result<CreateWidgetResult> {
        let wt = self.ctx.widget_tree_ptr;
        // parent_id=0表示挂到根节点
        let pid = if parent_id == 0 { (self.ctx.ui_get_root_id)(wt) } else { parent_id };

        // 辅助: 从params提取字段
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let width = params.get("width").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
        let height = params.get("height").and_then(|v| v.as_f64()).unwrap_or(30.0) as f32;
        let spacing = params.get("spacing").and_then(|v| v.as_f64()).unwrap_or(5.0) as f32;

        let c_text = CString::new(text).map_err(|e| anyhow::anyhow!("Invalid text: {}", e))?;
        let widget_id = match widget_type {
            // === 基础widget ===
            "Button" => (self.ctx.ui_create_button_in_parent)(wt, pid, width, height, c_text.as_ptr()),
            "Label" => (self.ctx.ui_create_label_in_parent)(wt, pid, width, height, c_text.as_ptr()),
            "Panel" => {
                // CreatePanelInParentFn: (wt, parent, w, h, r, g, b, a, border_r, border_g)
                // 10个参数 — border用br/bg两个颜色分量(简化)
                let r = params.get("r").and_then(|v| v.as_f64()).unwrap_or(0.15) as f32;
                let g = params.get("g").and_then(|v| v.as_f64()).unwrap_or(0.15) as f32;
                let b = params.get("b").and_then(|v| v.as_f64()).unwrap_or(0.15) as f32;
                let a = params.get("a").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                let br = params.get("br").and_then(|v| v.as_f64()).unwrap_or(0.2) as f32;
                let bg = params.get("bg").and_then(|v| v.as_f64()).unwrap_or(0.2) as f32;
                (self.ctx.ui_create_panel_in_parent)(wt, pid, width, height, r, g, b, a, br, bg)
            },
            "VStack" => (self.ctx.ui_create_vstack_in_parent)(wt, pid, spacing),
            "HStack" => (self.ctx.ui_create_hstack_in_parent)(wt, pid, spacing),
            "List" => {
                let direction = params.get("direction").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                (self.ctx.ui_create_list_in_parent)(wt, pid, spacing, direction)
            },

            // === 复合widget ===
            "TextEdit" => (self.ctx.ui_create_text_edit_in_parent)(wt, pid, width, height),
            "InputField" => (self.ctx.ui_create_input_field)(wt, pid, width, height),
            "Checkbox" => {
                let cw = params.get("width").and_then(|v| v.as_f64()).unwrap_or(20.0) as f32;
                let ch = params.get("height").and_then(|v| v.as_f64()).unwrap_or(20.0) as f32;
                let lw = params.get("label_width").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
                let lh = params.get("label_height").and_then(|v| v.as_f64()).unwrap_or(20.0) as f32;
                (self.ctx.ui_create_checkbox_in_parent)(wt, pid, cw, ch, lw, lh, c_text.as_ptr())
            },
            "Dropdown" => (self.ctx.ui_create_dropdown)(wt, pid, width, height),

            // === 新增widget ===
            "TabWidget" => (self.ctx.ui_create_tab_widget)(wt, pid, width, height, 0.0, 0.0),
            "TreeView" => (self.ctx.ui_create_tree_view)(wt, pid, width, height, 0.0, 0.0),
            "Slider" => {
                let min = params.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let max = params.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
                (self.ctx.ui_create_slider)(wt, pid, min, max)
            },
            "ScrollView" => (self.ctx.ui_create_scroll_view)(wt, pid, width, height, 0.0, 0.0),
            "SplitView" => {
                let direction = params.get("direction").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                (self.ctx.ui_create_split_view)(wt, pid, width, height, 0.0, 0.0, direction)
            },
            "Image" => (self.ctx.ui_create_image)(wt, pid, width, height, 0.0, 0.0),

            _ => bail!("Unknown widget type: {}", widget_type),
        };

        if widget_id == 0 { bail!("Failed to create widget '{}'", widget_type); }
        Ok(CreateWidgetResult { widget_id: SchemaU64(widget_id), widget_type: widget_type.to_string() })
    }

    fn run_script(&self, script_content: &str, entity_id: Option<u64>) -> Result<RunScriptResult> {
        // TODO: 需要以下新增才能实现:
        // 1. C#侧: EditorScript.ExecuteScript(string code, ulong entityId) — 编译+执行C#代码
        // 2. FfiContext新字段: execute_script: ExecuteScriptFn = extern "C" fn(*const c_char, u64) -> i32
        // 3. 线程安全: MCP在独立线程，Mono JIT在编辑器主线程 → 需queue机制
        // 当前返回not implemented
        bail!("run_script not implemented — requires C# ExecuteScript method + FfiContext field + thread-safe queue")
    }

    fn create_entity_from_template(&self, template_id: u64) -> Result<CreateEntityResult> {
        let entity_id = (self.ctx.asset_library_create_entity_from_template)(self.scene_ptr, template_id);
        if entity_id == 0 { bail!("Failed to create entity from template {}", template_id); }
        Ok(CreateEntityResult { entity_id: SchemaU64(entity_id), name: read_entity_name(&self.ctx, self.scene_ptr, entity_id), entity_type: "EntityTemplate".to_string() })
    }

    fn available_capabilities(&self) -> Vec<String> {
        vec![
            "get_scene_tree", "list_entities", "get_entity_info",
            "get_project_info", "list_asset_categories", "list_assets",
            "set_entity_position", "set_entity_scale", "rotate_entity",
            "set_game_state", "project_save", "capture_screenshot",
            "create_entity", "remove_entity", "create_widget",
            "create_entity_from_template",
        ].into_iter().map(String::from).collect()
    }
}