# ChemSema GUI 完整测试总清单与进度

最后更新：2026-08-11  
状态：持续实施；**尚未达到完整 GUI 资格，也尚未达到展示资格**  
登记场景：**42**

当前产品候选：`b4465999da835e091ca6eef89a5c39a6584a7740f2848ac037d2dde8d7c9a5d2`（源码闭包 `f2d88eb180cd08f87a91606e283b923e55cec361b8b96c46f83939caa524c90d`）

当前源码闭包已登记场景资格：**41/42 reusable passed，0 product failed，1 pending，0 qualification diagnostics**。Electron→Nitrogen 批次已通过，重建的不可变 41/41 qualification 也已通过；六元环 Bond 融合、Element、正电荷及 Lone pair 的 locator/oracle 首次失败证据永久保留。Lone pair 首批已证明产品正确保持 `N/0/H2`、`NH2`、零 chemistry delta 与 atom/link 身份，失败来自 oracle 未把省略的 canonical 零 radical count 归一为 `0`；当前仅重启这一修复闭包。完整 GUI 功能矩阵仍未完成，本文总体状态不变。

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
| ✅ | 独立文件 oracle | 已支持化学计数及 Node、Bond、Arrow、Text、Shape、Symbol、Bracket、Table、Orbital、Chromatography 精确属性；Node oracle 会杀死错误元素、原子序数、电荷、radical count 及标签语义，Bond oracle 会杀死错误阶数、线型、粗细和楔键立体语义 |
| 🟡 | 独立前端状态 oracle | 已用当前候选实机通过焦点归属、上下文菜单结束后的画布焦点恢复、键盘焦点环、`:hover`、禁用态、受限计算样式、字体变化后文本紧边界、键选择框 12 CSS px 最小可操作尺寸、双选择框 16 个 6×6 CSS px 控制点、实际 viewport 与 `devicePixelRatio`；尚缺完整键盘 `:focus-visible` 顺序、全部光标/主题/DPI/窗口尺寸矩阵 |
| ✅ | 失败证据保留 | 首次失败、截图、DOM、日志、trace、保存文件和 manifest 不被后续通过覆盖 |
| ✅ | 性能 trace 与动作分阶段计时 | 区分定位、输入、产品完成、原生窗口消失、回传和最终状态 |
| ✅ | fail-closed 资格汇总 | 缺失、候选混用、证据哈希错误、先失败后通过均保持红灯 |
| 🟡 | 脱离 Codex 的连续后台队列 | 单批执行器已有单实例租约、15 秒心跳、PID 清单、提交/候选/profile/queue 哈希绑定、逐场景 checkpoint、资源暂停、停止请求和 evidence manifest 哈希；Electron 附着与重建的 41/41 qualification 已通过。六元环 Bond 融合、Element、正电荷及 Lone pair 的首次 test/oracle 失败证据均保留；当前只重启 Lone pair 的零 radical count oracle 修复闭包，且仍需 supervisor/子进程重启故障注入和长期终态唤醒验收 |
| 🟡 | 精确影响选择与证据复用 | 已有 source→component→capability→scenario 传递图；仍需覆盖全部源文件、生成物、安装包和环境轮换 |
| ⬜ | 自动场景生成、模型探索与失败收缩 | generator/model/shrinker 尚未形成正式可执行闭环 |
| ⬜ | 正式 CI 分层 | `gui-pr`、`gui-nightly`、demo/release qualification 尚未全部接入托管 CI |

### B. 用户功能与对象族

| 状态 | 功能族 | 已有真实覆盖 | 明确剩余 |
|---|---|---|---|
| 🟡 | 分子、原子、键 | 单键绘制、历史、多键/混合选择与剪贴板；十种非单键工具、Chain/环连接、Nitrogen Element/`NH2` 标签、正电荷 `N/+1/H3/NH3`、负电荷 `O/-1/H0`、Radical cation `N/+1/H2/radical1`、Radical anion `N/-1/H0/radical1` 及 Electron `N/0/H1/radical1` 附着已取得真实 OS 输入与精确持久化证据；Lone pair 场景已登记 | Lone pair `N/0/H2/radical0` 待实机证据；其他元素、氢值域、模板、端点反转、双键位置循环、立体化学、反应语义、全部属性与格式 |
| 🟡 | Arrow | 多对象属性、锁定混合、属性持久化 | 全部直接绘制预设、所有 head/curve/no-go/color 值、组合/大文档 |
| 🟡 | Text | 新建、既有编辑、多行、主要样式、行距、取消、历史、持久化 | 局部选区、全部字体/字号/对齐/行距边界、IME/composition、Formula、端点标签、锁定/组合/大文档 |
| 🟡 | Shape | 四种 kind、五种代表样式、批量样式、历史、持久化 | 全颜色/Faded、控制点、缩放/旋转、锁定/组合/剪贴板、格式与大文档 |
| 🟡 | Charge/Electron Symbol | 八种公开 symbol、批量颜色、历史、持久化；正电荷到 Nitrogen、负电荷到 Oxygen、Radical cation、Radical anion 与 Electron→Nitrogen 的化学/Link 精确场景已通过，Lone pair→Nitrogen 场景已登记 | Lone pair 附着待实机证据；其余两种原子/标签附着、轨道式放置、重分配、变换、组合、格式与大文档 |
| 🟡 | Bracket | 三种成对括号、可见侧属性、层级、历史、持久化 | 标签、repeat Link、分子包含、控制柄、锁定/组合、格式与大文档 |
| 🟡 | Table | 插入、2×2→3×3、对齐、边框、历史、持久化 | 全部增删位置、内容、清空/适应、全部对齐/边组合/颜色、锁定/剪贴板/格式/大文档 |
| 🟡 | Orbital | 七种模板、双向几何迁移、批量模板/样式/相位、历史、持久化 | 全颜色、全部 style×phase、原子/标签附着、变换、组合、格式与大文档 |
| 🟡 | TLC/Gel Chromatography | 两种板、12 泳道、批量颜色、斑点/条带移动、历史、精确文件校验 | 泳道/标记增删、多标记、TLC 开关全值、凝胶标签/宽高/可见性/范围/单位、格式与大文档 |
| 🟡 | Rings、Chain、Template Library | 六种平面环、双 Chair、Benzene、四键可变长度 Chain、Chain 端点附着续画、六元环 Bond 融合、端点附着及环顶点续画已取得真实输入、精确拓扑与持久化证据 | 其他长度/相位、每种公开库模板、属性、历史、其他格式、搜索/库切换 |
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

## 4. 已登记的 42 个场景

所有 42 个场景均已实现并进入 registry。当前不可变候选已取得前 41 个场景的完整 qualification；随后按影响图中同一化学能力的独立 Lone pair 分支新增 Lone pair→Nitrogen 附着 production 场景，因此当前 registry 为 41/42，新增场景待独立后台证据。这一 registry 闭包即使最终全绿，也不表示对应功能族或本文列出的完整 GUI 矩阵已经覆盖。

| 当前候选 | 场景 | 验证内容 |
|---|---|---|
| ✅ | `core.bond.draw-single` | 浏览器公开输入绘制单键基线 |
| ✅ | `core.bond.draw-single.production` | production 真实 OS 输入绘制单键 |
| ✅ | `core.bond.ten-variant-persistence.production` | 十种非单键工具的真实 OS 输入、精确 CCJS 阶数/线型/粗细/楔键语义 |
| ✅ | `core.ring.six-planar-persistence.production` | 六种公开平面环的真实 OS 点击、累计拓扑、原生保存与精确节点/键/分子计数 |
| ✅ | `core.ring.chair-benzene-persistence.production` | 双 Chair 与 Benzene 的真实 OS 点击、精确分量计数及交替芳香键级持久化 |
| ✅ | `core.ring.bond-fusion-persistence.production` | 先真实绘制单键，再于键中点插入六元环，要求共享键去重并持久化为单一六节点环；首次 test-locator 失败证据保留，修复后的精确闭包已通过 |
| ✅ | `core.ring.endpoint-attachment-persistence.production` | 在已绘制单键的精确端点插入六元环，要求共享一个节点并持久化为 7 节点、7 键、单分子；独立后台实机证据与文件 oracle 已通过 |
| ✅ | `core.ring.vertex-bond-continuation-persistence.production` | 从已附着六元环的精确外侧顶点继续拖出单键，要求共享环顶点并持久化为 8 节点、8 键、单分子；独立后台实机证据与文件 oracle 已通过 |
| ✅ | `core.atom.element-label-persistence.production` | 在 GUI 绘制的单键端点通过公开周期表选择 Nitrogen，要求渲染唯一原子标签并精确持久化元素、原子序数、中性电荷、默认价态 `NH2` 显示/源标签与拓扑；错误 rail scope 与错误裸 `N` oracle 证据均已保留，修复后的独立后台实机证据已通过 |
| ✅ | `core.atom.charge-symbol-attachment-persistence.production` | 从真实单键端点与 Nitrogen Element 状态继续，以公开 Charge/Electron Symbol 工具附着默认正电荷；精确持久化 +1 formal charge、三个隐式氢、`NH3` 标签、symbol chemistry delta、目标 atom ID 与 auto-link 来源的修复批次已通过 |
| ✅ | `core.atom.negative-charge-symbol-attachment-persistence.production` | 从真实单键端点与 Oxygen Element 状态继续，在 Secondary toolbar 选择 Circle minus 并附着；精确持久化 -1 formal charge、零隐式氢、`O` 标签、symbol chemistry delta、目标 atom ID 与 auto-link 来源的独立后台批次已通过 |
| ✅ | `core.atom.radical-cation-symbol-attachment-persistence.production` | 从真实单键端点与 Nitrogen Element 状态继续，在 Secondary toolbar 选择 Radical cation 并附着；精确持久化 +1 formal charge、两个隐式氢、`NH2` 标签、radical count 1、双 chemistry delta、目标 atom ID 与 auto-link 来源的独立后台批次已通过 |
| ✅ | `core.atom.radical-anion-symbol-attachment-persistence.production` | 从真实单键端点与 Nitrogen Element 状态继续，在 Secondary toolbar 选择 Radical anion 并附着；精确持久化 -1 formal charge、零隐式氢、`N` 标签、radical count 1、双 chemistry delta、目标 atom ID 与 auto-link 来源的独立后台批次已通过 |
| ✅ | `core.atom.electron-symbol-attachment-persistence.production` | 从真实单键端点与 Nitrogen Element 状态继续，在 Secondary toolbar 选择 Electron 并附着；精确持久化中性 formal charge、一个隐式氢、`NH` 标签、radical count 1、radical chemistry delta、目标 atom ID 与 auto-link 来源的独立后台批次已通过 |
| 🟡 | `core.atom.lone-pair-symbol-attachment-persistence.production` | 从真实单键端点与 Nitrogen Element 状态继续，在 Secondary toolbar 选择 Lone pair 并附着；首批产品结果已正确保持中性 formal charge、两个隐式氢、`NH2` 标签、零 chemistry delta、目标 atom ID 与 auto-link 来源，但 oracle 错把 canonical 省略的零 radical count 当成 `null` 失败；首次证据保留，当前待修复闭包证据 |
| ✅ | `core.chain.drag-count-persistence.production` | Chain 工具的真实 OS 可变长度拖拽、四键之字形提交、原生保存与精确连通拓扑 |
| ✅ | `core.chain.endpoint-attachment-continuation.production` | 从已存在端点继续真实 OS Chain 拖拽，要求八键仍为单一九节点分子并精确持久化 |
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

当前候选的 production 物理批次 `impact-11c5030-production-1786444842194` 已无人值守连续通过 26/26；26 份报告、26 个 manifest 和 204 个证据对象（209,052,037 bytes）已独立复算一致，完成审计 SHA-256 为 `f9ff74b12ce68716bbb7cfcc7df8126286db7762ade36113bd75aa4e6c0f81a2`。浏览器批次 `impact-af6ed1f-browser-1786445716424` 也已通过，7 个证据对象（9,841,659 bytes）及状态、heartbeat、checkpoint、提交、候选、profile、queue 和全部 SHA-256 均复算一致；完成审计 SHA-256 为 `20b42ace3898e03cd8f9bd80347326d2f7042f88bc753ce78b6efab1ca1e20c4`。

上述 27 份报告和 27 个 manifest 已合并为不可变 qualification `9810802f-541d-4a5f-9871-5fca59b2676c`：27/27 passed、0 failed、0 missing、0 diagnostics，211 个证据对象共 218,893,696 bytes 全部复算通过；qualification SHA-256 为 `ea597b18a6a2c219019edc73b441ebdd6527fa89e2d709cd5254eb70fd9a0742`。十键 Bond 批次 `impact-f56a237-bonds-production-1786446644922` 随后通过：报告 SHA-256 `df0ffd6f5fd846790c02b3b6c6da9e1097127d6df65678d70cbfed87e80288c9`，manifest SHA-256 `6b7abbed9a945eabbb3edc15d7ec7e07bcdbd37f2547b0ddc062e2088e6ed2b7`，9 个证据对象共 8,165,367 bytes 全部复算一致。六种平面环批次 `impact-0ef5551-rings-production-1786447434009` 也已通过：报告 SHA-256 `28e71f948efe090e1a85a92d224a940df2040d6e00edede0c58002e5fda854d4`，manifest SHA-256 `639cb10c6b625c9253afe01b57fa35cdceac5a415624aa7f47895881107dac78`，9 个证据对象共 7,785,406 bytes 全部复算一致。Chair/Benzene 批次 `impact-c80b1d1-chair-benzene-production-1786447798935` 随后通过：报告 SHA-256 `c1f009686b020e337e9a8d1b199b6e28a2a255cc3156efd858eeefb87a8e651b`，manifest SHA-256 `2ae0c2c70800b98c55b92709748f2ab0bbd90572d0fa4231f17c4102ac126946`，9 个证据对象共 7,556,403 bytes 全部复算一致。30 份报告现已合并为不可变 qualification `d2ddab7c-a5d6-4e28-81b5-0f7cd0dacb0d`：30/30 passed、0 failed、0 missing、0 diagnostics，238 个证据对象共 242,400,872 bytes；qualification SHA-256 为 `b10599dc2f5f34e581ad2b523f65a92c29fef6459e6f4607aec45ec02cb71b8a`。四键 Chain 批次 `impact-5b70bd9-chain-production-1786448480563` 随后通过：报告 SHA-256 `9f65fad1309b3012234aae7c687d8ba53d2c77d5e40de1f734730f1372b581d6`，manifest SHA-256 `8f11c5290a58cd4129f6cdc0627e6948f1f50424a97a54099043331c4c175a5c`；31 份报告已合并为不可变 qualification `0415d7ab-27fd-4273-bb93-af18854632f2`：31/31 passed、247 个证据对象、249,810,670 bytes、0 diagnostics，SHA-256 `876f83be8939e26f666df6913792f0f170370fa2e06de9dc2396cb452f7a97d5`。Chain 端点续画批次 `impact-5b232e8-chain-continuation-production-1786448909072` 随后通过：报告 SHA-256 `018296ba35cff8178d6444a5709302cc69640241b8cfe99a8f009c3231d4a787`，manifest SHA-256 `5391274a0a2c746325bda0e9a61e076d3e34cc70f7b69512feb89d1d87cbe704`，9 个证据对象共 7,431,366 bytes 全部复算一致；32 份报告现已合并为不可变 qualification `7ff5e238-eff4-4ad7-b418-e885fb2307a2`：32/32 passed、0 failed、0 missing、0 diagnostics，256 个证据对象共 257,242,036 bytes，qualification SHA-256 为 `baee0dcdb9177af92b4fd9fa73f3f5a751bf78d8c162434b101566faf2e8e041`。六元环 Bond 融合首次批次 `impact-a5cc10b-ring-fusion-production-1786449294018` 在 `choose-ring-6` fail closed：主复合按钮与已选二级按钮同时暴露 “6-membered ring”，无 scope 的 role locator 命中 2 个控件；失败报告 SHA-256 `a6440edb38ad4f2273c31d25ec7995071bda945dbeb6c7d1c87611645d32b0e9`，manifest SHA-256 `864c64cc12b278fa284fe348613d7a32e33b577a13a2b6404802f6675c171cf8`，失败证据永久保留。通用规则要求所有二级选项 role locator 限定 `Secondary toolbar`；修复后的 8 场景闭包 `impact-8a2c556-secondary-toolbar-production-1786449628624` 已通过：200/200 actions、29/29 oracles、72 个证据对象共 64,958,926 bytes 全部复算一致。33 份当前报告已重建为不可变 qualification `1a8f1d0b-90c4-42e7-8dfd-427950924b7d`：33/33 passed、0 failed、0 missing、0 diagnostics，265 个证据对象共 264,717,934 bytes，qualification SHA-256 为 `1acc344951a2b8a3a7be0b17095183c3d48aac398043f6cc1cd8d8d260c60bda`。

六元环端点附着批次 `impact-b2f18ab-ring-endpoint-production-1786450363071` 已通过：10/10 actions、3/3 oracles、0 diagnostics；报告 SHA-256 `7766b621ef38e7f8d1566b2c584c6d4f2c3d16eee57157f1112aba963680bf1d`，manifest SHA-256 `52d06ec3e045c4d5a20f9e907423aa358c6485d3410b9d04d2c56461a0468287`，9 个证据对象共 7,472,199 bytes 全部复算一致。34 份当前报告已合并为不可变 qualification `99fce458-faf1-4a38-a0b4-9336ae9c61f8`：34/34 passed、0 failed、0 missing、0 diagnostics，274 个证据对象共 272,190,133 bytes，qualification SHA-256 为 `e218f96a8de621eafd7018c6a936ef39bb93d07ac1fa0ddfba588f5b9f07e5e9`。

环顶点单键续画批次 `impact-587995c-ring-vertex-continuation-production-1786450892866` 已通过：12/12 actions、4/4 oracles、0 diagnostics；报告 SHA-256 `e98d5d9085b8f2a883ee8097d104bc81f253df7f826fbca0f935c896cfc5aec3`，manifest SHA-256 `077ac9d56c408a095bc73a317a47ef376305372d6e16f5a5179a3209b05e93fc`，9 个证据对象共 7,539,901 bytes 全部复算一致。35 份当前报告已合并为不可变 qualification `ce0eedc8-83e4-4cd2-b25c-5a4bd1629584`：35/35 passed、0 failed、0 missing、0 diagnostics，283 个证据对象共 279,730,034 bytes，qualification SHA-256 为 `adb2eed00f75251590b5328a78a28a0abdf0285ac597a17a5a28abf68e45fe82`。

Element/原子标签首次批次 `impact-a49f068-atom-element-label-production-1786451437924` 在 `open-element-palette` fail closed：场景错误地在 `Main Drawing Rail` 内寻找 `Element` role button，而真实公开控件是 rail 外唯一的 `data-quick-palette-mode="element"` 模式按钮；产品已正确绘制前置单键，故归类为 test-locator failure。失败报告 SHA-256 `02eb9e6a73df3b4d2a90564b1a06e7bef3d7911b0b1d68fccb9b390fb3601962`，manifest SHA-256 `362b514b16819cd00ad83a6da36868614f00262707a779e4495760ca22c79a33`，7 个 failure-retention 证据对象共 7,075,986 bytes 全部复算一致。通用 coverage audit 现强制 Element palette-open 动作使用稳定 mode-toggle selector，并以错误 rail scope mutant 证明可被门禁杀死。

定位器修复后的第二批次 `impact-3e9c87f-atom-element-label-production-1786451730134` 完成 10/10 actions：公开周期表准确选择 Nitrogen、端点渲染唯一标签、原生保存与 2 节点/1 键/单分子拓扑均通过；Node oracle 随后因预期裸 `N` 而拒绝实际正确的 `N`、atomic number 7、charge 0、`NH2` display/source label，故归类为 oracle-specification failure，不是产品失败。失败报告 SHA-256 `043b082a67834e97956b245a2017e4990cea0c0c06bb1344654870d30a13c5fb`，manifest SHA-256 `18796adf97d3e0d78634eb902c9435f49f7a9dfe43210dae3bada83a65c01277`，9 个 failure-retention 证据对象共 7,537,106 bytes 全部复算一致。Node oracle 回归现以 `NH2` 为真实端点价态语义并杀死裸 `N`、`NH`、错误 source label、元素、原子序数及电荷 mutants。

Element/原子标签第三批次 `impact-90caebb-atom-element-label-production-1786451997211` 已通过：10/10 actions、4/4 oracles、0 diagnostics；报告 SHA-256 `c9ff3edd0a0864f1cd2726f7c931afa84469d1fdcec8d306e9c58da407ebaae9`，manifest SHA-256 `32c37e5810ca4d8d7b684efe982fa65129ffd1053eb4379dfd39f7fb6784fb92`，9 个证据对象共 7,521,315 bytes 全部复算一致。36 份当前报告已合并为不可变 qualification `9a7268a6-4054-4773-a09a-e26310046a65`：36/36 passed、0 failed、0 missing、0 diagnostics，292 个证据对象共 287,251,349 bytes，qualification SHA-256 为 `e9ddf68e427122b879e593107a97ae186e9288ac4ab547770bf5ef61076be034`。

正电荷 symbol→atom 首批 `impact-66be831-atom-charge-attachment-production-1786452833181` 完成 12/12 actions：默认 circle-plus 真实附着后，保存文件精确得到 2 节点、1 键、单分子、2 对象，Nitrogen 为 charge `+1`、`numHydrogens=3`、display/source label `NH3`，无产品 diagnostics。失败仅来自 Symbol oracle specification：对象使用引擎全局共享序号 `obj_symbol_4` 而非错误预期的 per-kind `obj_symbol_1`，且未编辑颜色的默认 symbol 只在 payload 保存黑色，不应强制存在独立 `styleRef`。失败报告 SHA-256 `5e291a290963f1707d14e7b215e7332f3a8f0366aa58e017e4730b0f9c9d9fa4`，manifest SHA-256 `f184fa6580ca09616f546cb063ddc71c9992f4fe7f76c24cbe476d590b317aac`，9 个 failure-retention 证据对象共 7,659,629 bytes 全部复算一致。通用 schema 现把 style surface 改为按需断言，并以错误对象 ID、错误 atom ID、零 charge delta、错误 link provenance 与陈旧氢数 mutants 证明门禁可杀死这一类错误。

正电荷修复批次 `impact-32f305c-atom-charge-attachment-production-1786453236300` 已通过：12/12 actions、6/6 oracles、0 diagnostics；报告 SHA-256 `a4c2a2f9057c851ad0af8a4ee6abb7d4800a621d3b0d0b29ad7ee6e63715dbf3`，manifest SHA-256 `a013a466a35a6e7d5642624ab6379273eb4f449c85374eb0c97b98299fc2ce86`，9 个证据对象共 7,639,053 bytes 全部复算一致。37 份当前报告已合并为不可变 qualification `67f74340-b0fc-4ec9-bfbb-908154d33495`：37/37 passed、0 failed、0 missing、0 diagnostics，301 个证据对象共 294,890,402 bytes，qualification SHA-256 为 `b0b3d2ff58a4a6b910d055b4564664d080be4be4b21ca5d8a039ed117e215589`。

负电荷 Oxygen 附着批次 `impact-006fb25-atom-negative-charge-attachment-production-1786453793228` 已通过：13/13 actions、6/6 oracles、0 diagnostics；报告 SHA-256 `07e55270be7fe8fc9ae80a766378b6cad852959a11f826796e663b6ceb8182a2`，manifest SHA-256 `7d833dce2c2f435b5065eda0c2aa1315ef91235a18d4fa439987b511d7c5859b`，9 个证据对象共 7,685,811 bytes 全部复算一致。38 份当前报告已合并为不可变 qualification `3e92c440-10c7-436f-a179-c873707b4688`：38/38 passed、0 failed、0 missing、0 diagnostics，310 个证据对象共 302,576,213 bytes，qualification SHA-256 为 `8778a07612ae00d7acad7a0535d01e42353ed8c9f25047b179fff9dc57531248`。

Radical cation→Nitrogen 附着批次 `impact-9c2853b-atom-radical-cation-attachment-production-1786454222982` 已通过：13/13 actions、6/6 oracles、0 diagnostics；报告 SHA-256 `dac65c0ef6117b76d20add98cc737e5f76de4b41ab5bff91037827df43cfd88e`，manifest SHA-256 `0e6db2869032bdffc41eec0d95b3150aa6b65dc4b34e23f19a7db8222226e4ce`，9 个证据对象共 7,673,831 bytes 全部复算一致。39 份当前报告已合并为不可变 qualification `730eb02d-3a1a-4c46-8df0-a9586213babe`：39/39 passed、0 failed、0 missing、0 diagnostics，319 个证据对象共 310,250,044 bytes，qualification SHA-256 为 `3ec6914de3c7aaaad2ab66500f24cdd61c095551bf6f12134599e6f541d6df96`。

Radical anion→Nitrogen 附着批次 `impact-0f32f0d-atom-radical-anion-attachment-production-1786454854879` 已通过：13/13 actions、6/6 oracles、0 diagnostics；报告 SHA-256 `f9bfcce145567f4907f1cc35f380f6f99b18be049c0917ae3da6479835c2f96c`，manifest SHA-256 `92b8ff090a12fd53a81c1d4449af2ae59cb92383a8c946d092013a5de96b0c70`，9 个证据对象共 7,667,810 bytes 全部复算一致。40 份当前报告已合并为不可变 qualification `e38ef34a-dab9-4477-9c72-a4b0c99d76dc`：40/40 passed、0 failed、0 missing、0 diagnostics，328 个证据对象共 317,917,854 bytes，qualification SHA-256 为 `3ee3c68e1af45eb9de527b469f1067b9ce0ef17d257471918a61796cde946a0a`。

Electron→Nitrogen 附着批次 `impact-df5db12-atom-electron-attachment-production-1786455340323` 已通过：13/13 actions、6/6 oracles、0 diagnostics；报告 SHA-256 `8168f20d2040d4af72547cacb8c243f3c1b50f711e4577c0d2dacc8b9072c0f4`，manifest SHA-256 `28b3537040ce4092a7cb1c91493369beba638de6e73337dac75cd81aacadf340`，9 个证据对象共 7,662,891 bytes 全部复算一致。41 份当前报告已合并为不可变 qualification `c396b930-d682-4a97-982a-081069da0dc1`：41/41 passed、0 failed、0 missing、0 diagnostics，337 个证据对象共 325,580,745 bytes，qualification SHA-256 为 `b184d1b6d4dd048e2ad4b8e443db4ccc4320b06eac8a981db19c8366054286aa`。

Lone pair→Nitrogen 首批 `impact-0b349a2-atom-lone-pair-attachment-production-1786455781632` 完成 13/13 actions，拓扑、DOM 与保存均通过，产品精确持久化 `N/0/H2/NH2`、Lone pair `chargeDelta=0`、`radicalDelta=0`、`attachedAtomId=n_2` 与 auto-link；ChemSema CLI 独立 detail 也确认相同 node、symbol 及 atom-symbol Link。失败仅来自 Node oracle specification：canonical 零 radical count 不写 `meta.radicalCount`，检查器却把省略值报告为 `null` 并拒绝预期 `0`。失败报告 SHA-256 `ee27b026f0c788db522828f79f9dcb7fa0bcf939a203f8de48c951aa936a4ee4`，manifest SHA-256 `1e42396f0904db1d3251670fac31054d6882edf12c6fe97bbc29af4e16073f60`，9 个 failure-retention 证据对象共 7,674,637 bytes 全部复算一致。通用 oracle 现只把字段缺失归一为语义零，同时以显式 `null`、错误非零和缺失 node mutants 证明仍会 fail closed。

当前候选的两个前端 production 场景均通过。真实鼠标/键盘观测为 1280×900 CSS viewport、DPR 1.5；键盘焦点环、hover、disabled cursor/opacity 均通过。真实绘制、字体切换和全选后，文本选择框同时满足字形包含与字体度量紧边界，单键选择框为 40×12 CSS px，两个独立选择框共有 16 个 6×6 CSS px resize handle；上下文菜单提交后画布重新取得 focused、focus-within 和 hover。选择几何与前端状态报告 SHA-256 分别为 `0a06c635c68851063938202f7e961219206d0aba0d22b643a6cf7b6591a00b15`、`aaf54a0ffa03f51318d13736e7247c0fae753b580ca3e1068fda108d945ef72b`。

## 4.1 物理工作节点第一阶段记录

- 正式仓库由 GitHub 全新克隆，最低可信基线 `dc9d8a78b1f7ebfcc42b7077ec49f842650fef20` 已验证；退役项目仓库按日期完整归档，用户化学文档未删除。
- 全新依赖基线：`npm ci` 0 漏洞、GUI 平台初始 72/72；物理节点、守护进程和前端 oracle 测试持续增加，当前新增 Bond 语义 mutation 回归，audit 28 场景/0 gap/0 warning；每次提交仍需 `CI=true npm run verify`。
- 本机 profile 位于 `%LOCALAPPDATA%\\ChemSema\\gui-test\\profiles\\physical-current.json`；机器名、账户、MachineGuid 哈希和证据均不提交 Git。
- 物理 adapter 与 Hyper-V adapter 并存；Hyper-V 仍强制专用 guest 账户，物理 adapter 精确绑定本机当前账户和 session 1，不配置 autologon。
- 扩展前 registry 的 41 场景已有单候选完整资格；当前 42 场景 registry 保留六元环 Bond 融合、Element 及正电荷附着的首次 test/oracle 失败，正/负电荷、Radical cation/Radical anion 及 Electron 附着已通过，新增 Lone pair→Nitrogen 附着场景待证据。这不关闭尚未登记的功能、属性、格式、规模、环境和稳定性缺口。
- 第一阶段尚未完成：正式 NSIS 安装/文件关联验证、长期 supervisor/子进程重启续跑与终态事件触发验收、PR CI 收口。

## 5. 下一阶段执行顺序

执行顺序是有限的，不再按“想到一个测一个”推进：

1. 🟡 **当前缺陷族与 oracle 收口**：27/27 qualification 已完成；继续补齐轨道模板迁移、轨道/括号归一化前语义检查点，以及 supervisor/子进程重启故障注入。
2. 🟡 **化学绘制主干**：十种非单键工具、六种平面环、双 Chair、Benzene、Chain/环连接、Element/原子标签、正/负电荷、Radical cation/Radical anion 及 Electron symbol→atom 精确语义批次已完成；当前只跑 Lone pair→Nitrogen 附着，再推进 uncircled plus/minus、氢值域、Template Library、反应连接与属性。
3. **补齐已开工对象族值域**：Arrow、Text、Shape、Symbol、Bracket、Table、Orbital、Chromatography 的公开值和 `0/1/2/many`。
4. **Biology 与其他专用对象**：24 个 biology kind、plasmid、Image/Spectrum/Geometry/Constraint/Annotation/Stoichiometry。
5. **文档与外部边界**：多标签、所有格式、恢复、系统剪贴板、Office、文件关联。
6. **Complex/Large/Xlarge**：先从空白构建，再验证打开后继续编辑；建立硬性能和资源阈值。
7. **环境、故障、模型与 mutation**：补齐非正常路径和主动杀错能力。
8. **功能矩阵闭合后再做长稳**：复杂/large/xlarge 与功能矩阵闭合后，依次完成 24 小时混合 soak、最终安装包和连续 1,000 次正式展示；不得用重复稳定性运行挤占功能覆盖。

## 6. 强制更新规则

本清单不是一次性说明：

- 每新增、删除或重命名一个 registry 场景，必须同步修改“登记场景”数字和第 4 节表格；自动测试会逐个检查 42 个场景 ID，漏项直接失败。
- 每完成一个对象族或发现新的公开缺口，必须同时更新第 3 节状态和“明确剩余”，不能只在长架构文档末尾追加段落。
- 每产生新候选或 qualification，必须更新页首候选哈希、通过/缺失/失败数和最新证据。
- 每个本地测试提交必须让本文反映该提交后的真实状态；不得把“场景通过”写成“功能族完成”。
- 失败只有在根因修复并产生新候选后才能从“当前阻断”移走；原失败证据仍永久保留。
