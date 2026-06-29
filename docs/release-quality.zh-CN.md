# ChemCore 发布质量矩阵

这份矩阵记录主要公开能力当前的可信度。它是发布质量边界，不是营销承诺。

| 表面 | 状态 | 验证方式 |
| --- | --- | --- |
| CDXML 导入 | Beta | 公开 fixture、论文图、golden SVG snapshot、解析回归 |
| CDX 导入/导出 | Beta | round-trip 测试和二进制存储回归 |
| SVG 导出 | Usable | golden SVG snapshot 和像素比较脚本 |
| Office/OLE 复制与嵌入 | Beta | 剪贴板 payload、EMF preview、Word 粘贴/回读验证 |
| 浏览器编辑器 | Beta | viewer 交互 smoke test 和用户路径稳定性脚本 |
| 桌面端 | Beta | Tauri build、文件关联配置、hybrid latency 回归、安装验证 |
| CLI one-shot 命令 | Usable | Rust 测试、`npm run verify`、稳定性报告、输出写入验证 |
| CLI JSONL session | Experimental/usable | session 单测和大文件性能报告 |
| Agent 精确截图 | Usable beta | PNG/SVG capture 测试、公开 fixture crop、README 示例图 |
| Agent context/detail | Usable beta | selector/context/detail 测试和公开 fixture 示例 |
| 安装器 CLI PATH/App Paths | Beta | NSIS hook 和干净安装/卸载验证 |

## 安全基线

当前 beta 把这些区域作为硬化优先级：

| 区域 | 基线 |
| --- | --- |
| 文件导入 | 已有公开 fixture 和解析回归；恶意输入 corpus 继续扩展 |
| XML/CDXML 解析 | 已有 parser 测试；深度和大小限制属于 beta 硬化项 |
| 栅格/矢量导出 | 已验证输出路径、字节数；渲染超时和超大输出限制属于 beta 硬化项 |
| CLI session | 已有确定性 JSONL 协议；请求超时和资源预算策略属于 beta 硬化项 |
| 文件写入 | 已验证输出存在和字节数；更严格的写入范围策略属于后续工作 |
| Office payload | 已有剪贴板/OLE schema 测试；畸形 payload 验证继续补强 |

## 发布门禁

公开 beta 发布前：

1. 运行 `npm ci`。
2. 运行 `cargo build -p chemcore-office -p chemcore-cli --release`。
3. 运行 `cargo test`。
4. 运行 `npm run verify`。
5. 用 `npm run desktop:build` 构建安装包。
6. 确认 GitHub CI 在 `main` 和 release tag 上通过。
7. 上传安装包并记录 SHA256。

## 当前对外边界

ChemCore 已经在 CDXML 保真、Office 工作流和 agent-oriented CLI 上形成可验证原型。
它仍是 beta，需要更多真实文件、真实工作流、安全硬化和干净安装验证，之后才能被描述为完整 ChemDraw 替代品。
