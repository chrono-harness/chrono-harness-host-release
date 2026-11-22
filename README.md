# chrono-harness-host-release

构建和发布宿主平台包。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/host-release)

安装本仓清单固定的 chrono-judge-registration v0.1.0-alpha.1，并放入 PATH。发布配方从 execution-methods 读取登记方法；预览按固定提交读取登记字节，并从 test-owners 取得测试归属。

本仓发行在 Linux x86_64 和 macOS arm64 分别构建。安装或回退时从同一标签读取二进制和 SHA256SUMS，核对对应平台摘要。
