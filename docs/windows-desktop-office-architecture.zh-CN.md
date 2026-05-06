# Windows 桌面端与 Office 集成长期架构

本文记录 Chemcore Windows 桌面端和 Office 集成的长期方案。目标不是先做一个临时桌面壳再重构，而是从第一阶段就沿着最终产品形态建设：同一个 Rust 化学内核、一个专业 Windows 桌面应用、一个真正的 Office/OLE 集成层，以及一个仍然可共享的 Web 端适配层。

## 目标体验

最终 Windows 版 Chemcore 应达到类似 ChemDraw 的系统集成体验：

- 双击 `.ccjz`、`.ccjs`、`.cdxml` 可直接打开 Chemcore。
- Word、PowerPoint、Excel 中可以插入 Chemcore 对象。
- Office 文档中显示高质量预览图，而不是只显示附件或普通图片。
- 双击 Office 里的 Chemcore 对象可以打开 Chemcore 编辑。
- 编辑完成后，Office 内对象数据和预览同步更新。
- 从 Chemcore 复制到 Office 时，既有可再编辑的 Chemcore native object，也有 CDXML、SVG、PNG 等 fallback。
- Web 端、桌面端和 Office 对象使用同一个 Rust engine，不分叉业务逻辑。

## 总体架构

Chemcore 长期应是：

```text
同一个 Rust 化学内核
+ 一个专业 Windows 桌面 UI 壳
+ 一个 Windows/OLE/Office 集成层
+ 一个 Web 端适配层
```

建议仓库逐步形成这些模块：

```text
crates/chemcore-engine
  唯一化学内核：document model、editing commands、CDXML、SVG、render primitives、hit testing。

crates/chemcore-document
  原生文档容器：.ccjz/.ccjs、版本迁移、manifest、预览图、资源、校验。

crates/chemcore-render
  可复用渲染输出：SVG、PNG、EMF/WMF/PDF 等导出或预览目标。

crates/chemcore-desktop-service
  桌面文档服务：打开、保存、最近文件、锁文件、自动恢复、批量导出。

apps/chemcore-desktop
  Tauri Windows 桌面应用：窗口、菜单、快捷键、文件对话框、UI、WebView。

apps/chemcore-office
  Windows Office/OLE 集成：COM/OLE server、对象嵌入、预览、激活、粘贴。

viewer/
  Web UI 和桌面 UI 共用的编辑界面层。
```

重要原则：

- Rust `chemcore-engine` 仍是编辑、导入导出、命中测试、render primitives 和文档 mutation 的权威。
- 桌面端不复制一套化学逻辑。
- Office 集成层不直接解析和修改 Chemcore JSON。
- Web viewer 和桌面 viewer 不应分叉出两套行为。
- 系统能力通过 service/adapter 暴露，不能让 UI 层随意绕开 engine。

所有入口最终都应走同一组服务 API：

```text
open_document()
apply_command()
render_document()
save_document()
export_document()
generate_preview()
migrate_document()
```

## 桌面端技术路线

桌面端采用 Tauri 2 + WebView2。Tauri 官方 Windows 前置条件包括 Microsoft C++ Build Tools、Microsoft Edge WebView2、Rust 和 Node.js。Microsoft WebView2 Runtime 是 WebView2 app 的底层 Web 平台，发布时应选择 Evergreen 或 Fixed Version runtime 分发策略。

Tauri 在本项目中不是临时套壳，而是长期系统 adapter：

- Windows 窗口。
- 原生菜单栏。
- 快捷键。
- 文件打开/保存/另存为对话框。
- 最近文件。
- 拖拽文件打开。
- 系统剪贴板。
- 文件关联。
- 单实例和外部文件唤醒。
- 调用 Rust desktop service。
- 承载 Chemcore 专业编辑 UI。

WebView 只是显示和交互容器，不意味着产品要像浏览器。窗口中不应出现地址栏、浏览器菜单或临时网页式布局。桌面 UI 应按专业绘图软件设计：顶部菜单和工具栏、左侧工具箱、中间画布、右侧属性栏、底部状态栏。

## EngineHost 抽象

为了让 Web 和 Desktop 同步演进，需要在 UI 层和内核之间建立 host 抽象：

```text
EngineHost
  WasmEngineHost
    浏览器 Web 版：通过 wasm-bindgen 调用 chemcore-engine。

  TauriEngineHost
    Windows 桌面版：通过 Tauri command 调用 Rust native desktop service。
```

短期桌面版可以加载现有 viewer 和 WASM，以便快速启动桌面窗口。但这只能作为同一架构下的阶段性实现，不应把桌面版永久锁死在 WebView WASM 文件读写模型里。长期桌面版应让 Tauri Rust 后端直接调用 `chemcore-engine`，文件系统、gzip、Office 对象、预览生成和批量导出都走 native Rust。

截至 2026-05-06，代码中已经开始落实这条边界：

```text
viewer/engine_host.js
  前端 EngineHost 入口。Web 使用 WasmEngineHost；Tauri dev 使用 DesktopHybridEngineHost。
  hybrid 模式中 UI 仍走 WASM，同步编辑行为不变，同时启动时运行 native engine probe。

crates/chemcore-desktop-service
  原生桌面 document/engine service。直接持有 chemcore-engine::Engine session。

apps/chemcore-desktop/src-tauri
  Tauri command 边界。当前已经暴露 desktop_engine_* 命令给未来 TauriEngineHost 使用。
```

当前阶段 Web 仍默认使用 `WasmEngineHost`，桌面端使用 `DesktopHybridEngineHost`。这是为了保持编辑器同步调用模型稳定，同时保证 Tauri native command 通路不再只是空代码。下一阶段切换到 native path 时，不应让 UI 直接散落调用 Tauri command，而应只实现 `TauriEngineHost`，让它满足同一套 editor-facing session API。

同日后续推进中，桌面端非 Office 原生能力继续加厚：

```text
crates/chemcore-desktop-service
  已开始负责原生文件读写：.ccjz gzip、.ccjs、.cdxml、.svg。
  已持久化最近文件列表，供桌面菜单使用。

apps/chemcore-desktop/src-tauri
  已增加原生 File/Edit/View 菜单、快捷键、文件打开/保存/另存为对话框、拖拽打开、
  启动参数打开、最近文件菜单和 .ccjz/.ccjs/.cdxml 文件关联配置。
  已接入 Tauri single-instance 插件：第二次启动会把可打开文件参数转发给已有窗口，并唤醒主窗口。
  已接入 Windows 原生剪贴板 command：复制/剪切时写入 Chemcore 选择片段、整文档 JSON、CDXML、
  SVG 和 Unicode text fallback；粘贴时优先读取 Chemcore 选择片段并插入当前画布。
  已支持 PDF preview 导出：当前先由 WebView 将 SVG 预览栅格化并封装为单页 PDF。
  已支持基础 EMF preview 导出：当前由 Tauri 后端把 document render primitives 映射到 Win32 GDI
  Enhanced Metafile。该路径适合预览/Office fallback，后续仍应继续提升 path、字体和高级填充的保真度。

viewer/desktop_file_host.js
  WebView 内的桌面文件 host。桌面端优先走 Tauri native file commands；
  浏览器端继续走 File System Access API 或下载 fallback。
```

这仍不表示 Office/OLE 已完成，也不表示编辑事件已经切到 native engine path。当前桌面端的化学编辑交互仍主要通过 WebView + WASM engine 同步执行；这部分保持不变，是为了避免在文件系统原生化时同时改动 editor-facing API 的同步/异步模型。基础 EMF preview 已经落地，但还不是最终 Office/OLE 对象渲染后端；后续应继续把更多 SVG/path/text 细节迁到可测试的 native vector renderer。

建议的切换顺序：

1. 让 `TauriEngineHost` 先支持只读 session：create/free、documentJson、stateJson、renderListJson、renderBoundsJson、documentSvg、documentCdxml。
2. 再迁移文件打开/保存：`.ccjz` gzip、`.ccjs`、`.cdxml` 都由 `chemcore-desktop-service` 处理。
3. 最后迁移编辑命令、pointer events、text edit、clipboard。每迁移一组，都保持 Web viewer 仍走 `WasmEngineHost`。
4. native path 稳定后，桌面端默认使用 `TauriEngineHost`；Web 端继续使用 `WasmEngineHost`。

这个顺序可以避免 UI 和 engine 同时大改，也避免后续 Office 层绕开 desktop service。

## Office 集成策略

如果目标是 ChemDraw 级 Office 体验，核心不是单纯 Office Add-in，而是真正的 Windows OLE/COM 嵌入对象。

Office 集成分三层：

### 1. 文件关联

`.ccjz`、`.ccjs`、`.cdxml` 注册到 Chemcore。用户在文件系统、Outlook 附件、Office 最近文件或下载目录中双击这些文件时，Windows 用 Chemcore 打开。

Tauri bundle 可以配置 file associations；Windows 底层应使用明确的 extension + ProgID 方案。

### 2. 自定义协议

注册：

```text
chemcore://open?file=...
chemcore://open?id=...
chemcore://edit-object?id=...
```

这用于外部系统、网页、Office Add-in 或文档链接唤醒 Chemcore。它不是 Office 嵌入对象的替代品，只是启动和定位机制。

### 3. OLE/COM 嵌入对象

长期目标是实现 Chemcore OLE Object：

```text
Chemcore OLE Object
  - 内部存储 .ccjz 或等价 native object payload
  - 向 Office 暴露高质量预览
  - 支持双击激活编辑
  - 支持复制/粘贴为可编辑对象
  - 支持从 Office 文档保存和恢复对象状态
```

实现建议：

- 业务核心仍使用 Rust。
- COM/OLE 边界优先尝试 Rust `windows` crate。
- 如果 OLE 接口实现成本过高，可以允许非常薄的 C++/Win32 shim。
- C++ shim 只能负责 COM/OLE 注册、接口转发和 Windows 生命周期，不允许实现化学逻辑。

Office Add-in 可作为后续增强，用于 Ribbon 按钮、模板库、批量插入、选中对象编辑、导入导出入口等。但 Add-in 不应替代 OLE 对象，因为 Add-in 无法单独提供 ChemDraw 式双击对象编辑体验。

## 原生文档容器

当前 `.ccjz` 是 gzip JSON，适合早期阶段。但为了 Office 对象、预览、缩略图和资源管理，长期应把 `.ccjz` 的 API 设计成容器模型。

对外扩展名可以保持 `.ccjz`，内部逐步演进为：

```text
manifest.json
document.ccjs
preview.svg
preview.emf
preview.png
resources/
  images/
  fonts-or-glyph-cache/
meta/
  app-version.json
  migration.json
```

容器格式可以后续选择 zip、zstd package 或其他实现。第一阶段内部仍可保持 gzip JSON，但所有调用方都应通过稳定 API：

```text
load_ccjz()
save_ccjz()
extract_preview()
update_preview()
migrate()
```

这样后续从 gzip JSON 升级到多文件容器时，不需要推翻 Web、Desktop 或 Office 调用方。

## 剪贴板格式

ChemDraw 级体验必须重视剪贴板。复制 Chemcore 对象时，应同时写入多种格式：

```text
Chemcore native object
CDXML
SVG
PNG
Plain text / SMILES / InChI（后续可选）
```

粘贴时按优先级读取：

```text
Chemcore native > CDXML > SVG/PNG > text chemistry
```

这样在 Chemcore、Office、ChemDraw、浏览器、聊天工具之间都能有合理 fallback。

## 预览与导出格式

Office 中的对象预览不能只依赖 SVG。长期需要：

- SVG：Web 和现代 Office。
- PNG：通用 fallback。
- EMF：Windows Office 高质量嵌入预览。
- PDF：打印和发布导出。

这些输出应由 engine/render service 统一生成，Office 层不能自行画化学结构。

## 开发阶段

### 阶段 0：环境与依赖

- Windows 原生工具链跑通。
- 移除活跃运行入口中的 Bash/WSL 依赖。
- 安装 Tauri 项目级依赖：`@tauri-apps/cli`、`@tauri-apps/api`。
- 确认 WebView2 Runtime、MSVC Build Tools、Rust、Node.js 可用。

### 阶段 1：最终目录结构

- 新建 `apps/chemcore-desktop`。
- 新建 `apps/chemcore-office`。
- 新建 `crates/chemcore-document`。
- 新建 `crates/chemcore-desktop-service`。
- 先只建立边界和空实现，不急于迁移大量逻辑。

### 阶段 2：Document Service

- 包装现有 `chemcore-engine`。已开始：`crates/chemcore-desktop-service` 现在持有 native engine sessions。
- 定义打开、保存、导出、预览、迁移、命令执行 API。已开始：当前先暴露 document JSON、state JSON、render list、bounds、SVG、CDXML。
- Web 和 Desktop 都通过该 API 语义建模。

### 阶段 3：Tauri 桌面壳

- 建立 Tauri app。已完成：`apps/chemcore-desktop/src-tauri`。
- 加载现有 viewer UI。已完成：`npm run desktop:dev` 可启动 Windows 桌面窗口。
- 增加菜单、快捷键、文件对话框、最近文件、拖拽打开、单实例。已完成到单窗口原生菜单、快捷键、文件对话框、最近文件、拖拽打开和启动参数打开；单实例仍未接入。
- 配置 `.ccjz/.ccjs/.cdxml` 文件关联。已写入 Tauri bundle 配置，需通过 installer 安装后在 Windows 系统层验证。

### 阶段 4：桌面 Native Engine Path

- Tauri 后端直接调用 Rust engine。已开始：Tauri 已持有 `DesktopDocumentService`，并暴露 `desktop_engine_*` commands。
- WebView 不再负责本地文件系统、gzip 和路径权限。已开始：桌面打开/保存/另存为优先走 Tauri native file commands，`.ccjz` gzip 由 Rust service 处理。
- viewer 只负责 UI 和交互。

### 阶段 5：文档容器与预览

- `.ccjz` API 容器化。
- 增加 preview generation。
- 增加 format version 和 migration。
- 增加缩略图和资源管理预留。

### 阶段 6：Windows 剪贴板

- 先支持 native + SVG + PNG。
- 再加 CDXML。
- 后续补 EMF 和 text chemistry。

### 阶段 7：Office OLE 原型

- 注册 Chemcore OLE object。
- 在 Office 中插入对象。
- 显示 preview。
- 双击打开 Chemcore desktop。

### 阶段 8：Office 完整生命周期

- Office 文档保存和恢复 Chemcore object payload。
- 编辑后更新 Office preview。
- 支持复制/粘贴可编辑对象。
- 支持对象内嵌 `.ccjz` 数据。

### 阶段 9：Office Add-in 增强

- Ribbon 按钮。
- Insert Chemcore Object。
- Edit Selected Chemcore Object。
- Export/Convert。
- 模板库。

### 阶段 10：安装、签名、更新

- MSI/NSIS。
- 文件关联。
- COM/OLE 注册。
- WebView2 runtime 分发策略。
- 代码签名。
- 自动更新。

## 禁止路线

- 不做临时 Electron 版。
- 不做桌面专用化学编辑逻辑。
- 不让 Office 插件直接解析或修改 Chemcore JSON。
- 不只做 SVG 粘贴再以后补可编辑对象。
- 不把 `.ccjz` API 设计死成永远单一 gzip JSON。
- 不把 Tauri 后端变成第二套业务层。

## 当前环境状态

截至 2026-05-06：

```text
Tauri CLI:          2.11.0
@tauri-apps/api:    2.11.0
WebView2 Runtime:   147.0.3912.98
WebView2 location:  C:\Program Files (x86)\Microsoft\EdgeWebView\Application
MSVC Build Tools:   Visual Studio Build Tools 2022 17.14.21
Rust:               1.95.0, x86_64-pc-windows-msvc
Node.js:            D:\nodejs-24.15.0
```

当前仍未完成：

- 单实例和第二次打开文件唤醒已有窗口。
- 代码签名和自动更新。
- Windows 原生剪贴板多格式写入。
- EMF/PDF 等非 SVG 预览导出。
- Office/OLE/COM 集成。

## 参考资料

- Tauri prerequisites: https://v2.tauri.app/start/prerequisites/
- Tauri project creation and CLI installation: https://v2.tauri.app/start/create-project/
- Tauri CLI reference: https://v2.tauri.app/reference/cli/
- Tauri configuration and file associations: https://v2.tauri.app/fr/develop/configuration-files/
- Microsoft WebView2 Runtime distribution: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution
- Windows file type registration: https://learn.microsoft.com/en-us/windows/win32/shell/how-to-register-a-file-type-for-a-new-application
- Office Add-ins overview: https://learn.microsoft.com/en-us/office/dev/add-ins/overview/office-add-ins
