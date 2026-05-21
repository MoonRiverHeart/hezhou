use hezhou_scripting::ffi_context::WidgetTreeHandle;
use hezhou_ui::ffi as ui_ffi;
use hezhou_dfx::*;
use std::ffi::CString;

fn save_scene_bindings() -> Vec<super::SavedEntityBinding> {
    unsafe {
        if let Some(scene_ptr) = super::SCENE {
            let scene = &*scene_ptr;
            let mut saved = Vec::new();
            
            for entity in &scene.root_entities {
                let bindings = scene.entity_bindings.get(&entity.id).cloned().unwrap_or_default();
                if !bindings.is_empty() {
                    let saved_bindings: Vec<super::SavedScriptBinding> = bindings.iter().map(|b| super::SavedScriptBinding {
                        script_path: b.script_path.clone(),
                        class_name: b.class_name.clone(),
                        enabled: b.enabled,
                    }).collect();
                    saved.push(super::SavedEntityBinding {
                        entity_id: entity.id,
                        bindings: saved_bindings,
                    });
                }
            }
            
            dfx_info!("HotReload", "保存绑定数据: {} entities, {} bindings", saved.len(), saved.iter().map(|e| e.bindings.len()).sum::<usize>());
            saved
        } else {
            Vec::new()
        }
    }
}

fn restore_scene_bindings(saved: &[super::SavedEntityBinding]) {
    unsafe {
        if let Some(scene_ptr) = super::SCENE {
            let scene = &mut *scene_ptr;
            
            for saved_entity in saved {
                let entity = hezhou_core::Entity::new(saved_entity.entity_id);
                for binding in &saved_entity.bindings {
                    scene.attach_script_binding(entity, binding.script_path.clone(), binding.class_name.clone());
                    if !binding.enabled {
                        let count = scene.get_script_binding_count(entity);
                        if count > 0 {
                            scene.set_script_binding_enabled(entity, count - 1, false);
                        }
                    }
                }
            }
            
            dfx_info!("HotReload", "恢复绑定数据: {} entities", saved.len());
        }
    }
}

pub fn compile_editor_script() {
    use std::process::Command;
    
    if !std::path::Path::new("C:\\Program Files\\Mono\\bin\\mcs.bat").exists() {
        dfx_info!("Demo", "mcs.bat not found - standalone release mode, skipping compilation");
        return;
    }
    
    let result = Command::new("C:\\Program Files\\Mono\\bin\\mcs.bat")
        .args([
            "-target:library",
            "-out:scripts/bin/Mono/EditorScript.dll",
            "scripts/EditorScript.cs",
            "scripts/EditorScript.State.cs",
            "scripts/EditorScript.View.cs",
            "scripts/EditorScript.Presenter.cs",
            "scripts/UI.cs",
            "scripts/UI.Widgets.cs",
            "scripts/UI.ComplexWidgets.cs",
            "scripts/UI.NewWidgets.cs",
            "scripts/UI.Scene.cs",
            "scripts/UI.AssetProject.cs",
            "scripts/DFX.cs",
        ])
        .output();
    
    match result {
        Ok(output) => {
            if output.status.success() {
                dfx_info!("Demo", "EditorScript.dll编译成功");
            } else {
                dfx_error!("Demo", "编译失败: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            dfx_error!("Demo", "mcs not found: {:?}", e);
        }
    }
}

fn recompile_editor_script() -> bool {
    use std::process::Command;
    
    if !std::path::Path::new("C:\\Program Files\\Mono\\bin\\mcs.bat").exists() {
        dfx_info!("HotReload", "mcs.bat not found - assuming precompiled DLL");
        return true;
    }
    
    dfx_info!("HotReload", "执行mcs编译...");
    
    let result = Command::new("C:\\Program Files\\Mono\\bin\\mcs.bat")
        .args([
            "-target:library",
            "-out:scripts/bin/Mono/EditorScript.dll",
            "scripts/EditorScript.cs",
            "scripts/EditorScript.State.cs",
            "scripts/EditorScript.View.cs",
            "scripts/EditorScript.Presenter.cs",
            "scripts/UI.cs",
            "scripts/UI.Widgets.cs",
            "scripts/UI.ComplexWidgets.cs",
            "scripts/UI.NewWidgets.cs",
            "scripts/UI.Scene.cs",
            "scripts/UI.AssetProject.cs",
            "scripts/DFX.cs",
        ])
        .output();
    
    match result {
        Ok(output) => {
            if output.status.success() {
                dfx_info!("HotReload", "✓ 编译成功");
                true
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                dfx_error!("HotReload", "✗ 编译失败:\n{}", stderr);
                false
            }
        }
        Err(e) => {
            dfx_error!("HotReload", "✗ mcs执行失败: {:?}", e);
            false
        }
    }
}

pub fn handle_hot_reload(widget_tree_handle: WidgetTreeHandle) -> bool {
    dfx_info!("HotReload", "触发热更新...");
    dfx_trace_begin!("HotReload", "reload");
    
    let skip_frame = unsafe {
        if let Some(ref mut executor) = super::EXECUTOR {
            let ffi_ptr_val = super::FFI_PTR.unwrap_or(std::ptr::null());
            
            if let Some(callback) = super::STATUS_TEXT_CALLBACK {
                let status_cstr = CString::new("正在热更新脚本...").unwrap();
                callback(status_cstr.as_ptr());
            }
            
            dfx_info!("HotReload", "[1] 保存Entity-Script绑定数据...");
            let saved_bindings = save_scene_bindings();
            super::SAVED_BINDINGS = saved_bindings;
            
            dfx_info!("HotReload", "[2] 清理旧的UI widgets...");
            ui_ffi::ui_clear_widget_tree(widget_tree_handle as ui_ffi::WidgetTreeHandle);
            
            dfx_info!("HotReload", "[3] 卸载当前assembly...");
            executor.shutdown();
            
            dfx_info!("HotReload", "[4] 重新编译C#脚本...");
            let compile_result = recompile_editor_script();
            
            if !compile_result {
                if let Some(callback) = super::STATUS_TEXT_CALLBACK {
                    let status_cstr = CString::new("热更新失败: 编译错误").unwrap();
                    callback(status_cstr.as_ptr());
                }
                dfx_error!("HotReload", "编译失败!");
                
                dfx_info!("HotReload", "尝试恢复旧assembly...");
                executor.reload().ok();
                executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr_val as usize).ok();
                
                dfx_trace_end!("HotReload", "reload");
                true
            } else {
                dfx_info!("HotReload", "[5] 加载新assembly...");
                match executor.reload() {
                    Ok(_) => {
                        dfx_info!("HotReload", "Assembly reload成功!");
                        
                        dfx_info!("HotReload", "[6] 调用Initialize重建UI...");
                        executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr_val as usize)
                            .expect("Initialize failed");
                        
                        dfx_info!("HotReload", "[7] 恢复Entity-Script绑定数据...");
                        restore_scene_bindings(&super::SAVED_BINDINGS);
                        
                        dfx_info!("HotReload", "[8] 调用OnHotReloadComplete回调...");
                        if let Some(callback) = super::HOT_RELOAD_COMPLETE_CALLBACK {
                            callback();
                        }
                        
                        if let Some(callback) = super::STATUS_TEXT_CALLBACK {
                            let status_cstr = CString::new("热更新完成").unwrap();
                            callback(status_cstr.as_ptr());
                        }
                        
                        dfx_info!("HotReload", "UI重新初始化完成!");
                    }
                    Err(e) => {
                        if let Some(callback) = super::STATUS_TEXT_CALLBACK {
                            let status_cstr = CString::new(format!("热更新失败: {:?}", e).as_str()).unwrap();
                            callback(status_cstr.as_ptr());
                        }
                        dfx_error!("HotReload", "Reload失败: {:?}", e);
                    }
                }
                dfx_trace_end!("HotReload", "reload");
                false
            }
        } else {
            dfx_trace_end!("HotReload", "reload");
            false
        }
    };
    
    skip_frame
}