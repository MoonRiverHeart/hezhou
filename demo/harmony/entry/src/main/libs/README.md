# 放置 Rust 编译的 SO 文件
# 将 `libcsharptorust_lib.so` 复制到此目录
# 路径: harmony/entry/src/main/libs/arm64-v8a/libcsharptorust_lib.so
#
# 可通过运行以下脚本自动复制:
# cd demo/rust
# cargo +nightly build -Zbuild-std --target aarch64-unknown-linux-ohos --release --lib
# cp target/aarch64-unknown-linux-ohos/release/libcsharptorust_lib.so ../harmony/entry/src/main/libs/arm64-v8a/
