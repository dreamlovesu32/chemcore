# 光谱对象规则

本文记录 ChemSema 对 CDX/CDXML `spectrum` 的来源无关模型、ChemDraw 实测规则、编辑边界和回归要求。氢谱、碳谱预测由独立的 ChemSema NMR 内核负责；本仓库负责把预测结果转换为同一套原生分子、文本和光谱对象。

## 1. 原生模型

场景对象固定使用 `type: "spectrum"`，几何放在 `payload.bbox`，语义数据放在 `payload.spectrum`：

- `class`
- `xLow`
- `xSpacing`
- `xType`
- `xAxisLabel`
- `yLow`
- `yScale`
- `yType`
- `yAxisLabel`
- `dataPoints`

这些都是来源无关的明确字段，不使用 CDXML `face`，也不把核心语义藏进 `meta` 或交换层。

枚举严格覆盖官方值；无法识别的枚举、非有限数、空数据数组、无效边界框、旋转和非单位 `scale` 都直接报错，不猜测、不回退。

## 2. 数据解码

第 `i` 个实际 Y 值：

```text
y[i] = yLow + dataPoints[i] * yScale
```

X 轴另一端：

```text
xHigh = xLow + dataPoints.length * xSpacing
```

ChemDraw 实测中第 0 个采样位于边界框右端，第 `i` 个采样位置为：

```text
x[i] = right - width * i / dataPoints.length
```

这会在左端保留一个采样间隔，不能把分母改成 `length - 1`。

## 3. CDXML 与 CDX 的存储差异

- CDXML 文本保存原始 `dataPoints`，`YLow`、`YScale` 是显式存储变换。
- CDX 的 `0x0A86 Spectrum_DataPoint` 是一个包含整组连续 `FLOAT64` 的属性，不是每个点一个属性。
- ChemDraw 21 写 CDX 时把 `/CS/CD/assign` 的估计范围折叠为标称位移（例如 `0.878-0.943` 变为 `0.91-0.91`）；CDXML 保留原范围。CDX 往返必须明确遵循这一格式差异，不能伪造已丢失的范围。
- ChemDraw 写 CDX 时先展开 `YLow + raw * YScale`，不再写 `YLow/YScale`；因此 CDX 导入后的规范形式为 `yLow = 0`、`yScale = 1`。
- `Class`、`XType`、`YType` 在 CDX 中是数值枚举，在 CDXML 中使用官方名称。

交换层仍保留未知属性与子对象；原生字段由 `payload.spectrum` 覆盖写出，删除原生对象后不得由交换层复活。

## 4. 绘制规则

- 先绘制完整矩形边框，再绘制坐标刻度、标签和曲线。
- X 轴刻度方向遵循实际 X 范围；数据方向始终以第 0 点在右端为准。
- 曲线 Y 范围取实际数据最小值与最大值，并在上下各扩展原始跨度的 5%；常量数据按其绝对量级生成非零范围。
- 有 `YAxisLabel` 时才绘制 Y 轴刻度、数值和竖排轴标签。
- 刻度使用统一的 1/2/5 × 10ⁿ nice-step 规则，不按光谱类别或样例分支。
- 大数组按可见宽度分桶，同时保留每桶极小值与极大值；绘制点数有上限，但原始数组不改变，窄峰不能被平均掉。
- GUI、SVG、PNG、EMF 使用同一组 `Line`、`Polyline`、`Text` 原语。

## 5. 编辑规则

支持：

- 结构化修改全部光谱字段；
- 移动和边界框拉伸；
- 组合、层级、颜色、线宽、复制粘贴、删除、锁定、显隐和撤销；
- CCJS、CDXML、CDX、SVG、PNG、EMF 导入导出。

不支持旋转。光谱没有 CDX/CDXML 原生旋转字段，边界框拉伸就是尺寸编辑；对象始终保存 `rotate = 0`、`scale = [1, 1]`。

完整选中且只选中一个连通分子时，右键菜单显示 `NMR Prediction`，其下提供 `Generate ¹H NMR Spectrum` 与 `Generate ¹³C NMR Spectrum`。预测成功后始终打开一个新的、可编辑的文档标签页，不修改来源分子。

预测结果页严格复用普通原生对象，不建立第二套显示或存储模型：

- 位移赋值写入对应 Node 的 `nmrAssignments`，ChemDraw `/CS/CD/assign` 导入也写入同一字段；
- 标题、质量图例和计算协议使用普通 `text` 对象；
- 谱图使用普通 `type: "spectrum"` 和 `payload.spectrum`；
- 结果页内的分子仍是普通 `molecule_fragment2d`，选择、移动、编辑和导出行为与任意分子一致。

ChemDraw 21.0.0.28 实测的页面布局为：页面 `523.32 × 769.92 pt`；标题从 `(25, 16.05)` 开始；分子区域从 `(28.8, 58)` 开始；质量图例从 `(14.4, 96.5)` 开始；谱图边界为 `(14.4, 119.85, 464.4, 319.85)`；协议文本从 `(14.4, 327.1)` 开始。位移标签字号为 `7.5 pt`，质量颜色固定为 good `#0000ff`、medium `#ff00ff`、rough `#ff0000`。

## 6. 实测矩阵与复跑

可重复探针：

```text
npm run probe:chemdraw-spectra
```

覆盖：

- NMR / ppm；
- IR / transmittance；
- 非默认 `YLow/YScale`；
- 反向边界框；
- 字体、字号、字形和线宽样式。

探针优先使用 64 位 ChemDraw COM；许可证不可用时明确重试 32 位 COM。每个样例保留 ChemDraw 生成的 CDXML、CDX、SVG、EMF，禁止用文件名或样例 ID 写渲染特例。
