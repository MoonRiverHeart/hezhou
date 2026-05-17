# VStack/HStack布局系统

## 概述

Rust实现的自动布局容器，C#只需指定控件宽高，布局由Rust自动计算。

## 使用方式

### C#代码示例

```csharp
// 创建VStack（垂直布局）
VStack vstack = new VStack(spacing: 10f);
vstack.SetPosition(centerX, centerY);  // 只设置容器位置

// 添加控件（自动排列，不需要计算坐标）
Label label = new Label(vstack.Id, 200f, 30f, "Hello");
Button btn1 = new Button(vstack.Id, 200f, 50f, "Button 1");
Button btn2 = new Button(vstack.Id, 200f, 50f, "Button 2");

// 注册回调
btn1.SetOnClick((id) => btn1.Text = "hello");
btn2.SetOnClick((id) => btn2.Text = "hello");
```

### 运行效果

```
[屏幕上方]
┌──────────────────┐
│    Label         │
├──────────────────┤
│    Button 1      │
├──────────────────┤
│    Button 2      │
└──────────────────┘
[屏幕下方]
```

## 布局计算

### VStack

从上往下排列子控件：
- 第一个子控件：`y = parent_y + total_height - height`
- 后续子控件：`y -= height + spacing`

```rust
// ui/src/widget_tree.rs: layout_vstack_children
let total_height = child_heights + spacing * (n-1);
let mut current_y = parent_y + total_height;

for child in children {
    current_y -= child_height;
    set_layout(x, current_y, width, height);
    current_y -= spacing;
}
```

### HStack

从左往右排列子控件：
- 第一个子控件：`x = parent_x`
- 后续子控件：`x += width + spacing`

## 坐标系统

- Vulkan viewport: y从上到下增加（y=0在上方，y=height在下方）
- Shader: `-clip_pos.y` 翻转y坐标使文字正常显示
- 布局: y大在上方，y小在下方（与屏幕坐标一致）

## FFI接口

| 函数 | 说明 |
|------|------|
| `ui_create_vstack(handle, spacing)` | 创建VStack容器 |
| `ui_create_hstack(handle, spacing)` | 创建HStack容器 |
| `ui_create_button_in_parent(handle, parent_id, w, h, text)` | 在父容器中创建Button |
| `ui_create_label_in_parent(handle, parent_id, w, h, text)` | 在父容器中创建Label |
| `ui_set_widget_layout(handle, id, x, y, w, h)` | 设置控件布局 |

## 窗口Resize响应

```csharp
UI.RegisterResizeCallback((width, height) => {
    float centerX = width / 2f - 100f;
    float centerY = height / 2f - 80f;
    vstack.SetPosition(centerX, centerY);
});
```

## 关键文件

| 文件 | 内容 |
|------|------|
| `ui/src/widget_tree.rs` | perform_layout, measure_and_layout |
| `ui/src/ffi.rs` | ui_create_vstack/hstack FFI |
| `scripts/UI.cs` | VStack, HStack, Button, Label类 |
| `scripts/TestScript.cs` | 使用示例 |
| `shaders/ui/ui.vert` | y坐标翻转处理 |