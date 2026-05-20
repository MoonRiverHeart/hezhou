# Bug Fix Log

## 2026-05-20: Release Event Compiler Optimization Bug

### Problem
按键Release事件在Release模式下无法正确传递到C#：
- 按一下一直移动
- 按反向暂停
- 再按不生效

### Root Cause
Rust编译器在Release模式下对FFI参数传递做了激进优化：
- `key.action` 值被优化为常量
- `volatile_read` 和 `black_box` 单独使用都无法阻止优化
- 只有`log`强制使用才能阻止优化

### Evidence
```
# 无日志时 - 所有事件都是 pressed=True
OnKey: keycode=45 pressed=True selected=True  (Press)
OnKey: keycode=45 pressed=True selected=True  (Release - 错误!)

# 有日志时 - Release正确传递
GLFWKey: action=1 → InputHandler: keycode=45 action=0 → pressed=True
GLFWKey: action=0 → InputHandler: keycode=45 action=1 → pressed=False ✓
```

### Solution
在两个位置添加volatile read + log：
1. `ui_vulkan_renderer.rs`: GLFW action volatile read + log
2. `input_handler.rs`: KeyAction volatile read + log

```rust
// ui_vulkan_renderer.rs - WindowEvent::Key
let action_ptr = &action as *const Action as *const i32;
let action_raw = unsafe { std::ptr::read_volatile(action_ptr) };
dfx_info!("GLFWKey", "action={}", action_raw);  // log是关键

// input_handler.rs - on_key_event
let action_ptr = &key.action as *const KeyAction as *const u32;
let action_val = unsafe { std::ptr::read_volatile(action_ptr) };
dfx_info!("InputHandler", "keycode={} action={}", key.keycode as u32, action_val);
```

### Key Files Modified
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs:2593-2732`
- `engine/ui/src/platform/input_handler.rs:164-195`

---

## 2026-05-19: Vulkan Y-Flip Bug

### Problem
Game Pass渲染的立方体上下颠倒。

### Root Cause
Vulkan坐标系Y轴向下，而OpenGL/WebGL Y轴向上。
perspective矩阵需要翻转Y轴。

### Solution
```glsl
// rotation.vert - perspective matrix
mat4 perspective = mat4(
    f, 0, 0, 0,
    0, -f, 0, 0,  // Y轴翻转: -f (原来是 f)
    0, 0, (far+near)/(near-far), -1,
    0, 0, (2*far*near)/(near-far), 0
);
```

### Key Files Modified
- `engine/shaders/rotation.vert:47`

---

## 2026-05-19: Depth Buffer ACCESS_VIOLATION

### Problem
FXAA framebuffer创建时ACCESS_VIOLATION崩溃。

### Root Cause
game_render_pass需要2个attachment (color + depth)，但FXAA framebuffer只创建了1个。

### Solution
1. 创建专门的 `create_depth_image` 使用 `DEPTH_STENCIL_ATTACHMENT` usage
2. FXAA framebuffer也需要2个attachment

```rust
// ui_vulkan_renderer.rs
fn create_depth_image(&self, width: u32, height: u32) -> Result<ImageData, RhiError> {
    let image_create_info = vk::ImageCreateInfo::default()
        .format(vk::Format::D32_SFLOAT)
        .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT);
    // ...
}
```

### Key Files Modified
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs:1162-1210` (create_depth_image)
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs:558-640` (game_render_pass)
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs:3005-3184` (resize rebuild)

---

## 2026-05-19: View Transform Order Bug

### Problem
摄像机移动方向错误：按Left应该向左移动，但实际向右移动。

### Root Cause
View变换顺序错误：rotate → translate (错误)
正确顺序：translate → rotate

### Solution
```glsl
// rotation.vert
vec3 translated_pos = model_pos - camera_pos;
vec3 pitched_pos = vec3(...);  // pitch rotation
vec3 view_pos = vec3(...);      // yaw rotation
```

公式：`view_pos = rotation × (model_pos - camera_pos)` 而非 `rotation × model_pos - camera_pos`

### Key Files Modified
- `engine/shaders/rotation.vert:90-110`

---

## 2026-05-19: Normal Direction Bug (Back Face Culling)

### Problem
立方体侧面5个面消失，只有Top面可见。

### Root Cause
背面剔除配置：
- UI pipeline: NONE (无剔除)
- Game pipeline: BACK + CCW (剔除背面，顺时针为正面)

侧面5面的vertex_indices顺序错误（CCW，应该是CW）。

### Solution
修改侧面5面的vertex_indices为CW顺序：
- Back: 4,5,6,7 → vertex_indices保持CCW
- Front, Left, Right, Bottom: 改为CW顺序

```rust
// rotation_shader.rs
// Top face: CCW (正确)
// Back/Front/Left/Right/Bottom: CW (需要反转)
```

### Key Files Modified
- `engine/rhi-vulkan/src/rotation_shader.rs:vertex_indices`

---

## KeyCode Mapping Reference

### GLFW → Rust KeyCode (glfw_backend.rs)
```
GLFW Left (263) → KeyCode::Left (45)
GLFW Right (262) → KeyCode::Right (46)
GLFW Up (265) → KeyCode::Up (47)
GLFW Down (264) → KeyCode::Down (48)
GLFW Escape → KeyCode::Escape (39)
```

### GLFW Action → Rust KeyAction
```
GLFW Press (1) → KeyAction::Press (0)
GLFW Release (0) → KeyAction::Release (1)
GLFW Repeat (2) → KeyAction::Repeat (2)
```

注意：Press/Release值相反！GLFW Press=1，Rust Press=0。

---

## Lessons Learned

1. **FFI参数传递优化问题**
   - Release模式下编译器会激进优化
   - `volatile_read` + `log` 是可靠的解决方案
   - `black_box` 不够强

2. **Vulkan坐标系差异**
   - Vulkan Y轴向下，需要perspective矩阵翻转
   - 所有Y轴相关计算需验证方向

3. **RenderPass/Framebuffer一致性**
   - 所有framebuffer的attachment必须匹配renderpass
   - depth buffer需要专门的usage flags

4. **View变换顺序**
   - 先translate再rotate
   - 公式：`rotation × (pos - offset)` 而非 `rotation × pos - offset`

5. **背面剔除调试**
   - 使用NONE剔除模式先验证渲染
   - 然后逐步启用剔除，检查每个面的vertex顺序