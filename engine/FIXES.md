# Bug Fixes Log

## 2026-10-18: Project Save/Load Round-Trip + Editor File Shortcuts

### Problems
1. `Project::sync_to_scene()` appended entities to the current scene instead of
   replacing it, so loading a project duplicated the existing scene.
2. Directional lights and empty entities were saved as `"Cube"` and restored as
   cube meshes. Light direction/color/intensity and `visible` were not persisted.
3. `Project::load()` only accepted a directory and always appended
   `project.json`; the editor passed a `.json` file path, so Open Scene/Project
   failed.
4. `Ctrl+S`/`Ctrl+O`/`Ctrl+N` were displayed in menus but not handled in `OnKey`.
   “Save As” created a directory named `project.project.json` because the editor
   passed a file path to `Project::create_new()`.
5. Tree refresh code iterated `_entityNodeMap.Keys` while removing entries,
   which could throw `InvalidOperationException` when loading/new-scene.
6. Right-click “Delete Entity” bypassed the confirmation dialog and left stale
   selection/property state.

### Fixes
- Added `World::clear()` + `Scene::clear()`; project load now replaces the scene.
- Added `ProjectLight`, `visible` and `light` fields with serde defaults for
  backward compatibility with old project files.
- `sync_from_scene()` writes `"DirectionalLight"` / `"None"` for non-mesh
  entities and persists light + visibility data.
- `sync_to_scene()` recreates lights/empty entities/renderables and resets the
  scene to `Editing`.
- `Project::load()` accepts either a project directory or a direct JSON file;
  `Project` now tracks the actual project file path, so Save works for both.
- Editor: Ctrl+S save, Ctrl+Shift+S save-as, Ctrl+O open project, Ctrl+N new
  scene. Save As now selects a parent directory and creates a project folder.
- Loading refreshes the entity tree, selection and property panel; Delete from
  the context menu now uses the same confirmation flow as the Delete key.
- Added `engine/core/tests/project_roundtrip.rs` covering entity kinds,
  transforms, visibility, lights, scripts, replacement-on-load, direct JSON
  loading and Save-As file paths.
- Updated `TestRunner.cs` expectations for the new `RotatingEntity` defaults
  (`Max=100`, `Initial=10`); startup self-test now reports `PASS=35, FAIL=0`.

### Verification
```bash
cd engine
cargo test -p hezhou-core --offline   # 2 project round-trip tests pass
cd scripts
powershell -ExecutionPolicy Bypass -File build_mono.ps1   # C# compile passes
```

---

## 2026-05-20: Outline Winding Order Fix

### Problem
- Outline not rendered when cube rotates from initial back face to front
- Initial state: back face is normal, has outline
- After 180° rotation: back face becomes front, but no outline visible

### Root Cause
- Back face defined with CCW winding from背面 perspective (indices: 0,2,1)
- After 180° Y-axis rotation, same vertices appear CW from正面 perspective
- Vulkan Y-flip: physical CW → screen CCW
- Pipeline `front_face: CLOCKWISE` + `cull_mode: BACK` → rotated back face incorrectly culled as "back face"

### Solution
- Changed outline pipeline `cull_mode` from `BACK` to `NONE`
- Outline only needs to render visible faces, no culling required
- Depth test still works correctly with `LESS_OR_EQUAL` compare

### Files Changed
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs:848` - `cull_mode: NONE`
- `engine/shaders/rotation.frag:alpha_threshold` - 0.01 (unchanged, already correct)

### Technical Details

**Winding Order Analysis:**
```
Back face vertices (from -Z): v0(-0.5,-0.5,-0.5), v1(+0.5,-0.5,-0.5), v2(+0.5,+0.5,-0.5), v3(-0.5,+0.5,-0.5)
Indices: 0,2,1 (triangle 1), 0,3,2 (triangle 2)

Physical CCW from背面 → Physical CW from正面 after 180° rotation
Vulkan Y-flip → Screen CCW → Identified as "back face" by CLOCKWISE front_face
Result: Culled when cull_mode=BACK
```

**Correct Behavior with cull_mode=NONE:**
- All visible faces rendered regardless of winding order
- Depth test ensures proper occlusion
- Alpha blend creates transparent overlay effect

### Verification
Run: `cd engine && cargo run --bin mono_editor_demo --features mono --release`
- All 6 faces should have orange outline at any rotation angle
- Outline alpha=0.3, renders on top of game geometry

---

## 2026-05-20: ESC Key Handling in Running Mode

### Problem
- ESC key in Running mode only canceled PreviewWindow selection
- Status bar showed "退出预览窗", but cube kept rotating
- Clicking "编辑" button could switch back, but ESC was broken

### Root Cause
- C# OnKey() handled ESC by only calling `UI.SetPreviewWindowSelected(false)`
- This canceled the PreviewWindow selection state
- But **renderer.game_state** stayed at Running(1)
- Cube rotation is controlled by `renderer.game_state`, not PreviewWindow selection

### Solution
- Running mode ESC now switches to Editing mode
- Calls `_gameScene.SetGameState(GameState.Editing)`
- Calls `UI.SetRendererGameState(0)`
- Calls `UI.SetPreviewWindowEditMode(true)` for orange border
- Updates button text to "运行"
- Restores saved camera position

### Files Changed
- `engine/scripts/EditorScript.cs:173-223` - OnKey() ESC handling logic

### Code Flow Comparison

**Before (broken):**
```
ESC pressed → SetPreviewWindowSelected(false) → PreviewWindow deselected
             → renderer.game_state still = 1 → Cube keeps rotating
```

**After (fixed):**
```
ESC pressed → SetGameState(Editing) → Scene state = Editing
             → SetRendererGameState(0) → renderer.game_state = 0
             → SetPreviewWindowEditMode(true) → Orange border
             → SetPreviewWindowSelected(false) → PreviewWindow deselected
             → Cube stops rotating ✅
```

### Verification
1. Click "运行" → cube starts rotating, PreviewWindow has blue border
2. Press ESC → cube stops, PreviewWindow has orange border, button shows "运行"
3. Press ESC again (Editing mode) → clears entity selection if any