# 质粒图对象规则

本文记录 ChemSema 对 ChemDraw `plasmidmap`、`plasmidregion` 和 `plasmidmarker` 的原生语义、交互与交换规则。实现不得把质粒图降级成普通圆、箭头和文本，也不得用缺省值掩盖缺失的关键字段。

## ChemDraw 入口与实测行为

- 创建入口：`View > Show BioDraw Toolbar`，选择 BioDraw 工具栏中的 Plasmid Map，再点击画布。
- 首次点击弹出 `Insert Plasmid Map`，要求输入碱基对总数。
- 选中质粒图后，右键菜单提供 base pairs、显示碱基数、Regions 和 Markers 等编辑入口。
- 真实 ChemDraw 21 探针中，Regions 对话框把输入的 `11000–1000` 规范化为 `1000–11000`；箭头位于起点还是终点由独立选项决定，不能从端点输入顺序推断。
- 标记文字可以独立拖动。拖动文字只改变标签角度和径向距离，不改变标记对应的碱基位置。
- 区域起点、终点和中段控制点具有不同语义：端点改变碱基位置，中段只改变径向偏移。

探针夹具为 `crates/chemsema-engine/tests/fixtures/cdxml/plasmid-map.cdxml`；同一对象另存为 CDX 后确认对象标签为 `0x8026/0x8027/0x8028`，分别对应 map、marker、region。

## CCJS 原生模型

质粒图存储在 shape 对象的 `payload.plasmidMap`，而不是 `payload.extra` 中的未类型化字段：

- `numberBasePairs`：大于零的整数，碱基坐标域为 `1..=numberBasePairs`。
- `radius`、`lineWidth`、`boldWidth`、`marginWidth`：单位均为 pt。
- `showBasePairs`、`labelFont`、`labelSize`、`labelFace`、`color`：完整保存中央标签与样式。
- region：独立保存 `start/end/offset/arrowAtStart/arrowAtEnd/fill/width/color/alpha`。
- marker：独立保存 `position/label/offset/labelAngle/color`。`labelAngle` 可空；为空时跟随碱基位置，有值时表示用户已经独立移动标签。
- region 与 marker 的 ID 必须非空并在同一质粒图内唯一。所有数值均要求有限；不合法输入直接报错，不进入 fallback。

## 坐标与换算

- `0°` 位于十二点方向，角度顺时针增加。
- 原生碱基位置的圆周角：`(position - 1) / numberBasePairs * 360°`。
- ChemDraw `MarkerAngle` 使用另一套定点值：`position / numberBasePairs * 600 * 65536`。两种公式不可混用。
- `RingRadius` 是 16.16 定点坐标，导入时除以 `65536`，导出时乘以 `65536`。
- `MarkerOffset`、`RegionOffset` 和 `ArrowShaftSpacing` 在实测文件中以百分之一 pt 保存，导入除以 `100`，导出乘以 `100`。
- 导入标记时，碱基语义取 `Value`；标签位置取子文本的 `p`。由标签位置相对圆心反算 `labelAngle` 和 `offset`，不使用标记碱基角度覆盖显式标签位置。

## 渲染与命中

- 圆环、区域、标记引线、中央碱基数和标签全部通过共享 `RenderPrimitive` 生成，因此 GUI、SVG、PNG 和 EMF 使用同一语义。
- region 的圆弧方向由 start 到 end 的顺时针 sweep 决定；跨零点的外部文件仍可无损显示。
- 多圈不是独立对象特例：不同 region 的 `offset` 直接形成不同半径。
- 选择工具聚焦圆环、区域或语义控制点时显示全部 region/marker 控制点；普通子圆、子箭头和子文本不会再被重复导入，因而不会抢占命中。
- 标签拖动实时写入 `labelAngle/offset`；region 端点拖动写入碱基位置；region 中段拖动写入 `offset`。三者都进入同一撤销栈。

## GUI 行为

- 左侧 BioDraw 工具进入顶部 BioDraw 工具栏；当前首个子工具为 Plasmid Map。
- 单击或拖拽创建后，内核发出质粒编辑对话框描述。对话框可编辑总碱基数、圆半径、显示开关、字体/线宽样式、region 和 marker。
- region 反向输入按 ChemDraw 行为排序数值端点；起点/终点箭头开关保持独立。
- 新建对话框取消时回滚创建；确认时创建与最终参数合并为一个撤销步骤。右键 `Plasmid Map...` 编辑现有对象时只产生一个普通编辑步骤。
- 新增 ID 使用当前对象内的最小可用递增编号，不使用随机数或时间戳。

## CDX/CDXML 往返

- CDXML 原生写回 `<plasmidmap>`，包含中央文本、圆环 graphic、regions、markers 及其标签/引线。
- 通用 graphic/text/arrow 导入器会跳过 `plasmidmap` 内部派生子对象，避免一份语义被导入两次。
- CDX 对象标签：map `0x8026`、marker `0x8027`、region `0x8028`。
- CDX 专用属性：map 的 `NumberBasePairs=0x1300`、`RingRadius=0x1307`；marker 的 `MarkerOffset=0x1302`、`MarkerAngle=0x1303`；region 的 `RegionStart=0x1304`、`RegionEnd=0x1305`、`RegionOffset=0x1306`。
- 保存再打开必须保留总碱基数、区域范围和箭头端、标记位置及显式标签几何；CDX 中重新分配对象 ID 不构成语义差异。

## 回归门禁

- ChemDraw 真实 CDXML 和 CDX 双格式导入。
- CDXML、CDX 二次解析。
- CCJS 数值域、唯一 ID 和非有限数值拒绝。
- 单击创建、对话框确认/取消、右键编辑和一次撤销。
- marker 标签、region 起点/终点和径向偏移的直接拖动。
- GUI、SVG、EMF 的非空原生渲染，以及通用子对象不重复导入。

