# Glyph 裁剪规则（定稿）

## 目标

本文固定 ChemCore 当前字形裁剪规则，避免内核、SVG、EMF、Word/OLE 再各自推导出不同结果。

运行时唯一有效的裁剪几何来源是：

- [shared/glyph_clip_polygons.json](../shared/glyph_clip_polygons.json)

生成脚本是：

- [scripts/generate-glyph-clip-polygons.py](../scripts/generate-glyph-clip-polygons.py)

运行时消费方是：

- [crates/chemcore-engine/src/glyph_kernel.rs](../crates/chemcore-engine/src/glyph_kernel.rs)
- [crates/chemcore-engine/src/render/labels.rs](../crates/chemcore-engine/src/render/labels.rs)
- [crates/chemcore-engine/src/render_objects.rs](../crates/chemcore-engine/src/render_objects.rs)
- [apps/chemcore-office/src/windows_office/emf_preview/renderer.rs](../apps/chemcore-office/src/windows_office/emf_preview/renderer.rs)

## 已废弃的旧逻辑

下面这些运行时形状合成逻辑已经不再参与裁剪：

- `ellipse`
- `rect-cut-*`
- `petal-*`
- 运行时 `convex_hull`
- 运行时圆采样拼接

`shared/glyph_profiles.json` 里的 `shape` 字段目前只保留为历史元数据，不再驱动裁剪几何。

## 现行规则

### 1. 布局基准

布局仍然使用 `glyph_profiles.json` 给出的：

- `advanceEm`
- `inkLeftEm / inkTopEm / inkRightEm / inkBottomEm`
- script 缩放与 baseline shift

也就是说，普通文本排版 metrics 不靠 `glyph_clip_polygons.json` 重新计算。

### 2. 大写字母

ASCII 大写字母 `A-Z` 的裁剪多边形按下面的固定流程离线生成：

1. 从 `Arial` 真实字形轮廓出发。
2. 做自然外扩：`0.30 * glyph height`。
3. 取字形锚点。
4. 锚点统一向字形内部偏移：`0.22 * glyph height`。
5. 以偏移后的点为圆心，做圆补强：半径 `0.60 * glyph height`。
6. 把“自然外扩区域”和“圆补强区域”取并集。
7. 结果离散为归一化多边形，写入 `shared/glyph_clip_polygons.json`。

### 3. 其他符号

除了 ASCII 大写字母以外，其余可见字符只做一件事：

- 自然外扩：`0.30 * glyph height`

不再追加 petal circle。

### 4. 未知字符 fallback

如果字符可见，但没有出现在 `glyph_clip_polygons.json` 中，运行时 fallback 为：

- 以 ink box 为基准
- 上下左右统一外扩 `0.30 * glyph height`
- 生成保守矩形

## 大写字母锚点表

记法说明：

- `point(c0, i)`：取第 0 个 contour 的第 `i` 个真实 on-curve 顶点
- `midpoint(c0, i, j)`：取第 `i`、`j` 两个真实 on-curve 顶点的中点
- `M/W` 明确排除中间谷底那组点，只取外轮廓四点
- 圆边、弧边采样点不算顶点；只有字形真实 contour 顶点参与

| 字母 | 锚点规则 |
| --- | --- |
| `A` | `midpoint(c0,1,2)`, `point(c0,0)`, `point(c0,3)` |
| `B` | `point(c0,1)`, `point(c0,0)` |
| `C` | 无圆补强 |
| `D` | `point(c0,1)`, `point(c0,0)` |
| `E` | `point(c0,1)`, `point(c0,2)`, `point(c0,0)`, `point(c0,11)` |
| `F` | `point(c0,1)`, `point(c0,2)`, `point(c0,0)` |
| `G` | 无圆补强 |
| `H` | `point(c0,1)`, `point(c0,6)`, `point(c0,0)`, `point(c0,7)` |
| `I` | `midpoint(c0,1,2)`, `midpoint(c0,0,3)` |
| `J` | `midpoint(c0,9,10)` |
| `K` | `point(c0,1)`, `point(c0,5)`, `point(c0,7)`, `point(c0,0)` |
| `L` | `point(c0,1)`, `point(c0,0)`, `point(c0,5)` |
| `M` | `point(c0,1)`, `point(c0,9)`, `point(c0,0)`, `point(c0,10)` |
| `N` | `point(c0,1)`, `point(c0,5)`, `point(c0,0)`, `point(c0,6)` |
| `O` | 无圆补强 |
| `P` | `point(c0,1)`, `point(c0,0)` |
| `Q` | `midpoint(c0,2,3)` |
| `R` | `point(c0,1)`, `point(c0,0)`, `point(c0,14)` |
| `S` | 无圆补强 |
| `T` | `midpoint(c0,2,3)`, `midpoint(c0,4,5)`, `midpoint(c0,0,7)` |
| `U` | `midpoint(c0,11,12)`, `midpoint(c0,0,1)` |
| `V` | `point(c0,1)`, `point(c0,9)`, `midpoint(c0,0,10)` |
| `W` | `point(c0,1)`, `point(c0,16)`, `point(c0,0)`, `point(c0,17)` |
| `X` | `point(c0,2)`, `point(c0,10)`, `point(c0,0)`, `point(c0,12)` |
| `Y` | `point(c0,2)`, `point(c0,10)`, `midpoint(c0,0,12)` |
| `Z` | `point(c0,6)`, `point(c0,7)`, `point(c0,0)`, `point(c0,12)` |

## 关键实现约束

### 1. 运行时不再推导 petal family

运行时不得再根据：

- `petal-nehkxz`
- `petal-a`
- `ellipse`
- `rect-cut-*`

之类的标签即时合成裁剪多边形。

### 2. EMF 不得另起一套

EMF/Office 预览必须直接消费引擎算出的 glyph polygons。

也就是说：

- 内核负责生成裁剪多边形
- EMF 只负责重放这些多边形
- 不能在 EMF 层再做一次独立的字形裁剪推导

### 3. 普通文本显示不要被裁剪规则反向影响

裁剪多边形和文本 metrics 分离：

- 文本布局、advance、baseline shift 继续来自 `glyph_profiles.json`
- 裁键/knockout 几何来自 `glyph_clip_polygons.json`

这样可以避免为了调裁剪，把普通文本显示再带坏。

## 重新生成

更新字形裁剪规则时，顺序固定为：

1. 修改 [scripts/generate-glyph-clip-polygons.py](../scripts/generate-glyph-clip-polygons.py)
2. 重新生成 [shared/glyph_clip_polygons.json](../shared/glyph_clip_polygons.json)
3. 跑 Rust 测试
4. 验证 SVG / EMF / Word 复制粘贴链路

不要再回到运行时用 `ellipse / chamfer / petal` 即时拼轮廓。
