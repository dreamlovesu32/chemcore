# ChemCore

[![CI](https://github.com/dreamlovesu32/chemcore/actions/workflows/ci.yml/badge.svg)](https://github.com/dreamlovesu32/chemcore/actions/workflows/ci.yml)
[![在线 Demo](https://img.shields.io/badge/demo-GitHub%20Pages-2ea44f)](https://dreamlovesu32.github.io/chemcore/)
[![Windows 安装包](https://img.shields.io/badge/Windows-installer-0078d4)](https://github.com/dreamlovesu32/chemcore/releases/download/v1.0.0-beta.3/Chemcore_1.0.0-beta.3_x64-setup.exe)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](./LICENSE)
[![Version](https://img.shields.io/badge/version-1.0.0--beta.3-orange)](https://github.com/dreamlovesu32/chemcore/releases/tag/v1.0.0-beta.3)

ChemCore 是我从零开始开发的开源化学结构编辑器，目标是无缝衔接 ChemDraw 级别的日常科研绘图、论文排版和 Office 复制粘贴工作流。Windows 用户可以直接下载当前 beta 的 [ChemCore 1.0.0-beta.3 x64 安装包](https://github.com/dreamlovesu32/chemcore/releases/download/v1.0.0-beta.3/Chemcore_1.0.0-beta.3_x64-setup.exe)。这个安装包包含桌面端和 Windows Office/OLE 集成服务；目前还没有代码签名，beta 阶段 Windows 可能会弹出 SmartScreen 提醒。它是化学结构编辑器，不是通用 cheminformatics toolkit。作者：Jiajun ZHANG，邮箱：[dreamlovesu@hotmail.com](mailto:dreamlovesu@hotmail.com)，欢迎试用、反馈、提交 issue 或参与贡献。我希望 ChemCore 能成为一个完全免费的科研基础设施平台；未来还可以继续接入自动化、批量处理、AI 辅助科研接口，以及更多用心打磨的科研软件。ChemCore 只是第一步。

ChemCore 的核心架构是共享 Rust engine + 轻量 Web 界面。Rust 负责文档模型、编辑命令、命中测试、化学标签逻辑、隐式氢规则、CDXML/CDX 导入导出、渲染 primitive 和 Office/OLE 所需的矢量输出；前端主要负责事件采集、界面状态和显示。选择 Rust，是因为这类软件需要长时间维护的几何计算、格式解析和状态机：内存安全、可测试性、性能和强类型边界都很重要。同一套 Rust 内核既可以编译为 WASM 运行在浏览器端，也可以作为原生库服务桌面端和 Windows Office 集成；桌面壳使用 Tauri/WebView2，因此编辑器 UI 能复用 Web 技术，而核心行为仍由跨平台内核统一保证。

![ChemCore 编辑器界面](./docs/assets/readme/product-screenshot.png)

## 项目历史

ChemCore 项目始于 2026 年 4 月 23 日。早期开发历史已公开保留在
[`history/pre-public-release`](https://github.com/dreamlovesu32/chemcore/tree/history/pre-public-release)
分支，方便读者了解项目在公开发布前的演进过程。

ChemCore development started on April 23, 2026. The early development history is
published on the [`history/pre-public-release`](https://github.com/dreamlovesu32/chemcore/tree/history/pre-public-release)
branch so readers can follow how the project grew before the public release.

## 真实论文图谱对比

下面的两组 CDXML 文件来自我已公开发表的论文：

Jiajun ZHANG, Pinhong Chen,* Guosheng Liu*, Copper-Catalyzed Site- and Enantioselective C–H Cyanation of Trisubstituted Allenes, [Chin. J. Chem. 2026, 44, 1729–1734](https://onlinelibrary.wiley.com/doi/full/10.1002/cjoc.70531).

这不是专门为演示准备的简单样例，而是真实论文中的复杂反应图谱。它包含多种文本、结构、反应箭头、括号、颜色、自由基/单电子点、图形对象和排版细节。下图左侧是 ChemDraw 导出的 SVG，右侧是 ChemCore 从同一份 CDXML 导入后导出的 SVG。

这些 benchmark CDXML 文件由维护者本人绘制，并随仓库一起提供，用于可复现的渲染对比。

![ChemDraw 与 ChemCore 真实论文 CDXML 渲染对比](./docs/assets/readme/comparison/published-cdxml-comparison.svg)

原始 CDXML 文件也已放在仓库根目录：[figure1.cdxml](./figure1.cdxml) 和 [figure2.cdxml](./figure2.cdxml)。对应的 SVG 与 EMF 矢量产物保留在 [docs/assets/readme/comparison](./docs/assets/readme/comparison/) 中，包含 ChemDraw 和 ChemCore 各自导出的版本。对 ChemCore 来说，这类真实文件的兼容性和 Office 级矢量导出能力，是项目最重要的宣传点之一。

## 当前状态

当前版本：`1.0.0-beta.3`。

Windows x64 安装包已经放在 [v1.0.0-beta.3 release](https://github.com/dreamlovesu32/chemcore/releases/tag/v1.0.0-beta.3) 中，包含 Tauri/WebView2 桌面端、文件关联和 Office/OLE 集成服务。当前安装包仍是 beta 版本，尚未代码签名。浏览器 demo 通过 GitHub Pages 发布：<https://dreamlovesu32.github.io/chemcore/>。

## 产品特色

- **面向真实科研绘图工作流**：ChemCore 不是只展示分子图的演示型编辑器，而是围绕“画结构、排反应式、复制到 Word/PowerPoint、再回来编辑”这条实际链路设计。
- **尽可能兼容 ChemDraw 文件与排版习惯**：项目把 CDXML/CDX 导入导出作为一等能力处理，目标是在结构、文本、箭头、括号、符号、颜色和对象位置上尽量保留源文件表现。
- **浏览器端、桌面端、Office 集成共享同一套内核**：编辑规则、命中测试、化学标签、渲染 primitive、导入导出都集中在 Rust engine 中，避免不同端出现行为分叉。
- **低延迟编辑体验**：鼠标 hover、focus、选择、拖拽预览、旋转和缩放等高频交互走本地 WASM/Rust 热路径，不把每一次鼠标移动都变成跨进程请求。
- **现代桌面软件体验**：桌面端基于 Tauri/WebView2 构建，支持文件打开保存、拖拽打开、最近文件、标签页、未保存提醒、快捷键和 Windows 文件关联等基础体验。
- **为 Office 粘贴与嵌入认真设计**：复制时不只写一张图片，而是同时考虑 ChemCore native、CDXML、SVG、EMF、RTF/OOXML 和 OLE 对象等多格式链路，让 Office 里的显示和后续编辑都尽量可靠。

## 已实现的关键能力

- **CDXML/CDX 导入导出**：Rust engine 内置 CDXML/CDX 解析与写出路径，可把 ChemDraw 文件转换为 ChemCore 文档模型，并保留足够的源文件绘图信息用于重渲染和回写。
- **统一文档与渲染模型**：文档模型、运行时 scene、命中测试、选择状态和 render primitive 都在内核中定义；前端主要负责事件采集和显示，不重新发明化学规则。
- **复杂键绘制几何**：已实现普通键、双键、三键、实/虚楔形键、虚线键、哈希键、label clipping、键键接触、交叉白边和 ChemDraw 风格模板参数等规则。
- **箭头与图形对象**：支持反应箭头、平衡箭头、空心箭头、弯箭头、括号、线条、图形和符号对象，并持续对齐 ChemDraw 的交互和渲染细节。
- **选择、拖拽、旋转与排列**：支持对象级和分子局部选择，支持多选拖拽预览、旋转、翻转、对齐、分布、颜色应用和可撤销命令历史。
- **文本与标签编辑**：支持普通文本、端点元素替换、标签编辑、文本选择、样式同步，以及化学标签与普通自由文本之间的行为区分。
- **隐式氢与元素规则**：主族元素的自动加氢、价键计数、电荷影响和特殊周期规则在内核中统一处理，避免前端和导出路径各算一套。
- **缩写与基团识别**：支持常见缩写和 functional group 识别，例如 Me、Et、Ph、CN、NO2、Boc、Ts、TMS、TBDMS、TBDPS 等；缩写在翻转、分子式和分子量统计中作为整体处理。
- **分子式和质量统计**：选中结构后可计算 Formula Weight、Exact Mass 等信息，并把可识别缩写的展开组成纳入统计。
- **桌面和 Office 基础链路**：桌面端、浏览器端和 Office/OLE 服务已经建立长期边界；Windows 剪贴板、EMF 预览、Word OOXML/OLE payload 等路径已有实现基础。

## 设计细节

ChemCore 的很多实现选择都围绕“用起来顺不顺手”展开，尤其重视那些只有真实绘图时才会暴露出来的细节。这些逻辑尽量放在 Rust engine 中统一处理，浏览器端、桌面端和 Office 导出路径共享同一套几何结果。

### 文本裁剪

ChemCore 不把标签简单当作一个矩形框来避让。化学标签会先被拆成 styled runs 和 line runs，再由内核根据字体大小、上下标、基线和字符 advance 生成逐字 glyph polygon。渲染键时，键端从节点出发沿键方向与这些 glyph polygon 的每条边求线段交点，取离节点最远的出界交点作为真正的键端，再沿键方向额外退让 `label_clip_margin`。默认模板下这个退让为 `1.2pt`，ACS Document 1996 模板为 `0.8pt`，并会随文档样式同步写入 bond-level 参数。

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

ChemCore 的编辑区不是固定页面截图，而是一个运行时 `viewBox`。前端维护 `runtimeViewBox`、缩放比例和滚动容器：SVG 的 `viewBox` 使用文档世界坐标，CSS 宽高按 `pt -> css px -> zoom` 换算，滚动条只负责查看当前世界坐标窗口。默认空文档会以可见区域为基础，在四周预留 `0.6` 倍可见宽高的 buffer。

每次文档渲染后，前端会用内核 render primitive 计算 document bounds。如果内容接近当前 `viewBox` 边缘 `0.18` 倍可见宽高以内，就自动向对应方向扩展画布，并在左侧或上侧扩展时同步补偿 scroll delta，避免画布扩张造成视觉跳动。缩放时也会保存当前焦点区域，优先围绕选区、文档或视口中心缩放，而不是简单把滚动位置归零。

### 对象稳定性与标签页

选择框、hover、拖拽预览、旋转控制点、弯箭头控制点、文本框和图形对象都需要在大文件中保持稳定反馈。对象被选中后，内部原子、键和文本不应继续产生 hover 抖动；拖拽时前端先用 preview transform 做实时跟随，提交时再把结果落到内核，避免每个鼠标移动都跨端提交完整文档。

桌面端支持多标签文档体验。新建和拖入文件会以标签页方式组织；空白且未修改的文档可以直接被新文件替换，已有内容或有改动的文档在关闭标签页或退出软件时会进入保存确认流程。每个标签页保存自己的文档、缩放和运行时视图状态，切换回来时尽量保持用户离开前的工作位置。

架构上，ChemCore 尽量把长期会影响一致性的逻辑放在内核里：命中测试、选择范围、hover 行为、绘制几何、文本裁剪、键交汇、隐式氢、缩写识别、CDXML 解析、导出渲染都尽量由 Rust engine 统一负责。这样浏览器端、桌面端和 Office 路径才能共享同一套行为，而不是靠前端补丁临时拼出来。

项目目前仍在快速迭代中，复杂 CDXML 文件、Office 复制粘贴、OLE 嵌入对象和 ChemDraw 级别的格式保真仍会继续打磨。如果你发现任何不顺手、不兼容或“看起来差一点”的地方，非常欢迎反馈。

## 欢迎体验

如果你也长期使用 ChemDraw，或者对免费的科研基础设施、AI 辅助软件开发、化学绘图工具链感兴趣，欢迎试用 ChemCore、提交 issue、参与讨论，或者直接贡献代码。

我尤其欢迎两类反馈：一类是具体文件和截图，帮助我们对齐 ChemDraw 的显示与交互；另一类是实际科研写作中的复制粘贴、Office 编辑、排版和导出问题。ChemCore 的目标不是做一个看起来像编辑器的演示项目，而是做一个真正能进入日常科研工作流的工具。

欢迎直接通过 README 开头的邮箱联系作者。

## 仓库结构

```text
chemcore/
  crates/chemcore-engine/          Rust 文档、编辑、渲染、CDXML 和 WASM 内核
  crates/chemcore-desktop-service/ 桌面端原生 engine session 与文件能力
  apps/chemcore-desktop/           Tauri Windows 桌面应用
  apps/chemcore-office/            Windows Office/OLE 集成服务
  viewer/                          浏览器编辑器宿主和生成的 WASM package
  docs/                            可公开的规则、规范、架构文档和 README 资产
  examples/                        ChemCore 原生文档示例
  fixtures/                        公开 synthetic CDXML 回归 fixture
  scripts/                         构建、验证和回归辅助脚本
  shared/                          Rust 和 viewer 共用 JSON 数据
```

## 环境要求

- Rust stable，Windows 桌面路径需要 MSVC toolchain
- Node.js 和 npm
- Python 3，用于本地静态服务和部分可选分析脚本
- `npm run build:engine-wasm` 会在需要时安装 `wasm-pack`
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

构建 release 二进制：

```bash
npm run desktop:build-fast
cargo build -p chemcore-office --release
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
cargo test -p chemcore-engine
cargo test -p chemcore-office
cargo test -p chemcore-engine public_cdxml_fixture_svg_golden_snapshots_match --test render_document
npm run build:engine-wasm
node --check viewer/app.js
```

公开 synthetic CDXML fixture 位于 [fixtures/cdxml](./fixtures/cdxml/)，对应 golden SVG 快照位于 [fixtures/expected/svg](./fixtures/expected/svg/)。对比与快照流程见 [渲染对比与回归资产](./docs/rendering-comparison.zh-CN.md)。

部分脚本会和本机桌面应用或 Office 做输出对照。这些流程是可选的，可能需要 Windows 专有软件、本地文档，或用 `CHEMCORE_PYTHON` 指向装有分析依赖的 Python 环境。

## 设计文档

- 缩写识别规则：[English](./docs/abbreviation-recognition-rules.md) / [中文](./docs/abbreviation-recognition-rules.zh-CN.md)
- 架构总览：[English](./docs/architecture.md) / [中文](./docs/architecture.zh-CN.md)
- 键绘制规则：[English](./docs/bond-rendering-rules.md) / [中文](./docs/bond-rendering-rules.zh-CN.md)
- 电荷与自由基符号规则：[English](./docs/charge-radical-symbol-rules.md) / [中文](./docs/charge-radical-symbol-rules.zh-CN.md)
- Document Commit 合同：[English](./docs/document-commit-contract.md) / [中文](./docs/document-commit-contract.zh-CN.md)
- 编辑器命令历史：[English](./docs/editor-command-history.md) / [中文](./docs/editor-command-history.zh-CN.md)
- 格式 v0.1：[English](./docs/format-v0.1.md) / [中文](./docs/format-v0.1.zh-CN.md)
- Glyph 裁剪规则：[English](./docs/glyph-clip-polygons.md) / [中文](./docs/glyph-clip-polygons.zh-CN.md)
- Glyph kernel：[English](./docs/glyph-kernel.md) / [中文](./docs/glyph-kernel.zh-CN.md)
- 隐式氢规则：[English](./docs/implicit-hydrogen-rules.md) / [中文](./docs/implicit-hydrogen-rules.zh-CN.md)
- 项目规则：[English](./docs/project-rules.md) / [中文](./docs/project-rules.zh-CN.md)
- 渲染对比与回归资产：[English](./docs/rendering-comparison.md) / [中文](./docs/rendering-comparison.zh-CN.md)
- Rust engine 架构：[English](./docs/rust-engine-architecture.md) / [中文](./docs/rust-engine-architecture.zh-CN.md)
- 文本符号与 glyph profile：[English](./docs/text-symbol-glyph-profile-rules.md) / [中文](./docs/text-symbol-glyph-profile-rules.zh-CN.md)
- 价键驱动标签识别：[English](./docs/valence-label-recognition-rules.md) / [中文](./docs/valence-label-recognition-rules.zh-CN.md)
- Windows 桌面端与 Office 架构：[English](./docs/windows-desktop-office-architecture.md) / [中文](./docs/windows-desktop-office-architecture.zh-CN.md)
- Release notes：[English](./CHANGELOG.md) / [中文](./CHANGELOG.zh-CN.md) / [v1.0.0-beta.3](./docs/releases/v1.0.0-beta.3.zh-CN.md)
- Roadmap：[English](./ROADMAP.md) / [中文](./ROADMAP.zh-CN.md)
- [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)

## 许可证

ChemCore 使用 Apache License, Version 2.0 授权。见 [LICENSE](./LICENSE) 和 [NOTICE](./NOTICE)。
