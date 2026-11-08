# third_person fixture（task-59）

用于验证 Player 第三人称 demo：

- `assets/scenes/third_person.scene`：Project JSON（Ground 大 cube + DirectionalLight），
  启动后由 Player host 恢复为起始场景；
- `scripts/Game.dll`：占位（真实窗口测试用 `scripts/PlayerGame.cs` 编译的 DLL 覆盖）；
- `runtime/libmono.so`：占位 runtime（真实测试用系统 Mono/mono-host）；
- `build-config.json`：`product_name = "Third Person Demo"`，Player host 识别该名称后
  自动设置 `HEZHOU_PLAYER_DEMO=third_person`，C# PlayerGame 进入第三人称 demo 分支；
- `cook_manifest.json`：scene + font 两条 entry（真实窗口构建走 PlayerBuilder）。

真实窗口运行：

```bash
cd engine
cargo test -p hezhou-player --features mono-host \
  --test window_host actual_window_host_renders_third_person_demo -- --ignored --nocapture
```

操作：WASD/方向键移动、Space 跳跃；相机第三人称跟随（距离 5，高 2.5，pitch -15°）。
