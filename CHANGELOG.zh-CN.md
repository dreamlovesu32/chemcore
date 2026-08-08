# Changelog

ChemSema 的公开变更记录会保存在这里。

## 未发布

- 将当前原生文档体系升级为 CCJS 0.2 + CCJZ Container v1：规范化平铺 scene、独立 hierarchy/relations、确定性分块 ZIP、内容寻址资源、opaque attachments、旧 gzip 只读迁移、原子保存和 hash-chain recovery journal。
- 增加精确 Document Patch 桌面更新路径、Rust/浏览器 JavaScript/独立 Python 交叉读取、`validate`/`canonicalize`/`migrate`/`conformance` CLI，以及容器和 5000 原子交互性能门禁。
- 补齐 CCJS 0.2 原先列明的实现短板：稳定结构化诊断、显式分子化学校验、CCJS/CCJZ/CDXML/CDX/SDF 目标格式往返、编辑器可见区分块加载、保留 undo 的 hydration、未变 entry 与附件 copy-on-write、浏览器 Zip64，以及相应 conformance/performance 门禁。
- 确立 GUI 测试平台与展示可靠性长期架构：测试平台留在主仓库并独立版本化场景、报告、覆盖和 Test ABI，最终覆盖真实 Tauri、Playwright WebView2、Windows UIA、生产安装包黑盒、模型/故障/变异测试及展示资格。
- 新增可复现的公共 CDXML/CDX 往返基准集，固定 RDKit、Indigo、cdxml-toolkit、
  SAMPL6 和 SAMPL9 中 413 个许可证清楚的文件，并生成机器可读的基线报告。
- 将公共语料门禁升级为三代语义指纹，并修复其中暴露的全部未预期漂移：元素与同位素
  标签、单原子、悬空键、表示电荷的符号、元素列表查询、无头箭头、括号分组与锚点，
  以及双键副线的虚线样式。
- 恢复导入 attached atom label 的连接感知布局：CDXML 文本的横向对齐不再压过
  `NH`/`NH2` 的自动翻转或上下堆叠，同时节点级显式 `LabelDisplay` 仍保持最高优先级。
- CDXML 文件加载现在可以兼容少量错误声明为 UTF-8 的 Windows-1252 标点字节；其他
  文档格式仍保持严格 UTF-8 路径。

## ChemSema 1.0.0-beta.1 — 2026-07-19

这是 ChemSema 名称下的第一个公开 beta。

- 仓库、代码接口、包、应用、CLI、skills、文档、生成 bindings 和公开网址统一改为
  ChemSema 身份。
- 新品牌公开版本从 `1.0.0-beta.1` 重新开始；为保留既有历史 tag，采用唯一 tag
  `chemsema-v1.0.0-beta.1`。
- 为此前的 GitHub 仓库和 Pages 入口保留永久兼容处理，并加入自动监测。
- 包含此前 beta 线最新的编辑器、Office/OLE、agent CLI、CDXML/CDX、渲染和化学
  标签改进。
- 新增共享 Rust 化学语义层，在原生端和浏览器端统一提供 SMILES 导入、canonical
  isomeric SMILES 分析、常见价态与芳香性校验、隐式氢、四面体与 E/Z 立体化学、
  分子属性以及官方 IUPAC InChI/InChIKey。
- 新增仓库引用与归档元数据，包括作者 ORCID、GitHub 引用入口和 Zenodo release
  元数据。
- 本地验证改为比较 WASM 构建前后的生成物，不再把开发中有意保留的未提交 WASM
  变更误判为生成物未同步。

## 1.0.0-beta.8

对齐 ChemDraw 的化学标签解释与连接锚点、稳定且语义可读的 CDX/CDXML 往返、
短键渲染保真度，以及覆盖所有 ChemSema 外壳的 Windows/Ubuntu CLI 交付链路。

- 重构结构化化学标签的 token 与显示方向规则。`C10H21` 这类分子式组合既保留
  内部元素/计数结构，又能在 attached label 翻转时作为一个整体移动；多组合分子式
  按化学组翻转，因此右侧连接的 `C10H21O3` 可以显示为 `O3C10H21`，同时保留
  source text 和数字下标。`TFA` 等已知缩写、`Cu(II)` 等氧化态标签，以及金属开头
  的化学文本现在都有明确解释规则，不再依赖普通文本 fallback。
- 收紧标签化学诊断和隐式氢行为。价态解析器不再为了让第二周期元素通过校验而
  暗中添加形式电荷或扩展八隅体，并区分“价态无效”和“文本无法解释”；显式的
  node 级氢数量 override（包括作者写入的 `NumHydrogens="0"`）可稳定经过 CDXML
  往返。
- 重做 CDX/CDXML 富文本导入导出，使对齐、翻转、普通/上标/下标 run、字号、
  字体 face 和标准颜色表在 open-save-open 后保持稳定。原生 CCJS/JSON 现在保存
  可读的语义值，例如字体名称、明确的文本样式、十六进制颜色和具名锚点目标；
  源格式的字体、face、颜色和 attachment 编号只在 CDX/CDXML 边界重新生成。
- 加入 ChemDraw `BeginAttach` / `EndAttach` 的语义支持。显式的标签内部连接点会
  根据当前 glyph geometry 解析，并在导出时写回；当源锚点不存在或无效时，内核
  保留结构节点/主键锚点，不再猜测离节点最近的 glyph，使 fallback 行为稳定一致。
- 修复配位结构和分子式标签的连接几何。同一行多字符裁切现在使用每个 glyph 自己
  的轮廓和相邻字符真实的垂直重叠范围，较低的括号、下伸部和上下标不会再把无关
  字符的裁切区域或键锚点向下拉。`N`、`Pd`、`P(OPh)2` 等标签由统一规则恢复接近
  ChemDraw 的垂直对齐，不使用逐分子特例。
- 修复标签附近的双键退让。中心双键的两根子线沿各自实际偏移轴线计算裁切，并在
  每个端点共同采用较大的退让量，从而保持最终可见长度一致；侧双键的主键继续位于
  结构/attachment 轴线上，不再因为平行线间距把末端标签偏移半个双键间距。
- 修复未冻结双键在新增取代键后出现精确侧别平局时的自动换边。编辑过程按最新键
  所在侧裁决，而无编辑上下文的导入仍保持确定性；GUI 会立即反映新位置，不再需要
  切换 ACS/default 样式才能刷新正确。
- 对齐 ChemDraw 对极短虚键的自适应分段，包括从三段到两段、再到一段可见线段的
  过渡。OCR 对比规则会把只剩一段的虚键视为与对应单键在视觉上不可区分，但不会
  改变文档中的实际键语义。
- 重新构建并交叉核验浏览器、Windows 桌面端、鸿蒙 ArkWeb 外壳、原生 CLI 和
  Codex skills 使用的共享内核。Web 与鸿蒙包携带完全相同的 WASM artifact，桌面端
  和 CLI release build 使用同一 Rust 内核行为；鸿蒙构建也能在干净环境中生成
  unsigned HAP，且不会覆盖本地签名配置。
- 新增一等的 Ubuntu/WSL CLI 验证和 Linux skill 打包。ChemSema CLI skill 现在同时
  携带带 manifest hash 的 `win-x64` 与 `linux-x64` 运行时；新增 WSL build/smoke/test
  命令验证 Linux binary，并在 CI 中加入独立 Ubuntu job，执行 engine/CLI 测试和
  skill contract 检查。无头 PNG 截图在 fontconfig 没有提供 alias 时，也会把通用
  字体族绑定到 Linux 环境中实际存在的字体，因此最小化 Ubuntu 环境即使没有
  Microsoft 字体也能正常渲染文本。
- 扩展化学标签翻转、CDX/CDXML 稳定导出、显式与推断锚点、逐 glyph 裁切、中心/
  侧双键和短虚键的回归覆盖。更新后的内核还对 ChemSema OCR registry 前 1000 个
  分子完成了新旧像素对比审阅，未替换数据库记录。

## 1.0.0-beta.7

对象级 agent 工作流、事务式 CLI 编辑、结构化文档 diff，以及围绕可编辑化学文档的
公开定位调整。

- 新增 `chemsema-cli` Object-Grounded Agent Layer。新的 `bundle` 命令可以把一个
  target selector 打包成完整工作单元，包含 `target.json`、`context.json`、
  确定性的 `capture.png`/`capture.svg`、只包含目标范围的可编辑子文档、
  `identity-map.json`、`provenance.json`，以及明确区分 editable scope 和
  visual scope 的 manifest。
- 新增结构化文档 diff：`chemsema-cli diff` 会按 object、resource、style、
  molecule node、molecule bond 和字段路径身份比较 ChemSema 文档，而不是做原始
  JSON 文本 diff。这样 before/after 报告可以用于 agent 审计、回归测试和人工确认。
- 新增事务式 command-script 支持。transaction envelope 可以声明期望的 document
  hash/revision、必需 selector、可编辑范围、是否允许创建/删除、dry-run 行为、
  atomic 执行，以及 document-valid、selector-exists、no-unexpected-changes 等
  postconditions。
- 扩展 JSONL session 工作流，支持 `bundle` 和 transaction execution。长驻 agent
  session 可以在同一个已载入文档上完成检查、打包、dry-run、编辑和验证，不需要
  反复解析文件或切换协议。
- 增强 bundle provenance 和 selector identity tracking。provenance 会记录隐私友好
  的源文件元数据、source document hash、source bounds、visual bounds、子文档平移、
  editable subset 计数和 identity-map 摘要，默认不把本地绝对路径写入持久 artifact。
- 新增真实对象级 workflow 示例：
  `examples/agent/07-object-grounded-edit`。该示例基于 `figure1.cdxml`，对一个真实
  molecule object 执行 bundle、before/after capture、dry-run transaction、受范围
  约束的 node-label 修改、结构化 diff、完整文档导出、target-only 可编辑导出，并
  生成 `acceptance.json` 验收报告。
- 更新 CLI protocol 文档、runtime schema/capabilities discovery、command-script 与
  session 指南、中英文 CLI guide，以及 ChemSema CLI skill 参考，使新 bundle、diff、
  transaction、identity-map 和 provenance 契约都能被机器调用方发现。
- 重构中英文 README 的前部定位，把 ChemSema 更明确地描述为可编辑化学文档平台，
  并突出同一套对象身份如何贯穿可视化编辑、Office 工作流、CLI 检查、受范围约束的
  agent 修改、验证和可编辑导出。
- 改进真实论文 CDXML 图的渲染保真度。键与键相交时会保留 ChemDraw 风格的透明
  margin knockout；标签退让由 glyph clipping 和导入的 `MarginWidth` profile 驱动；
  attached molecule label 现在对末端字母、prime、上下标使用按 ChemDraw 校准的同一
  行锚点。
- 刷新 README 公开视觉素材，包括 ChemSema 编辑器界面截图，以及基于
  `figure1.cdxml` 和 `figure2.cdxml` 重新生成的 ChemDraw/ChemSema 对比资产。

## 1.0.0-beta.6

鸿蒙 PC 端基础、GUI 一致性回归、agent/CLI 工作流扩展，以及新一轮编辑器保真度修复。

- 新增第一阶段鸿蒙 PC 端壳。新的 `apps/chemsema-harmony` 工程把现有 Web Viewer 和共享 Rust/WASM 内核打包进 ArkWeb rawfile 应用，面向桌面级 `2in1` 设备，并加入 DevEco 构建/签名模板、应用图标、viewer 同步/构建脚本，以及覆盖打开、保存、新建、文档标签页、剪贴板桥接、窗口标题和 rawfile 资源的 smoke/bridge 回归。
- 隔离浏览器、Tauri 桌面端和鸿蒙宿主壳行为。鸿蒙端现在使用 native-frame 顶栏和紧凑系统风格文档标签页，不显示桌面端自绘窗口控制；Tauri 桌面端保留自己的自绘标题栏；浏览器模式保持浏览器原生行为。
- 扩展共享 viewer 的端到端 GUI 回归，覆盖打开、保存、导出、`Ctrl+S`、应用内复制/粘贴/剪切、工具栏图标、光标、选择覆盖层、删除工具、缩放、文档样式预设、宿主壳隔离、工具栏健康、鸿蒙桥接和大 CDXML 速度检查。
- 将更多交互反馈和预览行为移动到共享 Rust/WASM engine 路径。键创建预览、对象坐标预览、渲染目标查询、悬停/聚焦反馈、图形命中半径、预览依赖追踪、缩放锚点和空文档渲染现在在浏览器、桌面端和鸿蒙端之间更一致。
- 修复编辑器选择和删除边界问题。单独选中键会渲染无白色描边的键中心选择点，完整分子选择会隐藏内部键中心点；极小端点框选与点击端点能力一致；删除工具点击已聚焦双键时会先降级为单键再删除；三键降级、端点删除和极短 wavy bond 渲染也都有回归覆盖。
- 扩展 CLI 图片和截图工作流。`convert`/`export` 可以写出 PNG，`capture` 省略 `--out` 时可默认写入临时 PNG，quiet report 返回机器可读输出路径；多目标 `capture`/`context` 支持 selection-only 裁图、重复或分号 selector、显式裁剪框、扩边、固定输出尺寸、PNG/SVG 输出和渲染元数据。
- 新增面向 agent 和脚本的 GUI 同语义编辑命令。`select-targets`、`select-all`、`clear-selection`、基于选择态的排列/样式/删除/组合/链接操作、`plan-bond`、`plan-template`、`insert-template`、显式双键位置和键线粗细覆盖，让 CLI/JSONL 工作流可以使用 GUI 同款语义，而不是模拟鼠标。
- 改进标签、OCR 和命令式化学编辑工作流。`label-query` 支持源文本和可见文本反查，并包含连接与含氢锚点语义；直接命令可编辑节点电荷和氢标签，保留 label source text，并保留导入或 OCR 得到的端点框、glyph polygon 和 label text position。
- 改进 CDXML/文档导入保真度和样式默认值。导入时保留 CDXML molecule fragment，默认把断开的化学结构拆成独立 molecule object，保留交叉但断开的 fixture 覆盖；ACS/文档样式默认值会贯穿导入、命令、渲染、标签、键、SDF/CDXML 路径和导出。
- 修复 Office 全文档复制行为，并刷新桌面/Office 架构文档，确保全文档和目标级剪贴板/OLE payload 路径清晰，同时化学逻辑仍保留在共享 engine 中。
- 新增公开的 ChemSema Codex skill 套件和 agent 示例。本版本包含 CLI、development、drawing-agent、Office skills 及中英文说明、安装/同步脚本、运行时发现、session helper、仓库卫生和构建测试参考，并加入完整 reaction-agent POC：请求、命令、截图、context/detail/targets JSON、Office payload、CDXML/SVG 输出和运行器。
- 更新中英文 README、CLI 指南、command-script/JSONL session 协议说明、CLI-GUI parity checklist、编辑器命令历史、项目规则、架构文档和公开元数据；同时重建共享浏览器/鸿蒙 WASM engine artifact，使所有壳使用最新内核行为。

## 1.0.0-beta.5

- 加入正式的 CLI 协议/版本报告、面向机器的 protocol contract、agent demo
  corpus、agent POC 工作流说明、发布质量矩阵，并把扩展后的 agent CLI 代码拆成
  `agent/` 模块目录，降低后续维护成本。

面向 agent 的 CLI 扩展、安装入口完善，以及桌面端/浏览器端的进一步稳定化。

- 将 `chemsema-cli` 作为桌面端安装包的一部分安装，加入安装器 PATH 注册、`chemsema-entrypoints.json`、随包 agent 指南，并可通过 `guide` / `doctor` 让机器调用方发现入口和文档。
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

- 新增 `chemsema-cli` crate 和直接 engine 命令，支持 headless 文档检查、转换、导出、编辑，以及结构化 JSON 执行报告。
- 新增 `--document-json`、`--inspect-after`，并改进 `.json`/`.ccjs` 处理，让脚本和 agent 可以不驱动 GUI 就交换 ChemSema 文档。
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

- 修复 Windows NSIS 安装器里的 Office/OLE 注册 hook：现在会在实际安装目录中查找 `chemsema-office.exe`，不再固定假设它位于旧的 `resources` 子目录。
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
