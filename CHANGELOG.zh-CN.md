# Changelog

ChemCore 的公开变更记录会保存在这里。

## 1.0.0-beta.2

第二个公开 beta release。

- 修复泛基团标签的化学摘要逻辑：选中含有 `R`、`R'`、`R''` 或已连接 `Ar` 的分子时，不再显示会暗示组成已确定的分子式或分子量。
- 将已连接的 `Ar` 标签按芳基泛基团处理，不再在结构标签编辑时误判为氩元素；显式元素替换仍通过元素工具链路完成。
- 修复右键菜单中损坏的勾选/子菜单指示字符，改用稳定 ASCII 标记，避免出现乱码。
- 重建浏览器 WASM engine 和 Windows 桌面端可执行文件，确保 Web 与桌面端使用同一套修正后的内核行为。
- 添加泛基团化学摘要回归测试，同时保持完整缩写展开的分子式/分子量摘要能力。

## 1.0.0-beta.1

第一个公开 beta release。

- 公开共享 Rust 化学编辑内核、浏览器 viewer、Windows 桌面壳，以及 Office/OLE 集成基础。
- 添加 CDXML/CDX 导入导出、SVG 导出、EMF preview 生成，以及面向 Word 的剪贴板/OLE payload 支持。
- 加入公开 synthetic CDXML 回归 fixture，并保留维护者本人绘制的真实论文图 benchmark 文件。
- 添加 GitHub Actions CI、GitHub Pages demo 部署、issue templates、roadmap 和渲染对比文档。
- 记录当前 beta 状态：源码构建已可用，Windows 安装包仍在测试中。
