# Hezhou 0.4.0 发布说明

> 状态：Release Candidate / 待双平台 CI 验收。基线：0.3.0。

## Highlights

- 0.4 生态：本地包管理器、声明式编辑器插件 API、项目模板
- 0.4 网络：传输层（UDP/TCP/framing/Session/Room）、服务器权威快照（全量/Delta/ack）、客户端插值、RPC
- 1.0 稳定性：60s soak CI 门禁、30 分钟 soak、诊断包导出
- 现有 0.2/0.3 能力与 Player 全保留

## 包管理器

- `hezhou-package`：manifest/registry/依赖拓扑/安装/lockfile
- 编辑器「包」Tab
- 示例 registry：hezhou-hello / hezhou-mathx / hezhou-gameplay-kit

## 插件 API

- `hezhou-plugin`：manifest/registry/启用状态/action 白名单
- 编辑器「插件」Tab + Command Palette
- 示例插件：sample-tools / scene-tools
- 不执行任意代码；无动态 DLL/权限沙箱

## 网络

- UDP/TCP Transport + framing
- Session/Room/NetServer/NetClient
- WorldSnapshot 全量/Delta、baseline ack、ReplicationServer/Client
- SnapshotBuffer 插值（position lerp / rotation slerp）
- RpcRegistry
- 测试：58 tests（UDP loopback/3 客户端/snapshot/RPC）

## 稳定性

- `soak_smoke` 60s CI 门禁 `soak-quick`
- 30 分钟本机 soak：1800s、107999 ticks、0 overruns、PASS；RSS +1.77MB
- `diagnostics.ps1` 诊断包（脱敏/排除敏感文件/大小限制），CI 双平台自检
- 性能基线 `perf-baseline`

## Validation

- 本地 `ci_local.ps1 -Offline` 全绿
- 编辑器启动自检 PASS=117
- FFI 343/343/343
- 网络 58 tests；包管理 6 tests；插件 5 tests
- 双平台 Editor + Player 构建待 0.4.0 tag CI

## Known Issues / 1.0

- 平台扩展（macOS/Android/Web）未完成，当前发布平台仍为 Windows/Linux
- 网络无预测/回滚/加密/可靠 UDP/AOI
- 插件为声明式，无动态代码/权限沙箱/插件市场
- 包管理器仅本地 registry，无远程/签名/完整 semver
- 30 分钟 soak 相对内存增长 30.5%（绝对 +1.77MB，低基数），需 1.0 继续优化/观察
- 非团队开发者文档/入门验证未完成
