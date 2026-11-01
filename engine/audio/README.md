# hezhou-audio

Hezhou 引擎 0.2 的**音频能力模块**（Rust crate `hezhou-audio`，对应 `engine/audio/`）。

目标：为 3D 动作游戏提供一套**通用、可测试、可组合**的音频能力——播放状态机、主音量/总线音量、
2D/3D 距离衰减、无声卡优雅降级、WAV/OGG 解码与资源缓存，以及 C#/FFI 接入草案。

> **ADR-001 已定稿（2026-11）**：音频后端采用 **cpal + 自研混音**（不选 kira），
> OGG 采用 **lewton**。二者以可选 feature 提供；默认构建仍保持零外部依赖，
> `cargo test -p hezhou-audio` 可离线通过、无需声卡。

## Features

| Feature | 依赖 | 作用 | 默认 |
|---|---|---|---|
| （无） | 无 | 纯 Rust 混音 + WAV 解码 + `NullBackend` 无声后端；离线/CI 可用 | ✅ |
| `cpal-backend` | cpal 0.15 | 真实音频输出设备；`default_backend()` 返回 `CpalBackend`，无设备/打开失败自动降级 | ❌ |
| `ogg` | lewton 0.10 | OGG/Vorbis 解码；自动注册到 `DecoderRegistry` | ❌ |

```bash
cd engine

# 1) 默认：零外部依赖，可离线
cargo test -p hezhou-audio

# 2) 真实设备后端 + OGG 解码
cargo test -p hezhou-audio --features cpal-backend,ogg

# 3) 只开其一也可以
cargo test -p hezhou-audio --features cpal-backend
cargo test -p hezhou-audio --features ogg

# 4) 质量门禁
cargo clippy -p hezhou-audio --all-targets --all-features
cargo fmt -p hezhou-audio -- --check
```

应用侧启用方式（`engine/examples` 或未来的 `engine/player`）：

```toml
[dependencies]
hezhou-audio = { path = "../audio", features = ["cpal-backend", "ogg"] }
```

`AudioEngine::new(default_backend())` 即可：启用 `cpal-backend` 时自动使用真实设备，
否则（或设备打开失败）自动使用无声后端，上层代码不需要区分。

## 目录结构

```
engine/audio/
├── Cargo.toml            # 可选依赖 cpal / lewton（默认不编译）
├── README.md             # 本文件
├── tests/assets/
│   ├── README.md                 # 测试资源来源说明
│   └── tone_440hz_100ms.ogg      # 自生成 0.1s 440Hz 正弦（OGG/Vorbis，~4KB）
└── src/
    ├── lib.rs            # 模块导出 + crate 文档 + doctest
    ├── error.rs          # AudioError / AudioResult
    ├── sound.rs          # SoundData（f32 交错 PCM）、正弦/静音生成
    ├── wav.rs            # 纯 Rust WAV 解码 + PCM16 编码（测试资源生成）
    ├── decoder.rs        # AudioDecoder trait、DecoderRegistry、OGG 容器探测
    ├── ogg_vorbis.rs     # [feature `ogg`] lewton OGG/Vorbis 解码器
    ├── cache.rs          # AssetCache（id -> Arc<SoundData>，命中统计）
    ├── bus.rs            # AudioBus（音量/静音/clamp 规则）
    ├── spatial.rs        # Vec3、Listener、Attenuation 衰减模型、声像 pan
    ├── mixer.rs          # Mixer：voice 状态机、重采样、混音、总线/主音量
    ├── backend.rs        # AudioBackend trait、NullBackend、MixerSink、默认后端工厂
    ├── backend_cpal.rs   # [feature `cpal-backend`] cpal 真实设备后端
    ├── engine.rs         # AudioEngine / SoundHandle / 播放选项 / 统计
    └── ffi.rs            # 最小 C ABI 草案（不接 FfiContext）
```

## 架构

```text
          ┌──────────────── AudioEngine（门面，&self 线程安全）───────────────┐
          │  AssetCache（资源缓存） · DecoderRegistry（WAV + [ogg] Vorbis）    │
          │  Mixer：voice 状态机 / 总线 / 主音量 / 3D 衰减 / 重采样 / 限幅     │
          └───────────────────────────┬──────────────────────────────────────┘
                                      │ MixerSink（共享内部锁，设备回调拉数据）
                   ┌──────────────────┴──────────────────┐
                   │ AudioBackend trait                  │
                   ├──────────────────┬──────────────────┤
                   │ NullBackend      │ CpalBackend       │
                   │ （无声降级/CI）  │ [cpal-backend]    │
                   └──────────────────┴──────────────────┘
```

关键设计：

- **单锁状态**：混音器/缓存/声部 id 在一把 `Mutex` 内；解码器注册表单独 `RwLock`，
  避免加载大文件时阻塞设备回调线程。
- **纯 Rust 混音**：`Mixer::render_interleaved` 输出 f32 交错立体声，可离线渲染测试；
  重采样为线性插值（`source_rate / output_rate` 步进）。
- **音频周期**：
  - 真实设备（`cpal-backend`）：独立线程持有 `cpal::Stream`，设备回调通过
    `MixerSink::render` 拉取混音数据；`AudioEngine::update(dt)` 返回 0（不重复推进）；
  - 无声后端（默认/降级）：由 `AudioEngine::update(dt)` 或 `advance_frames(n)` 推进，
    状态机确定性可测。
- **无声卡降级**：`AudioBackend::open` 失败 → 引擎自动换成 `NullBackend`，
  播放/暂停/循环/音量/3D 全逻辑继续工作，仅不发声；原因记录在
  `EngineStatus::degraded_reason`（如 `"cpal 后端打开失败: ..."`）。

## 快速开始（Rust）

```rust
use hezhou_audio::{default_backend, AudioEngine, PlayOptions, SoundData, Vec3};

// 启用 cpal-backend：真实设备（无设备自动降级）；未启用：无声后端
let engine = AudioEngine::new(default_backend());

// 1) 资源：程序化生成、WAV/OGG 文件或内存字节（按魔数自动选择解码器）
engine.load_sound("click", SoundData::sine("click", 880.0, 0.05, 48_000).unwrap());
// engine.load_audio_file("click", "assets/click.wav")?;
// engine.load_audio_file("bgm", "assets/theme.ogg")?;   // 需 --features ogg
// engine.load_from_memory("click", include_bytes!("assets/click.ogg"))?;

// 2) 播放
let click = engine.play("click").unwrap();
let roar = engine.play_3d("click", Vec3::new(10.0, 0.0, 0.0)).unwrap();
let bgm = engine.play_with(PlayOptions::new("click").looping(true).volume(0.5).bus("bgm")).unwrap();

// 3) 每帧推进：真实设备下返回 0（设备线程推进），无声后端下按 dt 推进
engine.update(1.0 / 60.0);

// 4) 全局控制 + 相机同步
engine.set_master_volume(0.8);
engine.set_bus_volume("bgm", 0.6);
engine.set_listener(hezhou_audio::Listener::default());

// 5) 状态机
click.pause();
click.resume();
click.stop();
# let _ = (roar, bgm);
```

> **集成提示**：编辑器/Player 每帧调用 `engine.update(dt)` 与 `set_listener(...)` 即可；
> 真实设备模式下 `update` 是空操作（返回 0），不需要分支处理。

## API 摘要

### `AudioEngine`（`engine.rs`）

| 方法 | 说明 |
|---|---|
| `new(backend)` / `with_config(backend, config)` | 创建引擎；后端打开失败自动降级 |
| `null()` | 始终无声引擎（离线/CI；即使启用 cpal feature 也不打开设备） |
| `status()` / `audio_output_available()` / `sample_rate()` | 后端与降级状态 |
| `load_audio_file(id, path)` | 解码 + 缓存（WAV；`ogg` feature 下含 OGG） |
| `load_wav_file(id, path)` | `load_audio_file` 的兼容别名 |
| `load_from_memory(id, bytes)` | 从内存解码 + 缓存（同 id 命中不重复解码） |
| `load_sound(id, SoundData)` / `sound(id)` / `unload(id)` / `cached_sound_ids()` | 直接管理资源与缓存 |
| `play(id)` / `play_3d(id, pos)` / `play_with(PlayOptions)` | 播放，返回 `SoundHandle` |
| `set_master_volume` / `master_volume` / `set_muted` / `is_muted` | 主音量与静音 |
| `set_bus_volume` / `bus_volume` / `set_bus_muted` / `bus_names` | 总线（不存在自动创建） |
| `set_listener` / `listener` | 3D 监听者（相机） |
| `update(dt)` | 推进混音时钟（无声后端）；返回推进帧数 |
| `advance_frames(n)` / `render_interleaved(buf)` | 手动推进/离线渲染（测试、自定义后端） |
| `stop_all` / `active_voice_count` / `stats` / `voice_handle(id)` | 运维与统计 |
| `register_decoder` / `decoder_names` | 注册额外解码器（启用 `ogg` 时 Vorbis 已内置） |

### `SoundHandle`（可克隆）

`state()`、`is_valid()`、`pause()`、`resume()`、`stop()`、`set_volume`/`volume`、
`set_looping`/`is_looping`、`set_position`/`position_3d`、`set_bus`/`bus`、
`position_secs()`、`duration_secs()`、`sound_name()`、`id()`。

**句柄释放语义**：丢弃句柄**不会**停止播放（声部由引擎拥有）；声部自然结束或 stop 后句柄自动失效，
所有查询安全返回默认值（`is_valid() == false`、`state() == Stopped`）。

### `PlaybackState`

`Playing` / `Paused` / `Stopped`；`as_i32()` 供 FFI/C# 使用（0/1/2）。

### `AudioBus`

`volume`/`set_volume`/`muted`/`set_muted`/`effective_gain`；音量统一 clamp 到 `[0,1]`，
`NaN` 视为 0（静音）。最终增益 = `master × bus × voice × 3D 衰减`。

### 3D 空间（`spatial.rs`）

- 右手系：`+X` 右、`+Y` 上、`-Z` 前（`Listener::default`）。
- `Attenuation::gain_at(d)`：
  - `d <= min_distance` 或 `None` 模型 → `1.0`；
  - `d >= max_distance` → `0.0`（硬截止）；
  - `Linear`：`1 - t`；`Inverse`：`min/(min + rolloff*(d-min))` 再乘线性收尾 `(1-t)`；
  - `Exponential`：`(1-t)^rolloff`；其中 `t = (d-min)/(max-min)`。
- `pan`：声源方向在监听者右向量上的投影 `[-1,1]`；`pan_to_gains` 常功率归一化
  （居中 `(1,1)`，硬左 `(√2,0)`，硬右 `(0,√2)`）。

### 解码与缓存

- `WavDecoder`（始终）：PCM 8/16/24/32、float 32/64、`WAVE_FORMAT_EXTENSIBLE`、
  跳过未知 chunk、帧对齐校验；
- `LewtonOggDecoder`（`--features ogg`）：OGG/Vorbis，自动注册；
- `DecoderRegistry::decode` 在缺少解码器时返回**可读错误**（含检测到的格式/编解码器名称）；
- `AssetCache` 统计 `hits/misses/bytes`，`AudioEngine::stats()` 汇总。

## 真实设备后端（cpal，`--features cpal-backend`）

实现位于 `src/backend_cpal.rs`，行为约定：

- **线程模型**：`CpalBackend::open` 启动 `hezhou-audio-cpal` 线程，在其中创建并持有
  `cpal::Stream`；设备回调线程通过 `MixerSink::render(&mut [f32])` 拉取混音数据。
  这样不依赖平台相关的 `cpal::Stream: Send`，也避免阻塞引擎主线程。
- **格式**：仅使用 **f32 立体声**输出，采样率取引擎配置（默认 48kHz）；
  在 `supported_output_configs()` 中精确寻找 `F32/2ch/目标采样率`；
  找不到时返回可读错误（含设备可用配置列表），引擎随即降级为无声后端。
- **设备选择**：`default_host().default_output_device()`（Windows WASAPI / Linux ALSA
  默认设备）。
- **生命周期**：`close()` 置停止位并 `join` 线程、在创建线程内 drop stream；
  `Drop` 兜底自动关闭（防止调用方忘记 close 导致线程泄漏）。
- **降级路径**：没有设备、格式不支持、build/play 失败、采样率非法规格 → `BackendError`
  → 引擎降级 `NullBackend`（`EngineStatus::degraded_reason` 记录原因），不 panic。

测试策略（CI 不需要声卡）：

- `open` 的参数校验（非法采样率/声道）**不触碰设备**，可离线断言；
- 真实设备集成测试 `engine_with_cpal_backend_is_device_optional` 同时接受两种环境：
  有声卡 → `backend=cpal / mode=Device`；无声卡 → `backend=null` 且降级原因可读；
- 本机（Windows，Realtek 默认设备）实测：`backend=cpal, mode=Device`，真实设备打开成功。

> 扩展其他后端（如未来 ASIO/专用设备）：实现 `AudioBackend` 并在
> `backend::default_backend()` 按平台选择即可，引擎与混音器无需改动。

## OGG/Vorbis 解码（`--features ogg`）

实现位于 `src/ogg_vorbis.rs`：

- `LewtonOggDecoder` 实现 `AudioDecoder`，`supports` 基于 `OggS` 魔数；
- 启用 `ogg` 后 `DecoderRegistry::with_builtin()` 自动注册，引擎无需额外配置；
- 解码输出 f32 交错 `SoundData`（i16 / 32768 归一化），采样率/声道数取自 Vorbis 头；
- 损坏/截断文件返回 `AudioError::InvalidData`（含 "OGG/Vorbis ..." 上下文），不 panic；
- 测试资源 `tests/assets/tone_440hz_100ms.ogg`：numpy 正弦 + libsndfile 生成，
  无第三方版权内容（生成脚本见 `tests/assets/README.md`）。

未启用 `ogg` 时保持原行为：OGG 数据得到“检测到 OGG/Vorbis……请注册解码器”的可读错误。

## C# / FFI 集成点

`src/ffi.rs` 提供**最小 C ABI 草案**（未接入现有 283 字段 `FfiContext`）。
导出函数（`hezhou_audio_*`）：

| 分类 | 函数 |
|---|---|
| 生命周期 | `engine_create` / `engine_destroy` / `engine_last_error` / `engine_backend_name` / `engine_is_silent` |
| 资源 | `load_audio_file` / `load_wav_file`(兼容别名) / `load_wav_memory` / `load_tone` / `unload` |
| 播放 | `play` / `play_3d` / `voice_state` / `voice_command`(1 暂停/2 恢复/3 停止) |
| 声部 | `voice_set_volume` / `voice_set_loop` / `voice_set_position` |
| 全局 | `set_master_volume` / `master_volume` / `set_muted` / `set_bus_volume` / `set_bus_muted` / `set_listener` |
| 帧推进/统计 | `update` / `active_voice_count` / `stop_all` |

C# 侧（示意，字符串按 UTF-8）：

```csharp
using System;
using System.Runtime.InteropServices;

internal static class HzAudio
{
    private const string Lib = "hezhou_audio";

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr hezhou_audio_engine_create();

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void hezhou_audio_engine_destroy(IntPtr engine);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int hezhou_audio_load_audio_file(
        IntPtr engine, [MarshalAs(UnmanagedType.LPUTF8Str)] string id, [MarshalAs(UnmanagedType.LPUTF8Str)] string path);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern ulong hezhou_audio_play(IntPtr engine, [MarshalAs(UnmanagedType.LPUTF8Str)] string soundId);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int hezhou_audio_voice_command(IntPtr engine, ulong voiceId, int command);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void hezhou_audio_set_master_volume(IntPtr engine, float volume);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern ulong hezhou_audio_update(IntPtr engine, float dtSeconds);
}
```

正式接入 `FfiContext` 时建议：

- 在 C# `UI.cs` 同文件（项目约定）新增 delegate 与 P/Invoke 包装；
- 在 `scripting/src/ffi_context/` 增加对应函数指针字段，并在 `editor/mod.rs` 组装；
- 命名沿用现有风格：Rust `audio_engine_create`、C# `AudioEngineCreate`；
- 本模块未修改 `FfiContext`，避免与并行模块冲突，交由集成负责人统一接线。

## 测试

```bash
cd engine
cargo test -p hezhou-audio                          # 默认零依赖（77 + 2 doctest）
cargo test -p hezhou-audio --features cpal-backend,ogg   # 真实后端 + OGG（86 + 3 doctest）
cargo test -p hezhou-audio --features cpal-backend
cargo test -p hezhou-audio --features ogg
cargo clippy -p hezhou-audio --all-targets --all-features
cargo fmt -p hezhou-audio -- --check
```

覆盖矩阵：

| 主题 | 覆盖 |
|---|---|
| 状态机 | play/pause/resume/stop、重复操作、自然结束、循环、无效/已释放句柄 |
| 音量 | 主音量、总线音量、静音、NaN/clamp、渲染 RMS 比例验证 |
| 3D 衰减 | 距离单调性、min/max 边界、各衰减模型、rolloff、声像 pan、监听者移动/旋转 |
| 无声卡降级 | 后端打开失败的自动降级、降级后播放/推进/结束、`status` 原因 |
| 句柄释放 | 丢弃句柄不停止播放、引擎销毁后句柄安全、声部结束后自动失效 |
| 资源 | WAV 各编码/位深/多声道/extensible/异常 chunk、编码往返、缓存命中/未命中/卸载、错误路径 |
| 混音 | 重采样结束时机、循环回绕、多声部叠加限幅、采样插值、静音时仍推进位置 |
| 后端 | NullBackend 生命周期、MixerSink 渲染（真实后端回调路径）、自动/手动推进语义、cpal 参数校验与设备可选集成测试 |
| OGG（feature） | 真实文件解码（采样率/声道/幅度）、自动注册、幂等注册、文件/内存加载、截断/损坏错误 |
| FFI | 全生命周期、错误上报、空指针安全 |

**无声卡策略**：声音输出只依赖 `AudioBackend`；默认 `NullBackend` 不打开任何设备，
状态机由 `update(dt)`/`render_interleaved` 确定性推进，因此 CI 无需声卡；
真实设备打开失败也会自动降级，不会 panic。

## CI 配置建议

- 默认 `rust-tests` 保持现状：`cargo test -p hezhou-audio`（不编译 cpal/lewton，
  无需系统音频开发库）；
- 可选新增一个 **audio-feature** 测试步骤：
  `cargo test -p hezhou-audio --features cpal-backend,ogg`
  - **Windows runner**：无需额外系统依赖；
  - **Ubuntu runner**：cpal 编译需要 ALSA 开发头文件，`apt-get install -y libasound2-dev`；
    测试**不要求**真实声卡（无设备时自动验证降级路径）；
- 注意：首次启用 feature 会更新 `engine/Cargo.lock`（新增 cpal/lewton 及其传递依赖），
  由集成负责人统一提交。

## 已知限制 / 后续工作

- 真实设备后端只支持 **f32 立体声**输出，且要求设备支持引擎配置的采样率
  （不支持时会降级为无声，而不是自动重采样到设备采样率）；
- 无流式播放（长音频整段解码进内存）、无精确 seek、无变速/变调；
- 3D 无多普勒/低通/混响/HRTF/Occlusion；总线只有增益/静音，无 EQ/压缩/发送效果；
- 多声道（>2）目前取前两路，未做规范下混；
- OGG 仅 Vorbis（lewton 只解码 Vorbis；Opus/FLAC 仍会得到可读的“未注册解码器”错误）；
- 大文件解码在调用线程同步完成（已尽量不阻塞设备回调，但会阻塞调用线程）。

## 依赖策略说明

- ADR-001 定稿为 cpal + 自研混音、OGG 用 lewton；两者均为 **optional**，
  默认构建保持零外部依赖（离线可用、不要求声卡、不增加默认 CI 负担）；
- 依赖拉取已验证可用：`cpal 0.15.3`、`lewton 0.10.2`（含 `ogg 0.8` 等传递依赖）；
- 启用 feature 后 `mixer`/`engine`/`spatial` 等核心代码不变，只是多了一个
  `AudioBackend` 实现和一个 `AudioDecoder` 实现，保证接口与 task-1 版本完全兼容。
