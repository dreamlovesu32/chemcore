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
- 已完成：`validate structural|chemical|roundtrip` 的稳定结构化 issue、显式分子化学校验和 CCJS/CCJZ/CDXML/CDX/SDF 目标格式语义/视觉往返。
- 已完成：编辑器可见区 scene chunk 加载、保留编辑与 undo 的 hydration、未变 entry/附件的 copy-on-write 保存，以及浏览器 Zip64 读写与安全整数拒绝边界。
- 已完成：smoke 门禁记录首 chunk I/O、只改末块的复用比例、附件吞吐，并把 Zip64/可见区行为纳入跨实现 conformance。
- 待发布归档：统一 Rust/JavaScript/Python 拒绝类固定 corpus，并运行和保存 10 万/100 万对象及 100 MB/1 GB 附件的 full performance 报告。

## 产品体验

- 改进在线 demo，让用户可以拖入 CDXML、导出 SVG/CDXML，并直接从浏览器整理可共享的 reduced repro。
- 添加简洁的入门示例，同时保持编辑器第一屏是可用工具界面。
- 为尚未支持的 CDXML 对象和部分导入情况提供更清楚的诊断信息。

## GUI 测试平台与展示可靠性

- 在主仓库内建设独立 `packages/gui-test`、版本化场景/报告/覆盖协议和测试构建专用 Test ABI；大型 trace、soak、VM 与安装包进入带哈希的外部制品存储。
- 以 WebdriverIO Tauri 验证真实桌面程序，以 Playwright 验证浏览器和 WebView2/视觉，以 Windows UIA/真实输入验证原生窗口、文件、剪贴板、Office、触摸和笔，并保留最终安装包 production black-box 门禁。
- 把现有 GUI、viewer interaction、stability、toolbar、text、large-document 和 Office 脚本迁移为同一数据场景，不继续扩展独立千行脚本；最终必须通过真实点击/拖拽/绘制覆盖每个用户功能、每类对象、全部公开属性及 `0/1/2/many` 同类/异类多对象组合。
- 在隔离 Hyper-V guest 中运行真实输入，不占用用户前台；所有 worker 合计限制为 10 个 CPU execution unit/30 GiB，并以多个隔离桌面并行。建立 source-to-scenario 影响图和内容寻址证据，只重跑受影响、过期和不可缓存测试，同时持续执行复杂/大文档真实构建与长期 soak。
- 建立状态模型、固定 seed 生成、自动失败收缩、fault profile 和 mutation qualification；flaky 不允许通过重跑转绿。
- 建立 `gui-pr`、`gui-nightly`、`release-qualification` 和 `demo-qualification`；正式展示候选须按[长期架构](./docs/gui-test-platform-and-demo-reliability.zh-CN.md)完成最终安装包、干净 VM、连续重复和长时间 soak 证据。

## 社区

- 通过 issues 和 discussions 收集真实兼容性文件，并把它们化简为可共享 fixture。
- 按来源应用、对象类型和输出路径标注兼容性报告。
- 文档持续聚焦稳定行为契约。
