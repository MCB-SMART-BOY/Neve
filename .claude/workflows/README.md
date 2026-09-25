# n3v3 Agent Workflows / n3v3 代理工作流

This directory contains agent workflows for repository-root validation and release readiness.
本目录包含用于仓库根目录验证和发布就绪检查的代理工作流。

## Contract boundary / 契约边界

No runner semantics are defined in this repository. This document records the behavior currently relied upon by the workflows and the boundary of what they can verify.
仓库内没有定义运行器语义；本文档记录工作流当前依赖的行为，以及工作流能够验证的边界。

Workflow files in this directory are loaded by the harness in its CJS-style workflow context. A plain `bun build` treats top-level `return` as invalid ESM syntax; that is expected for this runner contract and must not become a separate syntax gate.
本目录工作流文件由 harness 以 CJS 风格的工作流上下文加载。普通 `bun build` 会把顶层 `return` 判为无效 ESM 语法；这符合当前运行器契约，不应据此增加独立语法闸门。

The runner provides the workflow primitives (`phase`, `agent`, `parallel`, and `log`) and evaluates the exported result. The workflow files must not assume undocumented runner features or silently manufacture a pass result.
运行器提供工作流原语（`phase`、`agent`、`parallel` 和 `log`），并处理导出的结果。工作流文件不得假定未记录的运行器功能，也不得静默伪造通过结果。

## Metadata schema / 元数据 schema

Each workflow exports one `meta` object:
每个工作流导出一个 `meta` 对象：

```js
export const meta = {
  name: string,
  description: string,
  phases: Array<{
    title: string,
    detail: string,
  }>,
}
```

`name` is the registry name, `description` is a short user-facing summary, and `phases` is the declared ordered phase structure. `title` and `detail` describe the same phase from different levels; they are not a second execution plan.
`name` 是注册名，`description` 是面向用户的简短说明，`phases` 是声明性的有序阶段结构。`title` 和 `detail` 从不同层次描述同一阶段；它们不是第二份执行计划。

## Primitives / 原语

### `phase(title)`

`phase(title)` marks the beginning of the named execution phase and keeps runtime output aligned with `meta.phases`. Every executable phase should have a corresponding declaration.
`phase(title)` 标记指定执行阶段的开始，使运行时输出与 `meta.phases` 对齐。每个可执行阶段都应有对应声明。

It is an observation boundary, not a success assertion. A phase is successful only when its commands and delegated work produce successful results.
它是观察边界，不是成功断言。只有当阶段内命令和委托工作都产生成功结果时，阶段才算成功。

### `agent(prompt, {label})`

`agent` delegates `prompt` to the runner and returns the agent's text result. `label` identifies the delegated operation in runtime output.
`agent` 将 `prompt` 委托给运行器，并返回代理的文本结果。`label` 用于在运行时输出中标识该委托操作。

The prompt must name the repository-root command, the expected evidence, and the failure condition. A non-zero command must be exposed as a non-empty failure result. The workflow must not use a fallback such as `result || 'passed'`, discard stderr, or turn an absent result into success.
提示必须写明仓库根目录命令、预期证据和失败条件。命令非零退出必须以非空失败结果暴露。工作流不得使用 `result || 'passed'` 之类的回退、丢弃 stderr，或把缺失结果转成成功。

### `parallel([...])`

`parallel` starts the supplied task functions concurrently and returns their results in input order. Each task keeps its own label and evidence.
`parallel` 并发启动传入的任务函数，并按输入顺序返回结果。每个任务保留自己的标签和证据。

A failed task must remain visible in the returned collection or be propagated by the runner. Callers must not filter failures away with `filter(Boolean)` or replace them with placeholders.
失败任务必须在返回集合中保持可见，或由运行器传播失败。调用方不得用 `filter(Boolean)` 删除失败，也不得用占位值替换失败。

### `log(msg)`

`log(msg)` records diagnostic output for the current phase. It does not establish a pass condition and cannot replace a command result.
`log(msg)` 记录当前阶段的诊断输出。它不建立通过条件，也不能替代命令结果。

### `return value`

The final `return value` is the structured workflow result consumed by the runner. Its fields must correspond to executed phases and preserve raw or clearly attributed evidence.
最终的 `return value` 是由运行器消费的结构化工作流结果。其字段必须对应已执行阶段，并保留原始证据或清楚标注证据来源。

The current result shapes are `{ validation, counts }` for `full-test` and `{ validation, counts, releaseMetadata }` for `pre-release`. `validation` is the single quality-gate result; `counts` is the mechanical comparison result; `releaseMetadata` is the release version/changelog result.
当前结果形状是：`full-test` 返回 `{ validation, counts }`，`pre-release` 返回 `{ validation, counts, releaseMetadata }`。`validation` 是唯一质量闸门结果；`counts` 是机械比较结果；`releaseMetadata` 是发布版本与 changelog 检查结果。

Returning a result does not override a failed command. A workflow must propagate failure through the result or the runner's failure mechanism; it must never catch and swallow a failure.
返回结果不会覆盖失败命令。工作流必须通过结果或运行器的失败机制传播失败；绝不得捕获后静默吞掉失败。

## Repository commands and verification boundary / 仓库命令与验证边界

Workflow scripts assume that their current working directory is the repository root. Relative paths such as `scripts/validate.sh`, `scripts/counts.sh`, and `docs/project/changelog.md` are resolved from that directory.
工作流脚本假定当前工作目录是仓库根目录。`scripts/validate.sh`、`scripts/counts.sh` 和 `docs/project/changelog.md` 等相对路径都从该目录解析。

`validate.sh` is the only project quality-gate entry point: workflows call `scripts/validate.sh --ci` (full-test) or `scripts/validate.sh --release` (pre-release) and do not re-list cargo gates. `scripts/counts.sh --json` and the documentation checker provide the requested mechanical evidence and release metadata checks; they do not define another test gate.
`validate.sh` 是项目唯一质量闸门入口：工作流分别调用 `scripts/validate.sh --ci`（full-test）或 `scripts/validate.sh --release`（pre-release），不自行罗列 cargo 闸门。`scripts/counts.sh --json` 和文档校验器提供所需的机械证据与发布元数据检查，但不定义另一套测试闸门。

Counts are evidence only when compared mechanically against their declarations. A number printed without an expected value, threshold, or failing comparison is not a validation phase.
只有在与声明做机械比较时，计数才是证据。仅打印数字而没有期望值、阈值或失败比较，不构成验证阶段。

## Timeouts and unverifiable work / 超时与不可验证工作

Every delegated command must state its timeout or the runner's default timeout assumption when a timeout matters. A timeout, cancellation, unavailable tool, or missing source file is a failed or explicitly unverified outcome; it must be retained in the result and never inferred to be passing.
当超时会影响结果时，每个委托命令都必须说明超时，或说明所依赖的运行器默认超时。超时、取消、工具不可用或源文件缺失都属于失败或必须明确标记为“未验证”的结果；必须保留在结果中，绝不能推断为通过。

If the local environment cannot exercise runner behavior, report that boundary explicitly. This repository records the workflow contract, but it does not define agent scheduling, process timeout enforcement, stdout/stderr capture, or the exact shape of runner-thrown exceptions.
如果本地环境无法执行运行器行为，必须明确报告该边界。本仓库记录工作流契约，但不定义代理调度、进程超时执行、stdout/stderr 捕获方式，或运行器抛出异常的确切形状。
