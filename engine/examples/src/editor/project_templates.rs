//! 0.4 项目模板与新建工程（task-49）。
//!
//! - 扫描 `engine/templates/*/template.json` 返回模板元数据；
//! - 从模板创建工程目录：按白名单复制文件、替换占位符
//!   （`{{PROJECT_NAME}}` / `{{VERSION}}` / `{{TEMPLATE_ID}}` / `{{ENGINE_VERSION}}`）；
//! - 校验模板 id、目标目录（不存在或空目录、禁止 `..` 越界）、项目名；
//! - 失败安全：创建失败清理半成品并写日志，不 panic。
//!
//! 限制：无版本迁移/依赖解析/包管理器；模板文件仅支持文本占位符替换与二进制原样复制。

use std::ffi::c_char;
use std::path::{Component, Path, PathBuf};

use hezhou_dfx::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 模板元数据文件。
const TEMPLATE_META_FILE: &str = "template.json";
/// 允许占位符替换的文本扩展名。
const TEXT_EXTENSIONS: [&str; 8] = ["json", "md", "txt", "cs", "scene", "prefab", "settings", "meta"];
/// 当前引擎版本（用于 `{{ENGINE_VERSION}}`）。
const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TemplateMeta {
    id: String,
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
    /// 白名单：相对模板根的文件列表；为空时递归复制（不含 template.json）。
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    min_engine_version: String,
}

/// 定位模板根目录：开发目录 `engine/templates` 优先，其次安装目录候选。
fn templates_root() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("templates"));
        candidates.push(cwd.join("..").join("templates"));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("templates"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("templates"));
            candidates.push(dir.join("..").join("templates"));
            candidates.push(dir.join("..").join("share").join("hezhou").join("templates"));
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_dir())
}

fn read_template_meta(template_dir: &Path) -> Result<TemplateMeta, String> {
    let meta_path = template_dir.join(TEMPLATE_META_FILE);
    let text = std::fs::read_to_string(&meta_path)
        .map_err(|error| format!("读取 {} 失败: {error}", meta_path.display()))?;
    let meta: TemplateMeta = serde_json::from_str(&text)
        .map_err(|error| format!("解析 {} 失败: {error}", meta_path.display()))?;
    if meta.id.trim().is_empty() || meta.name.trim().is_empty() {
        return Err(format!("模板元数据缺少 id/name: {}", meta_path.display()));
    }
    Ok(meta)
}

/// 扫描模板列表（按 id 排序）。
fn scan_templates() -> Result<Vec<TemplateMeta>, String> {
    let root = templates_root().ok_or_else(|| "未找到 templates 目录".to_string())?;
    let mut templates = Vec::new();
    let entries = std::fs::read_dir(&root)
        .map_err(|error| format!("读取模板目录 {} 失败: {error}", root.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match read_template_meta(&path) {
            Ok(meta) => templates.push(meta),
            Err(error) => dfx_warn!("Template", "跳过无效模板: {}", error),
        }
    }
    templates.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(templates)
}

fn is_valid_template_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

/// 项目名校验：非空、≤64、无路径/非法字符、非 `.`/`..`。
fn sanitize_project_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("项目名不能为空".to_string());
    }
    if trimmed.chars().count() > 64 {
        return Err("项目名不能超过 64 个字符".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("项目名非法".to_string());
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
    {
        return Err("项目名包含非法字符（/ \\ : * ? \" < > |）".to_string());
    }
    Ok(trimmed.to_string())
}

/// 相对路径安全校验（禁止绝对路径/`..` 越界）。
fn is_safe_relative_path(path: &str) -> bool {
    let candidate = Path::new(path);
    if candidate.is_absolute() || path.trim().is_empty() {
        return false;
    }
    candidate.components().all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

/// 目标目录解析：相对路径基于当前目录；拒绝 `..` 组件。
fn resolve_destination(dest: &str) -> Result<PathBuf, String> {
    let trimmed = dest.trim();
    if trimmed.is_empty() {
        return Err("目标目录不能为空".to_string());
    }
    let raw = PathBuf::from(trimmed);
    if raw.components().any(|component| component == Component::ParentDir) {
        return Err("目标目录不能包含 ..（禁止越界）".to_string());
    }
    if raw.is_absolute() {
        Ok(raw)
    } else {
        let cwd = std::env::current_dir().map_err(|error| format!("获取当前目录失败: {error}"))?;
        Ok(cwd.join(raw))
    }
}

fn is_text_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| TEXT_EXTENSIONS.iter().any(|allowed| ext.eq_ignore_ascii_case(allowed)))
        .unwrap_or(false)
}

fn render_placeholders(text: &str, project_name: &str, template: &TemplateMeta) -> String {
    text.replace("{{PROJECT_NAME}}", project_name)
        .replace("{{VERSION}}", if template.version.is_empty() { "0.4.0" } else { template.version.as_str() })
        .replace("{{TEMPLATE_ID}}", &template.id)
        .replace("{{ENGINE_VERSION}}", ENGINE_VERSION)
}

/// 收集需要复制的相对路径（白名单优先；否则递归扫描，排除 template.json）。
fn collect_template_files(template_dir: &Path, template: &TemplateMeta) -> Result<Vec<String>, String> {
    if !template.files.is_empty() {
        let mut files = Vec::new();
        for relative in &template.files {
            if !is_safe_relative_path(relative) {
                return Err(format!("模板文件路径非法: {relative}"));
            }
            if !template_dir.join(relative).is_file() {
                return Err(format!("模板文件缺失: {relative}"));
            }
            files.push(relative.clone());
        }
        return Ok(files);
    }

    let mut files = Vec::new();
    collect_files_recursive(template_dir, template_dir, &mut files).map_err(|error| {
        format!("扫描模板文件失败（{}）: {error}", template_dir.display())
    })?;
    files.sort();
    Ok(files)
}

fn collect_files_recursive(
    root: &Path,
    current: &Path,
    out: &mut Vec<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(root, &path, out)?;
        } else if path.is_file() {
            if let Ok(relative) = path.strip_prefix(root) {
                let relative = relative.to_string_lossy().replace('\\', "/");
                if relative != TEMPLATE_META_FILE {
                    out.push(relative);
                }
            }
        }
    }
    Ok(())
}

/// 从模板创建工程目录；返回创建的目录路径。
fn create_project_from_template(
    template_id: &str,
    dest: &str,
    project_name: &str,
) -> Result<PathBuf, String> {
    let template_id = template_id.trim();
    if !is_valid_template_id(template_id) {
        return Err(format!("模板 id 非法: {template_id}"));
    }
    let project_name = sanitize_project_name(project_name)?;
    let templates = scan_templates()?;
    let template = templates
        .iter()
        .find(|template| template.id == template_id)
        .ok_or_else(|| format!("模板不存在: {template_id}（可用: {}）", templates.iter().map(|t| t.id.as_str()).collect::<Vec<_>>().join(", ")))?
        .clone();

    let root = templates_root().ok_or_else(|| "未找到 templates 目录".to_string())?;
    let template_dir = root.join(&template.id);
    if !template_dir.is_dir() {
        return Err(format!("模板目录缺失: {}", template_dir.display()));
    }
    let files = collect_template_files(&template_dir, &template)?;
    let destination = resolve_destination(dest)?;

    if destination.exists() {
        if !destination.is_dir() {
            return Err(format!("目标路径已存在且不是目录: {}", destination.display()));
        }
        let mut entries = std::fs::read_dir(&destination)
            .map_err(|error| format!("读取目标目录失败（{}）: {error}", destination.display()))?;
        if entries.next().is_some() {
            return Err(format!("目标目录已存在且非空: {}", destination.display()));
        }
    } else {
        std::fs::create_dir_all(&destination).map_err(|error| {
            format!("创建目标目录失败（{}）: {error}", destination.display())
        })?;
    }

    let result = (|| -> Result<(), String> {
        for relative in &files {
            let source = template_dir.join(relative);
            let target = destination.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    format!("创建目录失败（{}）: {error}", parent.display())
                })?;
            }
            if is_text_file(&source) {
                let text = std::fs::read_to_string(&source).map_err(|error| {
                    format!("读取模板文本失败（{}）: {error}", source.display())
                })?;
                let rendered = render_placeholders(&text, &project_name, &template);
                if source.extension().and_then(|ext| ext.to_str()).map(|ext| ext.eq_ignore_ascii_case("json")).unwrap_or(false) {
                    serde_json::from_str::<serde_json::Value>(&rendered).map_err(|error| {
                        format!("模板 JSON 无效（{}）: {error}", relative)
                    })?;
                }
                std::fs::write(&target, rendered).map_err(|error| {
                    format!("写入失败（{}）: {error}", target.display())
                })?;
            } else {
                std::fs::copy(&source, &target).map_err(|error| {
                    format!("复制失败（{} -> {}）: {error}", source.display(), target.display())
                })?;
            }
        }
        Ok(())
    })();

    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(&destination);
        return Err(error);
    }

    dfx_info!(
        "Template",
        "从模板 {} 创建工程: {}（{} 个文件）",
        template.id,
        destination.display(),
        files.len()
    );
    Ok(destination)
}

fn write_c_string(buffer: *mut c_char, buffer_size: usize, text: &str) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(buffer_size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_len);
        *buffer.add(copy_len) = 0;
    }
    copy_len
}

// === FFI ===

/// FFI：列出可用项目模板（JSON：`{"count":N,"templates":[...]}`）。
#[unsafe(no_mangle)]
pub extern "C" fn template_list_json(buffer: *mut c_char, buffer_size: usize) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let templates = match scan_templates() {
        Ok(templates) => templates,
        Err(error) => {
            dfx_warn!("Template", "模板扫描失败: {}", error);
            Vec::new()
        }
    };
    let items: Vec<serde_json::Value> = templates
        .iter()
        .map(|template| {
            json!({
                "id": template.id,
                "name": template.name,
                "version": template.version,
                "description": template.description,
                "min_engine_version": template.min_engine_version,
                "file_count": template.files.len(),
            })
        })
        .collect();
    let value = json!({ "count": items.len(), "templates": items });
    write_c_string(buffer, buffer_size, &value.to_string())
}

/// FFI：从模板创建工程。
///
/// 返回 0=成功；-1=参数/模板无效；-2=目标目录已存在且非空/不是目录；-3=IO 失败。
#[unsafe(no_mangle)]
pub extern "C" fn template_create(
    template_id: *const c_char,
    dest_dir: *const c_char,
    project_name: *const c_char,
) -> i32 {
    if template_id.is_null() || dest_dir.is_null() || project_name.is_null() {
        return -1;
    }
    let template_id = unsafe { std::ffi::CStr::from_ptr(template_id).to_string_lossy().into_owned() };
    let dest_dir = unsafe { std::ffi::CStr::from_ptr(dest_dir).to_string_lossy().into_owned() };
    let project_name =
        unsafe { std::ffi::CStr::from_ptr(project_name).to_string_lossy().into_owned() };

    match create_project_from_template(&template_id, &dest_dir, &project_name) {
        Ok(path) => {
            dfx_info!("Template", "工程创建成功: {}", path.display());
            0
        }
        Err(error) => {
            let target_exists = std::path::Path::new(dest_dir.trim()).exists()
                && std::fs::read_dir(dest_dir.trim())
                    .map(|mut entries| entries.next().is_some())
                    .unwrap_or(false);
            dfx_warn!("Template", "创建工程失败: {}", error);
            if target_exists {
                -2
            } else if error.contains("目标目录") || error.contains("已存在") {
                -2
            } else if error.contains("模板 id") || error.contains("模板不存在") || error.contains("项目名") || error.contains("非法") {
                -1
            } else {
                -3
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_target(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("target")
            .join("template_tmp")
            .join(format!("{tag}_{}_{}", std::process::id(), nanos))
    }

    #[test]
    fn lists_bundled_templates() {
        let templates = scan_templates().expect("模板目录应存在");
        let ids: Vec<&str> = templates.iter().map(|template| template.id.as_str()).collect();
        assert!(ids.contains(&"empty-3d"), "ids={ids:?}");
        assert!(ids.contains(&"action-sandbox"), "ids={ids:?}");
        for template in &templates {
            assert!(!template.name.is_empty());
            assert!(!template.files.is_empty(), "{} 应有文件白名单", template.id);
        }
    }

    #[test]
    fn creates_empty_project_and_replaces_placeholders() {
        let target = unique_target("empty");
        let created = create_project_from_template("empty-3d", target.to_str().unwrap(), "My Test Game")
            .expect("创建 empty-3d 应成功");
        assert_eq!(created, target);
        let project = std::fs::read_to_string(target.join("project.json")).unwrap();
        assert!(project.contains("\"My Test Game\""), "{project}");
        assert!(!project.contains("{{PROJECT_NAME}}"));
        let settings = std::fs::read_to_string(target.join("build.settings.json")).unwrap();
        assert!(settings.contains("\"My Test Game\""));
        assert!(!settings.contains("{{VERSION}}"));
        assert!(target.join("assets").join("scenes").join("main.scene").is_file());
        assert!(target.join("scripts").join("README.md").is_file());
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn creates_action_sandbox_with_entities_and_script() {
        let target = unique_target("action");
        create_project_from_template("action-sandbox", target.to_str().unwrap(), "Action Demo")
            .expect("创建 action-sandbox 应成功");
        let project = std::fs::read_to_string(target.join("project.json")).unwrap();
        assert!(project.contains("\"Cube\""));
        assert!(project.contains("\"DirectionalLight\""));
        assert!(project.contains("pbr_material"));
        assert!(target.join("scripts").join("Spin.cs").is_file());
        let scene = std::fs::read_to_string(target.join("assets").join("scenes").join("main.scene")).unwrap();
        assert!(scene.contains("Action Demo"));
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn rejects_existing_non_empty_target_and_invalid_names() {
        let target = unique_target("reject");
        create_project_from_template("empty-3d", target.to_str().unwrap(), "First")
            .expect("首次创建应成功");
        let error = create_project_from_template("empty-3d", target.to_str().unwrap(), "Second")
            .expect_err("非空目标应被拒绝");
        assert!(error.contains("非空"), "{error}");

        let error = sanitize_project_name("../evil").expect_err("路径分隔符应被拒绝");
        assert!(error.contains("非法"), "{error}");
        let error = sanitize_project_name("").expect_err("空名应被拒绝");
        assert!(!error.is_empty());
        assert!(create_project_from_template("no-such-template", "target/tmp/x", "X").is_err());
        assert!(!is_safe_relative_path("../escape.json"));
        assert!(is_safe_relative_path("assets/scenes/main.scene"));
        let _ = std::fs::remove_dir_all(&target);
    }
}

