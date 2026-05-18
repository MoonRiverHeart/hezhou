# TextEdit 技术文档

## 概述
TextEdit 是 Hezhou UI 框架中的文本输入控件，支持：
- 键盘输入（字母、数字、符号、中文、emoji）
- 键盘导航（方向键、Home/End）
- 点击定位光标
- 文本选择（Shift+Click）
- 复制/粘贴（Ctrl+C/V）
- 自动换行

## 核心架构

### 双索引系统
TextEdit 使用双索引来处理 Unicode 文本：

```
┌─────────────────────────────────────────────────┐
│  String (UTF-8)                                 │
│  "abc你好😊" → bytes: [a,b,c,好1,好2,你1,你2,😊1..4]│
└─────────────────────────────────────────────────┘
         ↓ unicode-segmentation
┌─────────────────────────────────────────────────┐
│  Grapheme Clusters (用户感知的字符)              │
│  [a, b, c, 好, 你, 😊]                          │
│  cursor_grapheme_index = 3 (光标在"好"之后)     │
│  cursor_byte_index = 6 (对应String中的位置)     │
└─────────────────────────────────────────────────┘
```

| 字段 | 用途 | 示例 |
|------|------|------|
| `cursor_grapheme_index` | 用户界面显示、键盘导航 | 3 (第3个grapheme后) |
| `cursor_byte_index` | String操作（插入/删除） | 6 (byte位置) |

### 关键代码
```rust
// grapheme → byte 转换
fn grapheme_index_to_byte_index(&self, grapheme_idx: usize) -> usize {
    use unicode_segmentation::UnicodeSegmentation;
    self.text.grapheme_indices(true)
        .nth(grapheme_idx)
        .map(|(i, _)| i)
        .unwrap_or(self.text.len())
}
```

## 坐标系统

### Window → Widget Relative 转换
点击事件从 GLFW 传入的是**窗口绝对坐标**，需要转换为**Widget相对坐标**：

```
┌─────────────────────────────────────────────────┐
│  Window (800x600)                               │
│  点击位置: (366, 355) ← GLFW传入                │
├─────────────────────────────────────────────────┤
│  VStack (300, 200)                              │
│  绝对位置: (300, 200)                           │
│  点击相对位置: (66, 155)                        │
├─────────────────────────────────────────────────┤
│  TextEdit (位于VStack内)                        │
│  绝对位置: (300, 331)                           │
│  点击相对位置: (66, 24) ← 最终用于光标定位      │
└─────────────────────────────────────────────────┘
```

### 转换实现 (event_dispatcher.rs:72)
```rust
fn dispatch_bubbling(&self, widget_id: WidgetId, event: &mut Event) {
    if let Event::Touch(TouchPhase::Begin, mut touch_data) = event {
        let tree = self.widget_tree.lock();
        if let Some(abs_layout) = tree.get_absolute_layout(widget_id) {
            // Window坐标 → Widget相对坐标
            touch_data.x -= abs_layout.x;
            touch_data.y -= abs_layout.y;
        }
    }
    // ... 分发事件给widget
}
```

## 光标定位算法

### find_cursor_position_at
点击时找到最近的光标位置：

1. **找到最近的行**（Y方向）
```rust
for layout in &self.char_layouts {
    let y_distance = (click_y - layout.y).abs();
    if y_distance < min_y_distance {
        min_y_distance = y_distance;
        closest_line_y = layout.y;
    }
}
```

2. **在该行找到最近的grapheme**（X方向）
```rust
for layout in &self.char_layouts {
    if layout.y == closest_line_y {
        let grapheme_center_x = layout.x + layout.width / 2.0;
        let x_distance = (click_x - grapheme_center_x).abs();
        
        if x_distance < best_distance {
            best_distance = x_distance;
            // 判断点击在grapheme左侧还是右侧
            if click_x < grapheme_center_x {
                best_grapheme_idx = layout.grapheme_index;
            } else {
                best_grapheme_idx = layout.grapheme_index + 1;
            }
        }
    }
}
```

### 光标渲染位置计算
```rust
// 光标X位置 = 当前grapheme或文本末尾
let cursor_x = if cursor_grapheme_index < char_layouts.len() {
    char_layouts[cursor_grapheme_index].x
} else {
    char_layouts.last().unwrap().x + char_layouts.last().unwrap().width
};

// 光标Y位置 = baseline - max_bearing_y (文字顶部)
let cursor_y = baseline_y - max_bearing_y;

// 光标高度 = font_size (覆盖完整文字高度)
let cursor_height = font_size;
```

## 键盘导航

### 方向键处理
```rust
// ui_vulkan_renderer.rs: 显式匹配方向键
if key == Key::Left {
    input_handler.on_key_event(KeyEvent { code: KeyCode::Left, ... });
}
if key == Key::Right {
    input_handler.on_key_event(KeyEvent { code: KeyCode::Right, ... });
}
if key == Key::Up {
    input_handler.on_key_event(KeyEvent { code: KeyCode::Up, ... });
}
if key == Key::Down {
    input_handler.on_key_event(KeyEvent { code: KeyCode::Down, ... });
}
```

### TextEdit 键盘事件处理
```rust
fn on_key_event(&mut self, event: &KeyEvent) -> EventResult {
    match event.code {
        KeyCode::Left => {
            if self.cursor_grapheme_index > 0 {
                self.cursor_grapheme_index -= 1;
                self.cursor_byte_index = self.grapheme_index_to_byte_index(self.cursor_grapheme_index);
            }
        }
        KeyCode::Right => {
            let total_graphemes = self.text.grapheme_indices(true).count();
            if self.cursor_grapheme_index < total_graphemes {
                self.cursor_grapheme_index += 1;
                self.cursor_byte_index = self.grapheme_index_to_byte_index(self.cursor_grapheme_index);
            }
        }
        KeyCode::Home => {
            self.cursor_grapheme_index = 0;
            self.cursor_byte_index = 0;
        }
        KeyCode::End => {
            self.cursor_grapheme_index = self.text.grapheme_indices(true).count();
            self.cursor_byte_index = self.text.len();
        }
        // Up/Down: 跨行移动（自动换行时）
        // ...
    }
}
```

## 文本选择与复制

### Shift+Click 选择
```rust
fn on_touch_begin(&mut self, touch_data: &TouchData) {
    let new_pos = self.find_cursor_position_at(touch_data.x, touch_data.y);
    
    if touch_data.modifiers.shift {
        // 设置选择范围
        self.selection_start = self.cursor_grapheme_index.min(new_pos);
        self.selection_end = self.cursor_grapheme_index.max(new_pos);
    } else {
        // 普通点击：清除选择，移动光标
        self.selection_start = 0;
        self.selection_end = 0;
        self.cursor_grapheme_index = new_pos;
    }
}
```

### Ctrl+C 复制选中内容
```rust
KeyCode::C if event.modifiers.ctrl => {
    if self.selection_end > self.selection_start {
        let start_byte = self.grapheme_index_to_byte_index(self.selection_start);
        let end_byte = self.grapheme_index_to_byte_index(self.selection_end);
        let selected_text = &self.text[start_byte..end_byte];
        // 复制到剪贴板...
    }
}
```

## 布局缓存

### layout_dirty 标志
避免每次渲染都重新计算布局：

```rust
// 文本变化时标记dirty
fn set_text(&mut self, text: &str) {
    self.text = text.to_string();
    self.layout_dirty = true; // 触发重新布局
}

// 渲染时只在dirty时计算
fn draw(&mut self, canvas: &mut Canvas) {
    if self.layout_dirty {
        self.char_layouts = canvas.layout_text_for_cursor_with_wrap(...);
        self.layout_dirty = false;
    }
    // 使用缓存的char_layouts渲染
}
```

## CharLayout 结构
每个grapheme的布局信息：

```rust
struct CharLayout {
    x: f32,                    // grapheme左边缘X坐标（widget相对）
    y: f32,                    // grapheme顶部Y坐标（widget相对）
    width: f32,                // grapheme宽度（可能包含多个char）
    height: f32,               // 文字高度 = font_size
    grapheme_index: usize,     // grapheme序号
    grapheme_start_byte: usize, // 对应String中的起始byte
    grapheme_end_byte: usize,   // 对应String中的结束byte
}
```

## Emoji/组合字符支持

### unicode-segmentation 库
正确处理：
- Emoji: 😊 (4 bytes, 1 grapheme)
- 组合字符: é (2 bytes, 可能是e+´组合)
- 中文: 好 (3 bytes, 1 grapheme)

```rust
use unicode_segmentation::UnicodeSegmentation;

// 正确分割grapheme
for (idx, grapheme) in text.grapheme_indices(true) {
    println!("grapheme {}: '{}' at byte {}", idx, grapheme, grapheme_start);
}
```

## 关键文件

| 文件 | 功能 |
|------|------|
| `ui/src/widgets/text_edit.rs` | TextEdit核心实现 |
| `ui/src/event_dispatcher.rs:72` | Window→Widget坐标转换 |
| `ui/src/widget_tree.rs` | get_absolute_layout() |
| `ui/src/font_atlas.rs` | layout_text_for_cursor_with_wrap() |
| `rhi-vulkan/src/ui_vulkan_renderer.rs` | 方向键显式匹配 |

## 测试方法

```bash
cd engine
cargo test --lib -p hezhou-ui widgets::text_edit::tests
```

### 测试覆盖
| 测试 | 覆盖内容 |
|------|----------|
| `test_click_before_first_grapheme` | 点击第一个字符之前 → 返回位置0 |
| `test_click_after_last_grapheme` | 点击最后一个字符之后 → 返回末尾 |
| `test_click_between_graphemes` | 点击两个字符之间 → 正确定位 |
| `test_click_at_grapheme_center` | 点击字符中心 → 定位到下一个位置 |
| `test_empty_text` | 空文本点击任意位置 → 返回0 |
| `test_click_far_above_text` | 点击文字上方很远 → 选择最近行 |
| `test_click_far_below_text` | 点击文字下方很远 → 选择最近行 |
| `test_cursor_movement_left_boundary` | 左边界不能再左移 |
| `test_cursor_movement_right_boundary` | 右边界不能再右移 |
| `test_home_end_keys` | Home/End键边界测试 |
| `test_delete_last_character` | 删除字符直到空文本 |
| `test_insert_at_boundaries` | 开头/中间/末尾插入 |
| `test_grapheme_to_byte_conversion` | ASCII/中文/Emoji索引转换 |

### 光标样式
- 宽度: 3.0px
- 高度: font_size (完整文字高度)
- 位置: baseline - max_bearing_y (文字顶部)

## 已解决的问题

| 问题 | 解决方案 |
|------|----------|
| 键盘输入不响应 | broadcast_key_event遍历所有widget |
| 方向键不工作 | ui_vulkan_renderer显式匹配Key::Left/Right |
| 点击光标位置错误 | Window坐标→Widget相对坐标转换 |
| 光标Y位置跳动 | cursor_y = baseline - max_bearing_y |
| 光标高度不足 | cursor_height = font_size（完整高度） |
| 光标宽度太细 | cursor_width = 3.0（更明显） |
| Emoji分割错误 | unicode-segmentation库 |
| UTF-8边界错误 | is_char_boundary验证 |
| 点击超出边界 | 首字符前/末字符后特殊处理 |