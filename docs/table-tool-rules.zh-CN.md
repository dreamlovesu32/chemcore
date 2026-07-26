# Table Tool 与表格对象规则

## 1. ChemDraw 入口与实测结论

ChemDraw 21.0.0 的入口是独立的 `Table Tool`，不是 Shape Tool 的一种样式：

1. `Tools palette -> Table Tool`
2. 在空白处按下并拖出表格外框
3. 松开后出现 `Insert Table`
4. 输入 `Rows`、`Columns`；默认均为 2
5. `OK` 创建

Table Tool 下悬停或点击会聚焦单元格。右键单元格提供：

- `Borders...`
- `Add Row Before/After`
- `Add Column Before/After`
- `Delete Row/Column`
- `Clear Contents`
- `Size To Fit Contents`
- `Align`

`Borders...` 打开 `Table Borders`，包含 None、Box、All、Custom、Solid/Dashed、颜色、线宽和四边选择。

## 2. 官方对象层级

CDX/CDXML 的稳定层级是：

```text
table
└─ page (一个表格单元格)
   ├─ 单元格内容对象
   └─ border (0..4，分别为 top/left/bottom/right)
```

`border` 只在“作为表格单元格的 page”内有意义，不是独立顶层对象。相邻单元格共享的视觉边在文件中保存两份；渲染和导出不得擅自去重，因为两份样式可能不同。

没有显式 `border` 时使用表格/文档默认边框。显式 `LineWidth="0"` 表示隐藏该边。官方 `LineType` 的 Solid、Dashed、Bold、Wavy 均保存在内核；当前 ChemDraw 的 Table Borders UI 只暴露 Solid/Dashed，实测把输入的 Bold/Wavy 重新保存为 Solid，因此 ChemSema UI 同样只提供被验证的两项，但不会在导入时丢失官方值。

## 3. CCJS 原生模型

表格是一等 `type: "table"` 对象，不能使用 `type: "shape"` 或 `payload.kind: "crossTable"`。

```json
{
  "id": "obj_table_1",
  "type": "table",
  "transform": { "translate": [120, 80], "rotate": 0, "scale": [1, 1] },
  "payload": {
    "bbox": [0, 0, 200, 100],
    "table": {
      "rows": 2,
      "columns": 4,
      "rowGuides": [0, 50, 100],
      "columnGuides": [0, 50, 100, 150, 200],
      "cells": [],
      "defaultBorder": {
        "visible": true,
        "lineStyle": "solid",
        "width": 0.75,
        "color": "#000000"
      }
    }
  }
}
```

每个 cell 必须有唯一 id、明确 row/column、`contentObjectIds`、水平/垂直对齐和 top/left/bottom/right 四个可选覆盖。导线数组必须严格递增，长度分别为 `rows + 1` 和 `columns + 1`；cell 必须恰好覆盖所有行列位置。

## 4. 交互规则

- 选择工具：点击选中整表；拖动移动；边中点/角点拉伸整表；不提供旋转柄。
- Table Tool：空白处拖拽创建；表内悬停和点击只聚焦一个单元格，不把整表当成新建拖拽。
- 创建对话框规格由内核产生；前端只负责呈现和提交明确的 `add-table` 命令。
- 行/列插入复制相邻行/列的尺寸，并使表格外框相应增长；删除使外框按被删尺寸收缩。
- `Clear Contents` 只删除该 cell 显式关联的对象。
- `Size To Fit Contents` 使用内容视觉边界加 4 pt 四周内边距。
- 对齐会更新 cell 的语义字段，并把关联内容移动到对应位置。
- 所有操作必须进入统一命令历史，支持撤销/重做、复制粘贴和跨标签页传输。

## 5. 渲染与往返

- 每个单元格先绘制白色背景，再按 top/left/bottom/right 顺序绘制四边。
- 共享边不去重。
- Dashed 使用 butt 端帽；当前实测默认虚线节距为约 2.5 pt。
- Bold 和 Wavy 走明确渲染分支，不降级到 Solid。
- CDXML 导出使用 `table/page/border`；CDX 通过同一官方对象/属性表编码。
- CCJS、CDX、CDXML、SVG、EMF 必须由同一 `TableData` 产生，不允许格式专属隐藏 fallback。

CDX 二进制探针还确认了两个不能只照静态 SDK 表抄写的规则：

- ChemDraw 21 实际写出的 `border` 对象 tag 是 `0x801A`；发布版静态表中的 `0x8020` 是已记录勘误，`0x802A` 也不是 ChemDraw 的实际值。
- `Side` 属性 tag 是 `0x0825`，枚举值依次为 `top=1`、`left=2`、`bottom=3`、`right=4`。CDX 导入导出必须走明确枚举，不得把边方向当作未知数值保留。

单元格内容仍是正常的 ChemSema 场景对象，但只能被一个 cell 拥有。整表移动、拉伸、删除、复制和跨标签页粘贴必须同时处理 `contentObjectIds`；复制时表对象、cell id 和内容对象 id 都要重分配，导出 CDXML/CDX 时内容必须重新嵌入对应的 `table/page`，不能散落到顶层。

## 6. 可复现实验

运行：

```powershell
npm run probe:chemdraw-tables
```

输出位于 `tmp/chemdraw-table-probe`。脚本会让 ChemDraw 静默重存代表性 CDXML，并导出 CDXML、CDX、SVG、EMF，用于核对默认边、显式边、隐藏边和单元格内容。该目录是探针缓存，不进入版本库。
