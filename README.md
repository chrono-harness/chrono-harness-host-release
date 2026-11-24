# chrono-harness-host-release

构建和发布宿主平台包。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/host-release)

安装本仓清单固定的 chrono-judge-registration，并放入 PATH。发布配方从 execution-methods 读取登记方法，预览按固定提交读取登记字节，并从 test-owners 取得测试归属。两项查询的子进程使用宿主所选 chrono-harness 所在的目录作为 PATH，以调用同一份 runner。

本仓发行在 Linux x86_64 和 macOS arm64 分别构建。安装或回退时从同一标签读取二进制和 SHA256SUMS，核对对应平台摘要。

原生发行前端使用 PATH 中提供 `original-report-v1` 的 chrono-host-bootstrap 验收构建原件。本仓安装清单固定该工具的发行版本与两平台摘要。导出从宿主根目录读取登记的构建报告与操作流；历史验收向报告生产者传入原提交和原树。发布配方及工作流由 release 自己从宿主的固定 Git 树读取。

本仓 Cargo 构建只需要自身源码和已锁定的第三方依赖。运行发布命令时，安装清单中的 chrono-harness 通过 `observed-command-v1` 提供实时原始输出、执行身份、超时和终态。各发布单元把两流及回执保留在自己的输出目录。

可选的 GitHub 发布配方在选择预览和晋升正式版时使用以下两个只读命令。每个命令从 stdin 读取一个 JSON 对象，接受时退出 0，拒绝时向 stderr 写原因并退出 2。

- `template-run-v1` 接受 `repository`、`branch`、`workflow`（工作流文件名）、`sha` 和原始 GitHub `run`。它核对两处仓库身份、工作流、分支、提交及该 push 的完成结果。
- `template-acceptance-v1` 接受 `sha`、`required_jobs`、`step` 与 GitHub 指定 run/attempt 返回的 `jobs` 数组。每个要求的作业都要在该提交成功，且至少有一次指定步骤成功。调用方负责从指定运行及尝试读取这些原件。

`template-consumer-pin-v1` 从下游已落地清单判定安装版本，成功返回 JSON 数组 `[tag, checksums]`。Copier 请求提供 `kind="copier"`、`repository`、`version`、`rolling` 和解析后的 `answers`，源模板产物的 checksums 为 null。二进制请求提供 `kind="binary"`、`repository`、`version`、`binary` 和 mise 的 `tool` 对象。返回对应 Linux x86_64 与 macOS arm64 安装资产的完整名称和 SHA-256。调用方从该宿主选定提交读取 YAML 或 TOML，随后以返回的标签和摘要核对公开发行。

`template-next-version-v1` 接受一个 JSON 对象，例如 `{"floor":"0.1.2","latest":"0.1.11"}`，返回 JSON 字符串 `"0.1.12"`。首次发布传 `latest: null`。后续版本取计划下限与上一正式版本的下一补丁版中较大的版本。三个十进制分量允许任意位数并使用规范 SemVer 拼写。

可选的 GitHub issue-roadmap 工作流用 `chrono-host-release roadmap-v1` 判定哪些 GitHub 对象需要更新。标准输入和输出各为一个 JSON 值。无效输入退出 2，调用方应停止写入。工作流自行分页读取 GitHub 原件并按返回结果调用原 API。

| 输入中的 `operation` | 其余输入 | 输出 |
| --- | --- | --- |
| `published` | `releases` 数组，每项含 `draft` 和 `published_at` | 已发布且有时间的条目的零基下标 |
| `unassigned` | `issues` 数组，取 GitHub 的 `number/state/pull_request/milestone` | 可分配的 open issue 号码 |
| `milestone` | `final_release` 布尔值、`open_issues` 非负整数 | `"closed"` 或 `"open"` |
| `release-run` | 工作流的 `status/conclusion` | 是否需要查询该次运行的历史事项 |
| `release-issue` | `status/conclusion/run_id` 和带 release-failure 标签的 `issues` 原件 | `{"action":"none/close/reopen/create","issue":号码或null}` |

发布重试沿用同一个 run ID。恢复只关闭该运行由 github-actions[bot] 创建、正文以运行标记开始的 open issue。再次失败可以重开同一事项；用户以 `not_planned` 关闭的事项会保留原状。
