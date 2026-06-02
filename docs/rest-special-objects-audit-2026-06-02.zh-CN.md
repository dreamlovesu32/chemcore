# `tmp/rest.cdxml` 四类特殊对象审计

日期：2026-06-02

目的：对 `tmp/rest.cdxml` 中的 4 类特殊对象做一次严格审计，区分：

- 哪些结论已经被 `CDXML` 源文件和 `ChemDraw SVG` 直接证实
- 哪些实现已经落地且基本可靠
- 哪些地方仍然带有推断或明显遗漏

本审计不把“当前看起来像”当成通过标准，优先采用以下证据链：

1. `tmp/rest.cdxml`
2. `ChemDraw` 导出的 `SVG`
3. 现有内核/前端实现
4. `EMF` 对照

## 证据来源

### 源文件

- [`tmp/rest.cdxml`](D:/Projects/chemcore/tmp/rest.cdxml:1)

其中 4 类对象分别是：

- 波浪键：bond `28`，`Display="Wavy"`，见 [rest.cdxml](D:/Projects/chemcore/tmp/rest.cdxml:98)
- 空心锲形键：bond `32`，`Display="HollowWedgeBegin"`，见 [rest.cdxml](D:/Projects/chemcore/tmp/rest.cdxml:118)
- `田字`：`<table id="34">`，见 [rest.cdxml](D:/Projects/chemcore/tmp/rest.cdxml:124)
- TLC 板：`<tlcplate id="35">` 及其 `tlclane/tlcspot`，见 [rest.cdxml](D:/Projects/chemcore/tmp/rest.cdxml:147)

### ChemDraw SVG

- 现有 oracle：[`tmp/oracle-svg/rest.svg`](D:/Projects/chemcore/tmp/oracle-svg/rest.svg:1)

已经明确观察到：

- 波浪键是连续圆弧链，不是自由贝塞尔波线
- 空心锲形键导出结果表现为若干实心黑片，但这只是导出结果，不应直接当成内核对象建模规则
- `table` 是 2x2 表格
- `tlcplate` 有板框、两条虚线、底部刻度、黑色圆点

### EMF

本轮没有拿到新的可用 `ChemDraw EMF` 对照。尝试运行：

- `node scripts/chemdraw-oracle.mjs --out tmp/rest-oracle --formats svg,emf tmp/rest.cdxml`

结果 `SaveAs` 在 `EMF` 阶段失败，当前只能确认 `SVG` 证据链可靠，`EMF` 证据链待补。

相关工具：

- [`scripts/chemdraw-oracle.mjs`](D:/Projects/chemcore/scripts/chemdraw-oracle.mjs:1)
- [`scripts/compare-emf-oracle.mjs`](D:/Projects/chemcore/scripts/compare-emf-oracle.mjs:1)

## 对象逐项审计

### 1. 波浪键

#### 已证实

- `CDXML` 使用 `Display="Wavy"`，见 [rest.cdxml](D:/Projects/chemcore/tmp/rest.cdxml:105)
- `CDXML` 根节点设置了 `MarginWidth="2"`，见 [rest.cdxml](D:/Projects/chemcore/tmp/rest.cdxml:29)
- `ChemDraw SVG` 输出的是连续圆弧链，不是二次贝塞尔自由波形，见 [rest.svg](D:/Projects/chemcore/tmp/oracle-svg/rest.svg:1)

#### 当前实现

- 导入/导出映射已经接上：
  - [`cdxml.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/cdxml.rs:1267)
  - [`export.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/cdxml/export.rs:1581)
- 工具条已经接上：
  - [`toolbar.js`](D:/Projects/chemcore/viewer/toolbar.js:559)
  - [`wasm.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/wasm.rs:852)
- 渲染在：
  - [`render_bonds.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/render_bonds.rs:677)

#### 当前问题

- 当前实现虽然已经从“贝塞尔猜波形”改到了“圆弧链”，但仍然没有完成严格对齐。
- 最关键的遗漏是：**没有把 `MarginWidth` 真正接进波浪键的几何规则**。
- 目前波浪键吃到的是 `label_clip_margin_for_bond(...)` 这条链，不等于 `MarginWidth` 本身参与波形绘制。

#### 判断

- 可信度：低
- 当前状态：未完成
- 结论：这块仍然存在明显推断成分，不能视为已经对齐 `ChemDraw`

### 2. 空心锲形键

#### 已证实

- `CDXML` 使用 `Display="HollowWedgeBegin"`，见 [rest.cdxml](D:/Projects/chemcore/tmp/rest.cdxml:118)
- `ChemDraw SVG` 的导出结果会表现成多块实心片，但这不应直接当成对象建模规则

#### 当前实现

- 导入/导出映射已经接上：
  - [`cdxml.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/cdxml.rs:1135)
  - [`export.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/cdxml/export.rs:1561)
- 工具条已经接上：
  - [`wasm.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/wasm.rs:855)
- 当前渲染已改回：
  - **复用实锲形键 polygon**
  - **空心描边**
  - **不再按“四块黑片”直接建模**
  - 见 [`render_bonds.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/render_bonds.rs:1071)

#### 当前判断

- 这条建模路线比“按 SVG 结果拆黑片”更正确
- 因为它天然继承实锲形键的退让和接触逻辑
- 但是否已经完整对齐 `ChemDraw` 的描边宽度、tip 端闭合、底部 join，还没有完成严格审计

#### 判断

- 可信度：中
- 当前状态：方向正确，但仍需细化核对

### 3. `田字` / table

#### 已证实

- `CDXML` 是标准 `<table>`，不是伪装成几根线，见 [rest.cdxml](D:/Projects/chemcore/tmp/rest.cdxml:124)
- `ChemDraw SVG` 也是清晰的 2x2 表格

#### 当前实现

- 导入为 `shape.kind = "crossTable"`：
  - [`import_objects.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/cdxml/import_objects.rs:427)
- 渲染为外框 + 中横线 + 中竖线：
  - [`render_objects/graphics.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/render_objects/graphics.rs:17)
- 导出回 `<table>`：
  - [`export.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/cdxml/export.rs:689)
- 工具条也已接入：
  - [`wasm.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/wasm.rs:793)

#### 判断

- 可信度：高
- 当前状态：四类对象里最稳的一块
- 剩余工作：主要是后续交互和样式细节，不是结构问题

### 4. TLC 板

#### 已证实

- `CDXML` 有正式 `tlcplate/tlclane/tlcspot` 语义，见 [rest.cdxml](D:/Projects/chemcore/tmp/rest.cdxml:147)
- `ChemDraw SVG` 明确包含：
  - 外框
  - 上下两条虚线
  - 底部 lane 刻度
  - 黑色 spot 圆点

#### 当前实现

- 导入为 `shape.kind = "tlcPlate"`：
  - [`import_objects.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/cdxml/import_objects.rs:528)
- 渲染在：
  - [`render_objects/graphics.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/render_objects/graphics.rs:13)
- 导出在：
  - [`export.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/cdxml/export.rs:736)
- 工具条与交互入口在：
  - [`viewer/index.html`](D:/Projects/chemcore/viewer/index.html:141)
  - [`toolbar.js`](D:/Projects/chemcore/viewer/toolbar.js:816)
  - [`engine.rs`](D:/Projects/chemcore/crates/chemcore-engine/src/engine.rs:1645)

#### 已确认的问题

- 后端一直都有 spot circle primitive
- 但前端主渲染器之前漏掉了普通 `circle` primitive，导致点完全不显示
- 这个问题是在 [`primitive_dom_renderer.js`](D:/Projects/chemcore/viewer/primitive_dom_renderer.js:26) 找到的

#### 交互风险

- TLC spot 上下拖动和 `Rf` 提示功能已经接入
- 但这一块目前还没有经过 `ChemDraw` 交互级严格对照
- 也就是说：静态对象可信度高于交互逻辑可信度

#### 判断

- 静态渲染可信度：中高
- 交互可信度：中

## 当前总体结论

### 哪些基本不是猜的

- `田字`
- TLC 的对象结构
- TLC spot 数据本身存在且已导入
- 空心锲形键采用“实锲形 polygon + 空心描边”的建模方向

### 哪些仍然带较强推断成分

- 波浪键整体
- TLC 交互细节
- 空心锲形键的最终描边细节

### 明确漏项

- 波浪键没有真正接入 `MarginWidth`
- `ChemDraw EMF` 证据链目前缺失
- TLC 交互尚未完成 `ChemDraw` 侧逐项核对

## 下一步建议顺序

1. 先把波浪键单独拉出来，补完 `MarginWidth` 语义，再只对这一类做严格比对
2. 再把空心锲形键做完描边细节核对
3. 补 `ChemDraw EMF` 可复现导出链
4. 最后做 TLC 交互级对照，而不是继续边写边猜

## 风险分级

- 波浪键：高风险
- 空心锲形键：中风险
- `田字`：低风险
- TLC 静态渲染：中低风险
- TLC 交互：中风险
