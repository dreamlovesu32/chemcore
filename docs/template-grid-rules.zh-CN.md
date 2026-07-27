# TemplateGrid 原生模型、编辑与往返规则

## 定位

`templategrid` 是 CTP 模板库的布局元数据，不是普通画布对象，也不是 `graphic`、`group` 或 `table` 的变体。它只描述模板库窗口如何把根级模板页排进槽位。普通 ChemSema 文档的对象列表中不得生成 TemplateGrid 场景对象。

ChemDraw 的用户入口是 Template Tool；Professional/Ultra 也可直接打开 CTP 文件。ChemSema 的对应入口位于左侧“模板库”切换器，顶部每个按钮代表一个模板库。左键打开模板库，右键进入布局编辑。

## 官方字段与单位

内核只接受一个根级 `templategrid`，并按官方类型读取以下必填字段：

| 字段 | CDX 类型 | 内核字段 | 规则 |
| --- | --- | --- | --- |
| `NumRows` | `INT16` | `rows` | 正整数 |
| `NumColumns` | `INT16` | `columns` | 正整数 |
| `PaneHeight` | `CDXCoordinate` | `paneHeight` | 正有限数；CDX 使用 16.16 定点解码 |
| `extent` | `CDXPoint2D` | `extent: [width, height]` | 两个正有限数 |

不允许用“数值较大时除以 65536”一类启发式修正。CTP/CDX 必须经过官方类型注册表和原生二进制解码器；ChemDraw COM 另存出的 CDXML 不作为字段单位的权威来源。

## 页、槽位与归属

1. 根级 `page` 按文档顺序映射到行优先槽位。
2. 有子对象或文本的页是模板；空页是显式空槽。
3. 根级页少于 `rows × columns` 时，只在末尾补空槽。
4. 根级页多于容量时拒绝导入，不截断。
5. 旧文件若把页嵌在 `templategrid` 下，只在根级页完全不存在时读取；再次写出时规范化为根级页，并保持同一槽位归属。
6. 每个非空模板必须恰好出现在一个槽位中。越界引用、重复引用、遗漏模板全部报错。
7. 模板身份使用源 `page@id`，不使用可因重排改变的数组序号。缺少页 ID 或页 ID 重复的模板库拒绝进入可编辑调色板。

`TemplateGridLayout` 是来源无关模型：

```json
{
  "rows": 2,
  "columns": 3,
  "paneHeight": 25.25,
  "extent": [2.75, 2.75],
  "cells": [0, null, 1, null, null, null]
}
```

`cells` 中的整数引用模板内容数组，`null` 表示空槽。模板调色板的 `chemsema.template-library.v1` 载荷对外使用稳定页 ID；布局编辑 API 内部使用经过完整校验的整数引用。

## 写回规则

- 修改布局时，只更新四个已建模字段和根级页顺序。
- 未建模的 `templategrid` 属性原样保留。
- 写出时每个槽位都有对应根级 `page`；空槽写成空页，最后写出空的 `templategrid`。
- `templategrid` 的子对象被清空，因为官方 CDXML 内容模型为 `EMPTY`。
- CDXML 与 CDX 使用同一布局模型；CDXML→CDX→CDXML 必须保持行列、窗格尺寸、单元尺寸、空槽和稳定页归属。
- 不存在 TemplateGrid、存在多个 TemplateGrid、缺少必填字段或字段非法时明确失败，不生成默认布局。

## GUI 行为

- 正常模式按 `NumColumns` 精确建列，按 `extent` 保持单元格宽高比，按 `PaneHeight` 控制可见窗格高度。
- 空槽可见但不可选择。
- 搜索只隐藏不匹配的模板，不重新排紧槽位。
- 模板可以拖到另一槽位；目标非空时交换，目标为空时移动。每次改变立即保存到本地模板库状态。
- 右键库按钮或弹窗中的 `Layout…` 打开由内核提供字段 schema 的布局对话框。
- 缩小容量若会移除非空模板必须拒绝；扩大会追加明确空槽。
- 布局状态跨关闭和重载保留。恢复供应商布局通过删除该库的本地布局状态完成，不能由隐式 fallback 覆盖。
- `Export` 下载已经应用当前布局的 CDXML 模板库；`Reset` 明确删除该库的本地布局并恢复来源文件。

## 提取和发布边界

`scripts/extract-chemdraw-template-libraries.mjs` 始终直接解码本机已授权的 `.ctp`，不再优先读取 ChemDraw COM 转换的 CDXML。目录清单使用 `chemsema.template-library-catalog.v2`，记录布局、占用槽数和空槽数。

ChemDraw 自带模板不进入仓库；`viewer/template-libraries` 仍是本地生成、被忽略的授权内容。仓库只提交解析、模型、编辑、门禁和提取脚本。

## 门禁

内核单测覆盖：

- 非默认行列、非整数尺寸和显式/尾随空槽；
- 嵌套页规范化；
- 容量不足、重复归属、遗漏和非法值；
- 未知 TemplateGrid 属性保留；
- 稳定页 ID 在重排和 CDX 往返后不变。

`template_library_gate` 对本机 29 个公开安装库、881 个非空模板检查：

- catalog 与内核字段一致；
- 容量、占用和空槽守恒；
- CDXML/CDX/CDXML 布局完全相等；
- 每个模板的 CCJS、渲染、图标和对象计数往返。

`template-library-browser-regression` 检查真实 WASM 页面中的精确 8×6 布局、4 个尾随空槽、窗格高度、右键字段对话框、7×7 扩容、拖拽到空槽、下载和重置持久状态。
