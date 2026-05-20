# AI Agent Instructions

## Build Commands

### Rust
```bash
cd engine
cargo build
cargo test
cargo run --bin mono_rotation_demo --features mono      # Mono JIT demo (console)
cargo run --bin mono_triangle_demo --features mono      # Mono JIT + Vulkan triangle
cargo run --bin mono_hot_reload_test --features mono    # Mono hot reload test
cargo run --bin mono_ui_thunk_demo --features mono      # Mono UI + Thunk callbacks
cargo run --bin mono_editor_demo --features mono        # Game Editor (1280x720)
cargo run --bin rotation_demo --features native-aot    # NativeAOT demo
```

### C# Scripts

#### Mono Version (Development - Hot Reload)
```bash
cd engine/scripts
powershell -ExecutionPolicy Bypass -File build_mono.ps1
# OR
build_mono.bat
```

#### NativeAOT Version (Release - High Performance)
```bash
cd engine/scripts
dotnet build RotationScript.NativeAOT.csproj -c Release
```

## Demo Programs

### mono_triangle_demo
- Vulkan rendering with rotating triangle
- Mono JIT calls C# `UpdateRotation(deltaTime)` to calculate rotation
- Press **R** key to trigger hot reload:
  1. Recompiles C# script using `mcs`
  2. Reloads Mono assembly
  3. New rotation speed takes effect immediately

### mono_rotation_demo
- Console-based demo without rendering
- Tests Mono JIT script execution
- Press Enter every 180 frames to trigger hot reload

### mono_hot_reload_test
- Automated test for hot reload functionality
- Modifies C# source, recompiles, reloads, verifies changes

### mono_ui_thunk_demo
- UI system with VStack/HStack layout containers
- C# creates widgets, Rust calculates layout
- Button click callback via Thunk mechanism
- Window resize triggers layout update

### mono_editor_demo
- Game Editor with professional layout
- Top toolbar (40px): 新建/打开/保存/运行 buttons
- Left panel (250px): Project structure tree
- Bottom-left (200px): Asset management
- Center: Game preview area
- Right panel (250px): Property editor
- Bottom status bar (30px): FPS display
- Window size: 1280x720
- Resize: Auto layout recalculation

## Important Notes

### Mono vs NativeAOT
- **Mono JIT**: Development mode, supports hot reload (Assembly unload/reload)
- **NativeAOT**: Release mode, high performance (function pointer calls)
- **Single C# source**: RotationScript.cs uses `#if MONO` / `#if NATIVEAOT`

### Mono/.NET 8 Incompatibility
- .NET 8 compiled DLLs have 0 methods when loaded by Mono
- **Solution**: Use Mono compiler `mcs` for Mono version
- Mono SDK must be installed: https://www.mono-project.com/download/stable/

### wrapped_mono Method Lookup Issue
- `mono_class_get_method_from_name` returns null for valid methods
- **Solution**: Iterate methods with `mono_class_get_methods`, match by name, then use `mono_runtime_invoke`

### DLL Paths
- Mono DLL: `engine/scripts/bin/Mono/Release/net8.0/RotationScript.Mono.dll`
- NativeAOT DLL: `engine/scripts/bin/NativeAOT/Release/net8.0/RotationScript.NativeAOT.dll`

### Hot Reload Flow
1. User modifies `InitializeDefaults()` in RotationScript.cs to set new `_rotationSpeed`
2. Press **R** key in the demo window
3. Rust calls `build_mono.ps1` to recompile C# with `mcs`
4. Rust calls `executor.reload()` to unload old assembly and load new one
5. Rust calls `ResetAll()` which triggers `InitializeDefaults()` with new speed value
6. New rotation speed takes effect immediately

### Example Hot Reload Test
```bash
# 1. Run the demo
cd engine/examples
cargo run --bin mono_triangle_demo --features mono

# 2. While demo is running, edit RotationScript.cs:
#    Change: _rotationSpeed = 90.0f;
#    To:     _rotationSpeed = 180.0f;

# 3. Press R in the demo window

# 4. Observe:
#    [HotReload] Calling ResetAll to reinit static vars...
#    [C# Static Constructor] Initialized: speed=180°/s
#    [HotReload] Speed after reset: 180°/s
#    Frame 120: angle=..., speed=180°/s
```

## Testing

```bash
# Mono JIT + Vulkan triangle (with hot reload)
cd engine/examples
cargo run --bin mono_triangle_demo --features mono

# Mono JIT console demo
cd engine/examples
cargo run --bin mono_rotation_demo --features mono

# Mono hot reload automated test
cd engine/examples
cargo run --bin mono_hot_reload_test --features mono

# Mono UI + Thunk callbacks
cd engine/examples
cargo run --bin mono_ui_thunk_demo --features mono

# Game Editor (1280x720)
cd engine/examples
cargo run --bin mono_editor_demo --features mono --release

# NativeAOT version
cd engine/examples
cargo run --bin rotation_demo --features native-aot
```

## TextEdit Widget

### Coordinate System
- **Window coordinates** → `event_dispatcher` converts to **Widget relative coordinates**
- TextEdit receives widget-relative coordinates from events

### Text Rendering vs Cursor Positioning
- `font_atlas.rs`: `layout_text_left` returns `char_y = baseline_y - info.bearing_y` (per-character bearing)
- `canvas.rs`: `layout_text_for_cursor_with_wrap` returns unified `baseline_y` for all characters in a line
- `max_bearing_y`: Maximum bearing_y across all characters in text, used for unified baseline

### Key Formula
- `baseline_y = container_y + font_ascent`
- `cursor_draw_y = baseline_y - font_ascent` (matches text render start y)
- `line_height = ascent - descent + line_gap` (from fontdue horizontal_line_metrics)

### Click Position Calculation
- Find closest line by comparing `click_y` against `cursor_draw_y` (not baseline_y)
- Line center y = `(baseline_y - max_bearing_y + font_size) / 2`
- Then find closest grapheme by x-coordinate within that line

### Cursor/Click position offset: Use `baseline_y - font_ascent` for comparison
3. **Selection highlight offset**: Highlight rect y should be `baseline_y - max_bearing_y`
4. **Ctrl+C/V not working**: GLFW `mods` parameter must be parsed and passed to `KeyModifiers`

### Mouse Selection
- `TouchBegin`: Set selection_start = current cursor position
- `TouchMove`: Update selection_end and cursor position
- `TouchEnd`: Finalize selection

### Keyboard Modifiers
- GLFW backend: Parse `glfw::Modifiers` to set `KeyModifiers.shift/ctrl/alt`
- UIInputHandler: Track Shift/Ctrl state via separate KeyEvents
- TouchData modifiers: bit 0 = Shift, bit 1 = Ctrl, bit 2 = Alt

## Vulkan Rendering

### Dynamic Viewport/Scissor
- **Problem**: Pipeline viewport stuck at initial size (512x512), ignoring `cmd_set_viewport()`
- **Symptom**: Rendered geometry aspect ratio wrong (compressed/stretched)
- **Root Cause**: Pipeline created without `p_dynamic_state` for VIEWPORT/SCISSOR
- **Solution**: Add `vk::PipelineDynamicStateCreateInfo` with VIEWPORT and SCISSOR states
- **Example**:
  ```rust
  p_dynamic_state: &vk::PipelineDynamicStateCreateInfo {
      dynamic_state_count: 2,
      p_dynamic_states: &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR] as *const _,
      ..Default::default()
  },
  ```
- **When needed**: Any pipeline that needs runtime viewport resize (e.g., game preview matching window size)

### Push Constant Layout
- Must match shader exactly: size and field order
- Common mistake: Rust array size mismatch with shader `layout(push_constant)` struct
- **Debug tip**: Hardcode aspect ratio in shader to isolate push constant vs viewport issue

### Outline Rendering
- **Technique**: Render back faces (CULL_FRONT) with scale > 1.0, then render front faces normally
- **Depth**: Outline should use `depth_write_enable: FALSE` to not block normal geometry
- **Blend**: Outline uses alpha blend (SRC_ALPHA, ONE_MINUS_SRC_ALPHA)

### Cube Rendering in rotation.vert

**Shader Path**: `engine/shaders/rotation.vert`

**8 Vertex Positions** (corners of unit cube centered at origin):
```
v0: (-0.5, -0.5, -0.5)  back-bottom-left   (左后下)
v1: (+0.5, -0.5, -0.5)  back-bottom-right  (右后下)
v2: (+0.5, +0.5, -0.5)  back-top-right     (右后上)
v3: (-0.5, +0.5, -0.5)  back-top-left      (左后上)
v4: (-0.5, -0.5, +0.5)  front-bottom-left  (左前下)
v5: (+0.5, -0.5, +0.5)  front-bottom-right (右前下)
v6: (+0.5, +0.5, +0.5)  front-top-right    (右前上)
v7: (-0.5, +0.5, +0.5)  front-top-left     (左前上)
```

**Face Colors**:
```
Back   (Z-, z=-0.5): RGB(1.0, 0.2, 0.2) 红色
Front  (Z+, z=+0.5): RGB(0.2, 1.0, 0.2) 绿色
Left   (X-, x=-0.5): RGB(0.2, 0.2, 1.0) 蓝色
Right  (X+, x=+0.5): RGB(1.0, 1.0, 0.2) 黄色
Bottom (Y-, y=-0.5): RGB(0.2, 1.0, 1.0) 青色
Top    (Y+, y=+0.5): RGB(1.0, 0.2, 1.0) 紫色
```

**Vertex Indices** (36 indices for 6 faces × 2 triangles × 3 vertices):
```
Back:   0, 2, 1, 0, 3, 2   (v0→v2→v1→v3, normal to -Z)
Front:  4, 7, 6, 4, 6, 5   (v4→v7→v6→v5, normal to +Z)
Left:   0, 7, 3, 0, 4, 7   (v0→v7→v3→v4, normal to -X)
Right:  1, 2, 6, 1, 6, 5   (v1→v2→v6→v5, normal to +X)
Bottom: 0, 1, 5, 0, 5, 4   (v0→v1→v5→v4, normal to -Y)
Top:    3, 7, 6, 3, 6, 2   (v3→v7→v6→v2, normal to +Y)
```

**Winding Order Note**:
- Vulkan perspective projection flips Y-axis: `mat4[1][1] = -f`
- Physical CCW (counter-clockwise) → Screen CW (clockwise) after Y-flip
- Pipeline `front_face: CLOCKWISE` means physical CCW triangles are front faces

### Depth Testing

**Problem**: Without depth testing, render order determines visibility (later triangles overwrite earlier ones regardless of spatial distance).

**Solution**: Add `p_depth_stencil_state` to game_pipeline:
```rust
p_depth_stencil_state: &vk::PipelineDepthStencilStateCreateInfo {
    depth_test_enable: vk::TRUE,       // Enable depth comparison
    depth_write_enable: vk::TRUE,      // Write depth values to buffer
    depth_compare_op: vk::CompareOp::LESS,  // Near objects occlude far objects
    depth_bounds_test_enable: vk::FALSE,
    stencil_test_enable: vk::FALSE,
    ..Default::default()
},
```

**File**: `engine/rhi-vulkan/src/ui_vulkan_renderer.rs:757-768`

**Render Pass**: game_render_pass has depth attachment (D32_SFLOAT format) at attachment index 1

## DFX Logging System

### Usage

**Rust:**
```rust
use hezhou_dfx::*;

fn main() {
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Info);
    
    // Enable file output
    let log_path = format!("logs/hezhou_{}.log", chrono::Local::now().format("%Y-%m-%d"));
    std::fs::create_dir_all("logs").ok();
    dfx.lock().get_logger().lock().enable_file_output(&log_path).ok();
    
    dfx_info!("Module", "Message");
    dfx_error!("Module", "Error: {}", error);
}

// Trace points
dfx_trace_begin!("FunctionName", "category");
dfx_trace_end!("FunctionName", "category");
```

**C#:**
```csharp
// When dfx_handle is null, falls back to Console.WriteLine
Log.Info("Module", "Message");
Log.Error("Module", "Error message");
```

### Log Levels
- Trace (0): Detailed debugging
- Debug (1): Debug information
- Info (2): General information (default)
- Warn (3): Warning messages
- Error (4): Error messages
- Fatal (5): Critical errors

### Output Format
- Console: `[2026-05-19 09:35:16.928][INFO][Demo] Message`
- File: `[2026-05-19 09:35:16.928][INFO][T:8076][Demo] Message`

### Run Location
- Run from `engine/` directory for correct log file path
- Log files: `engine/logs/hezhou_YYYY-MM-DD.log`