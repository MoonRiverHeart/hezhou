# UI Widgets

17 widget types for the game editor UI system.

## Overview
Each widget implements `Widget` trait (id, parent, children, layout, style, state, measure, draw, on_event). Created via FFI, wrapped in C#. All visual sizes multiplied by content_scale.

## Widget Types
| Widget | Purpose | Key Methods |
|--------|---------|-------------|
| Button | Click action | SetOnClick |
| Label | Text display | SetText, SetAlignment |
| Panel | Container | Background color/border |
| VStack | Vertical layout | spacing=8, AddLabel/AddButton |
| HStack | Horizontal layout | spacing=8, AddLabel/AddButton |
| List | Scrollable list | Vertical/Horizontal mode |
| ListItem | List entry | Text + border separator |
| Dropdown | Select from options | SetOptions, SetSelected, OnSelect callback |
| InputField | Single-line input | SetText, GetText, SetOnChange, SetPlaceholder |
| TextEdit | Multi-line editor | Selection, scroll, cursor, Ctrl+C/V |
| PreviewWindow | Show offscreen texture | texture_id=1 for game pass |
| TabWidget | Tab switcher | AddTab, SetActive, GetActive, OnSelect, dog-ear fold (Triangle) |
| TreeView | Tree container | AddNode, SetSelected, ExpandNode, OnSelect |
| TreeNode | Tree entry | depth, is_expanded, is_selected, user_data |
| GridView | Grid asset browser | AddItem, SetSelected, Clear, OnClick |
| PopupMenu | Dropdown menu | AddItem, AddSeparator, Show, Hide, OnClick |
| Dialog | Modal dialog | SetContent, AddButton, Show, Hide, OnResult, auto-size from content |
| FileBrowser | File system browser | SetPath, SetFilter, NavigateUp, OnSelect |
| ScrollView | Scrollable container | SetContentHeight, ScrollTo, OnScroll, ClipRect + scroll_offset_y |
| SplitView | Split pane | SetRatio, OnRatioChange, drag-resize propagates to child Panels |

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Add new widget | `widgets/xxx.rs` + `widgets/mod.rs` + `ffi.rs` + `thunk_manager.rs` | 4 files minimum |
| Fix widget rendering | `xxx.rs draw()` method | Uses Canvas draw_rect/draw_text/draw_line |
| Fix widget event | `xxx.rs on_event()` method | Returns EventResult::Handled/Stopped/Ignored |
| Fix layout | `widget_tree.rs measure_and_layout` | Must add type-specific match branch |

## Conventions
- Widget struct fields: id, parent_id, children, layout, style, state, flags, content_scale
- `measure(&self, font_atlas: &FontAtlas)` returns (width, height)
- `draw(&mut self, canvas: &mut Canvas)` renders via Canvas API
- `on_event(&mut self, event: &Event)` returns EventResult
- Callbacks: `Box<dyn FnMut(args) + Send + Sync>` stored in widget
- FFI: `ui_create_xxx(handle, parent_id, x, y, w, h)` returns u64 widget id

## Anti-Patterns
- NEVER trigger thunk callback in `on_event` — use `queue_callback(PendingCallback::...)`
- NEVER call `create_font_atlas()` — use `crate::font_atlas::get_font_atlas()` (OnceLock)
- NEVER call `calculate_size()` in `add_item()` — defer to `show()` or `measure()`
- NEVER skip adding widget type to `widget_tree.rs measure_and_layout` match