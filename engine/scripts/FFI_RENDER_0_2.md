# 0.2 渲染质量 / PBR 编辑器 FFI 接入（task-15）

本文档记录「渲染器 ↔ 编辑器 C#」的 FFI 握手，以及后续 audio / asset / build 能力
接入编辑器的统一做法。实现位置：

| 层 | 文件 |
|---|---|
| Rust 类型别名 | `engine/scripting/src/ffi_context/renderer.rs` |
| Rust FfiContext 字段（tail，顺序 = C# 顺序） | `engine/scripting/src/ffi_context/mod.rs` |
| Rust FFI 实现 | `engine/examples/src/editor/ffi_impl.rs` |
| Rust 装配（transmute/函数指针） | `engine/examples/src/editor/mod.rs` |
| C# delegate + IntPtr 字段 + static 包装 | `engine/scripts/UI.cs` |
| C# 工具栏/状态栏接入 | `engine/scripts/EditorScript.View.cs`、`EditorScript.cs`、`EditorScript.State.cs` |
| 契约校验 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 286`） |

## 1. 渲染质量 / PBR 三件套

| FFI | 签名 | 语义 |
|---|---|---|
| `renderer_set_quality_preset` | `extern "C" fn(i32) -> i32` | `0`=Low `1`=Medium `2`=High `3`=Legacy（默认，Blinn-Phong 回退）；返回 `0` 成功、`-1` renderer 未初始化、`-2` 非法档位 |
| `renderer_pbr_active` | `extern "C" fn() -> i32` | PBR draw path 是否已创建（`1`=是） |
| `renderer_get_pbr_summary` | `extern "C" fn(*mut c_char, usize) -> usize` | 写入 NUL 结尾摘要，返回字节数（不含 `\0`）；`0` 表示不可用 |

> 档位映射说明：`rhi-vulkan` 底层 `RenderQualityPreset` 的整数 3 是 `Custom`；编辑器 FFI 把
> 3 定为 **Legacy**（`set_render_settings(Default::default())`），以满足“默认 legacy、可一键回退”
> 的编辑器体验。需要 Custom 高级调参时，后续可增加 `renderer_set_render_settings` 系列 FFI，
> 不改变本表已有整数语义。

Rust 侧对应已就绪的渲染器 API（`hezhou_rhi_vulkan::UIVulkanRenderer`，本任务未修改）：

- `set_render_quality_preset(u32)`：0/1/2 → `RenderQualityPreset`，下一帧自动重建 PBR 通道；
- `set_render_settings(Default::default())`：`3` → Legacy，恢复 `RenderSettings::default()`
  （Blinn-Phong + FXAA，默认零回归）；
- `pbr_forward_active()` / `pbr_draw_path_summary()` / `render_quality_summary()`：
  状态栏摘要来源；PBR 尚未到下一帧时返回 `PBR pending ...`。

C# 用法：

```csharp
UI.RendererSetQualityPreset(2);              // High（PBR）
bool pbr = UI.RendererPbrActive();           // 下一帧后 true
string summary = UI.RendererGetPbrSummary(); // 状态栏文本
UI.ApplyQualityPreset(3);                    // EditorScript 封装：Legacy 回退 + 日志
```

编辑器 UI：

- 工具栏「画质: Legacy」按钮循环 `Legacy → Low → Medium → High → Legacy`；
- 状态栏「渲染: …」显示 `renderer_get_pbr_summary` 摘要（截断 44 字符）；
- 切换成功/失败都会写 Console 日志与状态栏提示；失败保持原档位，不崩溃；
- 默认 `_qualityPreset = 3`（Legacy），启动时不主动调用 FFI，保持 0.1 视觉零回归。
- 启动自检（`TestRunner.TestRendererQualityFfi`）覆盖：默认 Legacy 摘要/PBR 未激活、
  切 High 返回 0 且摘要 `quality=high`、非法档位 99 返回 -2、切回 Legacy、
  `EditorScript.ApplyQualityPreset(Low)` 工具栏同一路径；编辑器启动日志应出现
  `PASS=44, FAIL=0`（原 35 项 + 本功能 9 项）。

## 2. 新增 FFI 的固定步骤（audio / asset / build 后续照此接入）

1. `engine/scripting/src/ffi_context/<domain>.rs`：加 `XxxFn = extern "C" fn(...)` 类型别名；
2. `engine/scripting/src/ffi_context/mod.rs`：在 `FfiContext` **末尾**追加字段
   （tail 顺序必须与 C# 一致，避免运行时 `Marshal.PtrToStructure` 错位）；
3. `engine/examples/src/editor/ffi_impl.rs`：实现 `#[unsafe(no_mangle)] pub extern "C" fn`；
   失败路径只记录 `dfx_warn!/dfx_error!` 并返回错误码，禁止 panic/崩溃；
4. `engine/examples/src/editor/mod.rs`：在 `FfiContext { ... }` 装配处加字段；
5. `engine/scripts/UI.cs`：delegate + `FfiContext` IntPtr 字段（**delegate/字段只能在 UI.cs**）
   + `public static` 包装方法（字符串 buffer 用 `Marshal.AllocHGlobal` + `PtrToStringAnsi`）；
6. 需要出现在工具栏/状态栏时，在 `EditorScript.View.cs` 建控件、`EditorScript.cs` 刷状态、
   `EditorScript.State.cs` 加静态字段；
7. 更新 `engine/scripts/check_ffi_contract.py` 的 `EXPECTED_FIELD_COUNT`，跑
   `python engine/scripts/check_ffi_contract.py`；
8. `powershell engine/scripts/build_mono.ps1` 编译 C#，`cargo check -p hezhou-examples --features mono`
   验证 Rust 侧装配。

### 后续接入点建议

| 能力 | 建议 FFI（示意） | 备注 |
|---|---|---|
| audio | `audio_set_master_volume(f32)`、`audio_play_2d(path_ptr) -> u64`、`audio_get_summary(buf,len)` | 后端在 `hezhou-audio`；编辑器状态栏显示总线/设备状态 |
| asset | `asset_import(path_ptr) -> i32`、`asset_get_import_summary(buf,len)` | `hezhou-asset` 的 `AssetDatabase::import_gltf` 语义；导入进度/错误用状态栏 |
| build | `build_export_player(config_ptr) -> i32`、`build_get_progress(buf,len)` | `hezhou-player` 构建器；长任务建议异步 + 轮询进度，FFI 只做发起/查询 |

**约束**：以上新增都必须沿用本文件的 tail 追加原则；不要修改既有字段顺序；
`check_ffi_contract.py` 会在字段数/三方集合不一致时直接失败。
