# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目简介

`maidata-rs` 是一个 Rust 库，用于解析 simai 的 `maidata.txt` 文件格式（maimai 自制谱常用工具）。除库之外还包含多个 CLI 工具用于谱面检查、物化、变换和模拟。

## 构建与测试

```bash
# 构建
cargo build
cargo build --release

# 运行全部测试
cargo test

# 运行单个模块的测试（例如 parser）
cargo test --lib parser

# 运行某个 binary
cargo run --bin maidata-parse -- <path-to-chart-dir>
```

可用的 binary：`maidata-parse`、`maidata-inspect`、`maidata-materialize`、`maidata-transform`、`maidata-sensor`、`maidata-test`、`mai-simulator`。

无自定义 lint/format 配置，使用 Rust 默认工具链即可。

## 架构概览

处理流水线为单向数据流：

```
maidata.txt → container → parser → insn → transform → materialize → judge
```

### 核心模块

| 模块 | 可见性 | 职责 |
|------|--------|------|
| `maidata_file` | pub | 数据模型：`Maidata`、`BeatmapData`、`AssociatedBeatmapData` |
| `container` | pub | 解析入口：`lex_maidata`、`parse_maidata_insns` |
| `parser` | crate 内部 | 基于 nom 组合子的指令解析实现 |
| `span` | pub | 位置追踪基础设施：`Span`、`Sp<T>`、`NomSpan`、`PResult` |
| `diag` | pub | 解析诊断：`PWarning`、`PError`、`State` |
| `insn` | pub | 原始指令及参数类型定义（TAP/HOLD/SLIDE/TOUCH 等） |
| `transform` | pub | Slide 归一化（`normalize`）和谱面几何变换（`transform`） |
| `materialize` | pub | 将原始指令映射为带绝对时间的物化音符 |
| `judge` | pub | 判定模拟（包含 note 状态机和 simulator） |

### 顶层类型（lib.rs）

- `Difficulty` 枚举：Easy(1) ~ Original(7)
- `Level` 枚举：Normal(u8) / Plus(u8) / Char(char)

### 公开 API 设计

- `lib.rs` 通过显式 `pub use` 导出常用类型，不暴露 `parser` 内部实现
- `maidata_file` 只包含纯数据模型，不依赖 parser 的 `NomSpan`/`PResult`
- `container` 作为解析编排层，调用 parser 和 maidata_file

### 已知架构问题（后续阶段待处理）

`docs/refactor-plan.md` 中有详细的重构计划，阶段 1-2 已完成，剩余问题：

- `materialize` 内部直接调用 `transform::normalize`，跨层依赖
- `judge` 同时依赖 `materialize` 和 `transform` 的内部类型
- `bin/*` 入口重复编排应用逻辑（缺 app facade 层）

## 关键依赖

- **nom / nom_locate**：解析器组合子，用于谱面指令解析和位置追踪
- **enum_map**：枚举键 Map，通过 `enum_map!` 宏使用（全局启用 `#[macro_use]`）
- **serde / serde_json**：序列化，用于 CLI 工具的 JSON 输出
