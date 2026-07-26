# 分子着色与结构高亮规则

## 语义边界

ChemDraw 21 实测确认了两套相互独立的对象：

- 结构高亮写在原子和键的 `highlightColor` 属性上。它表示沿结构中心线绘制的粗色带，不是普通键色，也不是环内部填充。
- 环填充写成 fragment 内的 `ColoredMolecularArea`，以 `bgcolor` 保存颜色，以 `BasisObjects` 保存组成一个环的键。

ChemSema 在 CCJS 中也明确分开保存：

- Node/Bond 的 `highlightColor?: "#RRGGBB"`；
- MoleculeFragment 的 `coloredAreas[]`，每项含 `id`、`color` 和 `basisBonds[]`。

不得用 `face`、源颜色表编号、缓存多边形或 `meta` 代替这些字段。

## ChemDraw 实测绘制规则

### 结构高亮

对 ChemDraw 21 导出的 CDXML、SVG、EMF 做同一苯环探针后得到：

1. 高亮原子是以原子坐标为圆心的实心圆。
2. 高亮键是沿键中心线的圆帽粗线；等价几何是“矩形加两端圆”。
3. 原子圆半径和键色带半宽相同。
4. 半径为文档 `BoldWidth + MarginWidth`。探针中 `2.60 + 2.00 = 4.60 pt`。
5. 高亮层在普通键和标签下方；相邻原子、键依靠同色几何并集自然连接。

节点移动时只改变节点坐标，高亮几何在每次渲染时从当前坐标重算，不保存缓存轮廓。

### 环填充

`BasisObjects` 必须引用同一 fragment 中的键，并且这些键必须恰好形成一个连通简单环：

- 至少三条不同的键；
- 环内每个节点的度数恰好为 2；
- 键数等于节点数；
- 全部节点连通。

填充边界按环上节点的遍历顺序连接，位于普通键和标签下方。节点移动后多边形从当前节点坐标重算。

右键“Ring Fill”对当前选择中的所有无弦环逐个生成区域。这个规则使稠环选择得到各个最小环，而不会把带有内部共享键的外周大环误当成一个填充区域。开链或不完整环不显示该菜单，也不会创建无效对象。

## 生命周期

- 右键命令立即形成一次 Document Commit，进入 undo/redo、revision 和自动保存链路。
- 删除任一 basis 键会删除已失效的环填充区域。
- 复制完整环会复制区域并重映射所有 basis 键 ID；部分复制不携带该区域。
- 拆分 fragment 时，区域只进入完整包含其 basis 键的分量。
- CDX/CDXML 导出统一重建颜色表编号和对象 ID；CCJS 始终保存十六进制颜色与本地对象 ID。
- 非法颜色、重复区域 ID、跨 fragment 引用、非键引用和非简单环引用均为明确错误，不使用猜测或降级绘制。

## 右键菜单

- 选择至少一个分子原子、标签原子、键或完整分子时显示 `Highlight`。
- 只有当前选择包含至少一个完整无弦环时显示 `Ring Fill`。
- 两个子菜单均提供标准颜色、`Other...` 和 `Remove`。
- `Other...` 复用统一颜色选择器；最终修改仍通过 Rust 内核命令提交。

## 透明度边界

官方 CDXML DTD 中 `coloredmoleculararea` 只有 `id`、`bgcolor` 和 `BasisObjects`，没有 `alpha`、`bgalpha` 或其他透明度属性；ChemDraw 21 的该对象探针也只写这三个字段。因此本对象只接受 `#RRGGBB`，不凭空增加一个无法无损写回 CDX/CDXML 的透明度字段。根/page 的 `alpha`、`bgalpha` 是别的对象层级语义，不能移植到 `ColoredMolecularArea`。

## 回归门禁

门禁覆盖：

- ChemDraw 实际 `highlightColor` CDX tag `0x0308`；
- `ColoredMolecularArea` CDX object tag `0x8032`；
- CDXML、CDX、CCJS 往返；
- `BoldWidth + MarginWidth` 色带半径和圆帽；
- 环多边形层级与实时重算；
- 完整环/部分环右键菜单；
- 添加、改色、移除、撤销与删除 basis；
- 复制、粘贴、fragment 拆分中的强引用维护。
