# Player 最小 skinned fixture

用于验证 Player window host 消费真实 cook 导入蒙皮资产（task-29）与音频/资产 API（task-35）：

```text
cook_manifest.json              # gltf + mesh/skeleton/animation 子资产 + audio entry
assets/scenes/<uuid>.scene      # 起始场景 Project JSON（fallback cube + directional light）
assets/audio/test.wav           # 最小 PCM WAV 音频资产（task-35）
assets/imported/<uuid>.json     # ImportedScene：2 关节 skeleton、1 三角形蒙皮 mesh、
                                # 1 个 Y 轴 0→90° 旋转 clip
scripts/Game.dll                # 占位 C# 程序集
runtime/libmono.so              # 占位运行时
```

- 蒙皮 primitive：3 顶点、索引 `[0,1,2]`、joints/weights（0.5/0.5 混合）、skeleton=0；
- skeleton：root joint（node 0）+ child joint（node 1，局部 +Y 1.0），IBM 均为单位矩阵；
- animation：`Spin`，target_joint=1 的 rotation 通道，times `[0,1]`，linear 插值；
- 运行时会选择第一个蒙皮 mesh/skeleton/clip，转换为 `SkinnedVertex` 并每帧上传 palette；
  无 GPU 或 task-28 API 不可用时安全回退内置 cube + WARN。

手工冒烟（需要显示环境 + Vulkan GPU）：

```bash
cd engine
cargo build -p hezhou-player --features window-host
./target/debug/hezhou-player-build --config player/tests/fixtures/skinned/build-config.json \
  --cook player/tests/fixtures/skinned/cook_manifest.json \
  --scripts-dir player/tests/fixtures/skinned/scripts \
  --runtime-dir player/tests/fixtures/skinned/runtime \
  --exe target/debug/hezhou-player.exe --build-out ../dist/player-skinned
HEZHOU_PLAYER_WINDOW_FRAMES=60 ../dist/player-skinned/windows/skinned-fixture.exe \
  --run --host window --manifest ../dist/player-skinned/windows/manifest.json --log-dir ../dist/player-skinned-logs
```

