# chemcore

`chemcore` 是一个跨平台化学文档核心。

这个项目的目标不是“先做一个网页 demo，之后再重写成桌面版”。当前主线是把文档模型、编辑行为、命中测试、化学标签逻辑、CDXML 导入导出和 render primitive 生成都收在共享 Rust core 里。

## 当前范围

当前有效实现集中在 [`crates/chemcore-engine`](./crates/chemcore-engine)：

- `document.rs`：`chemcore` v0.1 文档模型和 JSON 解析
- `engine.rs` 与 `engine/*`：编辑状态、工具、命令历史、选择、删除、剪贴板、模板、文本编辑
- `render.rs` 与 `render_*`：后端无关 render primitives
- `cdxml.rs`：Rust 原生 CDXML 导入和导出
- `abbreviation.rs`、`label_rules.rs`、`symbols.rs`、`repeating_units.rs`：化学标签和符号行为
- `wasm.rs`：浏览器侧 engine 绑定

[`viewer`](./viewer) 是浏览器宿主，负责 toolbar、文件打开保存、浏览器事件、坐标换算和 SVG/DOM 绘制。化学行为应来自 engine 状态和 render primitives，不应在 viewer 里另写一套。

## 设计文档

当前设计基线在下面这些文件里：

- [docs/architecture.md](./docs/architecture.md)
- [docs/format-v0.1.md](./docs/format-v0.1.md)
- [docs/project-rules.zh-CN.md](./docs/project-rules.zh-CN.md)
- [docs/implicit-hydrogen-rules.zh-CN.md](./docs/implicit-hydrogen-rules.zh-CN.md)
- [docs/abbreviation-recognition-rules.zh-CN.md](./docs/abbreviation-recognition-rules.zh-CN.md)
- [docs/bond-rendering-rules.zh-CN.md](./docs/bond-rendering-rules.zh-CN.md)
- [docs/editor-command-history.md](./docs/editor-command-history.md)
- [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)
- [examples/document-v0.1.ccjs](./examples/document-v0.1.ccjs)

## 工作区结构

```text
chemcore/
  crates/chemcore-engine/    Rust 文档、编辑、渲染、CDXML、WASM 核心
  viewer/                    浏览器编辑器宿主和生成的 WASM package
  docs/                      架构、格式、渲染和行为文档
  examples/                  ChemCore 原生文档示例
  scripts/                   构建、验证和浏览器回归辅助脚本
  shared/                    Rust/viewer 共用 JSON 数据
```

## 常用命令

```bash
cargo test
npm run build:engine-wasm
npm run dev:engine
npm run verify
node --check viewer/app.js
```

`npm run verify` 会跑 Rust 测试、重建浏览器 engine WASM、检查 viewer 语法，并确认 `viewer/engine` 生成物已同步。
