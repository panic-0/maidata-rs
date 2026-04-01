# `maidata-rs` 重构文档

## 1. 项目现状概览

`maidata-rs` 当前是一个 Rust 库 + 多个命令行工具的组合项目，核心能力覆盖：

- `maidata.txt` / simai 相关文本内容的解析
- 原始指令到归一化结构的转换
- 谱面时间轴物化
- 判定模拟
- 若干调试/检查型 CLI 工具

当前主流程可以概括为：

`maidata 文本 -> container -> parser -> insn -> transform -> materialize -> judge -> bin/*`

结合现有代码组织，模块职责大致如下：

- `src/container`
  - 解析 `maidata.txt` 风格的键值对文本
  - 组装 `Maidata` / `BeatmapData`
  - 在组装过程中直接调用指令解析逻辑
- `src/parser`
  - 基于 `nom` 完成 maidata 指令解析
  - 维护错误/警告状态
  - 输出 `insn::*` 原始指令结构
- `src/insn`
  - 承载原始指令及参数类型
- `src/transform`
  - 负责 slide 归一化和谱面几何变换
- `src/materialize`
  - 将原始指令映射为带绝对时间的物化音符
- `src/judge`
  - 执行判定模拟逻辑
- `src/bin/*`
  - 封装多种命令行入口

从代码体量上看，项目还处在适合中等重组的阶段：结构已经形成，但边界还没有完全固化，适合在不推翻领域模型的前提下做一次清晰化重构。

## 2. 当前分层与依赖图

### 2.1 现有依赖关系

```text
bin/*
  ├─ 调用 container / materialize / judge / serde_json
  └─ 直接承担文件读取、状态打印、排序、序列化

lib.rs
  ├─ pub mod container
  ├─ pub mod insn
  ├─ pub mod judge
  ├─ pub mod materialize
  ├─ mod parser
  ├─ pub mod transform
  └─ pub use parser::*

container
  └─ 直接调用 parser::parse_maidata_insns

parser
  └─ 输出 insn::RawInsn / RawNoteInsn

transform
  ├─ normalize
  └─ transform

materialize
  ├─ 消费 insn::RawInsn
  └─ 直接调用 transform::normalize::normalize_slide_segment

judge
  ├─ 消费 materialize::Note
  └─ 依赖 transform::NormalizedSlideSegmentShape
```

### 2.2 当前分层问题

现有代码表面上看是流水线，但在实现层面已经出现了跨层依赖：

- `lib.rs` 通过 `pub use parser::*` 暴露了原本应当内部化的解析实现细节。
- `container` 同时承担“maidata 文件建模”和“maidata 文件解析入口”两类职责。
- `materialize` 内部直接调用 `transform::normalize`，说明归一化没有形成独立、稳定的阶段边界。
- `judge` 同时依赖 `materialize` 与 `transform` 的内部模型，导致判定层没有自己的稳定输入面。
- `bin/*` 反复重复应用层流程，库层没有为这些入口提供统一 facade。

### 2.3 结构判断

项目当前的主要问题不是“功能不够”，而是“职责分配不稳定”：

- 上游层与下游层之间缺少稳定契约
- 模型转换分散在多个模块内部
- 入口逻辑没有统一收口
- 测试分布无法为中后段模块提供充分保护

如果继续在现有结构上直接叠加功能，复杂度会继续集中在 `container`、`materialize`、`judge` 三个区域。

## 3. 主要架构问题

### 3.1 模块边界弱，存在跨层调用

当前设计希望形成 `parser -> transform -> materialize -> judge` 的单向流水线，但实现中并未严格遵守：

- `materialize` 直接使用 `transform::normalize::normalize_slide_segment`
- `judge` 直接感知 `transform::NormalizedSlideSegmentShape`
- `container` 直接调用 parser 内部入口而非更高层接口

这会导致两个后果：

- 阶段职责难以替换，修改一层容易牵连多层
- 中间阶段难以被单独测试或复用

### 3.2 领域模型重复，转换责任分散

当前至少存在三类相关模型：

- 原始输入模型：`insn::*`
- 归一化模型：`transform::*`
- 物化/判定模型：`materialize::*` 与 `judge::*`

问题不在于“有多层模型”，而在于这些模型之间的转换没有集中管理：

- 部分归一化逻辑出现在 `transform::normalize`
- 部分物化前规则判断埋在 `materialize`
- `judge` 的输入适配通过 `TryFrom<materialize::Note>` 分散在判定层内部

结果是：

- 从原始指令到最终判定对象的路径不清晰
- 每层真正依赖的最小输入集合不明确
- 模型演进时容易出现字段冗余和耦合泄漏

### 3.3 `container` 同时承担文件模型与解析编排

`src/container/mod.rs` 当前混合了三类责任：

- maidata 文件层的键值词法解析
- 项目内 `Maidata` / `BeatmapData` 模型定义
- 对每个难度的 `inote_x` 内容继续调用 `parser` 解析

这使得 `container` 既像“文件模型层”，又像“应用入口层”。这会带来：

- 文件格式模型与解析策略强绑定
- 后续若想替换解析入口或增加高层 facade，会遇到职责重叠
- `Maidata` 作为领域对象，难以与解析生命周期解耦

### 3.4 `bin/*` 入口逻辑重复

当前多个 CLI 入口重复承担以下工作：

- 读取文件
- 调用 `container` 或 `materialize`
- 打印 warning / error
- 构造 JSON
- 做简单排序或输出整理

这些逻辑属于典型的应用服务层职责，不应在每个 `bin` 中各自维护。否则会造成：

- 行为不一致
- 流程变更时需要多点修改
- 后续增加新的入口形态时无法复用

### 3.5 测试覆盖失衡

当前测试基线主要集中在：

- `parser`
- `transform::normalize`
- `container` 的少量工具函数

而对重构风险更高的区域缺少足够保护：

- `materialize`
- `judge`
- 应用层编排 / CLI 输出

现状意味着：

- 上游语法变更容易被发现
- 下游行为回归难以及时捕捉
- 重构最想改的部分，恰好是测试最薄弱的部分

## 4. 重构目标与非目标

### 4.1 目标

本轮重构以“中等重组”为目标，重点解决可维护性问题：

- 明确单向分层与依赖方向
- 收敛公开 API，缩小库根导出面
- 收敛中间模型的职责边界
- 从 `bin/*` 中抽离应用层流程
- 为 `materialize`、`judge`、应用编排补齐测试护栏

### 4.2 非目标

本轮不以以下事项为主线：

- 不推翻现有 `insn` / `transform` / `materialize` 的整体概念
- 不以性能优化为主目标
- 不在本轮引入大规模异步化、插件化或宏重写
- 不强行把所有历史 API 一次性删除

这次重构应该优先建立“稳定边界”，而不是追求一次性架构翻新。

## 5. 分阶段重构方案

### 阶段 1：收口公开接口与依赖方向

目标：先把对外接口和层间调用关系收紧，避免后续继续扩大耦合。

建议动作：

- 在 `lib.rs` 取消 `pub use parser::*`
- 改为显式暴露高层入口函数或 facade 模块
- 为“解析指令”“解析 maidata 文件”“物化谱面”“判定模拟”定义稳定入口
- 将 `parser` 保持为内部实现模块，不再整体透传

建议新增或收口的入口示意：

- `parse::parse_chart_insns(...)`
- `parse::parse_maidata_file(...)`
- `timeline::materialize_chart(...)`
- `judge::simulate_chart(...)`

这里的名字可以按仓库风格微调，但原则是：

- 上层只能看到阶段入口，不直接依赖具体 parser helper
- 每层输入/输出在模块边界上是显式的

### 阶段 2：拆分 `container`

目标：把“maidata 文件模型”和“maidata 文件解析入口”拆开。

建议拆分方式：

- 保留一个承载领域对象的模块，例如 `container` 或重命名为 `maidata_file`
- 将当前解析逻辑移动到独立入口模块，例如 `parse::maidata_file` 或 `app::maidata_file`

推荐职责划分：

- 文件模型层
  - `Maidata`
  - `BeatmapData`
  - `AssociatedBeatmapData`
- 文件解析层
  - 从文本读取键值对
  - 按 key 分发字段
  - 对 `inote_x` 内容调用 chart parser
  - 汇总 warning / error

这样做之后：

- `Maidata` 不再依赖“如何被解析”的实现细节
- 同一个模型未来可支持不同来源或不同构建方式
- 解析入口可更容易并入统一 facade

### 阶段 3：把归一化阶段从 `materialize` 中剥离

目标：让 `materialize` 只做时间轴映射，不再承担归一化细节。

当前关键问题是：

- `materialize_slide_segment` 内部直接调用 `transform::normalize::normalize_slide_segment`

建议改造方向：

- 明确引入独立的“语义化 / 归一化”阶段
- 由该阶段把原始 slide 结构变成稳定的规范化结构
- `materialize` 只消费规范化后的输入，不再关心几何校验细节

推荐结果形态：

- `normalize` 层输出完整的规范化 note / slide 模型
- `materialize` 只处理时间推进、duration 计算、事件落点

这样做有两个直接收益：

- 归一化失败与时间轴构造失败不再混杂
- `materialize` 可测试性更强，输入更稳定

### 阶段 4：给 `judge` 建立稳定输入模型

目标：让判定层只依赖“可判定谱面模型”，而不是上游内部结构。

当前问题表现为：

- `judge::note::Note::try_from(MaterializedNote)` 将适配逻辑放在判定层
- `judge` 同时依赖 `materialize::Note` 与 `transform::NormalizedSlideSegmentShape`

建议方案：

- 为判定层定义独立输入 DTO，或至少定义 `judge::chart` 输入模型
- 在 `materialize -> judge` 之间增加一个显式适配层
- `judge` 内部不再直接引用 `transform` 的归一化细节类型

推荐依赖形态：

`normalize -> materialize -> judge_input_adapter -> judge`

如果暂时不想增加新模块，也应至少把适配逻辑从 `judge::note` 中移出，避免判定领域对象承担外部模型转换职责。

### 阶段 5：抽离应用服务层

目标：让 `bin/*` 只负责参数和输出，业务流程进入共享 facade。

推荐新增 `src/app` 模块，统一封装：

- `app::parse_chart`
- `app::parse_maidata`
- `app::materialize_chart`
- `app::inspect_chart`
- `app::simulate_chart`

这些入口负责：

- 文本/文件读取后的流程编排
- 状态聚合
- 输出前整理
- 公共排序与转换

CLI 的职责则缩减为：

- 参数获取
- 调用 `app::*`
- 选择输出形式

这样可以显著降低 CLI 重复逻辑，并为未来接入 GUI / wasm / HTTP API 提供可复用能力。

### 阶段 6：补齐测试护栏

目标：在重构过程中始终维持可回归性。

建议测试分层如下：

- 语法层测试
  - 保留现有 `parser` 单测
- 文件集成测试
  - 覆盖完整 `maidata.txt` 到 `Maidata` / difficulties 的映射
- 物化层测试
  - 覆盖 BPM、拍号、rest、slide stop time、each/break/ex
- 判定层测试
  - 覆盖 tap / hold / touch / slide / fan slide 的关键状态机路径
- 应用层测试
  - 覆盖 facade 输出结构及错误/警告整合行为

## 6. 模块 / API 调整清单

### 6.1 建议保留但收口的接口

- `parse_maidata_insns`
  - 保留能力，但移动到明确的高层命名空间
  - 不再通过 `pub use parser::*` 暴露 parser 细节
- `lex_maidata`
  - 建议重命名或包装为更高层语义接口，例如“解析 maidata 文件”
- `MaterializationContext`
  - 保留其时间推进职责
  - 不再在其中掺入归一化逻辑

### 6.2 建议重新定义边界的接口

- `judge::note::Note::try_from(MaterializedNote)`
  - 不建议继续作为判定层主入口
  - 应迁移到独立适配层或应用层

### 6.3 新增 facade 的建议形态

建议新增一组面向调用方的统一入口：

- `app::parse_chart(...)`
- `app::parse_maidata(...)`
- `app::materialize_chart(...)`
- `app::inspect_chart(...)`
- `app::simulate_chart(...)`

这些接口应返回稳定结果结构，而不是让调用方自己拼接：

- 领域结果
- warning / error
- 必要的附加元信息

### 6.4 推荐依赖方向

允许的依赖方向：

- `parser -> insn`
- `normalize -> insn`
- `materialize -> normalize`
- `judge_input_adapter -> materialize`
- `judge -> judge_input_adapter` 或 `judge <- judge_input_adapter`
- `app -> parse / normalize / materialize / judge`
- `bin -> app`

禁止的依赖方向：

- `materialize -> normalize::具体 helper`
- `judge -> transform::具体归一化细节`
- `bin -> parser 私有实现`
- `container 模型 -> parser 实现细节`

说明：这里的重点不是模块名本身，而是“跨层只能走稳定入口，不能直接走内部 helper”。

## 7. 测试与验收标准

### 7.1 当前基线

当前仓库已具备可运行的测试基线：

- 现有单元测试共 11 项
- 当前基线应在重构过程中始终保持通过

### 7.2 新增测试要求

至少补齐以下几类测试：

- `container` / maidata 文件解析集成测试
- `materialize` 行为测试
- `judge` 行为测试
- `app` facade 流程测试

### 7.3 验收标准

重构完成后，至少满足以下条件：

- 现有 11 个单元测试继续通过
- `materialize` 与 `judge` 有明确回归测试覆盖关键路径
- 新 facade API 可以替代现有 `bin/*` 中重复流程
- `judge` 不再依赖 `transform` 的归一化内部细节
- `lib.rs` 对外导出面缩小，调用方无需访问 parser 内部实现
- 模块职责在目录和依赖关系上更容易被解释和维护

## 8. 风险、兼容性与迁移顺序

### 8.1 主要风险

本次重构的主要风险不是功能逻辑本身，而是边界调整带来的连锁修改：

- 公开 API 调整会影响多个 `bin/*`
- 中间模型收口可能影响 `serde` 输出结构
- 判定层输入适配外移后，可能暴露历史上未显式建模的假设

### 8.2 兼容性策略

建议采用“先加新入口，再迁旧调用”的方式：

1. 新增 facade 和适配层
2. CLI 迁移到新入口
3. 删除不再需要的直接依赖
4. 最后收缩 `lib.rs` 暴露面

这样可以避免一次性大改造成的不可控回归。

### 8.3 推荐迁移顺序

推荐按以下顺序执行：

1. 为现有关键行为补测试
2. 新增 facade，抽离 `bin/*` 公共流程
3. 收口 `lib.rs` 导出面
4. 拆分 `container`
5. 把归一化从 `materialize` 中剥离
6. 为 `judge` 建立稳定输入模型
7. 清理旧接口和冗余适配逻辑

### 8.4 完成标准

当以下条件同时成立时，可以认为本轮重构达标：

- 架构路径清晰，可用一句话解释每层职责
- 调用方通过 facade 使用库，而非直接拼装内部模块
- 下游模块不再依赖上游内部细节类型
- 测试足以覆盖核心行为回归
- 新功能开发不再默认堆积到 `container`、`materialize`、`judge` 的内部实现中

## 附：建议的目标结构示意

以下是推荐的目标形态，名称可按实际仓库风格微调：

```text
src/
  app/
    mod.rs
    parse.rs
    materialize.rs
    inspect.rs
    simulate.rs
  maidata_file/
    mod.rs
    model.rs
    parse.rs
  parser/
    ...
  insn/
    ...
  normalize/
    ...
  materialize/
    ...
  judge/
    chart.rs
    adapter.rs
    note/
    simulator.rs
  lib.rs
  bin/
    ...
```

目标不是强行追求目录变化本身，而是落实三个原则：

- 单向依赖
- 阶段入口清晰
- 模型转换集中
