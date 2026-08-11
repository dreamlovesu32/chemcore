# ChemSema

**中文** | [English](./README.md)

[![CI](https://github.com/dreamlovesu32/chemsema/actions/workflows/ci.yml/badge.svg)](https://github.com/dreamlovesu32/chemsema/actions/workflows/ci.yml)
[![在线 Demo](https://img.shields.io/badge/demo-GitHub%20Pages-2ea44f)](https://dreamlovesu32.github.io/chemsema/)
[![Windows 安装包](https://img.shields.io/badge/Windows-installer-0078d4)](https://github.com/dreamlovesu32/chemsema/releases/download/chemsema-v1.0.0-beta.1/ChemSema_1.0.0-beta.1_x64-setup.exe)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](./LICENSE)
[![Version](https://img.shields.io/badge/version-1.0.0--beta.1-orange)](https://github.com/dreamlovesu32/chemsema/releases/tag/chemsema-v1.0.0-beta.1)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.21443042.svg)](https://doi.org/10.5281/zenodo.21443042)

ChemSema 的原公开名称为 **ChemCore**。本次改名保留完整 Git 历史，当前代码、包、
文档和公开路径统一使用新名称。详见[项目改名与兼容说明](./docs/migration-to-chemsema.zh-CN.md)。

ChemSema 是一个面向可编辑化学文档的开源平台。它从真实科研绘图工作流出发：
绘制结构、排布反应式、导入导出 ChemDraw 文件，把可继续编辑的化学内容带入
Word 和 PowerPoint，并在需要时还能回到原文档继续修改。

浏览器编辑器、Windows 桌面端、Office/OLE 集成服务和 headless CLI 共享同一套
Rust engine。这套内核负责文档对象身份、化学编辑命令、命中测试、标签语义、
CDXML/CDX 导入导出、渲染 primitive、结构化 diff 和可编辑导出。ChemSema 关注的
不只是“把化学画出来”，而是保留足够完整的文档状态，让人、脚本和 agent 都能在
同一批对象上工作。

对 AI agent 来说，ChemSema 直接暴露这套文档模型：同一个 selector 可以贯穿
结构化检查、局部视觉观察、受范围约束的编辑、来源追踪、结果验证和可编辑导出，
中途不丢失对象身份。

```text
CDXML / CCJS
      |
targets -> object:obj_mol_001
      |-- detail.json
      |-- context.json
      |-- capture.png
      |-- bundle/
      |     |-- editable-subset.ccjs
      |     |-- identity-map.json
      |     `-- provenance.json
      |-- transaction
      |-- diff.json
      `-- target.cdxml
```

对科研工作者来说，ChemSema 是一个用于结构、反应式、论文图和 Office 文档的
可视化编辑器。对 agent 来说，ChemSema 是一层 object-grounded operation layer：
agent 可以定位单个对象，只读取必要数据，查看与它对齐的局部图像，执行范围受限
的修改，并证明文档其他部分没有被误改。

Windows 用户可以直接下载当前 beta 的 [ChemSema 1.0.0-beta.1 x64 安装包](https://github.com/dreamlovesu32/chemsema/releases/download/chemsema-v1.0.0-beta.1/ChemSema_1.0.0-beta.1_x64-setup.exe)。这个安装包包含桌面端和 Windows Office/OLE 集成服务；目前还没有代码签名，beta 阶段 Windows 可能会弹出 SmartScreen 提醒。作者：Jiajun ZHANG，邮箱：[dreamlovesu@hotmail.com](mailto:dreamlovesu@hotmail.com)，欢迎试用、反馈、提交 issue 或参与贡献。项目希望 ChemSema 能成为一个完全免费的科研基础设施平台；未来还可以继续接入自动化、批量处理、AI 辅助科研接口，以及更多用心打磨的科研软件。ChemSema 只是第一步。

![ChemSema 编辑器界面](./docs/assets/readme/product-screenshot.png)

## Object-Grounded Agent CLI

ChemSema CLI 是同一套编辑器内核之上的协议接口。它面向的是需要处理真实化学图
的 agent：不要求 agent 读取整份文档，不要求它从截图里猜对象，也不把导出的图片
当成事实来源。

核心工作单元是 selector，例如 `object:obj_mol_004` 或
`node:1176604361`。同一个 selector 可以稳定贯穿：

- 用 `targets` 发现对象
- 用 `detail` 查看原始结构
- 用 `context` 查看周边排版关系
- 用 `capture` 生成确定性的局部视觉图
- 用 `bundle` 打包对象级工作单元
- 用 `run` 执行受范围约束的命令
- 用 `diff` 审计 before/after
- 用 `export --target` 导出可编辑子文档

`bundle` 是 agent 接手一个对象时的交接点。它会写出 `target.json`、
`context.json`、`capture.png` 或 `capture.svg`、只包含目标范围的可编辑子文档、
`identity-map.json`、`provenance.json` 和 `manifest.json`。manifest 会明确区分
editable scope 与 visual scope：附近箭头、条件文本或相邻分子可以出现在截图和
context 里，但除非它们属于声明的目标范围，否则不能被修改。

transaction 在真正修改前增加一层安全声明。agent 可以说明自己期望的 document
hash 或 revision、允许编辑哪些 selector、是否允许创建或删除、是否只是 dry-run、
以及必须满足哪些 postconditions。结构化 `diff` 再按 ChemSema id 和字段路径比较
文档，让结果可以被 agent、测试或人工 reviewer 审计。

```bash
chemsema-cli targets figure1.cdxml --pretty
chemsema-cli bundle figure1.cdxml --target object:obj_mol_004 --out-dir tmp/mol-bundle --context-radius 55 --capture-format png --subset-format ccjs --pretty
chemsema-cli run figure1.cdxml transaction.json --out edited.ccjs --results report.json --pretty
chemsema-cli diff before.ccjs edited.ccjs --out diff.json --pretty
chemsema-cli export edited.ccjs target.cdxml --target object:obj_mol_004 --format cdxml
```

仓库中的 [object-grounded edit 示例](./examples/agent/07-object-grounded-edit/)
会在公开 fixture [figure1.cdxml](./figure1.cdxml) 上完整跑通这条链路：选中一个
真实分子对象，修改其中一个带标签的 node，验证没有意外 selector 被修改，并同时
导出修改后的完整文档和目标分子子文档。

精确截图只是这套接口里最容易看见的一部分。它会裁出和 GUI 选择框一致的视觉范围：
目标 object 或多选目标负责定义截图框，而这个框里实际可见的内容都会进入 PNG/SVG。
context 查询使用同一套 target 模型，并返回周边对象的 id、方向、距离、
`inside`/`partial` 选择框关系、group 层级和 link 信息。

下面的图片由公开示例 [figure1.cdxml](./figure1.cdxml) 直接生成：

| 单个对象精确截图 | 箭头对象的周边截图 | 多目标选择框截图 |
| --- | --- | --- |
| ![CLI 对 object obj_bracket_001 的精确截图](./docs/assets/readme/agent-cli/precise-bracket-object.png) | ![CLI 对箭头 object obj_line_001 周边的 context 截图](./docs/assets/readme/agent-cli/line-context.png) | ![CLI 对 bracket object 和附近文本目标的多选截图](./docs/assets/readme/agent-cli/multi-target-bracket-text.png) |

生成中间图片的 context 命令同时会返回结构化 id。比如在
`object:obj_line_001` 周围，它会报告目标箭头本身、部分重叠的分子
`object:obj_cdxml_merged_molecule`、部分重叠的条件文本
`object:obj_text_008`，以及下方附近的文本 `object:obj_text_025`。

## 项目历史

ChemSema 项目始于 2026 年 4 月 23 日。早期开发历史已公开保留在
[`history/pre-public-release`](https://github.com/dreamlovesu32/chemsema/tree/history/pre-public-release)
分支，方便读者了解项目在公开发布前的演进过程。

## 真实论文图谱对比

下面的两组 CDXML 文件来自开发者已公开发表的论文：

Jiajun ZHANG, Pinhong Chen,* Guosheng Liu*, Copper-Catalyzed Site- and Enantioselective C–H Cyanation of Trisubstituted Allenes, [Chin. J. Chem. 2026, 44, 1729–1734](https://onlinelibrary.wiley.com/doi/full/10.1002/cjoc.70531).

这不是专门为演示准备的简单样例，而是真实论文中的复杂反应图谱。它包含多种文本、结构、反应箭头、括号、颜色、自由基/单电子点、图形对象和排版细节。下图左侧是 ChemDraw 导出的 SVG，右侧是 ChemSema 从同一份 CDXML 导入后导出的 SVG。

这些 benchmark CDXML 文件由维护者本人绘制，并随仓库一起提供，用于可复现的渲染对比。

![ChemDraw 与 ChemSema 真实论文 CDXML 渲染对比](./docs/assets/readme/comparison/published-cdxml-comparison.svg)

原始 CDXML 文件也已放在仓库根目录：[figure1.cdxml](./figure1.cdxml) 和 [figure2.cdxml](./figure2.cdxml)。对应的 SVG 与 Office EMF 矢量产物保留在 [docs/assets/readme/comparison](./docs/assets/readme/comparison/) 中，包含 ChemDraw 和 ChemSema 各自导出的版本；README 中的对比图也由这些刷新后的资产重新生成。对 ChemSema 来说，这类真实文件的兼容性和 Office 级矢量导出能力，是项目最重要的宣传点之一。

## Agent Skills

ChemSema 在 [ChemSemaSkills](./ChemSemaSkills/) 中单独维护了一组 agent
skills。这些 skill 把项目专用的工作流打包起来，覆盖 CLI 协议、command script、
drawing-agent 规划、Office/OLE 调试和仓库开发。这套目录可以平铺安装为
Codex skills，也可以平铺安装为 Claude Code skills；总 README 只保留公开入口，
具体安装和使用细节放在独立 skill 套件里维护。

## 当前状态

当前版本：`1.0.0-beta.1`。

Windows x64 安装包已经放在 [chemsema-v1.0.0-beta.1 release](https://github.com/dreamlovesu32/chemsema/releases/tag/chemsema-v1.0.0-beta.1) 中，包含 Tauri/WebView2 桌面端、文件关联和 Office/OLE 集成服务。第一阶段鸿蒙 PC 壳已经在源码中提供，适合用 DevEco Studio 做实验，但不包含在 Windows 安装包里。当前安装包仍是 beta 版本，尚未代码签名。浏览器 demo 通过 GitHub Pages 发布：<https://dreamlovesu32.github.io/chemsema/>。

## 产品特色

- **面向真实科研绘图工作流**：ChemSema 不是只展示分子图的演示型编辑器，而是围绕“画结构、排反应式、复制到 Word/PowerPoint、再回来编辑”这条实际链路设计。
- **尽可能兼容 ChemDraw 文件与排版习惯**：项目把 CDXML/CDX 导入导出作为一等能力处理，目标是在结构、文本、箭头、括号、符号、颜色和对象位置上尽量保留源文件表现。
- **浏览器端、桌面端、Office 集成共享同一套内核**：编辑规则、命中测试、化学标签、渲染 primitive、导入导出都集中在 Rust engine 中，避免不同端出现行为分叉。
- **低延迟编辑体验**：鼠标 hover、focus、选择、拖拽预览、旋转和缩放等高频交互走本地 WASM/Rust 热路径，不把每一次鼠标移动都变成跨进程请求。
- **现代桌面软件体验**：桌面端基于 Tauri/WebView2 构建，支持文件打开保存、拖拽打开、最近文件、标签页、未保存提醒、快捷键和 Windows 文件关联等基础体验。
- **为 Office 粘贴与嵌入认真设计**：复制时不只写一张图片，而是同时考虑 ChemSema native、CDXML、SVG、EMF、RTF/OOXML 和 OLE 对象等多格式链路，让 Office 里的显示和后续编辑都尽量可靠。
- **面向 agent 的 headless CLI**：CLI 可以检查文档、转换格式、查询对象 id 和关系、生成精确 PNG/SVG 裁图、执行带审计报告的 JSON 编辑命令，并通过 cache/session 工作流复用大文件状态。

## 已实现的关键能力

- **CDXML/CDX 导入导出**：Rust engine 内置 CDXML/CDX 解析与写出路径，可把 ChemDraw 文件转换为 ChemSema 文档模型，并保留足够的源文件绘图信息用于重渲染和回写。
- **统一文档与渲染模型**：文档模型、运行时 scene、命中测试、选择状态和 render primitive 都在内核中定义；前端主要负责事件采集和显示，不重新发明化学规则。
- **可扩展原生文档**：CCJS 0.2 使用平铺实体与显式 hierarchy/relations；CCJZ 提供带哈希分块、可见区加载、未变 entry/附件复用、浏览器 Zip64，以及结构化化学/往返校验。
- **复杂键绘制几何**：已实现普通键、双键、三键、实/虚楔形键、虚线键、哈希键、label clipping、键键接触、交叉白边和 ChemDraw 风格模板参数等规则。
- **箭头与图形对象**：支持反应箭头、平衡箭头、空心箭头、弯箭头、括号、线条、图形和符号对象，并持续对齐 ChemDraw 的交互和渲染细节。
- **选择、拖拽、旋转与排列**：支持对象级和分子局部选择，支持多选拖拽预览、旋转、翻转、对齐、分布、颜色应用和可撤销命令历史。
- **文本与标签编辑**：支持普通文本、端点元素替换、标签编辑、文本选择、样式同步，以及化学标签与普通自由文本之间的行为区分。
- **隐式氢与元素规则**：主族元素的自动加氢、价键计数、电荷影响和特殊周期规则在内核中统一处理，避免前端和导出路径各算一套。
- **缩写与基团识别**：支持常见缩写和 functional group 识别，例如 Me、Et、Ph、CN、NO2、Boc、Ts、TMS、TBDMS、TBDPS 等；缩写在翻转、分子式和分子量统计中作为整体处理。
- **分子式和质量统计**：选中结构后可计算 Formula Weight、Exact Mass 等信息，并把可识别缩写的展开组成纳入统计。
- **桌面和 Office 基础链路**：桌面端、浏览器端和 Office/OLE 服务已经建立长期边界；Windows 剪贴板、EMF 预览、Word OOXML/OLE payload 等路径已有实现基础。

## 设计细节

ChemSema 的很多实现选择都围绕“用起来顺不顺手”展开，尤其重视那些只有真实绘图时才会暴露出来的细节。这些逻辑尽量放在 Rust engine 中统一处理，浏览器端、桌面端和 Office 导出路径共享同一套几何结果。

### 文本裁剪

ChemSema 不把标签简单当作一个矩形框来避让。化学标签会先被拆成 styled runs 和 line runs，再由内核根据字体大小、上下标、基线和字符 advance 生成逐字 glyph polygon。渲染键时，键端从节点出发沿键方向与这些 glyph polygon 的每条边求线段交点，取离节点最远的出界交点作为真正的键端。glyph polygon 本身已经包含光学裁剪余量，渲染器不得再额外叠加 label margin。

这样做的好处是，`NH`、`Ph`、`OTMS` 这样的标签不会因为整个文本框过宽而把键剪得太短；键也不会穿过 `N`、`O`、`H` 等大写字母的实际可见轮廓。若导入文件没有可用的 glyph polygon，内核才退回到 label bounding box 做保守裁剪。

### 大写字母与标签分组

字母轮廓来自共享的 glyph profile 和 glyph clip polygon 表。常用字符使用从 Arial 生成并人工调过的 height-centered clip polygon，例如 `N`、`I`、`+` 等都有独立多边形；`N` 的裁剪轮廓会略大于 ink box，避免键贴得太近，窄大写 `I` 的扩展按字高控制，而不是按很窄的 ink width 放大。未知的大写字母则使用保守的大写 fallback profile，未知 CJK 或全角字符使用近似方形 profile，保证裁剪不会漏掉文字。

化学标签还会按“大写字母开头的片段”和已知缩写分组，而不是逐字符处理。比如 `CF3` 会被识别为 `C` + `F3`，`OTMS` 会按 `O` + `TMS` 处理；右侧连接的标签翻转时按组反转，所以 `OTMS` 会变成 `TMSO`，不会变成 `SMTO`。锚点也会落在化学上真正连接的 terminal letter 上，避免把数字、隐式氢或缩写内部字符误当连接点。

### 键端交汇

共享端点的键不依赖 SVG 的 `stroke-linecap` 或浏览器默认连接样式，而是由内核计算真正的 polygon。每根键在端点处先转换成主轴、法向、左右轮廓线和 half width；普通键、粗键、单侧双键主线、三键主线和楔形键都会进入同一套 contact kernel。两根键相接时，内核分别计算 inner-inner 和 outer-outer 轮廓线交点，形成每根键自己的 endpoint profile；角度过尖时用 miter limit 截断，避免出现过长尖角。

三根及以上键共用一个节点时，内核先按极角排序，只处理周向相邻的轮廓 pair。相邻轮廓的延长线交点组成节点周围的一圈 profile，每根键只吃自己对应的那一段端点轮廓。这样多取代中心、三键节点、楔形键宽端和普通键混合时，连接处不会靠遮罩硬盖，也不会出现随机缺口。

### 键键交叉与白边

非共享端点的键键交叉走另一套逻辑。渲染同一个 fragment 时，内核按 bond 顺序逐根绘制，后绘制的键视为上层键。上层键绘制前，会检查它与已绘制下层键的内部线段交点；共享端点、近乎平行或重合的情况会被排除。对于真正交叉的键，内核按交叉角的 `sin` 值补偿白边长度，并用上层键的可见宽度加 `marginWidth` 生成一个沿上层键方向的 knockout polygon，让下层键在交叉处自然断开。Default 模板的 `marginWidth` 为 `2.0pt`，ACS Document 1996 为 `1.6pt`。

### 无限画布

ChemSema 的编辑区不是固定页面截图，而是一个运行时 `viewBox`。前端维护 `runtimeViewBox`、缩放比例和滚动容器：SVG 的 `viewBox` 使用文档世界坐标，CSS 宽高按 `pt -> css px -> zoom` 换算，滚动条只负责查看当前世界坐标窗口。默认空文档会以可见区域为基础，在四周预留 `0.6` 倍可见宽高的 buffer。

每次文档渲染后，前端会用内核 render primitive 计算 document bounds。如果内容接近当前 `viewBox` 边缘 `0.18` 倍可见宽高以内，就自动向对应方向扩展画布，并在左侧或上侧扩展时同步补偿 scroll delta，避免画布扩张造成视觉跳动。缩放时也会保存当前焦点区域，优先围绕选区、文档或视口中心缩放，而不是简单把滚动位置归零。

### 对象稳定性与标签页

选择框、hover、拖拽预览、旋转控制点、弯箭头控制点、文本框和图形对象都需要在大文件中保持稳定反馈。对象被选中后，内部原子、键和文本不应继续产生 hover 抖动；拖拽时前端先用 preview transform 做实时跟随，提交时再把结果落到内核，避免每个鼠标移动都跨端提交完整文档。

桌面端支持多标签文档体验。新建和拖入文件会以标签页方式组织；空白且未修改的文档可以直接被新文件替换，已有内容或有改动的文档在关闭标签页或退出软件时会进入保存确认流程。每个标签页保存自己的文档、缩放和运行时视图状态，切换回来时尽量保持用户离开前的工作位置。

架构上，ChemSema 尽量把长期会影响一致性的逻辑放在内核里：命中测试、选择范围、hover 行为、绘制几何、文本裁剪、键交汇、隐式氢、缩写识别、CDXML 解析、导出渲染都尽量由 Rust engine 统一负责。这样浏览器端、桌面端和 Office 路径才能共享同一套行为，而不是靠前端补丁临时拼出来。

项目目前仍在快速迭代中，复杂 CDXML 文件、Office 复制粘贴、OLE 嵌入对象和 ChemDraw 级别的格式保真仍会继续打磨。如果你发现任何不顺手、不兼容或“看起来差一点”的地方，非常欢迎反馈。

## 欢迎体验

如果你也长期使用 ChemDraw，或者对免费的科研基础设施、AI 辅助软件开发、化学绘图工具链感兴趣，欢迎试用 ChemSema、提交 issue、参与讨论，或者直接贡献代码。

项目尤其欢迎两类反馈：一类是具体文件和截图，帮助 ChemSema 对齐 ChemDraw 的显示与交互；另一类是实际科研写作中的复制粘贴、Office 编辑、排版和导出问题。ChemSema 的目标不是做一个看起来像编辑器的演示项目，而是做一个真正能进入日常科研工作流的工具。

欢迎直接通过 README 开头的邮箱联系作者。

## 仓库结构

```text
chemsema/
  crates/chemsema-engine/          Rust 文档、编辑、渲染、CDXML 和 WASM 内核
  crates/chemsema-cli/             Headless 文件检查、格式转换、导出和命令执行器
  crates/chemsema-desktop-service/ 桌面端原生 engine session 与文件能力
  apps/chemsema-desktop/           Tauri Windows 桌面应用
  apps/chemsema-office/            Windows Office/OLE 集成服务
  viewer/                          浏览器编辑器宿主和生成的 WASM package
  docs/                            可公开的规则、规范、架构文档和 README 资产
  ChemSemaSkills/                  ChemSema agent 与开发工作流的 Codex/Claude skills
  examples/                        ChemSema 原生文档示例
  fixtures/                        公开 synthetic CDXML 回归 fixture
  scripts/                         构建、验证和回归辅助脚本
  shared/                          Rust 和 viewer 共用 JSON 数据
```

## 环境要求

- Rust stable，Windows 桌面路径需要 MSVC toolchain
- Node.js 和 npm
- Python 3，用于本地静态服务和部分可选分析脚本
- 使用 `tools/wasm-pack.json` 声明的精确 `wasm-pack` 版本；缺失或版本不同
  时构建会失败关闭，CI 使用前还会校验锁定的 Windows 压缩包 SHA-256
- 桌面 shell 与 Office/OLE 集成需要 Windows

## 快速开始

```bash
npm install
cargo test
npm run build:engine-wasm
```

在仓库根目录启动浏览器编辑器：

```bash
python -m http.server 8765 --bind 127.0.0.1 --directory .
```

然后打开：

```text
http://127.0.0.1:8765/viewer/
```

运行 Windows 桌面端：

```bash
npm run desktop:dev
```

运行 headless 文件 CLI：

```bash
npm run cli -- inspect figure1.cdxml --pretty
npm run cli -- convert figure1.cdxml tmp/figure1.svg
npm run cli -- targets figure1.cdxml --pretty
npm run cli -- capture figure1.cdxml --target object:obj_bracket_001 --out tmp/bracket.png --width 1200 --expand 8 --pretty
npm run cli -- context figure1.cdxml --target object:obj_line_001 --radius 45 --expand-left 10 --expand-right 10 --expand-top 34 --expand-bottom 34 --capture-out tmp/line-context.png --out tmp/line-context.json --pretty
npm run cli -- new commands.json --out generated.cdxml --results results.json --pretty
npm run cli -- run input.cdxml commands.json --out edited.cdxml --results results.json --document-json after.ccjs --pretty
```

构建 release 二进制：

```bash
npm run desktop:build-fast
cargo build -p chemsema-office -p chemsema-cli --release
```

为当前用户注册 Office/OLE 集成：

```bash
npm run office:register-dev
```

取消注册：

```bash
npm run office:unregister-dev
```

## 验证

主要验证命令：

```bash
npm run verify
```

它会运行 Rust 测试、重建浏览器 engine WASM、检查 viewer JavaScript 语法，并确认 `viewer/engine` 生成物已同步。

常用定向命令：

```bash
npm test
cargo test -p chemsema-engine
cargo test -p chemsema-office
cargo test -p chemsema-engine public_cdxml_fixture_svg_golden_snapshots_match --test render_document
npm run build:engine-wasm
node --check viewer/app.js
```

公开 synthetic CDXML fixture 位于 [fixtures/cdxml](./fixtures/cdxml/)，对应 golden SVG 快照位于 [fixtures/expected/svg](./fixtures/expected/svg/)。对比与快照流程见 [渲染对比与回归资产](./docs/rendering-comparison.zh-CN.md)。

许可证清楚的公共往返语料见
[benchmarks/public-cdxml](./benchmarks/public-cdxml/README.zh-CN.md)。它固定五个上游项目中的
413 个 CDXML/CDX 文件，但不会把上游源文件直接提交进本仓库：

```bash
npm run benchmark:cdxml-public:fetch
cargo build -p chemsema-cli
npm run benchmark:cdxml-public
```

当前基线有 404 个连续三代精确往返案例；其余 9 个均明确分类为安全清洗、无损归一化、
预期拒绝或传输编码输入，未预期语义漂移和计数漂移均为 0。

部分脚本会和本机桌面应用或 Office 做输出对照。这些流程是可选的，可能需要 Windows 专有软件、本地文档，或用 `CHEMSEMA_PYTHON` 指向装有分析依赖的 Python 环境。

## 引用

如果 ChemSema 对已发表研究有实质贡献，请引用研究实际使用的软件归档版本。当前版本 DOI
为 [10.5281/zenodo.21443043](https://doi.org/10.5281/zenodo.21443043)，跨版本概念 DOI
为 [10.5281/zenodo.21443042](https://doi.org/10.5281/zenodo.21443042)。引用元数据和作者
ORCID 见 [CITING.md](./CITING.md)。GitHub 也会读取 [CITATION.cff](./CITATION.cff)，并显示
**Cite this repository** 入口。

## 设计文档

- 缩写识别规则：[English](./docs/abbreviation-recognition-rules.md) / [中文](./docs/abbreviation-recognition-rules.zh-CN.md)
- 架构总览：[English](./docs/architecture.md) / [中文](./docs/architecture.zh-CN.md)
- 键绘制规则：[English](./docs/bond-rendering-rules.md) / [中文](./docs/bond-rendering-rules.zh-CN.md)
- 电荷与自由基符号规则：[English](./docs/charge-radical-symbol-rules.md) / [中文](./docs/charge-radical-symbol-rules.zh-CN.md)
- Agent POC 工作流：[English](./docs/agent-poc-workflow.md) / [中文](./docs/agent-poc-workflow.zh-CN.md)
- ChemSema agent skills：[English](./ChemSemaSkills/README.md) / [中文](./ChemSemaSkills/README_ZH.md)
- ChemSema CLI 命令指南：[English](./docs/chemsema-cli-guide.md) / [中文](./docs/chemsema-cli-guide.zh-CN.md)
- CLI/GUI parity 清单：[docs/cli-gui-parity-checklist.md](./docs/cli-gui-parity-checklist.md)
- CLI protocol contract：[docs/protocol](./docs/protocol/README.md)
- Document Commit 合同：[English](./docs/document-commit-contract.md) / [中文](./docs/document-commit-contract.zh-CN.md)
- 编辑器命令历史：[English](./docs/editor-command-history.md) / [中文](./docs/editor-command-history.zh-CN.md)
- 当前 CCJS 格式 v0.2：[English](./docs/format-v0.2.md) / [中文](./docs/format-v0.2.zh-CN.md)
- CCJS 架构、格式比较与必要性：[中文](./docs/ccjs-architecture-and-format-rationale.zh-CN.md)
- CCJS v0.2 当前实现状态与 stable 发布门禁：[中文](./docs/ccjs-v0.2-stability-architecture.zh-CN.md)
- CCJZ 容器、Document Patch 与 Recovery Journal 合同：[docs/protocol](./docs/protocol/README.md)
- 旧版格式 v0.1：[English](./docs/format-v0.1.md) / [中文](./docs/format-v0.1.zh-CN.md)
- Glyph 裁剪规则：[English](./docs/glyph-clip-polygons.md) / [中文](./docs/glyph-clip-polygons.zh-CN.md)
- Glyph kernel：[English](./docs/glyph-kernel.md) / [中文](./docs/glyph-kernel.zh-CN.md)
- 隐式氢规则：[English](./docs/implicit-hydrogen-rules.md) / [中文](./docs/implicit-hydrogen-rules.zh-CN.md)
- 项目规则：[English](./docs/project-rules.md) / [中文](./docs/project-rules.zh-CN.md)
- 渲染对比与回归资产：[English](./docs/rendering-comparison.md) / [中文](./docs/rendering-comparison.zh-CN.md)
- Rust engine 架构：[English](./docs/rust-engine-architecture.md) / [中文](./docs/rust-engine-architecture.zh-CN.md)
- 文本符号与 glyph profile：[English](./docs/text-symbol-glyph-profile-rules.md) / [中文](./docs/text-symbol-glyph-profile-rules.zh-CN.md)
- 价键驱动标签识别：[English](./docs/valence-label-recognition-rules.md) / [中文](./docs/valence-label-recognition-rules.zh-CN.md)
- Windows 桌面端与 Office 架构：[English](./docs/windows-desktop-office-architecture.md) / [中文](./docs/windows-desktop-office-architecture.zh-CN.md)
- GUI 测试平台与展示可靠性架构：[English](./docs/gui-test-platform-and-demo-reliability.md) / [中文](./docs/gui-test-platform-and-demo-reliability.zh-CN.md)
- 发布质量矩阵：[English](./docs/release-quality.md) / [中文](./docs/release-quality.zh-CN.md)
- Release notes：[English](./CHANGELOG.md) / [中文](./CHANGELOG.zh-CN.md)
- Roadmap：[English](./ROADMAP.md) / [中文](./ROADMAP.zh-CN.md)
- [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)

## 许可证

ChemSema 使用 Apache License, Version 2.0 授权。见 [LICENSE](./LICENSE) 和 [NOTICE](./NOTICE)。
