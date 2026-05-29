# Hezhou Engine Knowledge Base

**Generated:** 2026-05-25
**Commit:** afb4f7d
**Branch:** main

## Overview
Cross-platform game engine: Rust core + C# scripting (Mono JIT/NativeAOT). Vulkan rendering, custom UI system with 17 widget types, ECS Scene, asset library, project file system. Inverted dependency: core depends on scripting (for FfiContext types).

## Structure
```
engine/
├── 🧩 core/           # ECS Scene, Transform, AssetLibrary, Project (see core/src/AGENTS.md)
├── 🖼️ ui/             # Widget system, Canvas, FontAtlas, EventDispatcher (see ui/src/AGENTS.md)
│   └── 📦 widgets/    # 17 widget types (see widgets/AGENTS.md)
│   └── 🔗 thunk/      # Callback dispatch (see ui/src/AGENTS.md)
│   └── 🌉 ffi/        # FFI bridge layer (see ui/src/AGENTS.md)
├── 🎨 rhi/            # Abstract RHI trait layer (see rhi/src/AGENTS.md)
├── ⚡ rhi-vulkan/     # Vulkan renderer impl (see rhi-vulkan/src/AGENTS.md)
├── 🎥 render/         # RenderEngine + Camera/Mesh/Texture (see render/src/AGENTS.md)
├── 📜 scripting/      # Mono JIT executor, FfiContext (see scripting/src/AGENTS.md)
│   └── 🏗️ ffi_context/ # 270-field FfiContext struct (see scripting/src/ffi_context/AGENTS.md)
├── 🤖 mcp/            # MCP server — LLM集成层 (see mcp/src/AGENTS.md)
│   └── 🔮 bridge.rs   # EngineBridge trait + 8返回结构体 + [Both]/[RustOnly]/[FfiOnly]标注
│   └── 🖥️ server.rs   # 18 #[tool]方法 + #[tool_router(server_handler)]自动生成ServerHandler
│   └── 🔀 rust_bridge.rs # 纯Rust路径: raw pointer, 可脱离编辑器独立运行
│   └── 🔗 ffi_bridge.rs  # C# FFI路径: FfiContext函数指针, 需编辑器+Mono运行时
│   └── 📡 transport.rs   # stdio/自定义io transport启动
├── 💻 scripts/        # C# source: EditorScript.cs, UI.cs (see scripts/AGENTS.md)
├── 🖥️ platform/       # GLFW/Harmony platform abstraction (see platform/src/AGENTS.md)
├── 📊 dfx/            # Logging, crash, trace, perf monitoring (see dfx/src/AGENTS.md)
├── 🎯 physics/        # PhysicsWorld, RigidBody, Raycast, Collider
├── 📱 harmony/        # HarmonyEngine + OpenHarmony native window
├── 📐 geometry/       # BoundingBox, MeshData, Vertex layouts
├── 🎮 examples/src/   # 15 demo programs (see examples/src/AGENTS.md)
│   └── ✏️ editor/     # Main editor module (see examples/src/editor/AGENTS.md)
├── 🔮 shaders/        # rotation.vert/frag + ui shaders (manually compiled to .spv)
└── 🔤 fonts/          # HarmonyOS_Sans_SC.ttf (bundled)
```

## Where To Look
| Task | Location | Notes |
|------|----------|-------|
| Add new widget | `ui/src/widgets/` + `ui/src/ffi/` + `ui/src/thunk/` + `scripting/src/ffi_context/` + `scripts/UI.cs` | 4 Rust files + 2 C# files + FfiContext (16-step checklist) |
| Fix rendering | `rhi-vulkan/src/ui_vulkan_renderer.rs` | 4094 lines, game+UI+outline passes |
| Fix UI layout | `ui/src/widget_tree.rs` measure_and_layout | Add type-specific layout in match |
| Fix callback deadlock | `ui/src/thunk/` | queue_callback + flush_pending_callbacks, NEVER trigger inside lock |
| Add FFI function | `ui/src/ffi/` + `scripting/src/ffi_context/` + `scripts/UI.cs` | Type alias + FfiContext field + C# delegate |
| Edit C# editor | `scripts/EditorScript.View.cs` + `.Presenter.cs` + `.State.cs` | Partial classes, 4 files |
| Hot reload | `examples/src/editor/hot_reload.rs` | build_mono.ps1 → mcs → executor.reload() |
| Entity/Scene | `core/src/ecs/scene.rs` + `core/src/ecs/scene_ffi_impl.rs` | ScriptBinding, entity_names |
| Asset library | `core/src/asset_library.rs` + `core/src/asset_ffi.rs` | 5 primitives + textures + materials |
| Project files | `core/src/project.rs` | JSON format, create/load/save |
| MCP module | `mcp/src/` — bridge.rs, server.rs, rust_bridge.rs, ffi_bridge.rs | 18 tools, EngineBridge trait, rmcp v1.7.0 |
| MCP server启动 | `mcp/src/transport.rs` + `examples/src/editor/mcp_init.rs` | HEZHOU_MCP env → stdio transport, Scene创建后启动 |

## Architecture Notes
- **Inverted dependency**: core → scripting (core needs FfiContext types from scripting). Scripting is the leaf crate.
- **4 mid-level crates produce cdylib**: scripting, ui, dfx, harmony — needed for standalone FFI export model
- **FfiContext**: 270-field flat #[repr(C)] struct of function pointers, assembled in editor/mod.rs with transmute
- **C# has NO Main() methods**: All scripts are DLL assemblies, entry via static Initialize() called from Rust
- **Shader compilation**: NO automated pipeline. `.spv` files manually compiled via glslc and committed to repo
- **Mono build**: 3 inconsistent PowerShell scripts (build_mono.ps1, build_mono_ui.ps1, build_ui_mono.ps1)
- **13 crate workspace**: core, ui, rhi, rhi-vulkan, render, scripting, platform, dfx, physics, harmony, geometry, examples, mcp
- **Platform feature gates**: `glfw` (Windows/Linux), `harmony` (OpenHarmony) — PlatformManager selects backend at init
- **RHI is trait-based**: rhi/ defines abstract Device/CommandBuffer/Pipeline traits, rhi-vulkan/ implements them

## Anti-Patterns (THIS PROJECT)
- **NEVER** call `create_font_atlas()` directly — use `get_font_atlas()` (OnceLock cached)
- **NEVER** trigger thunk callbacks inside `WidgetTree.lock()` — use `queue_callback()` + `flush_pending_callbacks()` 
- **NEVER** use `as any` or `@ts-ignore` in any language
- **NEVER** call `calculate_size()` inside `add_item()` — defer to `show()` or `measure()`
- **NEVER** use Button for menu items — use PopupMenu with shortcuts and separators
- **NEVER** use static Labels for tree structures — use TreeView/TreeNode
- **NEVER** use VStack+Label for asset lists — use GridView
- **NEVER** store C# callback delegates as local variables — must be static fields (GC risk)
- **NEVER** skip FfiContext initialization for new FFI functions (Rust AND C# both)
- **NEVER** define delegates or FfiContext fields outside `UI.cs` main file
- **NEVER** use .NET APIs not available in Mono mcs (no LINQ advanced, no async)
- **NEVER** use .NET 8 compiled DLLs with Mono — use `mcs` compiler instead
- **NEVER** destroy/recreate widgets on state switch — use `SetWidgetVisible` instead
- **NEVER** create Vulkan pipeline without `p_dynamic_state` for VIEWPORT+SCISSOR
- **NEVER** hardcode viewport size — use `cmd_set_viewport` at render time
- **NEVER** mismatch push constant layout with shader struct
- **NEVER** destroy FBO resources without `device_wait_idle()` first (causes VK_ERROR_DEVICE_LOST)
- **NEVER** omit UV coordinates from any vertex type — ALL vertices MUST be 8 floats (x,y,r,g,b,a,u,v) matching pipeline stride 32 bytes
- **NEVER** use fontdue for COLR/CBDT color emoji — use swash crate instead (fontdue only supports outline glyphs)
- **NEVER** use `as any` type erasure for World storage — use `Box<dyn Any>` + downcast for type-safe per-component storage

## Unique Styles
- **FFI naming**: `ui_create_xxx`, `ui_xxx_set_yyy`, `ui_xxx_get_yyy` (snake_case)
- **Thunk naming**: `ui_xxx_set_on_yyy_thunk_ptr` for per-widget, `ui_register_yyy_thunk_ptr` for global
- **C# naming**: PascalCase (CreateXxx, SetYyy, GetYyy) + Delegate suffix for types
- **Widget 3-layer**: Rust Widget trait → FFI extern "C" → C# static wrapper (4 Rust files + 2 C# files)
- **Thunk 2-phase**: `queue_callback(PendingCallback::...)` inside lock → `flush_pending_callbacks()` outside lock
- **Thunk lock-copy-release**: Trigger functions lock → copy fn ptr → drop lock → call C# (prevents deadlock)
- **Render layers**: Background(0) Content(1) Popup(2) Overlay(3) — PopupMenu=2, Dialog=3
- **InputField focus**: Global `FOCUSED_INPUT_FIELD`, draw() syncs `is_focused` each frame
- **GameState**: Editing(0), Running(1), Paused(2) — orange/blue/green preview border
- **KeyCode**: Left=45, Right=46, Up=47, Down=48, ESC=39
- **ContentScale**: DPI-aware (1.5 at 144 DPI), all visual sizes multiplied, input coords NOT scaled
- **Deferred FBO resize**: `pending_offscreen_resize` → `device_wait_idle()` → recreate at frame start
- **Widget layout**: SplitView/Dialog are parent-allocated (size from Panel), not auto-sized

## Commands
```bash
cd engine
cargo build                                          # Build all crates
cargo build --release                                 # Release build
cargo run --bin mono_editor_demo --features mono --release  # Game Editor
cargo run --bin mono_triangle_demo --features mono   # Hot reload demo
cargo run --bin trace_viewer_demo --release           # Trace viewer

cd engine/scripts
powershell -ExecutionPolicy Bypass -File build_mono.ps1  # Compile C# (mcs)

# MCP server (LLM集成 — 编辑器内嵌模式):
HEZHOU_MCP=1 cargo run --bin mono_editor_demo --features "mono,mcp" --release      # Editor + MCP (HTTP, 默认端口3000)
HEZHOU_MCP=1 HEZHOU_MCP_PORT=8080 cargo run --bin mono_editor_demo --features "mono,mcp" --release  # Editor + MCP (HTTP, 自定义端口)
HEZHOU_MCP=1 HEZHOU_MCP_TRANSPORT=stdio cargo run --bin mono_editor_demo --features "mono,mcp" --release  # Editor + MCP (stdio模式)
HEZHOU_MCP=1 HEZHOU_MCP_MODE=rust cargo run --bin mono_editor_demo --features "mono,mcp" --release  # Editor + MCP (纯Rust路径)
# OpenCode连接: 项目级.opencode/opencode.json已配置 → type:"remote", url:"http://127.0.0.1:3000/mcp"

# Shader compile (manual, no automation):
glslc shaders/rotation.vert -o shaders/rotation.vert.spv
glslc shaders/rotation.frag -o shaders/rotation.frag.spv

# Debug UI tree: Ctrl+Shift+D in editor
```

## Notes
- Mono/.NET 8 incompatibility: .NET 8 DLLs → 0 methods in Mono. Use `mcs` compiler.
- `mono_class_get_method_from_name` returns null → iterate `mono_class_get_methods` + match name
- Standalone release: dfx function pointers via FfiContext (exe doesn't export symbols)
- FontAtlas first init ~0.23s (OnceLock), subsequent calls instant
- Editor startup: FontAtlas init + C# compile + Mono load ≈ 2-3s
- 3 inconsistent Mono build scripts (build_mono.ps1 hardcoded mcs path, build_mono_ui.ps1 uses PATH + -unsafe flag, build_ui_mono.ps1 hardcoded + subset) — should consolidate
- No CI/CD pipeline — zero `.github/workflows`
- No rust-toolchain.toml — no pinned toolchain
- workspace.dependencies under-used: only 3 deps (parking_lot, tokio unused, wrapped_mono) — version skew risk
- editor/mod.rs uses 7 `static mut` globals — soundness hazard (undefined behavior from multiple threads)
- `.gitignore` excludes `*.json` and `*.png` — overly broad, blocks legitimate asset files
- Shader compilation has NO automation — .spv files manually compiled via glslc and committed

## Bug Fix History (2026-05-25 ~ 2026-05-26)

### 1. FBO销毁崩溃 — VK_ERROR_DEVICE_LOST Heisenbug
**现象**: 编辑器随机崩溃，Vulkan返回VK_ERROR_DEVICE_LOST
**根因**: `apply_pending_offscreen_resize()`销毁FBO(framebuffer/image_view/memory)前没有调用`device_wait_idle()`。GPU仍在使用旧FBO资源时销毁→设备丢失
**修复**: 在销毁FBO资源前加`self.device.device_wait_idle()`
**文件**: `rhi-vulkan/src/ui_vulkan_renderer.rs`

### 2. 正方体不显示 — World存储类型不匹配
**现象**: Scene中create_cube创建的Entity不显示
**根因**: World组件存储用`Vec<u8>` raw bytes，MeshData反序列化时类型不对
**修复**: World存储改为`Box<dyn Any>` + `downcast_ref::<MeshData>().cloned()`，类型安全取值
**文件**: `core/src/ecs/world.rs`

### 3. Font texture layout transition — VK_VALIDATION警告
**现象**: Vulkan验证层报font texture layout transition错误
**根因**: Font atlas texture在写入新glyph后没有正确的pipeline barrier：SHADER_READ_ONLY→GENERAL→写入→SHADER_READ_ONLY
**修复**: 添加正确的pipeline barrier chain：写入后GENERAL→SHADER_READ_ONLY，确保GPU完成写入后再采样
**文件**: `rhi-vulkan/src/ui_vulkan_renderer.rs`

### 4. truncate_text逻辑修复
**现象**: tab文字截断时显示`…`(单个Unicode省略号)而非`...`；max_text_width<=0时返回空串
**根因**: `…`(U+2026)在fontdue中可能是placeholder字符；`max_text_width<=0`直接返回空串而非原始文字
**修复**: `…`→`"..."`(3个ASCII点，确保fontdue支持)；`max_text_width<=0`时返回原始文字(溢出比消失好)
**文件**: `ui/src/widgets/tab.rs`

### 5. Font atlas尺寸增大
**现象**: CJK+emoji字符多，4096px atlas不够用，新glyph无法放入
**修复**: atlas尺寸从4096→8192 (3处：atlas_width, atlas_height, atlas_texture size)
**文件**: `ui/src/font_atlas.rs`, `rhi-vulkan/src/ui_vulkan_renderer.rs`

### 6. swash COLR彩色emoji集成
**现象**: emoji显示为线框(fontdue不支持COLR/CBDT彩色emoji)
**根因**: fontdue只能渲染outline glyph(白色+alpha)，无法渲染COLR彩色位图emoji
**修复**: 引入swash crate，双轨制渲染：
  - fontdue: 普通文字(CJK/ASCII) → white+alpha → shader用text_color tint
  - swash: COLR/CBDT emoji → RGBA位图 → shader白色vertex → atlas颜色直通
**决策**: 用户明确拒绝Unicode符号替换和自绘Canvas图标方案，要求原生彩色emoji
**文件**: `ui/src/font_atlas.rs`(is_color_glyph, rasterize_color_glyph, layout_text_left/centered), `ui/Cargo.toml`(加swash依赖)

### 7. bearing_y符号修复 (swash 3处)
**现象**: emoji垂直位置偏移(偏上或偏下)
**根因**: swash `placement.top`是baseline上方距离(正值)，与fontdue bearing_y语义一致，不应取负
**修复**: `-(placement.top)` → `placement.top as f32` (3处：rasterize_color_glyph, layout_text_left, layout_text_centered)
**文件**: `ui/src/font_atlas.rs`

### 8. is_color_glyph per-character修复
**现象**: CJK文字也走swash彩色路径→显示为白色方块
**根因**: `is_color_glyph`检查整个font是否有COLR表，而非per-character判断。CJK字体有COLR表但CJK字符不是彩色glyph
**修复**: 改为per-character Unicode范围检查：emoji Unicode范围(如U+1F300-U+1F9FF)→走swash，其他→走fontdue
**文件**: `ui/src/font_atlas.rs`

### 9. UI pipeline depth_test关闭
**现象**: Vulkan验证层报VUID-04864错误(depth_test enabled但无depth attachment)
**根因**: UI render pass没有depth attachment，但pipeline创建了depth_test_enable=TRUE
**修复**: UI pipeline depth_test_enable=vk::FALSE, depth_write_enable=vk::FALSE
**文件**: `rhi-vulkan/src/ui_vulkan_renderer.rs`
**注意**: 此修复不是奇数tab文字不显示的根因，但修复了验证错误

### 10. Vertex stride mismatch — 交替tab文字不显示 ★★★ 核心bug
**现象**: TabWidget奇数标签页(1-based位置1,3,5)文字不显示，偶数标签页正常。两个TabWidget都受影响
**根因**: `DrawCommand::Triangle`、`DrawCommand::Line`、`DrawCommand::RectOutline`的vertices只有6 floats(x,y,r,g,b,a)，**缺少UV坐标**。但UI pipeline的vertex stride=32 bytes(8 floats)。Rect fill和Text vertices有8 floats(stride正确)，Triangle/Line/RectOutline只有6 floats导致：
  1. 含Triangle的batch在flat vertex buffer中不对齐stride 8(如132 floats→132/8=16.5，余4 floats)
  2. 后续Text batch的`first_vertex=(offset/8)as u32`截断计算(16.5→16)，pipeline从offset 128读取而非132
  3. Text vertices整体偏移4 floats→position/color/UV全部错位→文字乱码或不可见
  4. 交替模式：含Triangle的batch产生4-float余数→下一个Text错位→再下一个Rect+Triangle累积84 floats(84/8=10.5)→再下一个Text又错位...直到某个组合凑整对齐→Text正常
**修复**: 给Triangle、Line、RectOutline每个vertex加`0.0, 0.0` UV坐标，使所有vertex类型统一8 floats/vertex(stride 32 bytes)。Shader中`frag_uv==(0,0)`触发`out_color=frag_color`直通路径(不采样纹理)，与Rect fill vertices行为一致
**文件**: `rhi-vulkan/src/ui_vulkan_renderer.rs`
  - Triangle: `[p.x,p.y,r,g,b,a]` → `[p.x,p.y,r,g,b,a,0.0,0.0]` (3 vertices × 8 floats)
  - Line: `[x,y,r,g,b,a]` → `[x,y,r,g,b,a,0.0,0.0]` (6 vertices × 8 floats)  
  - RectOutline: `[x,y,r,g,b,a]` → `[x,y,r,g,b,a,0.0,0.0]` (8 vertices × 8 floats)
**用户确认**: 两个TabWidget所有标签页文字正常显示✅

### 11. ensure_text_rasterized兜底 — 常见特殊字符预光栅化
**现象**: 搜索placeholder"搜索"中的CJK字符可能不在widget的get_text()中，未被预光栅化
**修复**: `ensure_text_rasterized`在遍历widget前，先光栅化14个常见特殊字符(…↑↓←→📁📄✓✗★●■▶◀)在所有字号
**文件**: `ui/src/widget_tree.rs`

### 12. MeshVertex加UV + Shader纹理 + Per-entity texture
**现象**: Entity mesh渲染只有颜色，没有纹理贴图
**修复**: MeshVertex从8 floats(position+color)扩展到32 bytes(position+normal+uv)；rotation shader加texSampler采样；TextureCacheEntry支持per-entity texture
**文件**: `render/src/mesh.rs`, `rhi-vulkan/src/ui_vulkan_renderer.rs`, `shaders/rotation.frag`