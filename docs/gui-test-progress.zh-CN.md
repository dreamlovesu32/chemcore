# ChemSema GUI 完整测试总清单与进度

最后更新：2026-08-11  
状态：持续实施；**尚未达到完整 GUI 资格，也尚未达到展示资格**  
登记场景：**27**

当前产品候选：`cea5f3522ac36743fb15e32e922a59d616efd91ebcc7c99d0e9e9736ba8f9b9b`（源码闭包 `2cd1a01b55b773db8bd2d16d032f4dac36a241646a3d53452859d2a74cf6326a`）

当前源码闭包已登记场景资格：**27/27 passed，0 failed，0 missing，0 diagnostics**；完整 GUI 功能矩阵仍未完成，本文总体状态不变

本文是 GUI 测试工作的唯一总进度表。[长期架构文档](./gui-test-platform-and-demo-reliability.zh-CN.md)说明为什么和怎样测试；本文只回答四个问题：已经完成什么、还缺什么、下一步是什么、什么时候才算结束。

## 1. 状态含义

- ✅ **已关闭**：该清单项定义的完整验收边界已有当前有效证据，不再包含未列明的公开值或用户路径。
- 🟡 **部分完成**：已有真实 GUI 证据，但仍存在本文明确列出的值域、对象数量、交互、格式或规模缺口。
- ⬜ **未开始/尚无正式证据**：可能已有产品代码、单元测试或旧脚本，但还没有进入版本化 production GUI 资格。
- ❌ **阻断**：存在未修复失败，不能通过后续重跑覆盖。

“某个场景通过”不等于“整个功能族完成”。例如色谱场景已经通过，但泳道增删、多个斑点、全部显示开关、凝胶标签/尺寸/值域和大文档仍然是公开缺口，所以色谱族仍标为 🟡。

## 2. 什么时候才算全部完成

只有以下六道总门禁全部为 ✅，本目标才结束：

- [ ] 所有用户可见工具、对象、菜单、对话框、属性和值域都进入机器可读 registry；没有“以后再补”的未登记功能。
- [ ] 每个功能完成真实点击/绘制/输入，覆盖默认值、普通值、边界值、无效/取消、混合值，以及 `0/1/2/many` 同类和异类对象。
- [ ] 每个适用功能完成撤销/重做、保存/关闭/重开、剪贴板、组合/锁定、导入导出和独立文件语义校验。
- [ ] `small/complex/large/xlarge` 均从空白文档真实构建，并完成继续编辑、增量更新、性能、内存、恢复和 canonical fingerprint 校验。
- [ ] 环境、故障注入、状态模型、mutation、无障碍、多 DPI/主题/区域/输入方式矩阵通过，并完成至少 24 小时混合 soak。
- [ ] 最终安装包在干净 VM 上完成升级/卸载/重装，正式展示流程连续 1,000 次无失败；第一次失败不能被重跑抹去。

## 3. 总体工作包

### A. 测试平台基础

| 状态 | 工作包 | 当前证据/剩余边界 |
|---|---|---|
| ✅ | 版本化场景、运行报告、证据与资格 Schema | 场景、run、artifact、qualification 均严格校验 |
| ✅ | 隔离 production 真实输入 | Hyper-V 专用桌面、真实鼠标键盘、UIA/CDP 观测，不占 host 前台 |
| ✅ | 专用物理 Windows 真实输入 | 独立 `physical-windows` adapter 使用当前专用机账户、真实 `SendInput`、UIA/CDP 分离观察、账户/会话/PID/可执行文件/前台窗口逐动作 fail-closed；不由 Computer Use 陪跑 |
| ✅ | 资源上限 | Hyper-V 保留原聚合门禁；物理机由本地 profile 和心跳动态记录资源，低于安全内存余量时暂停，不以固定 10 CPU/30 GiB 限制替代机器健康 |
| ✅ | 内容寻址候选 | 可执行文件与源码闭包哈希绑定，源码或二进制漂移时拒绝运行 |
| ✅ | 原生 Windows 对话框 | 保存/打开使用真实 UIA 与键盘输入，保存文件经 SHA-256 回传 |
| ✅ | 独立文件 oracle | 已支持化学计数及 Arrow、Text、Shape、Symbol、Bracket、Table、Orbital、Chromatography 精确属性 |
| 🟡 | 独立前端状态 oracle | 已用当前候选实机通过焦点归属、键盘焦点环、`:hover`、禁用态、受限计算样式、选择覆盖层/对象几何、控制点像素尺寸、实际 viewport 与 `devicePixelRatio`；尚缺完整键盘 `:focus-visible` 顺序、全部光标/主题/DPI/窗口尺寸矩阵 |
| ✅ | 失败证据保留 | 首次失败、截图、DOM、日志、trace、保存文件和 manifest 不被后续通过覆盖 |
| ✅ | 性能 trace 与动作分阶段计时 | 区分定位、输入、产品完成、原生窗口消失、回传和最终状态 |
| ✅ | fail-closed 资格汇总 | 缺失、候选混用、证据哈希错误、先失败后通过均保持红灯 |
| 🟡 | 脱离 Codex 的连续后台队列 | 已实现单实例租约、15 秒心跳、PID 清单、提交/候选/profile/queue 哈希绑定、逐场景 checkpoint、资源暂停、停止请求和 evidence manifest 哈希；已完成 7 场景无人值守批次及当前前端 2 场景批次，分别复算 49 和 14 个证据对象；事件唤醒仍需完成 active-writer 退避验收和重启续跑 |
| 🟡 | 精确影响选择与证据复用 | 已有 source→component→capability→scenario 传递图；仍需覆盖全部源文件、生成物、安装包和环境轮换 |
| ⬜ | 自动场景生成、模型探索与失败收缩 | generator/model/shrinker 尚未形成正式可执行闭环 |
| ⬜ | 正式 CI 分层 | `gui-pr`、`gui-nightly`、demo/release qualification 尚未全部接入托管 CI |

### B. 用户功能与对象族

| 状态 | 功能族 | 已有真实覆盖 | 明确剩余 |
|---|---|---|---|
| 🟡 | 分子、原子、键 | 单键绘制、历史、多键/混合选择与剪贴板 | 11 种键工具、元素/标签/电荷/氢、环、链、模板、立体化学、反应语义、全部属性与格式 |
| 🟡 | Arrow | 多对象属性、锁定混合、属性持久化 | 全部直接绘制预设、所有 head/curve/no-go/color 值、组合/大文档 |
| 🟡 | Text | 新建、既有编辑、多行、主要样式、行距、取消、历史、持久化 | 局部选区、全部字体/字号/对齐/行距边界、IME/composition、Formula、端点标签、锁定/组合/大文档 |
| 🟡 | Shape | 四种 kind、五种代表样式、批量样式、历史、持久化 | 全颜色/Faded、控制点、缩放/旋转、锁定/组合/剪贴板、格式与大文档 |
| 🟡 | Charge/Electron Symbol | 八种公开 symbol、批量颜色、历史、持久化 | 原子/标签附着、轨道式放置、化学/Link 结果、变换、组合、格式与大文档 |
| 🟡 | Bracket | 三种成对括号、可见侧属性、层级、历史、持久化 | 标签、repeat Link、分子包含、控制柄、锁定/组合、格式与大文档 |
| 🟡 | Table | 插入、2×2→3×3、对齐、边框、历史、持久化 | 全部增删位置、内容、清空/适应、全部对齐/边组合/颜色、锁定/剪贴板/格式/大文档 |
| 🟡 | Orbital | 七种模板、双向几何迁移、批量模板/样式/相位、历史、持久化 | 全颜色、全部 style×phase、原子/标签附着、变换、组合、格式与大文档 |
| 🟡 | TLC/Gel Chromatography | 两种板、12 泳道、批量颜色、斑点/条带移动、历史、精确文件校验 | 泳道/标记增删、多标记、TLC 开关全值、凝胶标签/宽高/可见性/范围/单位、格式与大文档 |
| ⬜ | Rings、Chain、Template Library | 仅有旧测试或底层能力 | 每种公开模板真实插入、连接、继续绘制、属性、历史、持久化、搜索/库切换 |
| ⬜ | Biology-assisted drawing | 产品含 10 个 family、24 种公开 kind | 每种对象实际绘制、全部专有属性/控制柄、组合、保存、格式与规模矩阵 |
| 🟡 | Selection/Group/Lock/Clipboard | `0/1/2/many`、区域/追加、混合、嵌套组、锁定部分适用、跨文档粘贴 | 重叠/隐藏/视口外、所有对象族、套索、排序/对齐/分布、跨 group、系统/Office 边界、大文档 |
| 🟡 | 文档生命周期 | CCJS Save As/Open/继续编辑、dirty close | 多标签、覆盖/权限/磁盘满、autosave/journal/crash recovery、所有格式、large/xlarge |
| ⬜ | Image/Spectrum/Geometry/Constraint/Annotation/Stoichiometry 等 | 有产品或格式代码，但无完整 production GUI 族 | 创建、命中、编辑、属性、历史、关系、保存和导入导出全部待登记 |
| 🟡 | 前端交互、无障碍与语义定位 | 主画布、主工具栏、二级 toolbar、菜单、原生输入已有稳定定位；已登记焦点环、hover、disabled、选择框/控制点和 150% 缩放场景 | 键盘 `:focus-visible` 全顺序、全部光标/禁用组合、屏幕阅读器语义、高对比度、多语言名称、全部 modal |
| ⬜ | Office 与外部边界 | 仅有既有 Office/CLI 诊断资产 | Word/PowerPoint 可编辑粘贴、回写、preview、剪贴板格式、失败恢复与最终安装包闭环 |

### C. 规模、可靠性与发布

| 状态 | 工作包 | 完成条件 |
|---|---|---|
| ⬜ | Complex 文档 | 从空白真实创建异类对象、分子、文本、图形、关系、嵌套组和混合属性的长序列 |
| ⬜ | Large 文档 | 数百对象或约 1,000 原子；从空白构建及打开后继续编辑两条路径 |
| ⬜ | Xlarge 文档 | 初始 5,000 原子或等价渲染/交互复杂度；增量刷新、内存、handle、保存和恢复 |
| 🟡 | 性能与资源 | 已有 trace/动作延迟/10 核 30 GiB 上限；尚缺按规模的正式延迟、内存和泄漏门槛 |
| 🟡 | 环境矩阵 | 已机器校验本机实际 150%（DPR 1.5）与 1280×900 CSS viewport；仍缺 100/125/175/200%、不同窗口尺寸/分辨率/多屏、主题、区域、WebView2/Windows 版本、GPU/软件渲染、触摸/笔/IME |
| ⬜ | 故障注入 | 磁盘满、权限、剪贴板占用、服务中断、保存失败、崩溃、网络/Office 不可用 |
| ⬜ | 状态模型探索 | 长随机动作序列与模型状态逐步比对，保存 seed 并自动复现 |
| ⬜ | Mutation qualification | 主动植入代表性错误，证明测试能杀死已知错误类别 |
| ⬜ | 24 小时 soak | 复杂混合旅程持续运行，零 crash/hang/未处理错误/不可恢复状态 |
| ⬜ | 1,000 次展示资格 | 同一不可变最终候选连续 1,000 次正式展示流程零失败 |
| ⬜ | 最终安装包资格 | 干净 VM 安装、冷启动、升级、卸载、重装、文件关联和回归闭包全部通过 |

## 4. 已登记的 27 个场景

所有 27 个场景均已实现并进入 registry，并已由当前不可变 production 候选取得单候选 27/27 资格。这只证明当前 registry 闭包，不表示对应功能族或本文列出的完整 GUI 矩阵已经覆盖；后续新增场景仍必须产生新证据，不能把本次绿色资格冒充最终产品资格。

| 当前候选 | 场景 | 验证内容 |
|---|---|---|
| ✅ | `core.bond.draw-single` | 浏览器公开输入绘制单键基线 |
| ✅ | `core.bond.draw-single.production` | production 真实 OS 输入绘制单键 |
| ✅ | `core.history.undo-redo-bond.production` | 单键撤销/重做 |
| ✅ | `core.selection.clipboard-delete-multi-bond.production` | 多键选择、复制粘贴、删除、历史 |
| ✅ | `core.selection.clipboard-delete-mixed-bond-arrow.production` | 分子/Arrow 混合剪贴板、删除、历史 |
| ✅ | `core.selection.region-additive-mixed-cardinalities.production` | `0/1/2/many` 区域与 Shift 追加选择 |
| ✅ | `core.group.nested-mixed-clipboard.production` | 混合嵌套组、复制、批量解组、历史 |
| ✅ | `core.clipboard.cross-document-mixed.production` | 跨文档标签页混合粘贴 |
| ✅ | `core.selection.locked-partial-delete.production` | 锁定对象在混合删除中的部分适用 |
| ✅ | `core.selection.locked-transform.production` | 锁定/可编辑对象混合移动与解锁 |
| ✅ | `core.selection.locked-molecule-arrow-transform.production` | 锁定分子与可编辑 Arrow 混合移动 |
| ✅ | `core.group.locked-ancestor-transform.production` | 锁定组祖先、后代静止、解锁恢复 |
| ✅ | `core.arrow.multi-property-history.production` | 多 Arrow 公共属性与事务历史 |
| ✅ | `core.arrow.locked-mixed-properties.production` | 锁定/可编辑 Arrow 混合属性 |
| ✅ | `core.arrow.property-matrix-persistence.production` | Arrow 属性矩阵与精确 CCJS |
| ✅ | `core.text.multi-property-persistence.production` | 双 Text 批量公开样式与精确持久化 |
| ✅ | `core.text.line-spacing-validation.production` | 多行、行距、非法值、取消、历史 |
| ✅ | `core.text.existing-edit-history.production` | 既有 Text 替换、取消、历史与持久化 |
| ✅ | `core.shape.multi-kind-style-history.production` | 四种 Shape、批量样式、历史与持久化 |
| ✅ | `core.symbol.eight-kind-color-history.production` | 八种 Symbol、颜色、历史与持久化 |
| ✅ | `core.bracket.three-kind-properties-history.production` | 三种 Bracket、可见侧、层级与持久化 |
| ✅ | `core.table.structure-border-history.production` | Table 结构、对齐、边框、历史与持久化 |
| ✅ | `core.orbital.seven-template-properties-history.production` | 七种 Orbital、几何迁移、属性与持久化 |
| ✅ | `core.chromatography.tlc-gel-mark-color-history.production` | TLC/Gel、内部颜色、标记拖动、历史与持久化 |
| ✅ | `core.document.save-open-roundtrip.production` | 原生保存、独立校验、重开与继续编辑 |
| ✅ | `core.frontend.focus-hover-disabled.production` | 真实点击后的焦点归属、焦点环、hover、disabled 样式与 150% DPI |
| ✅ | `core.frontend.selection-geometry.production` | 真实绘制/框选后的选择框、控制点、画布焦点/hover 与缩放几何 |

当前候选的剩余 production 批次 `physical-current-impact-remaining-production-20260811-1786432745932` 已无人值守连续通过 24/24；24 份报告、24 个 manifest 和 190 个证据对象（192,057,841 bytes）已独立复算一致。与前端 2 场景及浏览器基线合并后，正式 qualification 为 27/27 passed，211 个 artifact hashes 全部验证，qualification SHA-256 为 `7b322d35950d85357975f7f1d8f6ef62e45fdbb7353c9972381440be96199607`。资格文件位于仓库外 `%LOCALAPPDATA%\ChemSema\gui-test\qualification-current-9d4adab-r2\qualification.json`。过程中保留并修复了 action budget、Delete/方向键 SendInput、Windows 原子状态替换竞争等首次失败。

当前候选的前端批次 `physical-frontend-150-percent-20260811-1786432325957` 已通过 2/2。真实鼠标/键盘观测为 1280×900 CSS viewport、DPR 1.5；Zoom in hover 背景为 `rgb(238, 243, 248)`；Tab 后 Zoom out 为唯一焦点，outline 为 `auto/0.666667px`；Save 为 disabled，cursor `default`、opacity `0.35`。真实绘制和框选后有 1 个选择框、8 个 6×6 CSS px resize handle，画布同时满足 focused、focus-within 和 hover，active element 为 `viewer-container`。2 份报告、2 份 manifest 和 14 个证据对象已独立复算；报告 SHA-256 分别为 `60b814ece8c734ac5ea3a7c85caec4124c1a2dab64a72ee37293a29361413f11`、`a0c9632ccd1d6eaefceb0b29e10d420d1582cbfac8d929b7b1a1dd54cb46dddc`，manifest SHA-256 分别为 `a4683637e0be1a0f2379c9d5b2963f4753ce30c6651289b1d3c61d875d3129e1`、`859240a2dd85fea032cd63420be58c96d49cb86c67dc956207dcced93991d842`。此前发现的画布焦点滞留和 1.5 CSS px 手柄问题已修复并由当前候选回归关闭；PowerShell 空样式、单元素数组和 CDP 大小写键设施失败的原始证据仍保留。

## 4.1 物理工作节点第一阶段记录

- 正式仓库由 GitHub 全新克隆，最低可信基线 `dc9d8a78b1f7ebfcc42b7077ec49f842650fef20` 已验证；退役项目仓库按日期完整归档，用户化学文档未删除。
- 全新依赖基线：`npm ci` 0 漏洞、GUI 平台初始 72/72；物理节点、守护进程和前端 oracle 测试持续增加，当前为 86/86、audit 27 场景/0 gap/0 warning；每次提交仍需 `CI=true npm run verify`。
- 本机 profile 位于 `%LOCALAPPDATA%\\ChemSema\\gui-test\\profiles\\physical-current.json`；机器名、账户、MachineGuid 哈希和证据均不提交 Git。
- 物理 adapter 与 Hyper-V adapter 并存；Hyper-V 仍强制专用 guest 账户，物理 adapter 精确绑定本机当前账户和 session 1，不配置 autologon。
- 当前 registry 的 27 场景已有单候选完整资格；这不关闭尚未登记的功能、属性、格式、规模、环境和稳定性缺口。
- 第一阶段尚未完成：正式 NSIS 安装/文件关联验证、事件触发器 active-writer 无空档续接、重启续跑、PR CI 收口。

## 5. 下一阶段执行顺序

执行顺序是有限的，不再按“想到一个测一个”推进：

1. 🟡 **物理节点第一阶段收口**：完成 NSIS 安装/文件关联、事件触发无空档续接验收、干净提交与 PR/CI、重启 checkpoint 续跑；保持已登记 27/27 资格。
2. **化学绘制主干**：11 种键、原子/标签/电荷、环、Chain、Template Library、反应连接与属性。
3. **补齐已开工对象族值域**：Arrow、Text、Shape、Symbol、Bracket、Table、Orbital、Chromatography 的公开值和 `0/1/2/many`。
4. **Biology 与其他专用对象**：24 个 biology kind、plasmid、Image/Spectrum/Geometry/Constraint/Annotation/Stoichiometry。
5. **文档与外部边界**：多标签、所有格式、恢复、系统剪贴板、Office、文件关联。
6. **Complex/Large/Xlarge**：先从空白构建，再验证打开后继续编辑；建立硬性能和资源阈值。
7. **环境、故障、模型与 mutation**：补齐非正常路径和主动杀错能力。
8. **24 小时 soak → 最终安装包 → 连续 1,000 次展示**：三项均为不可折扣的最终门禁。

## 6. 强制更新规则

本清单不是一次性说明：

- 每新增、删除或重命名一个 registry 场景，必须同步修改“登记场景”数字和第 4 节表格；自动测试会逐个检查 27 个场景 ID，漏项直接失败。
- 每完成一个对象族或发现新的公开缺口，必须同时更新第 3 节状态和“明确剩余”，不能只在长架构文档末尾追加段落。
- 每产生新候选或 qualification，必须更新页首候选哈希、通过/缺失/失败数和最新证据。
- 每个本地测试提交必须让本文反映该提交后的真实状态；不得把“场景通过”写成“功能族完成”。
- 失败只有在根因修复并产生新候选后才能从“当前阻断”移走；原失败证据仍永久保留。
