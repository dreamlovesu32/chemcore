# Chemcore Rust Engine

Chemcore 的编辑能力从现在开始以 Rust 核心为准。Web、Windows、iPad 端只负责 UI、输入事件、文件系统和像素渲染；文档模型、命中测试、吸附、工具状态和命令行为都应进入同一套 engine。

## 目标边界

`crates/chemcore-engine` 负责：

- `chemcore.json` 文档模型和序列化
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

## 当前实现

第一版 Rust engine 已接管 Web 编辑器的单键绘制路径：

- 空白文档创建
- 单键工具状态
- 端点 hover 命中
- 空白处点击添加横向单键
- 端点点击按 120 度延伸
- 拖拽时固定键长并做角度吸附
- WASM API 输出当前 `chemcore.json` 和 overlay 状态

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

viewer 使用 `viewer/engine/chemcore_engine.js` 和 `viewer/engine/chemcore_engine_bg.wasm`。这些文件是当前 Web 壳的运行时产物，需要和 Rust core 改动一起更新。
