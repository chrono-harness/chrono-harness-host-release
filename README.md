# chrono-harness-host-release

构建和发布宿主平台包。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/host-release)

安装本仓清单固定的 chrono-judge-registration，并放入 PATH。发布配方从 execution-methods 读取登记方法，预览按固定提交读取登记字节，并从 test-owners 取得测试归属。两项查询的子进程使用宿主所选 chrono-harness 所在的目录作为 PATH，以调用同一份 runner。

本仓发行在 Linux x86_64 和 macOS arm64 分别构建。安装或回退时从同一标签读取二进制和 SHA256SUMS，核对对应平台摘要。

原生发行前端使用 PATH 中提供 `original-report-v1` 的 chrono-host-bootstrap 验收构建原件。本仓安装清单固定该工具的发行版本与两平台摘要。导出从宿主根目录读取登记的构建报告与操作流；历史验收向报告生产者传入原提交和原树。发布配方及工作流由 release 自己从宿主的固定 Git 树读取。

本仓 Cargo 构建只需要自身源码和已锁定的第三方依赖。运行发布命令时，安装清单中的 chrono-harness 通过 `observed-command-v1` 提供实时原始输出、执行身份、超时和终态。各发布单元把两流及回执保留在自己的输出目录。
