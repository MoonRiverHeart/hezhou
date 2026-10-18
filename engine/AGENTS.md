# Hezhou Engine Knowledge Base

**Generated:** 2026-05-25
**Commit:** afb4f7d
**Branch:** main

## Overview
Cross-platform game engine: Rust core + C# scripting (Mono JIT/NativeAOT). Vulkan rendering, custom UI system with 18 widget types, ECS Scene, asset library, project file system. Inverted dependency: core depends on scripting (for FfiContext types).

## Structure
```
engine/
├── 🧩 core/           # ECS Scene, Transform, AssetLibrary, Project (see core/src/AGENTS.md)
├── 🖼️ ui/             # Widget system, Canvas, FontAtlas, EventDispatcher (see ui/src/AGENTS.md)
│   └── 📦 widgets/    # 18 widget types (see widgets/AGENTS.md)
│   └── 🔗 thunk/      # Callback dispatch (see ui/src/AGENTS.md)
│   └── 🌉 ffi/        # FFI bridge layer (see ui/src/AGENTS.md)
├── 🎨 rhi/            # Abstract RHI trait layer (see rhi/src/AGENTS.md)
├── ⚡ rhi-vulkan/     # Vulkan renderer impl (see rhi-vulkan/src/AGENTS.md)
├── 🎥 render/         # RenderEngine + Camera/Mesh/Texture (see render/src/AGENTS.md)
├── 📜 scripting/      # Mono JIT executor, FfiContext (see scripting/src/AGENTS.md)
│   └── 🏗️ ffi_context/ # 283-field FfiContext struct (see scripting/src/ffi_context/AGENTS.md)
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
- **FfiContext**: 283-field flat #[repr(C)] struct of function pointers, assembled in editor/mod.rs with transmute
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

# Layered tests (UT + fuzz smoke + FFI contract + C# build + E2E autotest)
powershell -ExecutionPolicy Bypass -File scripts/run_tests.ps1 -Offline
powershell -ExecutionPolicy Bypass -File scripts/run_autotest.ps1 -Mode random -Steps 30 -Duration 10 -Seed 42
powershell -ExecutionPolicy Bypass -File scripts/run_autotest.ps1 -Mode intent

# Fuzz (nightly / cargo-fuzz)
cd fuzz
cargo +nightly fuzz run project_json -- -max_total_time=60

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
- CI: `.github/workflows/ci.yml`（Rust UT/UI/DFX、FFI 契约、fuzz harness、C# 编译）；E2E 需本地 Vulkan/GPU
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

### 13. 工程保存/加载往返修复 (2026-10-18)
**现象**: 加载工程时旧Entity未清除导致重复；方向光/空Entity被保存成Cube；编辑器传入.json文件路径但`Project::load`只接受目录；Ctrl+S/Ctrl+O/Ctrl+N只在菜单里显示但不生效。
**修复**:
- `World::clear()` + `Scene::clear()`，`Project::sync_to_scene()`先清空再加载
- `ProjectEntity`新增`visible`/`light`字段（serde default兼容旧工程），保存`"DirectionalLight"`/`"None"`类型
- `Project::load()`同时接受工程目录和直接json文件；Project记录实际工程文件路径
- 编辑器新增Ctrl+S/Shift+S/O/N，加载后刷新实体树/选中态/属性面板
- 右键删除改为与Delete键相同的确认流程
**测试**: `cargo test -p hezhou-core --offline`（`project_roundtrip.rs` 2个用例）；编辑器启动自检 `PASS=35, FAIL=0`
**文件**: `core/src/project.rs`, `core/src/ecs/scene.rs`, `core/src/ecs/world.rs`, `scripts/EditorScript.Presenter.cs`

### 14. 分层测试 / DFX / 编辑器 P0 (2026-10-18)
**新增测试**:
- UT: `core/tests/{scene_properties,scene_hierarchy_cycles,project_fuzz_smoke}.rs`、`ui/tests/widget_tree_hit_test.rs`
- 接口: `core/tests/{ffi_contract,project_ffi_contract}.rs` + `scripts/check_ffi_contract.py`（283 个 FfiContext 字段三方一致）
- FUZZ: `engine/fuzz/` cargo-fuzz target + 3000 组畸形 JSON 确定性 smoke
- E2E: `scripts/run_autotest.ps1`、`UITestRunner.Report.cs`、`TestTelemetry.cs`
  - random/sequential/stress/intent 模式
  - report.json / summary.txt / ui_tree.txt / final.png / DONE
  - 帧时间、FPS、卡顿、Warn/Error/Fatal 自动采集
- 意图: `IntentTestRunner.cs` 覆盖新场景、创建/选择/删除、父子层级、撤销重做、多选批量属性、Console、运行/暂停状态
**编辑器 P0**:
- 撤销/重做：场景 JSON 快照 + `scene_snapshot_json/scene_restore_json` FFI，ID 稳定的恢复，Ctrl+Z/Y
- 父子层级：TreeView 递归重建、右键设置/脱离父级、循环检测、删除父实体提升子实体
- 多选：Ctrl+Click、批量属性编辑
- Console：工具栏 Console 按钮，读取 C# 日志 ring buffer（最近 300 条）
- 加载工程前清理旧 C# 脚本实例（`CleanupScriptInstancesBeforeSceneReplace`）
**P1 追加**:
- 多选真正驱动 renderer outline：`scene_add_to_selection/remove/toggle` FFI + `Scene` additive selection API
- 多选删除（单个撤销快照 + 删除数量确认）
- 撤销/重做工具栏计数；项目树实体搜索；资产模型搜索；Console 级别过滤/搜索/30 帧自动刷新
- `visual` E2E 模式：脚本绑定/Console/搜索/多选 5 张关键截图 + `check_visual.py` 尺寸/非空校验
- CI：`.github/workflows/ci.yml`（Rust/FFI/fuzz harness/C#）
**文件**: `scripts/{run_tests,run_autotest,check_ffi_contract,check_autotest_report}.*`, `docs/ai_intent_testing.md`, `core/src/{asset_ffi,project,scene_ffi}.rs`, `core/src/ecs/*`, `ui/src/ffi/misc.rs`, `scripts/*.cs`

### 15. UI 视觉回归修复 (2026-10-18)
**问题**: 点击绑定没反应；RotationScript 无显示；资产搜索框填满面板；属性 Tab/配置行溢出；Console 对话框越界；新建场景残留旧脚本绑定行。
**根因**:
- `TestRunner` 自检后遗留 `_sourceFilesModified=true`，每次绑定触发 hot reload
- reload 重建 C# assembly，pending 静态字段与 `_selectedEntityId` 清零，绑定请求/面板刷新丢失
**修复**:
- 绑定即时生效：先 attach + CreateInstance + 刷新面板；仅 `.cs` 修改后才 reload
- `TestRunner` 自检结束清除 `_sourceFilesModified`
- hot reload 用 `reset_script_instance_ids()`，reload 后从 Rust Scene 恢复选中项
**其他修复**:
- `RotationScript.cs` 增加逐实体 `RotationScript` 类并注册，保留旧 `RotationController`
- 资产模型 Tab: `ScrollView → VStack → InputField + GridView`
- `TabWidget::truncate_text` 先判断完整宽度；属性面板加宽，Min/Max/Step/Init 两行布局
- Console Dialog 680x360；`ClearScriptBindingUI` 按容器子节点清理
- 工具栏深蓝灰主题 + 运行绿/编辑器蓝/橙色强调线
**视觉验证**: `run_autotest.ps1 -Mode visual` 输出 5 张截图，`check_visual.py` 校验尺寸/非空；人工检查确认绑定行、Console、搜索框、属性面板、多选

### 16. P0 双主题 Design Tokens (2026-10-18)
- Rust `ui/src/theme.rs`：Dark/Light 语义色板；C# `UI.Theme.cs` 只用 ThemeRole。
- 工具栏“浅色/深色”切换，`.hezhou_theme` 持久化；切换时
  `ui_clear_widget_tree_keep_callbacks` 重建 UI，保留 Scene/选中/Tab。
- 基础控件与编辑器外壳颜色迁移；属性页从 ScrollView 改 VStack，规避隐藏 Tab
  首次显示的 0 尺寸问题。
- `visual` 7 张截图（含双主题），FFI 283 字段。

### 17. SVG 图标系统 (2026-10-18)
- `assets/icons/*.svg` → `scripts/gen_icons.py` → `ui/src/icons.rs` + `scripts/IconIds.cs`
- 新增 `IconWidget`（矢量线段，按主题 tint，支持任意 DPI）
- FFI: `ui_create_icon_in_parent` / `ui_icon_set_color` / `ui_icon_count`
- 工具栏、游戏预览、属性面板标题已接入图标；FFI 283 字段

### 18. P0 Widget Gallery (2026-10-18)
- `EditorScript.WidgetGallery.cs` 整页展示 Buttons/Inputs/Icons/Tabs/Colors。
- 打开时隐藏主 SplitView/状态栏，关闭恢复；不破坏 Scene/选中。
- visual 扩展为 10 张截图，含 Dark/Light Gallery 与恢复验证。

### 19. Dock v1 布局持久化 (2026-10-18)
- `EditorScript.Dock.cs` 保存/恢复 SplitView 比例、Tab、右侧最大化到 `.hezhou_layout`。
- 右侧面板标题栏最大化按钮；主题切换/resize 后重新应用。
- visual 12 张，含最大化与恢复。
- v2：Tab 拖拽、跨区域停靠、浮动面板。

### 20. Shift 范围多选 (2026-10-18)
- `_entityDisplayOrder` + `_selectionAnchorEntityId`；Shift 点击按树可见顺序范围选择。
- 同步 Rust selection / C# 多选列表 / TreeView 高亮 / 属性面板。
- intent 10/10；visual 13 张（新增 `13_shift_range_select.png`）。

### 21. Command Palette (2026-10-18)
- `EditorScript.CommandPalette.cs`：Ctrl+Shift+P 打开，输入过滤，Esc 关闭。
- 11 个命令：新建/保存/撤销/重做/运行暂停/Console/主题/Gallery/Dock 最大化/重置/打开项目。
- intent 11/11；visual 15 张（新增 14/15 命令面板与关闭恢复）。

### 22. 状态栏 v2 + CI 固化 (2026-10-18)
- 状态栏：选择数量、主题、错误数即时更新。
- CI：Rust 1.95.0 / Mono 6.12.0.199 / Python 3.11；CMake generator 与并行度固定；
  rust-cache；图标生成一致性检查；workflow_dispatch/并发/超时。

### 23. 资产缩略图 (2026-10-18)
- GridView `icon:<id>` 缩略图 + 按资产名 fallback。
- Cube/Sphere/Plane/Cylinder/CornellBox 显示矢量图标；FFI `ui_grid_view_set_item_thumbnail`，字段 275。

### 24. Console 源码跳转 (2026-10-18)
- Log Warn/Error/Fatal 带 CallerFilePath/LineNumber；TestTelemetry 保存来源。
- Console 显示 `[文件:行]`，新增“跳转源码”按钮。
- TextEdit `set_cursor_to_line` + FFI `ui_text_edit_set_cursor_line`；FFI 283 字段。

### 25. Inspector 属性行重做 (2026-10-18)
- 单行 Label + 输入控件；Float3 三轴同排；长属性名截断。
- visual 16 张（新增 `16_inspector_rows.png`）。

### 26. Hierarchy 拖拽 reparent (2026-10-18)
- 全局鼠标拖拽把已选实体拖到目标节点，释放后 SetParent；循环层级被拒绝。
- intent 13/13；visual 17 张（新增 `17_hierarchy_drag.png`）。

### 27. Asset Browser 材质列表 (2026-10-18)
- 材质 Tab 从 AssetLibrary Materials 分类读取并列出。
- intent 14/14；visual 18 张（新增 `18_material_tab.png`）。
- 材质拖放/赋给实体仍未实现。

### 28. Gizmo v1 平移 (2026-10-18)
- 预览区 X/Y/Z 轴条，拖拽更新实体位置；intent 15/15；visual 19 张。
- 轴条位置基于 PreviewWindow 布局，投影中心跟随未完成。

### 29. 材质赋给实体 (2026-10-18)
- Scene renderable material 读写 + FFI；材质 Tab 按钮点击/拖拽赋材质。
- FFI 283 字段；intent 16/16；visual 20 张（新增 `20_material_assign.png`）。

### 30. Gizmo 缩放/旋转模式 (2026-10-18)
- Ctrl+G 循环平移/缩放/旋转；缩放/旋转测试通过；intent 17/17；visual 21 张。

### 31. Dock v2 Tab 拖拽重排 (2026-10-18)
- TabWidget reorderable 拖拽重排，同步 children 顺序；资产 Tab 开启。
- FFI 283 字段；intent 18/18；visual 22 张（新增 `22_tab_reorder.png`）。
- 跨区域停靠/浮动面板仍未实现。

### 32. Dock v2 面板收起/展开 (2026-10-18)
- 项目/资产面板收起与恢复，状态持久化；Command Palette 命令。
- intent 19/19；visual 24 张；跨区域停靠/浮动未实现。

### 33. Dock v2 浮动资产面板 (2026-10-18)
- 浮动 Dialog 资产面板 + Command Palette 命令；intent 20/20；visual 26 张。
- 可拖拽停靠的浮动 Dock 仍未实现。

### 34. Dock v2 浮动面板拖动 (2026-10-18)
- 浮动资产面板可拖动；intent 21/21；visual 27 张。
- 跨区域停靠仍未实现。

### 35. CI 本地镜像校验 (2026-10-18)
- `scripts/ci_local.ps1` 本地复刻 CI 步骤（离线），执行通过。
- GitHub runner 实跑仍待推送。

### 36. 材质赋值持久化 (2026-10-18)
- ProjectEntity.material 保存/加载；project_roundtrip 覆盖材质。

### 37. Asset Browser 纹理 Tab (2026-10-18)
- Textures 分类 + 程序化 solid/checker/UV 缩略图；intent 22/22；visual 28 张。

### 38. Dock v2 跨区域停靠（纹理页） (2026-10-18)
- 纹理页命令式停靠到右侧属性 Tab，状态持久化；intent 23/23；visual 30 张。
- 拖拽式跨区域停靠未实现。

### 39. Gizmo 轴条居中 (2026-10-18)
- Panel 被 PreviewWindow 覆盖；改用 Button 轴条并居中；visual 30 张。

### 40. Gizmo 跟随实体投影 (2026-10-18)
- C# 实现相机投影，Gizmo 轴条中心跟随选中实体；visual 30 张。

### 41. 文件纹理缩略图 (2026-10-18)
- BMP 解码降采样 8x8，GridView `pixels:` 绘制；intent 24/24；visual 31 张。

### 42. 拖拽式跨区域 Dock (2026-10-18)
- TabWidget drag-end 回调 + C# 区域判定；纹理页可拖到右侧/拖回。
- FFI 283 字段；intent 25/25；visual 33 张。

### 43. 引擎基础：ECS 系统 / 物理碰撞 / 几何射线 (2026-10-18)
- World::update 驱动 SystemScheduler；LifetimeComponent/LifetimeSystem。
- PhysicsWorld AABB 碰撞检测 + CollisionEvent + 简单分离。
- BoundingBox::intersects_ray 射线求交；Rust 测试通过。

### 44. 字体清晰度与控件风格 (2026-10-18)
- 修复 3x 字形未降采样导致的发虚；alpha gamma、像素对齐、默认字号提升。
- 圆角矩形绘制、Button 语义色 hover/pressed、Tab accent 指示条。
- Inspector 标签/输入宽度调整；visual 33 / intent 25 全通过。

### 45. PNG 完整解码 + UI 纹理上传 (2026-10-18)
- FontAtlas 解码 PNG 写入图集并注册 ImageRegion；GridView `img:<id>` 采样。
- FFI 283 字段；intent 26/26；visual 33 张。

### 46. 3D renderer Gizmo 轴线段 (2026-10-18)
- RasterPipeline 实体后绘制三轴薄四边形；outline pipeline + outline_color 分轴着色。
- FFI 283 字段；UI Button 仅作透明命中区域；visual 33 / intent 26 全通过。

### 47. 重力 / 角色 / 动画 / 天空盒 (2026-10-18)
- CharacterController 重力/跳跃/地面吸附；AnimationClip + AnimationSystem。
- 全屏天空盒渐变 pipeline；视觉测试 33 / 意图 26 全通过。
