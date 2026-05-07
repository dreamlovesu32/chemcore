# Chemcore Rust Engine

Chemcore 的编辑能力从现在开始以 Rust 核心为准。Web、Windows、iPad 端只负责 UI、输入事件、文件系统和像素渲染；文档模型、命中测试、吸附、工具状态和命令行为都应进入同一套 engine。

## 目标边界

`crates/chemcore-engine` 负责：

- `.ccjs` / `.ccjz` 原生文档模型和序列化
- 编辑工具状态
- pointer/key 事件归一化后的处理
- 端点、键、标签、形状等对象命中测试
- 键长、角度、空角等化学绘图吸附规则
- 文档 mutation 和 undo/redo command model
- 几何 display list 或可渲染 overlay 输出

平台壳负责：

- toolbar、菜单、文件打开保存
- DOM/SVG/Canvas/Skia/CoreGraphics 等具体绘制
- pointer/key/menu/file 事件采集
- 调用 engine 并渲染 engine 输出

## WASM 和 Native 的关系

WASM 不是前端 fallback。它是同一个 Rust `chemcore-engine` 在浏览器/WebView 内的运行形态。

长期运行时边界如下：

- 浏览器端：通过 `WasmEngineHost` 调用 WASM core。
- Windows 桌面端默认热编辑路径：通过 `DesktopHybridEngineHost` 调用 WebView 内 WASM core。
- Windows 桌面端系统能力：通过 Tauri command 调用 native desktop service。
- `TauriEngineHost` / `?engine=tauri-native`：保留为诊断和未来 native editor path 验证，不是当前桌面默认热交互路径。

pointer move、hover、focus、hit testing、selection、drag preview、rotate/scale/move、object settings 等高频编辑行为，不应同步跨 Tauri IPC 再取完整 JSON snapshot。除非 native path 已经具备增量更新、事件合并和大文件性能证明，否则这些行为必须留在进程内 core runtime。

无论调用形态是 WASM 还是 native，化学绘图规则、命中测试、选择语义和文档 mutation 都必须在 Rust engine 中实现。viewer 可以展示表单和按钮，但不能重新实现一套对象设置、右键菜单、旋转/缩放或化学键行为。

## 当前实现

第一版 Rust engine 已接管 Web 编辑器的单键绘制路径：

- 空白文档创建
- 单键工具状态
- 端点 hover 命中
- 空白处点击添加横向单键
- 端点点击按 120 度延伸
- 拖拽时固定键长并做角度吸附
- WASM API 输出当前原生文档 JSON 和 overlay 状态
- 键工具下的键中心聚焦
- 单键按钮状态下点击键中心循环：偏置双键、居中等长双键、另一侧偏置双键
- 选择单个端点或键
- 删除当前选择
- 基于文档快照的 undo/redo 骨架

旧的 JS 单键几何、命中、吸附和加键逻辑已经从 `viewer/app.js` 删除。viewer 现在只把 pointer 事件转成文档坐标，交给 Rust WASM engine。

## 构建

Rust 测试：

```bash
cargo test
```

Web engine WASM：

```bash
npm run build:engine-wasm
```

高频开发时可以开自动重建：

```bash
npm run dev:engine
```

提交或交付前跑完整校验：

```bash
npm run verify
```

viewer 使用 `viewer/engine/chemcore_engine.js`、`viewer/engine/chemcore_engine.d.ts` 和 `viewer/engine/chemcore_engine_bg.wasm`。这些文件是当前 Web 壳的运行时产物；日常开发允许短暂不同步，但用 viewer 验证或提交前必须和 Rust core 改动同步。
