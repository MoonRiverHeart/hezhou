# Hezhou Fuzz Targets

## 目标

- `project_json`：对 `ProjectFile` 的 JSON 反序列化做覆盖，输入任意字节都只能返回 `Err`，不能 panic。

## 前置

```powershell
cargo install cargo-fuzz
rustup toolchain install nightly
```

## 运行

```powershell
cd engine\fuzz
cargo +nightly fuzz run project_json
```

带超时/次数：

```powershell
cargo +nightly fuzz run project_json -- -max_total_time=60
cargo +nightly fuzz run project_json -- -runs=100000
```

## 语料

`corpus/project_json/` 存放最小合法工程 JSON 和边界样本。CI/本地可使用不依赖 nightly 的确定性 smoke 测试：

```powershell
cd engine
cargo test -p hezhou-core --offline project_fuzz_smoke
```

`engine/core/tests/project_fuzz_smoke.rs` 会生成 3000 组畸形 JSON，验证解析器不 panic，作为 cargo-fuzz 的快速回归层。
