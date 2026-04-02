# 基于传感器时序编码的舞萌官谱难度预测方案

## Summary
构建一个纯时序、弱监督的难度模型，不使用整谱全局统计特征。输入是按 `0.2s` 切分的传感器时序序列 `[T, 33, 5]`，33 个传感器按真实硬件布局编号，5 通道编码各类输入的时空占用；空间编码（Gaussian 热图等）由 Python 训练侧负责，Rust 仅导出原始传感器数据。模型先预测每个时间窗的相对局部难度 `local_score`，再用 `top-k` 加权聚合局部隐藏状态，回归整谱官方定数。

## Implementation Changes

### 1. 数据表示与编码（Rust 侧）
- 以单张谱面单难度为一个训练样本，标签为官方连续定数。
- 将谱面物化为绝对时间 note 序列，沿用现有 `materialize` 结果作为上游输入。
- 采用 `0.2s` 固定时间窗切分整谱，生成长度为 `T` 的帧序列。
- **导出格式为 `[T, 33, 5]` 传感器值矩阵**（u8 量化，×255），不再在 Rust 侧做 2D 热图渲染：
  - 33 个传感器按全局索引排列：0-7=A 环, 8-15=B 环, 16=C, 17-24=D 环, 25-32=E 环
  - 8 个按键映射到 A 环位置（key i → sensor i）
  - 5 个通道为：`tap_instant`, `touch_instant`, `hold`, `slide`, `break`
- 值语义：
  - 瞬时通道（`tap_instant`, `touch_instant`, `break`）：每个事件加 1.0，允许多事件叠加
  - 持续通道（`hold`, `slide`）：窗口内激活时长占 `0.2s` 的比例（0.0-1.0），叠加
- 空帧（33×5 全零）在导出时跳过，仅存储有内容的帧；manifest 中记录 `frame_offsets` 供还原时序。
- slide 路径编码使用现有 `SlideDataGetter` 获取 HitArea 序列，按距离比例分配时间。

### 2. 空间编码（Python 训练侧）
- Rust 导出的 33 个传感器坐标（归一化 2D 位置）作为模型侧常量。
- 空间编码方式（Gaussian 核撒点、图卷积、可学习嵌入等）由模型自由选择，不属于 Rust 导出职责。
- 传感器坐标源自真实 maimai DX 硬件测量，存储在 `sensor_position.md` 和 `src/heatmap/sensor.rs`。

### 3. 模型结构
- 空间编码器将每帧的 `[33, 5]` 传感器数据编码成一帧向量 `h_t`（具体结构由训练侧决定）。
- 时间编码器使用 TCN，对整段 `h_1...h_T` 建模，输出带上下文的窗口隐藏状态 `z_t`。
- 局部头 `LocalHead` 为每个 `z_t` 输出一个 `local_score_t`，表示相对局部难度。
- 聚合层使用固定比例 `top-k pooling`（top 10%，带最小窗口下限）。
- 整谱头 `ChartHead` 从聚合后的整谱表示回归官方连续定数。

### 4. 训练与弱监督
- 监督信号只使用整谱官方定数，不额外引入等级区间约束。
- 训练目标以整谱回归为主，损失函数优先使用 MAE。
- `local_score_t` 不做直接监督，由整谱回归目标通过 `top-k pooling` 反向驱动。
- 数据切分按歌曲维度分组，确保同曲不同难度不跨训练/验证/测试集合。

### 5. 数据导出与训练接口
- CLI: `maidata-heatmap-dataset <chart_root> <output_dir> [limit]`
  - `limit`: 可选，限制导出前 K 首歌（调试用）
- 定数自动从 diving-fish API 拉取，ID >= 100000 的宴会场曲子自动跳过。
- 输出目录结构：
  ```
  <output_dir>/
  ├── manifest.json         # 样本元数据
  ├── <song_id>_<diff>.npy  # [N, 33, 5] u8，仅含非空帧
  └── ...
  ```
- manifest.json 字段：
  - `song_id`, `difficulty`, `chart_constant`
  - `file`: npy 文件名
  - `total_frames`: 整谱总帧数（含空帧）
  - `frame_dt`: 0.2
  - `frame_offsets`: `Vec<u32>`，每个存储帧对应的原始帧索引

## Public Interfaces
- `src/heatmap/encode.rs`: `HeatmapEncoder` — 物化 note → `[T, 33, 5]` f32
- `src/heatmap/sensor.rs`: `SensorLayout` — 33 传感器的 2D 坐标（供 Python 侧读取）
- `src/bin/maidata-heatmap-dataset/`: 数据导出 CLI
- 固定参数：时间窗 `0.2s`，传感器数 `33`，通道数 `5`

## Test Plan
- 解析与时序一致性：
  - 抽样谱面验证 note 的绝对时间与窗口归属正确
  - BPM 变化、hold 时长、slide start/track 时长正确映射
- 传感器编码正确性：
  - 单个 tap 写入正确传感器索引的 `tap_instant` 通道
  - break tap 同时写入 `tap_instant` 和 `break` 通道
  - touch 写入 `touch_instant` 通道
  - hold 覆盖比例正确（满覆盖帧 ~1.0，部分覆盖帧比例值）
  - 多事件叠加值正确累加
  - slide 路径传感器序列和时间分配合理
- 数据导出验证：
  - 空帧被正确跳过
  - `frame_offsets` 与原始帧索引对应
  - u8 量化值正确（f32 × 255, clamp 255）
  - 宴会场曲子（ID >= 100000）被过滤
- 模型验收：
  - 整谱定数回归优于 naive baseline
  - `local_score` 高峰窗口与人工直觉中的爆发段一致

## Assumptions
- 官方连续定数数据可用；来源: https://www.diving-fish.com/api/maimaidxprober/music_data
- slide 路径使用 `SlideDataGetter` 的 HitArea 距离比例近似分配时间，后续可替换为更精确实现
- `top-k` 默认 `top 10%`，带最小窗口下限
- 第一版片段难度为"相对局部难度"，非官方局部定数
