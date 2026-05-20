# Bug Fixes Log

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