# Roadmap

ChemSema 当前处于公开 beta 阶段。近期路线图重点是让编辑器更容易体验、更容易验证，也更适合外部贡献者参与。

## v1.0.0-beta 系列

- 发布可复现的浏览器端与桌面端构建说明。
- 保持 Rust tests、WASM 生成和浏览器 JavaScript 语法检查在 CI 中稳定通过。
- 围绕标签、箭头、括号、轨道、反应图和 Office 导出边界情况扩展 synthetic CDXML fixtures 与 SVG golden snapshots。
- 保留真实论文图对比作为高信号保真度 benchmark，同时把常规测试逐步迁移到 synthetic assets。
- 在干净安装、升级、卸载和 Office/OLE 注册经过多轮验证前，未签名 Windows 安装包继续留在 beta 渠道。
- 在桌面打包、文件关联、更新行为和 Office 复制粘贴验证足够稳定后，发布签名 Windows 安装包。

## 保真度与兼容性

- 为公开 synthetic fixtures 增加更多 ChemDraw oracle 对比报告。
- 为本机装有 ChemDraw 和 Office 的 Windows 环境补充可选 pixel-diff 与 EMF-record diff 流程。
- 持续加强 CDXML/CDX round trip、文本布局、箭头几何、键交汇和对象堆叠。

## CCJS 0.2 稳定化

- 保持 CCJS 0.2、CCJZ Container v1、Document Patch v1 和 Recovery Journal v1 独立版本化，并禁止未声明 ZIP entry 或第二套层级真相。
- 将 `validate structural|chemical|roundtrip` 补齐为稳定结构化诊断：error code、JSON Pointer/entry、规范条款、严重级别和信息损失等级。
- 把按目标格式的语义/视觉往返夹具与 Rust、JavaScript、Python 拒绝类夹具统一成可发布 conformance corpus。
- 在编辑器接通可见区 scene chunk 懒加载和未变 entry 的 copy-on-write 保存；在此之前不把底层 range reader 宣传成端到端懒加载。
- 为浏览器 writer 明确经典 ZIP 上限和 Zip64 策略，并发布 1 万/10 万/100 万对象及 10 MB/100 MB/1 GB 附件的 full performance 报告。

## 产品体验

- 改进在线 demo，让用户可以拖入 CDXML、导出 SVG/CDXML，并直接从浏览器整理可共享的 reduced repro。
- 添加简洁的入门示例，同时保持编辑器第一屏是可用工具界面。
- 为尚未支持的 CDXML 对象和部分导入情况提供更清楚的诊断信息。

## 社区

- 通过 issues 和 discussions 收集真实兼容性文件，并把它们化简为可共享 fixture。
- 按来源应用、对象类型和输出路径标注兼容性报告。
- 文档持续聚焦稳定行为契约。
