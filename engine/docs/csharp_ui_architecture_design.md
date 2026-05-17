# C# UI控件系统架构设计

## 目标

实现C#面向对象风格的UI控件系统：
- C#创建控件对象（Button、Label、Panel等）
- 自动同步控件数据到Rust
- Rust负责渲染和事件检测
- 点击事件通过Thunk回调到C#

## 实现状态：已完成 ✅

**测试验证（2026-05-17）**
- C#创建Label和Button成功
- 点击Button触发C#回调成功
- 日志：`[C#] Button clicked!` 正确输出

## 架构

```
┌──────────────────────────────────────────────────────┐
│                    C# (Mono)                          │
│                                                       │
│  // 用户代码                                          │
│  Button btn = new Button("Click Me");                │
│  btn.Position = new Vector2(100, 200);               │
│  btn.OnClick += () => label.Text = "hello";          │
│                                                       │
│  // 内部自动同步                                       │
│  ┌─────────────────────────────────────────────────┐ │
│  │                 Widget类                         │ │
│  │  - widgetId (ulong)                             │ │
│  │  - Position, Size, Text                         │ │
│  │  - OnCreate() → 调用FFI获取Id                    │ │
│  │  - OnPropertyChanged() → 调用FFI同步             │ │
│  └─────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
                        │
                        │ FFI调用
                        ▼
┌──────────────────────────────────────────────────────┐
│                    Rust                               │
│                                                       │
│  ┌─────────────────────────────────────────────────┐ │
│  │               WidgetTree                         │ │
│  │  - 存储所有控件数据                               │ │
│  │  - Button, Label, Panel, VStack, HStack         │ │
│  │  - OnClick回调映射                               │ │
│  └─────────────────────────────────────────────────┘ │
│                        │                             │
│                        ▼                             │
│  ┌─────────────────────────────────────────────────┐ │
│  │            Vulkan Renderer                       │ │
│  │  - 渲染所有控件                                   │ │
│  │  - 检测点击事件                                   │ │
│  │  - 触发OnClick回调                               │ │
│  └─────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
```

## C#控件类设计

### 基类 Widget

```csharp
public abstract class Widget
{
    protected ulong _widgetId;
    protected bool _created;
    
    public Vector2 Position { get; set; }
    public Vector2 Size { get; set; }
    
    protected abstract void CreateInRust();
    
    protected void SyncToRust()
    {
        if (!_created) CreateInRust();
        // 调用FFI更新属性
    }
}
```

### Button

```csharp
public class Button : Widget
{
    private string _text;
    private Action _onClick;
    
    public string Text 
    {
        get => _text;
        set { _text = value; SyncText(); }
    }
    
    public Action OnClick
    {
        set { _onClick = value; RegisterCallback(); }
    }
    
    protected override void CreateInRust()
    {
        _widgetId = FFI.CreateButton(_text, Position, Size);
        _created = true;
    }
    
    private void SyncText()
    {
        if (_created) FFI.SetText(_widgetId, _text);
    }
    
    private void RegisterCallback()
    {
        if (_created) FFI.SetOnClick(_widgetId, _onClick);
    }
}
```

### Label

```csharp
public class Label : Widget
{
    private string _text;
    
    public string Text
    {
        get => _text;
        set { _text = value; SyncText(); }
    }
    
    protected override void CreateInRust()
    {
        _widgetId = FFI.CreateLabel(_text, Position, Size);
        _created = true;
    }
}
```

### 容器控件

```csharp
public class VStack : Widget
{
    private List<Widget> _children = new();
    
    public void Add(Widget child)
    {
        _children.Add(child);
        SyncChildren();
    }
    
    protected override void CreateInRust()
    {
        _widgetId = FFI.CreateVStack();
        foreach (var child in _children)
            FFI.AddChild(_widgetId, child._widgetId);
    }
}
```

## Rust FFI扩展

需要新增的FFI函数：

```rust
// ui/src/ffi.rs

// 创建控件
pub extern "C" fn ui_create_button(text: *const c_char, x: f32, y: f32, w: f32, h: f32) -> u64;
pub extern "C" fn ui_create_label(text: *const c_char, x: f32, y: f32, w: f32, h: f32) -> u64;
pub extern "C" fn ui_create_panel() -> u64;
pub extern "C" fn ui_create_vstack() -> u64;
pub extern "C" fn ui_create_hstack() -> u64;

// 属性设置
pub extern "C" fn ui_set_position(widget_id: u64, x: f32, y: f32);
pub extern "C" fn ui_set_size(widget_id: u64, w: f32, h: f32);
pub extern "C" fn ui_set_text(widget_id: u64, text: *const c_char);  // 已有

// 容器操作
pub extern "C" fn ui_add_child(parent_id: u64, child_id: u64);

// 回调
pub extern "C" fn ui_set_on_click(widget_id: u64, callback: *const c_void);  // 已有
```

## 同步时机

| 操作 | 同步时机 |
|------|----------|
| 创建控件 | 立即调用FFI创建 |
| 设置属性 | 立即调用FFI更新 |
| 添加子控件 | 父控件创建后立即同步 |
| 注册回调 | 控件创建后立即注册 |

## 实现步骤

### 第一阶段：基础控件
1. 实现`Widget`基类
2. 实现`Button`控件（创建、属性、回调）
3. 实现`Label`控件（创建、属性）
4. 测试：创建Button和Label，点击触发回调

### 第二阶段：容器控件
1. 实现`VStack`容器
2. 实现`HStack`容器
3. 实现`Panel`容器
4. 测试：嵌套布局

### 第三阶段：完善
1. 实现更多属性（颜色、字体、边框等）
2. 优化同步机制（批量同步、延迟同步）
3. 完善错误处理

## 与现有Demo的区别

| 现有Demo | 新架构 |
|----------|--------|
| Rust创建控件 | C#创建控件 |
| C#只注册回调 | C#拥有完整控件对象 |
| 硬编码布局 | 动态布局 |
| 单个Button示例 | 完整UI框架 |

## 性能考虑

- 每次属性修改都调用FFI（~10ns）
- 可以考虑批量同步（收集修改，一次性同步）
- 控件数量多时，考虑延迟同步（定时更新）