# Player 最小测试 fixture

仅供 `engine/player` 的测试与手工冒烟使用，**不是**真实游戏内容：

- `assets/scenes/empty.scene`：3 字节空场景占位（运行时只检查存在性）；
- `assets/fonts/HarmonyOS_Sans_SC.ttf`：12 字节字体占位；
- `scripts/Game.dll`：21 字节假程序集占位（运行时只检查存在性）；
- `runtime/libmono.so`：19 字节假运行时库占位；
- `build-config.json` / `cook-manifest.json`：构建配置与 cook 清单（`size_bytes` 与文件实际大小一致）。

测试会额外把 `cargo` 产出的 `hezhou-player` 可执行文件通过 `--exe` 传入，构建出
一个只含占位内容的完整 Player 包，并用 `hezhou-player --validate` 做启动冒烟。

占位二进制后缀（`.dll` / `.so`）由 `engine/player/.gitignore` 的负向规则纳入版本管理。
