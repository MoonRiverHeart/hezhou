# UI System Core

Widget tree, Canvas, FontAtlas, EventDispatcher, FFI bridge, thunk callback system.

## Structure
```
ui/src/
├── 🏗️ lib.rs          # 18 pub modules, get_font_atlas (OnceLock)
├── 🌳 widget_tree.rs  # 1426 lines — WidgetTree, measure_and_layout, render data generation
├── 🎨 canvas.rs       # DrawCommand enum (Rect, Text, Line, Image, Shadow, ClipRect, Triangle, RectOutline, SetTransform, ResetTransform)
├── 🔤 font_atlas.rs   # 677 lines — FontAtlas, rasterize_char_with_fallback (emoji/CJK support)
├── 🧩 widget.rs       # Widget trait definition (6 required methods + optional)
├── 🖱️ event_dispatcher.rs # Mouse/key/focus event routing, hit_test
├── 🌉 ffi/            # 20 files — extern "C" FFI bridge (see below)
├── 📦 widgets/        # 25 files — 18 widget implementations (see widgets/AGENTS.md)
├── 🔗 thunk/          # 7 files — callback system (see below)
├── 📐 types.rs        # Point, Rect, Color, Layout, Event, EventResult, WidgetState
├── ✏️ text_layout.rs  # Text measurement and glyph positioning
├── 🎞️ animation.rs    # Animation easing functions
├── ⏱️ animation_engine.rs # Animation timeline system
├── 👆 gesture.rs      # Gesture recognition types
├── 🖖 gesture_recognizer.rs # Swipe/pan/pinch gesture handlers
├── 🎨 style.rs        # Style struct (background, border, padding)
├── 📐 layout.rs       # Layout struct (x, y, width, height)
├── 🔮 msdf.rs         # MSDF font rendering (optional feature)
└── 🖥️ platform/
    └── 🖱️ input_handler.rs # Platform input event conversion
```

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Fix widget layout | `widget_tree.rs` measure_and_layout | Add type-specific match branch for new widgets |
| Fix text rendering | `font_atlas.rs` rasterize_char_with_fallback | Emoji/CJK use fallback fonts, not prerasterize_chars |
| Fix event routing | `event_dispatcher.rs` | Mouse→hit_test→on_event chain, render layer sorting |
| Add DrawCommand | `canvas.rs` DrawCommand enum + draw_xxx method + renderer handler in rhi-vulkan | 3-file change |
| Fix ScrollView clipping | `widget_tree.rs` generate_render_data_recursive | ClipRect/ClearClip + scroll_offset_y |
| Add Panel→child sync | `widget_tree.rs` layout_panel_children | Unconditionally propagate Panel size to SplitView/TabWidget/ScrollView/PreviewWindow |

## FFI Bridge Layer (ui/src/ffi/)
- 20 files, each exposing `ui_create_xxx`, `ui_xxx_set_yyy`, `ui_xxx_get_yyy` functions
- `mod.rs`: WidgetTreeHandle type, all pub re-exports
- `widget.rs`: Button/Label/Panel/VStack/HStack/List basic widgets
- `checkbox.rs`, `slider.rs`, `dropdown.rs`: Complex interactive widgets
- `tab.rs`, `tree.rs`, `popup_menu.rs`, `grid_view.rs`: Container widgets
- `scroll_view.rs`, `split_view.rs`, `dialog.rs`: Layout containers
- `image.rs`, `input_field.rs`, `text_edit.rs`: Content widgets
- `misc.rs`: ui_widget_set_layer, ui_set/get_content_scale, global thunk ptr registration
- All functions: `#[unsafe(no_mangle)] pub extern "C" fn`, first arg is WidgetTreeHandle

## Thunk Callback System (ui/src/thunk/)
- `mod.rs`: UICallbacks struct (20+ HashMaps for per-widget callbacks + 7 Option fields for global callbacks)
- `pending.rs`: PendingCallback enum (20 variants) + queue_callback/flush_pending_callbacks
- `types.rs`: All callback type aliases (extern "C" fn signatures)
- `register.rs`: ui_register_xxx_callback functions (store in UICallbacks HashMap)
- `trigger.rs`: trigger_xxx_callback functions (lock-copy-release pattern — NEVER call inside lock)
- `global.rs`: SCREEN_SIZE, CONTENT_SCALE, FOCUSED_INPUT_FIELD (LazyLock Mutex globals)

## Conventions
- Widget trait: `id()`, `widget_type()`, `as_any()`, `as_any_mut()`, `measure()`, `draw()`, `on_event()` — plus optional `get_text()`, `set_content_scale()`
- Child inherits parent's render layer on add_widget
- PreviewWindow: auto-syncs offscreen FBO extent via find_preview_window_extent()
- Deferred FBO resize: pending_offscreen_resize stored, applied at frame start with device_wait_idle()

## Anti-Patterns
- NEVER trigger thunk callback in on_event — use queue_callback(PendingCallback::...)
- NEVER call create_font_atlas() — use get_font_atlas() (OnceLock)
- NEVER call calculate_size() in add_item() — defer to show()/measure()
- NEVER skip adding widget type to widget_tree.rs measure_and_layout match
- NEVER use as any or @ts-ignore in any language
- NEVER use fontdue for COLR/CBDT color emoji — use swash crate (fontdue only supports outline glyphs)
- NEVER use `…`(U+2026) for ellipsis in truncate_text — use `"..."`(3 ASCII dots) to ensure fontdue compatibility
- NEVER check font's COLR table presence for is_color_glyph — must check per-character Unicode range (emoji ranges only)

## Bug Fix History (2026-05-25 ~ 2026-05-26)

### truncate_text逻辑修复
- `…`(U+2026)→`"..."`(3个ASCII点)，fontdue可能不支持U+2026
- `max_text_width<=0`时返回原始文字而非空串(溢出比消失好)
- 文件: `widgets/tab.rs`

### Font atlas尺寸增大
- 4096→8192 (atlas_width, atlas_height, atlas_texture size 3处)
- 文件: `font_atlas.rs`

### swash COLR彩色emoji集成
- 双轨制: fontdue(普通文字white+alpha) + swash(COLR emoji RGBA)
- is_color_glyph: per-character Unicode范围检查，CJK走fontdue，emoji走swash
- bearing_y: swash placement.top取正值(不取负)
- rasterize_color_glyph: swash直接渲染RGBA位图到atlas
- 文件: `font_atlas.rs`, `widgets/tab.rs`(all_text字段)

### ensure_text_rasterized兜底
- 预光栅化21个常见特殊字符(…↑↓←→📁📄✓✗★●■▶◀📦🎬📜🧩📝🔷⬆)在所有字号
- InputField/FileBrowser/TabWidget类型单独日志记录
- 文件: `widget_tree.rs`

### generate_render_data返回Vec<RenderData>
- 从内部存储改为返回值，渲染器需赋值给render_data变量
- atlas上传准备阶段的调用只需side effect(光栅化glyph)，加分号丢弃返回值
- 文件: `widget_tree.rs`(函数签名), `rhi-vulkan/src/ui_vulkan_renderer.rs`(两处调用)

### is_preview_window_context per-widget reset
- PreviewWindow未选中时不重置is_preview_window_context→后续widget的transparent Rect背景跳过和border stroke路由异常
- 修复: 每个widget边界处重置is_preview_window_context=false
- 文件: `widget_tree.rs` generate_render_data_recursive

### TreeView不强制fill Panel宽度 (2026-05-27)
**现象**: 脚本编辑器只有目录结构(TreeView)，右侧TextEdit编辑区域消失。
**根因**: `layout_panel_children`把TreeView和PreviewWindow/TabWidget/GridView同等对待，强制fill到`panel_layout.width - child_layout.x`。脚本编辑器的TreeView设定宽度200px被扩展到~1270px，完全覆盖同Panel下TextEdit的右侧编辑区域。
**修复**: TreeView从fill-content列表分离，只fill高度(垂直滚动需要)，保持C#端设定的固定宽度。
**文件**: `widget_tree.rs` layout_panel_children

### Dialog居中修复 — 物理/逻辑像素坐标系混用 (2026-05-27)
**现象**: Dialog弹窗不居中，偏移到窗口右下角(下、右和窗口边缘对齐)。
**根因**: Dialog的`with_size()`、`layout_dialog_children`、`ffi/dialog.rs`三处居中计算混用物理像素(screen_size是swapchain extent)和逻辑像素(width/height是C#传入的未scale值)。x/y坐标用物理像素计算，但layout中width/height存逻辑值，导致坐标系不匹配。content_scale>1时偏移更严重。
**修复**: 三处统一改为逻辑像素坐标系 — `screen_size / content_scale`转成逻辑像素后再计算居中。`layout_dialog_children`的内部布局(scaled_title_height等)也改为逻辑值。
**文件**: `widgets/dialog.rs` with_size(), `widget_tree.rs` layout_dialog_children, `ffi/dialog.rs` ui_create_dialog

### Dialog布局修复 — 不应填充Panel + 内边距 (2026-05-30)
**现象**: Dialog的下、右边界贴着窗口边缘，不是居中显示。内容与Dialog边界之间没有间距。
**根因**: `layout_panel_children` 把Dialog当成"parent-allocated"容器（与SplitView同等），强制填充Panel剩余空间。`is_parent_allocated` 判断也包含Dialog，导致Dialog尺寸被Panel覆盖，而非根据内容自适应+居中。
**修复**:
1. `layout_panel_children`: Dialog从fill-Panel列表移除（只保留SplitView）
2. `measure_and_layout`: Dialog不再是parent-allocated，不参与auto_size填充
3. `widgets/dialog.rs`: 新增 `content_padding: 16.0` (pub(crate)字段) — Dialog内容与边界之间16逻辑像素间距
4. `layout_dialog_children`: content定位使用padding（`content_x = padding`, `content_y = title + padding`），宽度减去两侧padding，高度减去上下padding，min_dialog_height也包含padding
5. `dialog.rs get_content_rect()`: 使用content_padding计算带内边距的content区域
**文件**: `widget_tree.rs` layout_panel_children + measure_and_layout + layout_dialog_children, `widgets/dialog.rs` content_padding + get_content_rect