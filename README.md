# chrono-harness-host-release

构建和发布宿主平台包。

在仓库根目录运行 `mise exec -- ./check.sh`。本仓的 mise 配置固定 Verus 版本，检查按变更验证发布配方及其消费者。

本地主检出的准备记录、执行原件和失败诊断保留 24 小时。时间从各原件登记的起点计算；活跃读者释放后，由下一次维护删除到期原件。需要长期使用的结果请在窗口内转存。采用政策前已有的原件没有登记，需按原生产者与使用方核对。

已运行本仓检查并安装好工具的长期检出，可以把下列命令加入本机用户的 crontab，每小时维护一次。将路径替换为本机主检出的绝对路径。维护失败时退出非零，诊断写至标准错误。

```cron
0 * * * * cd '/absolute/path/to/chrono-harness-host-release' && PATH="$PWD/.chrono-harness/bin:$PATH" .chrono-harness/bin/chrono-worktree maintain >/dev/null
```

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/host-release)

安装本仓清单固定的 chrono-judge-registration，并放入 PATH。发布配方从 execution-methods 读取登记方法，预览按固定提交读取登记字节，并从 test-owners 取得测试归属。两项查询的子进程使用宿主所选 chrono-harness 所在的目录作为 PATH，以调用同一份 runner。

本仓发行在 Linux x86_64 和 macOS arm64 分别构建。安装或回退时从同一标签读取二进制和 SHA256SUMS，核对对应平台摘要。

原生发行前端使用 PATH 中提供 `original-report-v1` 的 chrono-host-bootstrap 验收构建原件。本仓安装清单固定该工具的发行版本与两平台摘要。导出从宿主根目录读取登记的构建报告与操作流；历史验收向报告生产者传入原提交和原树。发布配方及工作流由 release 自己从宿主的固定 Git 树读取。

本仓 Cargo 构建只需要自身源码和已锁定的第三方依赖。运行发布命令时，安装清单中的 chrono-harness 通过 `observed-command-v1` 提供实时原始输出、执行身份、超时和终态。各发布单元把两流及回执保留在自己的输出目录。

可选的 GitHub 发布配方在选择预览和晋升正式版时使用以下两个只读命令。每个命令从 stdin 读取一个 JSON 对象，接受时退出 0，拒绝时向 stderr 写原因并退出 2。

- `template-run-v1` 接受 `repository`、`branch`、`workflow`（工作流文件名）、`sha` 和原始 GitHub `run`。它核对两处仓库身份、工作流、分支、提交及该 push 的完成结果。
- `template-acceptance-v1` 接受 `sha`、`required_jobs`、`step` 与 GitHub 指定 run/attempt 返回的 `jobs` 数组。每个要求的作业都要在该提交成功，且至少有一次指定步骤成功。调用方负责从指定运行及尝试读取这些原件。同一尝试返回多条同名作业时，使用 GitHub 原数组中最后一条精确同名的记录。构建来源也按同一规则取作业。

`template-promotion-v1` 读取规划器从 GitHub 取得的晋升证据。请求的 `kind` 和成功时的 JSON 输出如下。

| kind | 输入 | 输出 |
| --- | --- | --- |
| `contains` | 按 base...head 读取的原始 `comparison` | head 包含 base 时返回 null |
| `candidate` | 上一正式提交...待发布提交的 `comparison` | 已覆盖返回 true，可继续返回 false，分叉或未知则报错 |
| `explicit` | 计划的 `tag` 和同名 `release` | 已公开正式版返回 true，草稿或预览返回 false，身份或必需字段不符则报错 |
| `adopted` | `checked_main`（手动晋升传 null）、预览提交 `selected`、`tag`、`version`、`repository`、`workflow`、源检查 `origin`、selected...dev 的 `development` 和 main...selected 的 `main` | 返回 null 表示已选择的公开预览可由该源检查晋升 |

调用方按固定的 tag 读取提交并用 `template-release-v1` 验证公开预览和资产。GitHub 的比较方向为 head 相对 base。晋升核对成功 dev push 的仓库、工作流、提交、实际正整数运行号和预览标签的规范十进制序号。GitHub 负责提供原始观察，更新 main 时仍须使用非强制快进。

`template-consumer-pin-v1` 从下游已落地清单判定安装版本，成功返回 JSON 数组 `[tag, checksums]`。Copier 请求提供 `kind="copier"`、`repository`、`version`、`rolling` 和解析后的 `answers`，源模板产物的 checksums 为 null。二进制请求提供 `kind="binary"`、`repository`、`version`、`binary` 和 mise 的 `tool` 对象。返回对应 Linux x86_64 与 macOS arm64 安装资产的完整名称和 SHA-256。调用方从该宿主选定提交读取 YAML 或 TOML，随后以返回的标签和摘要核对公开发行。

`template-binary-origins-v1` 从标准输入读取发布者的两个平台观察。根据固定 check、来源 SHA、release run/attempt、构建和上传步骤、artifact ID 与归档、文件 SHA-256 输出来源记录。同一 run 重试时可沿用较早且已经成功的平台作业。GitHub API 的读取与下载，以及下载文件的摘要计算，由发布适配器负责。

`template-latest-stable-v1` 从标准输入读取所有页的 GitHub Release 行组成的 JSON 数组，返回规范正式标签（例如 `"v1.10.0"`）或 `null`。选择只需要 tag_name、draft、prerelease、published_at。版本按三个规范十进制分量比较；发布适配器负责读取全部页并核对所选 Git 标签的提交。

`template-next-version-v1` 接受一个 JSON 对象，例如 `{"floor":"0.1.2","latest":"0.1.11"}`，返回 JSON 字符串 `"0.1.12"`。首次发布传 `latest: null`。后续版本取计划下限与上一正式版本的下一补丁版中较大的版本。三个十进制分量允许任意位数并使用规范 SemVer 拼写。

模板预览发布通过 `template-publication-v1` 检查草稿正文。传入 `{"kind":"notes","release":原始发行对象,"expected":预期正文字符串}`，返回 JSON 布尔值。只有原对象的 `body` 是字符串且全文相同时才返回 true。调用方据此更新草稿，并在上传资产后复核构建来源。

可选的 GitHub issue-roadmap 工作流用 `chrono-host-release roadmap-v1` 判定哪些 GitHub 对象需要更新。标准输入和输出各为一个 JSON 值。无效输入退出 2，调用方应停止写入。工作流自行分页读取 GitHub 原件并按返回结果调用原 API。

| 输入中的 `operation` | 其余输入 | 输出 |
| --- | --- | --- |
| `published` | `releases` 数组，每项含 `draft` 和 `published_at` | 已发布且有时间的条目的零基下标 |
| `unassigned` | `issues` 数组，取 GitHub 的 `number/state/pull_request/milestone` | 可分配的 open issue 号码 |
| `milestone` | `final_release` 布尔值、`open_issues` 非负整数 | `"closed"` 或 `"open"` |
| `release-block` | `milestone` 编号、`original` 完整正文、`lines` 发行展示行数组 | 新的完整正文。替换首对发行标记之间的块；无标记时追加。缺一侧标记或首个结束标记在开始标记前时退出 2 |
| `release-run` | 工作流的 `status/conclusion` | 是否需要查询该次运行的历史事项 |
| `release-issue` | `status/conclusion/run_id` 和带 release-failure 标签的 `issues` 原件 | `{"action":"none/close/reopen/create","issue":号码或null}` |

发布重试沿用同一个 run ID。恢复只关闭该运行由 github-actions[bot] 创建、正文以运行标记开始的 open issue。再次失败可以重开同一事项；用户以 `not_planned` 关闭的事项会保留原状。
