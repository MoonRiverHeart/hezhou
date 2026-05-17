# UI Button Click Callback Implementation

## Goal
实现完整的UI功能：
- Button点击回调（点击→文字变为"hello")
- 正确的布局顺序（第一排Label，第二排Button）
- 空格正确显示
- 窗口最大化时文字和控件不模糊

## Coordinate System Solution

### Vulkan Y轴翻转
Shader统一处理Y翻转：
```glsl
// ui.vert
gl_Position = vec4(clip_pos.x, -clip_pos.y, 0.0, 1.0);
```

### 渲染坐标 vs 逻辑坐标
Shader Y翻转导致：
- 逻辑y小（上方） → 渲染到屏幕下方
- 逻辑y大（下方） → 渲染到屏幕上方

### Hit Test坐标翻转
点击坐标翻转Y以匹配逻辑bounds：
```rust
// input_handler.rs
pub fn on_touch_event(&mut self, touch: &TouchEvent, timestamp: u64) {
    let x = touch.x;
    let y = self.screen_height - touch.y;  // 翻转Y
}

pub fn on_mouse_event(&mut self, mouse: &MouseEvent, timestamp: u64) {
    let x = mouse.x;
    let y = self.screen_height - mouse.y;  // 翻转Y
}
```

### VStack子元素顺序
反转添加顺序以匹配期望显示：
```rust
// Button先添加 → 逻辑y小 → shader翻转后显示在屏幕上方（第一排）
tree_guard.add_widget(Box::new(button), vstack_id);
// Label后添加 → 逻辑y大 → shader翻转后显示在屏幕下方（第二排）
tree_guard.add_widget(Box::new(label), vstack_id);
```

### Vertex生成不翻转
直接使用逻辑bounds坐标：
```rust
// ui_vulkan_renderer.rs
DrawCommand::Rect { bounds, .. } => {
    let x = bounds.x;
    let y = bounds.y;  // shader会翻转
}

DrawCommand::Text { bounds, .. } => {
    for (gx, gy, ..) in glyphs {
        let x = gx;
        let y = gy;  // shader会翻转
    }
}
```

## Font Rendering Fixes

### 1. 空格渲染
空格glyph width=0, height=0，需跳过：
```rust
// ui_vulkan_renderer.rs
for (gx, gy, gw, gh, ..) in glyphs {
    let w = gw as f32;
    let h = gh as f32;
    
    if w == 0.0 || h == 0.0 {
        continue;  // 跳过空格等空白字符
    }
    
    // 正常渲染...
}
```

### 2. 空格预渲染
空格必须预渲染到字体缓存：
```rust
// font_atlas.rs
let test_chars = "... Press SPACE to change text ...";
```

### 3. Font Size匹配
渲染时font_size乘以2，必须匹配预渲染sizes：
```rust
// font_atlas.rs
let sizes = [32.0, 16.0, 14.0];  // 预渲染尺寸

// 渲染时
font_size * 2.0  // 必须在sizes中

// 示例：
// Label size=16 → 16*2=32 ✓
// Button size=16 → 16*2=32 ✓  
// hint_label size=16 → 16*2=32 ✓
```

## Window Resize Support

### 架构变更
新增字段：
```rust
pub struct UIVulkanRenderer {
    needs_resize: bool,           // resize标志
    new_extent: vk::Extent2D,     // 新尺寸
    swapchain_format: vk::Format, // 保存format用于重建
    physical_device: vk::PhysicalDevice, // 保存物理设备
    swapchain_images: Vec<vk::Image>,    // 保存swapchain images
}
```

### Swapchain重建流程
```rust
unsafe fn recreate_swapchain(&mut self) -> Result<(), String> {
    // 1. 等待设备空闲
    self.device.device_wait_idle()?;
    
    // 2. 销毁旧资源
    for framebuffer in &self.framebuffers {
        self.device.destroy_framebuffer(*framebuffer, None);
    }
    for view in &self.swapchain_image_views {
        self.device.destroy_image_view(*view, None);
    }
    
    // 3. 用old_swapchain重建
    let old_swapchain = self.swapchain;
    let swapchain = self.swapchain_loader.create_swapchain(&vk::SwapchainCreateInfoKHR {
        surface: self.surface,
        min_image_count: 2,
        image_format: self.swapchain_format,
        image_extent: new_extent,
        old_swapchain,  // 关键：传递旧swapchain
        ...
    })?;
    
    // 4. 销毁旧swapchain
    self.swapchain_loader.destroy_swapchain(old_swapchain, None);
    
    // 5. 创建新资源
    let swapchain_images = self.swapchain_loader.get_swapchain_images(swapchain)?;
    let swapchain_image_views = create_image_views(...);
    let framebuffers = create_framebuffers(...);
    
    // 6. 更新状态
    self.swapchain = swapchain;
    self.extent = new_extent;
    self.input_handler.lock().set_screen_size(...);
}
```

### UI布局更新
```rust
unsafe fn update_ui_layout(&mut self) {
    // 1. 更新root_panel尺寸
    root_widget.set_layout(Layout::new(
        0.0, 0.0, 
        self.extent.width as f32, 
        self.extent.height as f32
    ));
    
    // 2. 重新居中VStack
    tree_guard.recenter_widget(vstack_id, 
        self.extent.width as f32, 
        self.extent.height as f32
    );
    
    // 3. 执行布局
    tree_guard.perform_layout(font_atlas);
}
```

### Resize触发流程
```
GLFW WindowEvent::Size → 设置needs_resize=true
                           设置new_extent={width, height}

draw_frame() → 检查needs_resize
              → recreate_swapchain()
              → update_ui_layout()
              → 清除needs_resize
              → 继续渲染
```

## Coordinate Flow Summary

```
用户点击屏幕(420, 324) → 翻转Y → hit_test(420, 276)
                     ↓
           Button逻辑bounds匹配(y≈257)
                     ↓
           Button回调触发 → 文字变为"hello"
```

渲染流程：
```
Button逻辑bounds(y=257) → shader翻转Y → 渲染到屏幕上方
Label逻辑bounds(y=313)  → shader翻转Y → 渲染到屏幕下方
hint_label(y=10)        → shader翻转Y → 渲染到屏幕左下角
```

窗口最大化流程：
```
Window maximize → GLFW Size event → recreate_swapchain
                                   → update_ui_layout
                                   → 文字清晰渲染（不模糊）
```

## Key Files Modified

### ui_vulkan_renderer.rs
- `UIVulkanRenderer`: 新增resize相关字段
- `recreate_swapchain()`: swapchain重建方法
- `update_ui_layout()`: UI布局更新方法
- `draw_frame()`: 检查needs_resize并处理
- `process_events()`: 监听WindowEvent::Size
- `setup_ui()`: VStack顺序调整，font_size修正
- vertex生成：移除坐标翻转，跳过空glyph

### input_handler.rs
- `on_touch_event()`: Y坐标翻转
- `on_mouse_event()`: Y坐标翻转

### font_atlas.rs
- `test_chars`: 包含空格和所有使用的文字
- 空格特殊处理：记录advance_x用于cursor推进

## Testing

```bash
cargo run --bin mono_ui_demo --features mono --release
```

验证：
1. 第一排：Label "Welcome to Hezhou UI!"（空格正确）
2. 第二排：Button "Click Me"（空格正确）
3. 左下角：hint "Press SPACE to change text"（黄色）
4. 点击Button → 文字变为"hello"
5. 窗口最大化 → 文字清晰不模糊
6. 窗口resize → UI正确重新布局