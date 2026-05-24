# UI System Core

Widget tree, Canvas, FontAtlas, EventDispatcher, FFI bridge, thunk callback system.

## Structure
```
ui/src/
├── lib.rs          # 18 pub modules, get_font_atlas (OnceLock)
├── widget_tree.rs  # 1426 lines — WidgetTree, measure_and_layout, render data generation
├── canvas.rs       # DrawCommand enum (Rect, Text, Line, Image, Shadow, ClipRect, Triangle, RectOutline, SetTransform, ResetTransform)
├── font_atlas.rs   # 677 lines — FontAtlas, rasterize_char_with_fallback (emoji/CJK support)
├── widget.rs       # Widget trait definition (6 required methods + optional)
├── event_dispatcher.rs # Mouse/key/focus event routing, hit_test
├── ffi/            # 20 files — extern "C" FFI bridge (see below)
├── widgets/        # 25 files — 18 widget implementations (see widgets/AGENTS.md)
├── thunk/          # 7 files — callback system (see below)
├── types.rs        # Point, Rect, Color, Layout, Event, EventResult, WidgetState
├── text_layout.rs  # Text measurement and glyph positioning
├── animation.rs    # Animation easing functions
├── animation_engine.rs # Animation timeline system
├── gesture.rs      # Gesture recognition types
├── gesture_recognizer.rs # Swipe/pan/pinch gesture handlers
├── style.rs        # Style struct (background, border, padding)
├── layout.rs       # Layout struct (x, y, width, height)
├── msdf.rs         # MSDF font rendering (optional feature)
└── platform/
    └── input_handler.rs # Platform input event conversion
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