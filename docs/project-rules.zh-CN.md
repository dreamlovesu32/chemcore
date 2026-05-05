# Chemcore 项目规则

这份文档记录当前开发阶段也必须保持的项目级规则。更细的行为规则仍放在各专题文档里，例如格式、键绘制和命令历史。

## 内核边界

- Rust `crates/chemcore-engine` 是当前编辑行为、文档 mutation、命中测试、吸附、选择、删除、命令历史和 render primitive 的权威。
- Viewer 只负责 toolbar、菜单、文件打开保存、浏览器事件采集、坐标换算和 SVG/DOM 绘制。
- 新的化学编辑行为不应重新散回 `viewer/app.js`。如果 viewer 需要知道几何，应优先消费 engine 输出的 primitive 或显式状态。

## 文档单位

- 当前 `chemcore.json` 文件单位固定为印刷点数：`format.unit = "pt"`。
- 文档坐标、对象位置、键长、线宽、字号、命中半径和粘贴偏移等持久化或 engine 世界坐标值，都按 `pt` 解释。
- CSS 像素只允许出现在 viewer 边界和浏览器输入/显示适配层。进入 engine 前必须显式换算。
- 代码中仍有 `WorldCm`、`*_CM` 等历史命名时，当前语义按 `pt` 规则理解；后续重命名只能作为独立重构处理，不能顺手混进行为修改。
- 旧文档或日志里出现的 `cm` 规则已被 2026-04-30 的 `pt` 决策取代。

## WASM 同步

- 日常开发允许 Rust 源码和 `viewer/engine` 生成物短暂不同步。
- 需要在 viewer 里验证 engine 行为时，必须先重建 Web engine：

```bash
npm run build:engine-wasm
```

- 高频修改 Rust engine 时，建议开一个自动重建进程：

```bash
npm run dev:engine
```

- 准备提交、交付或让别人验证 viewer 前，必须跑：

```bash
npm run verify
```

这个命令会跑 Rust 测试、重建 engine WASM、检查 viewer 语法，并确认 `viewer/engine` 没有未提交的生成物差异。

## 生成物

- `viewer/engine/chemcore_engine.js`、`viewer/engine/chemcore_engine.d.ts` 和 `viewer/engine/chemcore_engine_bg.wasm` 是 Web viewer 的运行时生成物。
- 修改 `crates/chemcore-engine/src/wasm.rs`、engine API 或 render primitive 结构后，必须同步更新这些生成物。
- `wasm-pack` 生成的 `viewer/engine/.gitignore` 不应保留；构建脚本会删除它。

## 渲染几何

- Fragment 路径的键绘制规则以 `docs/bond-rendering-rules.zh-CN.md` 为行为基线。
- 键接触、label clipping、dash/hash knockout、预览态和落地态几何应由 Rust render 路径统一定义。
- Viewer 不应靠 SVG `linecap`、额外中心 patch 或前台补丁重新定义化学键几何。

## 化学标签

- 隐式氢行为以 `docs/implicit-hydrogen-rules.zh-CN.md` 为当前行为基线。
- 缩写识别与 functional group 展开计划以 `docs/abbreviation-recognition-rules.zh-CN.md` 为当前设计基线。
- formula-like 标签的下一阶段价键解析规则以 `docs/valence-label-recognition-rules.zh-CN.md` 为设计基线。
- 电荷、自由基和孤对符号归属到分子原子后的语义，以 `docs/charge-radical-symbol-rules.zh-CN.md` 为设计基线。
- 标签识别、隐式氢数量、生成标签文本和画键锚点都属于 Rust engine 行为，不应在 viewer 里另写一套。

## 文本符号和 Glyph

- 文本里的 Unicode 符号和特殊字符，以 `docs/text-symbol-glyph-profile-rules.zh-CN.md` 为当前行为基线。
- `shared/text_symbols.json` 只定义文本符号表 UI 分组；字符进入文档后仍是普通 text run 内容。
- glyph advance、ink box、background box、glyph polygon 和未知字符兜底由 Rust glyph kernel 统一定义；viewer 只能消费共享 profile 和 engine layout。

## 命令历史

- 只有已提交的文档变化进入 history。
- hover、focus halo、preview、lasso、active drag 和 caret movement 都是临时交互状态，不进入 history。
- 新编辑功能应使用语义 `EditorCommand`；`legacy-mutation` 只能视为迁移期警告。

## 常用命令

```bash
cargo test
npm run build:engine-wasm
npm run dev:engine
npm run verify
node --check viewer/app.js
```
