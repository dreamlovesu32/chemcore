# chemsema 架构

## 目标

`chemsema` 的定位是一个长期存在的化学文档核心，供以下场景共享：

- 浏览器宿主
- 桌面宿主
- 导入 / 导出链路
- 编辑器、CLI 与 agent 工具

这个项目围绕长期核心架构设计，优先保证文档模型、运行时模型和导入导出链路的稳定性。

## 核心原则

### 1. 平台无关的核心优先

文档模型是核心资产。

核心层必须先定义清楚：

- 文档结构
- 对象身份
- 坐标系统
- 样式引用
- 分组和 z-order
- 带化学语义的对象
- 渲染约定

Web 和桌面是共享同一核心的两个宿主。

桌面端的长期路径是混合运行时：

- 同一个 Rust core 编译成 WASM editor runtime，供浏览器和桌面 WebView 的高频编辑路径同步调用。
- 同一个 Rust core 也被 native desktop service 调用，负责文件、剪贴板、导出、Office/OLE 和后台预览等系统能力。
- Windows 桌面默认编辑 host 是 `DesktopHybridEngineHost`；`TauriEngineHost` / `?engine=tauri-native` 是诊断和未来 native path 验证入口。
- 这符合“一核两壳”：一核是 `chemsema-engine`，两壳是 browser shell 和 Tauri desktop shell，WASM/native 是同一核心的两个运行形态。

### 2. 化学语义和文档语义分离

化学结构数据和文档对象数据解决的是两类不同问题。

化学语义包括：

- 原子
- 键
- 立体化学
- 分子缩写
- `molblock2d`

文档语义包括：

- 对象定位
- 分组
- 样式引用
- 文本框
- 箭头
- 可见性
- z-order
- 变换

架构上将这两类关注点拆分到各自模型中。

### 3. 文件格式稳定，运行时模型为执行优化

文件格式是持久化约定。

运行时场景模型是执行模型。

二者应该接近，并允许为执行效率保留差异。文件格式应该显式、可版本化、便于迁移。运行时模型则应该适合：

- 命中测试
- 局部重绘
- 选择
- 命令执行
- 撤销 / 重做

### 4. 渲染后端可替换

第一个 backend 可以是 web，但绘图 API 应保持后端无关，避免绑定 DOM、React 或浏览器专属原语。

长期可能需要的 backend 包括：

- SVG
- Canvas / WebGL
- 原生桌面渲染
- PDF / SVG 导出渲染器

### 5. 导入是一级子系统

`chemsema` 必须能吃进旧格式，尤其是 CDXML。

导入应该直接落到 `chemsema` 文档模型上。

## 分层结构

目标系统按层拆分如下。

### Layer A: 文件格式

持久化后的 `chemsema` 文档。

职责：

- 版本管理
- 对象序列化
- 样式表序列化
- 对象关系
- 元数据

边界：

- 运行时缓存由运行时层负责
- 只属于 UI 的临时状态由宿主层负责

### Layer B: 运行时文档模型

内存中的文档图。

职责：

- 按 id 查对象
- 父子关系
- 对象类型
- 变换
- 样式解析

这一层应该是确定性的，并且适合后端无关的渲染。

### Layer C: 场景与几何服务

Web 和桌面宿主都会用到的共享逻辑。

职责：

- 世界坐标
- 局部坐标
- 包围盒
- z-order 遍历
- 命中测试
- 变换组合
- 可见性判断

### Layer D: 渲染接口

后端无关的绘制约定。

接口至少需要支持：

- begin/end frame
- push/pop transform
- draw text
- draw line/path
- draw molecule
- apply style

接口保持后端存储和绘制实现无关。

### Layer E: 宿主适配层

平台相关实现。

示例：

- web viewer
- desktop shell
- CLI exporter

宿主层复用核心文档模型。

### Layer F：质量与资格平面

GUI 测试不是 viewer 内的临时脚本，而是横跨全部宿主的独立质量平面。版本化场景由浏览器、真实 Tauri/WebView2、Windows UIA/真实输入和最终安装包黑盒 driver 共同执行，并用交互、化学、渲染、无障碍、持久化和运行质量 oracle 判定。Test ABI 只存在于测试构建，生产候选必须另行接受黑盒资格。完整合同见 [GUI 测试平台与展示可靠性长期架构](./gui-test-platform-and-demo-reliability.zh-CN.md)。

## 为什么 CDXML 解析要留在核心里

CDXML 目前是最现实的导入入口，因为它能把基于 ChemDraw 的工作流接到 `chemsema` 文档上。

当前有效的 CDXML parser 和 writer 已经在 Rust engine 里：

- [crates/chemsema-engine/src/cdxml.rs](../crates/chemsema-engine/src/cdxml.rs)

它们当前的职责是：

- 把 CDXML 解析成原生 `ChemSemaDocument` 对象和 molecule fragment
- 保留足够的导入元数据，让源文件绘图选项可以延续
- 把当前文档导出成 ChemDraw 可识别的 CDXML

## 当前文档里程碑

当前持久化规范是 CCJS v0.2：scene entity 平铺保存，`hierarchy` 是单归属树索引，`relations` 表达类型化跨对象语义，运行时空间网格由快照派生。`.ccjs` 是完整 UTF-8 JSON；`.ccjz` 是确定性、带哈希、可分块读取的 Container v1。局部交互使用 Document Patch，崩溃恢复使用独立 Journal。

字段合同见 [format-v0.2.zh-CN.md](./format-v0.2.zh-CN.md)，设计理由见 [ccjs-architecture-and-format-rationale.zh-CN.md](./ccjs-architecture-and-format-rationale.zh-CN.md)，当前实现与 stable 前剩余门禁见 [ccjs-v0.2-stability-architecture.zh-CN.md](./ccjs-v0.2-stability-architecture.zh-CN.md)。v0.1 只作为迁移输入保留，不再代表当前架构。

仍未完成的长期扩展包括更完整的 ChemDraw 覆盖、富 query chemistry、高级聚合物语义、多页布局和多人协同。编辑器可见区 CCJZ 分块加载、保留 undo 的 hydration、未变 entry 复用和浏览器 Zip64 已实现；大型数组继续交给 HDF5/Zarr 等专业格式，通过 CCJZ attachment 与文档语义关联。
