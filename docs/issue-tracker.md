# Issue Tracker

Last updated: 2026-04-03

## Critical

### C1. label lookup key 修复 + level>=10 过滤未提交

**位置**: `src/bin/maidata-heatmap-dataset/main.rs`

工作区包含两个未提交的关键修复：
- `label_map.get(&song_id)` → `label_map.get(&numeric_id)`（API 返回纯数字 ID，目录名含后缀导致全部匹配失败）
- level < 10 的谱面过滤

committed 版本导出的 `chart_constant` 全为 `None`，训练无法获得有效标签。

**状态**: 已修复，待提交

---

### C2. `chart_constant` 为 None 时 `collate_fn` 会 crash

**位置**: `training/dataset.py` → `collate_fn`

`torch.tensor(labels)` 中若混入 `None` 会抛 `TypeError`。`split_by_song` 已过滤无标签样本，但直接构造 `ChartDataset` 绕过 `split_by_song` 时仍会触发。

**修复方案**: `ChartDataset.__getitem__` 中对 `None` 做防御，或构造时过滤。

**状态**: 未修复

---

### C3. CLAUDE.md 未列出 `maidata-heatmap-dataset` binary

**位置**: `CLAUDE.md`

文档中的 binary 列表缺少 `maidata-heatmap-dataset`（数据导出 CLI），这是训练管线的关键入口。

**状态**: 未修复

---

## High

### H1. heatmap 模块跨层依赖 transform 内部类型

**位置**: `src/heatmap/encode.rs`

```rust
use crate::transform::transform::{Transformable, Transformer};
use crate::transform::{NormalizedSlideSegment, NormalizedSlideSegmentParams, ...};
```

heatmap 直接 import transform 层的归一化 slide 内部表示，新增了 `heatmap → transform` 跨层耦合。CLAUDE.md 已记录 `materialize → transform` 耦合为已知问题，但未包含此项。

**修复方案**: 在 transform 层提供稳定的公开接口，或将 slide 路径展开逻辑上提至 heatmap 自身。

**状态**: 未修复

---

### H2. `expand_fan_slide` 用 `assert!` 会 panic

**位置**: `src/heatmap/encode.rs:231`

```rust
fn expand_fan_slide(track: &MaterializedSlideTrack) -> Option<Vec<(TouchSensor, f64, f64)>> {
    assert!(track.segments.len() == 1);
```

异常输入时崩溃整个导出进程。函数签名已返回 `Option`，应用 `return None` 替代。

**修复方案**: `assert!` → `if track.segments.len() != 1 { return None; }`

**状态**: 未修复

---

## Medium

### M1. `maidata-sensor` 有 `#[allow(unused_imports)]`

**位置**: `src/bin/maidata-sensor/main.rs:91-92`

```rust
#[allow(unused_imports)]
use maidata::Level;
```

未使用的 `Level` import，应直接删除。

**状态**: 未修复

---

### M2. `training/` 目录整体未提交

4 个 .py 文件从未进入版本控制：`config.py`, `dataset.py`, `model.py`, `train.py`。

**状态**: 未修复

---

### M3. `materialize/context.rs` 的 unwrap + TODO

**位置**: `src/materialize/context.rs:237`

```rust
// TODO: handle normalization error
let normalized = transform::normalize::normalize_slide_segment(start_key, segment).unwrap();
```

异常 slide 会 panic，应转为 `Result` 传播错误。

**状态**: 未修复

---

### M4. `judge/adapter.rs` 有 `todo!("")`

**位置**: `src/judge/adapter.rs:14`

```rust
MaterializedNote::Bpm(_) => todo!(""),
```

实践中 BPM note 不会进入 judge，但触碰即 panic。应跳过或返回 `Err`。

**状态**: 未修复

---

### M5. `compact_frames` 无测试覆盖

**位置**: `src/bin/maidata-heatmap-dataset/main.rs:125-162`

`compact_frames`（f32→u8 量化、空帧跳过、offset 记录）是训练数据的关键路径，无任何测试。bug 会静默损坏训练数据。

**状态**: 未修复

---

### M6. `docs/refactor-plan.md` 部分过时

**位置**: `docs/refactor-plan.md`

- 阶段 1-2 已完成但仍写为待做
- 测试基线写的是 11 个，实际已有 28 个
- CLAUDE.md "已知架构问题" 未标注哪些已解决

**状态**: 未修复

---

### M7. 本地领先 origin 1 个 commit

commit `2f36d3f` (feat: 新增传感器时序编码与热图数据导出) 未 push。

**状态**: 未修复

---

## Low

### L2. `fetch_labels` 无网络错误处理

**位置**: `src/bin/maidata-heatmap-dataset/main.rs:194-217`

API 不可用、返回非 JSON、schema 变化时均会以不透明错误退出。`ds` 长度非 4 或 5 的歌曲被静默跳过。

**状态**: 未修复

---

### L3. `encode_slide` 的 overlap 用整体 slide 时间而非 per-event 时间

**位置**: `src/heatmap/encode.rs:145-155`

多段 slide 的每个 hit area event 有自己的 `(ev_start, ev_end)`，但 overlap 计算用的是整体 `(slide_start, slide_end)`。多段 slide 各段的覆盖率可能不准确。

**状态**: 未修复，需确认是否为有意设计

---

