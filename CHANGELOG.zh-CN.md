# Changelog

ChemCore 的公开变更记录会保存在这里。

## 1.0.0-beta.5

- 加入正式的 CLI 协议/版本报告、面向机器的 protocol contract、agent demo
  corpus、agent POC 工作流说明、发布质量矩阵，并把扩展后的 agent CLI 代码拆成
  `agent/` 模块目录，降低后续维护成本。

面向 agent 的 CLI 扩展、安装入口完善，以及桌面端/浏览器端的进一步稳定化。

- 将 `chemcore-cli` 作为桌面端安装包的一部分安装，加入安装器 PATH 注册、`chemcore-entrypoints.json`、随包 agent 指南，并可通过 `guide` / `doctor` 让机器调用方发现入口和文档。
- 明确 CLI 的两种调用方式：PowerShell 单命令用于独立任务，JSONL `session` 用于同一个已载入文档上的连续操作。
- 扩展 CLI agent 工作流，加入 `targets`、`context`、`detail`、`capture` 和 `copy`，覆盖稳定 selector、周边对象摘要、原始对象/详情查询、精确裁图，以及 Office/OLE 剪贴板 payload 生成。
- 增加确定性的高清截图能力，支持 object、molecule、node、bond、all、显式 bounds 和多目标 selection。多选截图使用所有目标 bounds 的最小并集，和 GUI 选择框一致，并支持绝对/相对单边扩展、固定像素尺寸、渲染元数据，以及 PNG/SVG 落盘校验。
- 增加 selection-box context 报告，返回截图框内的 objects、molecules、nodes 和 bonds，区分 `inside` 与 `partial`，标记显式选中目标，并继续返回这个框上下左右的周边摘要。
- 增加轻量级 CLI `new`/`run` 审计报告，包含文档 hash/revision 变化、新建/更新/删除 selector 摘要、失败命令详情、可选 `--inspect-after` 快照，以及 `--continue-on-error`。
- 提升 CLI 的 agent 可用性：文档、JSON、截图和 payload 写出后会校验文件存在和字节数；缺参数会返回机器可读修复信息；命令拼写错误会返回相近命令、用途、usage 和示例。
- 增加长驻 JSONL `session` 模式和 CDXML/CDX 自动导入缓存，让大文件上的重复操作可以复用已载入或已缓存状态，不必每条命令都重新解析。
- 优化 CLI 大文件检查与截图路径，加入目标范围 bounds、局部渲染和 `performance:cli-large` 报告，覆盖 CDXML 转换、target 发现、detail 查询、context 截图、精确截图、session 流程和 SVG 导出。
- 修复带标签端点附近的中心/双键渲染：平行双键线会分别根据端点标签退让和裁剪；同时更新 Office EMF preview 的短双键裁剪段 stroke 转换。
- 拆分浏览器/editor host，将文档渲染、viewport、toolbar、tab 和窗口生命周期拆到独立模块，降低大 viewer 文件的维护成本。
- 扩展桌面端和浏览器端稳定性覆盖，包括 pointer workflow、hybrid latency、viewer 操作、大对象编辑、拖拽预览、文本编辑、生成 fixture 和可重复稳定性报告。
- 收紧编辑器交互行为，覆盖选择、拖动、绘制、符号、括号、hover/focus 生命周期、grouped objects、混合对象 workflow，以及当前工具下侧边栏面板的激活。
- 更新 README 语言入口、中文 README 文案、渲染对比资源，以及公开 CLI 指南。

## 1.0.0-beta.4

大文件交互、CDXML 兼容和 agent-friendly CLI beta 版本。

- 新增 `chemcore-cli` crate 和直接 engine 命令，支持 headless 文档检查、转换、导出、编辑，以及结构化 JSON 执行报告。
- 新增 `--document-json`、`--inspect-after`，并改进 `.json`/`.ccjs` 处理，让脚本和 agent 可以不驱动 GUI 就交换 ChemCore 文档。
- 改进 CDXML 导入导出保真度，覆盖标签、箭头、符号、粗线宽度、自由基价态、grouped graphics、堆叠/连接标签、标签内部缓存 fragment 和括号标签。
- 将导入的 CDXML 成对括号表示为 bracket group，左右括号边可独立拖动，同时保留 repeat count 和括号标签语义。
- 收紧 glyph clipping、标签几何、导入标签锚点，并同步 synthetic SVG 快照。
- 重构大文件交互性能：局部化渲染更新、缓存拖拽预览输入、加入 fragment bbox 粗筛、减少整页刷新路径，并更安全地同步 deferred document update。
- 重建选择和拖拽预览管线，让大结构、标签、箭头、图形、括号和导入对象在高频编辑时保持视觉同步。
- 修复绘制提交后的刷新瑕疵，包括键预览保持、键创建 patching，以及矩形/近水平键的显示质量问题。
- 统一选择工具 hover，并清理绘制、对象创建、已选对象拖拽、括号/箭头编辑和多分子操作后的 hover、focus 与 overlay 残留。
- 明确 grouped object 选择语义：普通子对象拖拽保持独立，显式整组选中仍折叠为组选择框，只有真正被选中的对象才一起移动。
- 优化箭头、括号、图形和对象控制点，包括忽略括号内部空白的命中测试、抑制已选对象 hover，以及统一控制点样式。
- 新增浏览器文件拖拽/当前 viewer 打开、shared display scale 处理、更快的桌面端/viewer 开发脚本，并扩展交互和性能回归覆盖。
- 新增中英文 CLI 命令指南、公开交互反馈规则、早期项目历史说明，并更新 README 架构说明。

## 1.0.0-beta.3

安装包热修复 beta 版本。

- 修复 Windows NSIS 安装器里的 Office/OLE 注册 hook：现在会在实际安装目录中查找 `chemcore-office.exe`，不再固定假设它位于旧的 `resources` 子目录。
- 同时兼容安装目录根部和 `resources` 子目录两种 Office server 布局，避免旧打包实验路径影响注册。
- 加固安装后注册流程：安装器会优先尝试 machine-wide COM/OLE 注册；如果 machine 注册无法启动或返回失败码，会自动降级为当前用户注册。
- 加固卸载清理流程：卸载时会同时尝试清理 machine-wide 和 current-user Office/OLE 注册。
- 重新构建 Windows x64 安装包，并在清理安装痕迹后完成手动干净安装验证。

## 1.0.0-beta.2

第二个公开 beta release。

- 添加括号与计数文本之间的 link 关系，用于表达重复单元；支持右键菜单 Link/Unlink、`Ctrl+L` / `Ctrl+Shift+L` 快捷键、CDXML 导入时自动配对，以及编辑后的重复单元语义刷新。
- 改进括号文本编辑：括号绘制后生成的空标签会在下一次工具动作时丢弃，非空标签会在切换工具前提交；括号标签在文本工具下仍可聚焦和编辑；括号标签的位置与默认字体大小按 ChemDraw 括号 fixture 对齐。
- 修复重复单元的化学摘要：当括号计数文本已 link 且重复单元定义明确时，计数会参与分子式和分子量计算；取消 link 会解除计数语义，但不破坏括号自身的选择关系。
- 扩展 group 与括号相关的选择行为：双击分子会带上包围它的括号和已 link 的计数文本；group 内的普通文本仍可编辑；从选择工具切到其他工具时会清除当前选中状态。
- 修复绘制、修改弯箭头弧度、以及在选中对象之间移动鼠标后的 hover/focus 残留；已选中的标签、键和原子不会在选择框内部继续显示内部 hover。
- 补齐桌面端与浏览器端的编辑细节：窗口上沿在弹窗状态下也能拖动；编辑区域会拦截浏览器右键菜单和常见浏览器快捷键；右键菜单的乱码指示符已替换为稳定字符。
- 修复泛基团标签的化学摘要逻辑：选中含有 `R`、`R'`、`R''` 或已连接 `Ar` 的分子时，不再显示会暗示组成已确定的分子式或分子量。
- 将已连接的 `Ar` 标签按芳基泛基团处理，不再在结构标签编辑时误判为氩元素；显式元素替换仍通过元素工具链路完成。
- 重建浏览器 WASM engine 和 Windows 桌面端可执行文件，确保 Web 与桌面端使用同一套修正后的内核行为。
- 添加括号 CDXML 导入、重复单元 link、group 编辑、选中对象 hover 抑制、泛基团化学摘要，以及完整缩写展开摘要的回归测试和公开 fixture。

## 1.0.0-beta.1

第一个公开 beta release。

- 公开共享 Rust 化学编辑内核、浏览器 viewer、Windows 桌面壳，以及 Office/OLE 集成基础。
- 添加 CDXML/CDX 导入导出、SVG 导出、EMF preview 生成，以及面向 Word 的剪贴板/OLE payload 支持。
- 加入公开 synthetic CDXML 回归 fixture，并保留维护者本人绘制的真实论文图 benchmark 文件。
- 添加 GitHub Actions CI、GitHub Pages demo 部署、issue templates、roadmap 和渲染对比文档。
- 记录当前 beta 状态：源码构建已可用，Windows 安装包仍在测试中。
