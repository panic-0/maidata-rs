# 基于传感器时序的谱面定数回归方案

## Summary

当前训练管线以整张谱面为单位，使用弱监督方式回归官方连续定数。输入是按 `0.2s` 切分的传感器时序 `[T, 33, 5]`；Rust 侧负责导出压缩后的传感器帧，Python 侧负责重建时序、切块、编码和训练。

## 当前实现

### 1. 数据表示

- Rust 导出 `[N, 33, 5]` 的非空帧 `npy` 文件，并在 `manifest.json` 中记录：
  - `song_id`
  - `difficulty`
  - `chart_constant`
  - `file`
  - `total_frames`
  - `frame_dt`
  - `frame_offsets`
- Python 数据集按 `frame_offsets` 将压缩帧重建为稠密序列 `[T, 33, 5]`。
- 重建后将每帧展平为 `[T, 165]`，再按固定窗口切成重叠 chunks：
  - `chunk = 20s` (`100` 帧)
  - `stride = 10s` (`50` 帧)

### 2. 模型结构

- `ChunkEncoder`
  - 每个 chunk 输入形状为 `[chunk_frames, 165]`
  - 先做线性投影 `165 -> d_model`
  - 再经过多层 dilated causal TCN
  - 对时间维做 `mean + max pooling`，输出 chunk 表示
- `ChartEncoder`
  - 将整谱的 chunk 表示序列送入 TransformerEncoder
  - 使用基于可学习 query 的 attention pooling 得到整谱表示
- `PredictionHead`
  - 通过两层 MLP 回归整谱定数
- 训练时还会输出 `chunk_scores` 作为辅助诊断信号，但当前主损失只监督整谱预测值

### 3. 训练与评估

- 标签只使用整谱官方连续定数
- 数据切分按歌曲维度进行，避免同曲不同难度泄漏到训练/验证两侧
- 当前主指标是 chart-level MAE

## 代码入口

- `training/dataset.py`: 数据重建、切块、batch padding
- `training/model.py`: `ChunkEncoder + ChartEncoder + PredictionHead`
- `training/train.py`: 训练、验证、日志与 checkpoint
- `training/predict.py`: 加载导出器与训练权重做定数预测

## 当前限制

- chunk 之间尚未加入显式位置编码
- 切块逻辑当前不会覆盖最后不足一个 stride 的尾段
- 输入目前直接展平为 `165` 维，尚未引入传感器拓扑的专门空间编码
- 训练/验证之外还没有独立 test split 或 baseline 对照
