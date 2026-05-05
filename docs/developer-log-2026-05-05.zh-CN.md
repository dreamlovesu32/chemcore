# Chemcore 开发者日志 - 2026-05-05

作者：张家骏

时间范围：2026-05-05 00:00 至 2026-05-05 23:59，Asia/Shanghai

基线提交：`56cdd4a feat: adopt ccjs and ccjz document extensions`

工作目录：`D:\chemcore`

### 总结

今天的主线是把项目从 WSL 交接到 Windows 原生开发环境，并确认 `chemcore-engine`、WASM viewer、npm 脚本、Git、VS Code 和浏览器保存流程在 Windows 上可以正常工作。迁移后的方向不变：Rust `chemcore-engine` 仍然是文档模型、编辑语义、导入导出、命中测试和 render primitives 的权威；`viewer/` 只作为浏览器端适配层。

今天还修复了 viewer 里的保存失败问题。直接原因是 `viewer/document_flow.js` 使用了 `CHEMCORE_TEXT_EXTENSION` 但没有导入，点击 Save as 时会抛出 `ReferenceError`。同时调整了保存顺序，让浏览器文件保存框先在用户点击手势内打开，之后再生成或压缩保存内容，避免 Chromium 的 File System Access API 因异步压缩丢失用户激活。

原根目录临时交接文件 `HANDOFF-2026-05-05.md` 的内容已经收敛进本日志，包括迁移背景、今日提交、交接注意事项、已知风险和后续路线。该临时文件已经删除，不再作为单独交接文档保留。

### 今日接收的迁移状态

迁移目标目录是 `D:\chemcore`，对应 WSL 路径为 `/mnt/d/chemcore`。原 WSL 源目录是 `/home/jiajun/chemcore`。迁移采用完整复制，包含 `.git`、`target/`、`node_modules/`、`tmp/`、`viewer/engine` 生成物和当时工作树状态。

迁移时两边 HEAD 一致：

```text
56cdd4a feat: adopt ccjs and ccjz document extensions
```

今天接手时需要吸收并写入日志的两个开发提交是：

```text
0e4e0e1 feat: improve cdxml rendering fidelity
56cdd4a feat: adopt ccjs and ccjz document extensions
```

`0e4e0e1` 是今天最大的一次功能提交，共涉及 49 个文件，约 4462 行新增和 542 行删除。它不是单点修补，而是围绕 ChemDraw/CDXML 保真、箭头语义、结构标签、glyph profile、文本符号和回归测试做了一轮系统性加厚。

CDXML 颜色系统被收拢到统一实现：

- 新增 `crates/chemcore-engine/src/cdxml/colors.rs`，把 CDXML `<colortable>` 的解析、颜色 id 映射和导出收敛到同一个模块。
- 明确 ChemDraw 颜色 id 语义：`color="0"` 表示前景色，默认黑色；`bgcolor="1"` 表示背景色，默认白色；用户颜色从 `<colortable>` 的 id `2` 开始。
- RGB 小数按 `0..1` 范围解析，再 round 到 `#rrggbb`，避免导入导出时因为浮点小数和整数颜色之间的转换产生漂移。
- CDXML 导入路径中，line、shape、text object、text run、fragment label、页面背景和对象样式都改成通过同一个颜色表解析。
- CDXML 导出路径中，先收集文档实际用到的颜色，再统一写出 `<colortable>`，并让对象颜色反查到稳定 id。
- 增加颜色相关测试，例如 duplicate color slots、Default/ACS 样例、非白页面背景和重新导入后的颜色保持。

箭头模型和渲染语义被显著扩充：

- 内部 document model 加厚了 `arrowHead` 和 `arrowGeometry`，不再只靠少量前端档位描述箭头。
- 保存 `kind` 区分 solid、hollow、open hollow；保存 `head`、`tail` 区分 full、left、right、none 等端点样式。
- 保存 `length`、`centerLength`、`width`，分别对齐 ChemDraw 的 `HeadSize`、`ArrowheadCenterSize` 和 `ArrowheadWidth`。
- 保存 `curve` 与椭圆弧几何，用于弯曲箭头和 curved double arrow。
- 保存 `noGo` 的 cross/hash 语义，以及 `bold` 粗箭头语义。
- `crates/chemcore-engine/src/render_objects/arrows.rs` 扩展为能渲染实心箭头、空心箭头、开口箭头、半边箭头、双头箭头、弯曲箭头、粗箭头和 no-go 标记。
- CDXML 导入读取箭头尺寸、类型、端点、弯曲参数和颜色；CDXML 导出写回 `ArrowheadType`、`ArrowheadHead`、`ArrowheadTail`、`ArrowheadCenterSize`、`ArrowheadWidth`、弯曲几何和颜色。
- 选择/编辑路径同步更新，包括 `editing/arrows.rs`、`engine/arrows.rs`、`engine/select/arrows.rs` 和 `editing/geometry.rs`，让 hover、拖拽、选中样式更新和缩放后的箭头几何保持一致。
- 新增或扩展测试覆盖：箭头头部不被尺寸下限覆盖、箭头尺寸相对线宽、open/hollow 独立尺寸模板、半边箭头在曲线上的视觉左右侧、CDXML 导入导出后箭头 fixture 稳定。

结构标签、缩写和 valence 识别继续推进：

- `abbreviation.rs`、`abbreviation/expansion.rs`、`abbreviation/valence.rs` 继续把化学缩写、开放价、终端基团和桥连基团识别规则收到 Rust engine。
- 分子 fragment label 不再当作普通 text object 处理，避免导入 CDXML 后把结构标签和自由文本混在一起。
- 结构 label 支持 `lineRuns`，用于保留类似上方 `H`、下方 `N` 的多行/多 run 标签布局。
- source runs 和 normalized display runs 分离：源文件中的 run 信息保存在 import meta，显示与编辑使用化学上归一化后的 runs。
- 文本编辑路径更新了 `engine/text_edit` 下的 geometry、labels、layout、runs，使 endpoint label、text object、reopen edit session、caret/selection geometry 更接近 Rust glyph kernel 的结果。
- 测试覆盖继续补齐，包括 terminal abbreviation、two-connection bridge、charged B/N/O 例外、P/S 隐式氢规则、卤素交替隐式氢规则、右侧 label anchor、重开文本编辑 session 后 bbox/anchor 精度稳定。

glyph kernel、文本符号和 viewer 文本渲染路径有一轮完整补强：

- 新增 `shared/text_symbols.json`，把常用文本符号和化学排版符号变成共享数据源。
- 新增 `viewer/text_symbol_palette.js`，viewer 侧可以展示文本符号 palette，并把选择的符号插入当前文本编辑器或切换到文本工具待插入。
- 新增 `scripts/generate-glyph-profiles.py`，用于生成/更新共享 glyph profiles。
- 新增 `scripts/text-symbol-regression.mjs`，用于文本符号回归检查。
- 更新 `shared/glyph_profiles.json`，让 glyph advance、ink box、background box、polygon 和 clipping 数据与 Rust kernel 保持一致。
- 更新 `glyph_kernel.rs`、`viewer/text_metrics.js`、`viewer/primitive_dom_renderer.js`、`viewer/object_fallbacks.js` 和 `viewer/styles.css`，让 viewer 消费 engine 和共享 profile，而不是重新发明文本测量逻辑。
- 新增 `docs/text-symbol-glyph-profile-rules.zh-CN.md`，记录文本符号与 glyph profile 的维护规则。
- `docs/glyph-kernel.md` 同步更新，继续强调 Rust glyph kernel 是 advance、ink box、background box、glyph polygon 和 label clipping 的权威。

渲染、格式和导入边界也同步加厚：

- `document.rs` 扩展了箭头、shape、text、style payload 等字段，以便 JSON 能保存导入自 CDXML 的真实语义，而不是只保存 viewer 当前能画出的近似。
- `render.rs`、`render_bonds.rs`、`render/bond_metrics.rs`、`render/style_payload.rs`、`render_objects/text.rs`、`render_primitives.rs` 和 `render_svg.rs` 继续收敛 render primitive 输出。
- shape 对象继续补齐 ChemDraw 样式，包括 rectangle、round rect、ellipse、shadowed、shaded、dashed 等几何和样式字段。
- bracket、shape、text bbox 等小对象不再被不合理固定下限撑大，导入后更接近源 CDXML 尺寸。
- ACS Document 1996 与 Default 的绘图参数继续分开维护，避免把 ACS 当成 Default 的简单缩放。
- JSON import boundary 会迁移 legacy aligned text box、补默认 arrow geometry、归一化 text/shape payload，避免旧文件打开后缺字段。
- `docs/format-v0.1.md` 和 `docs/format-v0.1.zh-CN.md` 同步记录这些模型字段，`docs/project-rules.zh-CN.md` 同步强调 engine/viewer 职责边界。

测试资产和验证面大幅扩展：

- `crates/chemcore-engine/tests/render_document.rs` 增加大量 CDXML、SVG、箭头、shape、glyph、文本和导入导出稳定性用例。
- `crates/chemcore-engine/tests/bond_tool.rs` 覆盖编辑器工具行为，包括箭头 hover/drag、模板、shape、select、text、symbol、bracket 等交互。
- `crates/chemcore-engine/tests/text_tool.rs` 覆盖 endpoint label、plain text object、text run、重开编辑、caret、selection、识别失败红框等文本编辑行为。
- 新增 fixture 稳定性检查，包括 CDXML import -> export -> import 后 render/SVG 是否稳定，以及 `tmp/` fixture 的对象语义是否保持。
- `viewer/engine/chemcore_engine_bg.wasm` 随 Rust engine 更新同步重建，让浏览器 viewer 可见行为跟内核一致。

`56cdd4a` 是今天第二个开发提交，共涉及 11 个文件，约 176 行新增和 77 行删除。它把产品自己的文件入口从泛泛的 `.json` 迁到 `.ccjs` / `.ccjz`，并把 viewer 打开/保存流程一起改到新格式。

原生格式的命名和职责被重新定下来：

- `.ccjz` 是默认产品保存格式，即 gzip 压缩后的 chemcore JSON。它适合用户日常保存和传递。
- `.ccjs` 是可读调试格式，即纯文本 chemcore JSON。它适合人工检查、diff 和问题复现。
- 不再把 chemcore 原生文件暴露成普通 `.json`，避免和任意 JSON 文件混淆，也避免用户误以为这是通用 JSON 数据。
- 示例文件从 `examples/document-v0.1.json` 重命名为 `examples/document-v0.1.ccjs`，让 examples 与新文件策略一致。

viewer 文件流被重构为集中处理格式判断：

- `viewer/file_io.js` 新增 `.ccjs` / `.ccjz` 的扩展名常量、MIME 常量、文件名格式判断、base name 处理、压缩和解压工具。
- `.ccjz` 读写使用浏览器 `CompressionStream` / `DecompressionStream`，内容仍是 chemcore JSON，只是以 gzip 形式存储。
- open accept list 同时接受 `.ccjz`、`.ccjs`、`.cdxml` 以及对应 MIME，方便浏览器文件选择器过滤。
- `documentTitleForFileName` 默认生成 `.ccjz` 文件名，保存时会从当前文档 title 或当前文件名推导安全文件名。
- `viewer/document_flow.js` 的 open path 会按文件名或 MIME 判断是否需要 gzip 解压，再按内容判断是否为 CDXML。
- Save as 支持 `.ccjz`、`.ccjs`、`.cdxml`、`.svg` 四类输出；未识别扩展名默认按 `.ccjz` 保存。
- `viewer/app.js` 适配新的示例文件扩展名和文件流 API。

文档和项目规则同步更新：

- `README.md` 与 `README.zh-CN.md` 把原生格式说明改为 `.ccjs/.ccjz`。
- `docs/format-v0.1.md` 与 `docs/format-v0.1.zh-CN.md` 明确 `.ccjz` 为 gzip JSON、`.ccjs` 为 debug JSON。
- `docs/project-rules.zh-CN.md` 写入原生格式规则，强调 `.ccjz` 是默认保存格式，`.ccjs` 是调试格式。
- `docs/rust-engine-architecture.zh-CN.md` 和 `docs/viewer-rendering-report.zh-CN.md` 同步术语，避免旧 `.json` 入口继续出现在开发规则里。
- 这次格式迁移也为后续 Tauri/桌面壳文件关联做了铺垫：产品可以关联 `.ccjz/.ccjs`，而不是抢普通 `.json`。

### Windows 原生环境重建

今天在 Windows 上完成了工具链重建和路径整理。目标原则是能安装到 D 盘的尽量安装到 D 盘；系统已有但版本不合适的工具不做强行兼容，而是更新或安装新的 Windows 原生版本。

已确认或设置的主要工具：

```text
Git:                 D:\Git
Git version:         2.54.0.windows.1
Git Bash:            D:\Git\bin\bash.exe
Git Bash version:    GNU bash 5.3.9

Rust cargo home:     D:\Rust\cargo
Rust rustup home:    D:\Rust\rustup
Rust toolchain:      stable-x86_64-pc-windows-msvc
rustc:               1.95.0
cargo:               1.95.0
Rust targets:        x86_64-pc-windows-msvc, wasm32-unknown-unknown
wasm-pack:           0.14.0

Node installed path: D:\nodejs-24.15.0
Node installed ver.: 24.15.0
npm installed ver.:  11.12.1

Playwright browsers: D:\ms-playwright
VS Build Tools:      C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools
VS Build version:    17.14.36717.8 / 17.14.21
```

保留的一个细节：当前已运行的 Codex/PowerShell 进程仍可能解析到旧 `D:\nodejs` 和 WSL `bash.exe`，因为进程 PATH 在启动时已经固定。但用户级 PATH 已经把这些目录放到前面：

```text
D:\nodejs-24.15.0
D:\Rust\cargo\bin
D:\Git\cmd
D:\Git\bin
D:\Git\usr\bin
```

新开的 PowerShell、VS Code terminal 或重新加载后的开发环境应优先使用新的 Node、Rust 和 Git Bash。npm 的 `script-shell` 已设置为：

```text
D:\Git\bin\bash.exe
```

因此 `npm run build:engine-wasm`、`npm run verify` 这类调用 `bash scripts/*.sh` 的脚本会走 Git Bash，而不是 WSL bash。

### Git、VS Code 和 Windows 文件系统处理

已设置 Git：

```powershell
git config --global core.autocrlf false
git config --global core.eol lf
git config core.filemode false
```

目的分别是：

- 避免 Windows 自动把 LF 改成 CRLF，导致跨环境 diff 噪音。
- 避免 NTFS/WSL 挂载导致大量 `100644 => 100755` 的 file mode 假修改。
- 让 Windows 目录 `D:\chemcore` 成为主开发目录，而不是继续通过 `\\wsl$` 编辑旧 WSL 目录。

VS Code 内置 Git 功能可直接显示跟踪状态，不需要额外插件。已更新 `%APPDATA%\Code\User\settings.json`：

```json
{
  "git.path": "D:\\Git\\cmd\\git.exe",
  "git.enabled": true
}
```

可选插件只有 GitLens，不是 Git 跟踪的必要条件。若 VS Code 仍看不到 Git 状态，应确认打开的是 `D:\chemcore` 本地目录，而不是 WSL remote 窗口，并重新加载 VS Code。

Windows 复制过来的 WSL metadata 还产生了私有用区冒号文件名，例如 `*Zone.Identifier` 和 `*mshield`。今天已清理 `compare/` 下这类未跟踪文件，并在 `.gitignore` 增加规则：

```text
*Zone.Identifier
*mshield
```

### 依赖安装与验证结果

已运行 `npm install`。npm 11 会把根 package 的 `name`、`version`、`license` 写入 `package-lock.json`，因此 lockfile 出现了小幅元数据变化。安装过程中出现过旧 `.bin` 临时 symlink 清理警告，但不阻塞安装和后续验证。

已完成的验证：

```text
cargo test
npm test
npm run build:engine-wasm
node --check viewer/app.js
node --check viewer/document_flow.js
```

`cargo test` 通过结果包括：

- library unit tests: 44 passed
- `bond_tool`: 141 passed
- `render_document`: 98 passed, 2 ignored
- `text_tool`: 36 passed
- doc-tests: 0

`npm test` 通过，内容是：

```text
cargo test && node --check viewer/app.js
```

`npm run build:engine-wasm` 通过。Windows 原生重建后 `viewer/engine/chemcore_engine_bg.wasm` 发生变化，重新构建两次得到稳定 SHA-256：

```text
BC3A87EF5A2310D622F6E58D44DB351C002348A2205D527FBDDF8A52036EA62C
```

这说明差异不是随机输出，而是 Windows 原生工具链重建后的稳定生成物差异。`npm run verify` 的测试、WASM build 和 JS syntax check 均通过，最后在 generated artifact sync check 阶段因 `viewer/engine/chemcore_engine_bg.wasm` 与仓库旧生成物不同而失败。后续提交时应把该 WASM 变化作为 Windows-native rebuild artifact 一并评审。

### 本地后台和 viewer 状态

已从 `D:\chemcore` 启动本地静态文件服务：

```text
URL:      http://127.0.0.1:8766/viewer/
Process: 8632
Runtime: C:\Python314\python.exe
```

选择 `8766` 是因为 `8765` 已有其他监听进程。`Invoke-WebRequest http://127.0.0.1:8766/viewer/` 返回 `200 OK`。

### 保存问题修复

用户在 viewer 中发现不能保存。排查路径：

- 顶部保存按钮在 `viewer/index.html` 中是 `data-command="save"`。
- `viewer/editor_bindings.js` 将该按钮绑定到 `saveCurrentDocumentAs`。
- `viewer/document_flow.js` 中 `saveCurrentDocumentAs` 使用 `CHEMCORE_TEXT_EXTENSION` 生成 `.ccjs` accept list，但文件顶部没有导入该常量。

直接错误：

```text
ReferenceError: CHEMCORE_TEXT_EXTENSION is not defined
```

修复：

- 在 `viewer/document_flow.js` 导入 `CHEMCORE_TEXT_EXTENSION`。
- 调整 `saveCurrentDocumentNative`、`saveCurrentDocumentCdxml`、`saveCurrentDocumentSvg` 的顺序：在支持 `showSaveFilePicker` 的浏览器中，先打开保存框，再生成、导出或压缩内容。
- 更新 `viewer/index.html` 中 `app.js` 的查询版本号为 `20260505-savefix`，减少浏览器缓存旧脚本的概率。

验证：

```text
node --check viewer/document_flow.js
node --check viewer/app.js
```

并用 Playwright 注入假的 `showSaveFilePicker` 后点击保存按钮，确认保存链路成功写出内容：

```json
{
  "suggestedName": "Untitled.ccjz",
  "write": {
    "constructorName": "Uint8Array",
    "byteLength": 524
  },
  "closed": true
}
```

### 今日工作树变化

截至本日志写入前，今日新增或修改的主要文件是：

```text
.gitignore
package-lock.json
viewer/document_flow.js
viewer/engine/chemcore_engine_bg.wasm
viewer/index.html
docs/developer-log-2026-05-05.zh-CN.md
docs/developer-log-2026-05-05.en.md
```

`HANDOFF-2026-05-05.md` 的内容已并入本日志，并已从根目录删除。

各项变化含义：

- `.gitignore`：忽略 Windows/WSL 复制产生的私有用区冒号 metadata 文件。
- `package-lock.json`：npm 11 写入根 package 元数据。
- `viewer/document_flow.js`：修复保存未导入常量，并把保存框调用提前到内容生成前。
- `viewer/index.html`：更新 `app.js` cache-busting query。
- `viewer/engine/chemcore_engine_bg.wasm`：Windows-native WASM rebuild 后的稳定生成物变化。
- `docs/developer-log-2026-05-05.zh-CN.md`：本中文开发者日志。
- `docs/developer-log-2026-05-05.en.md`：本英文开发者日志。

### 后续注意事项

Windows 主开发应直接使用 `D:\chemcore`。旧 WSL 目录可以作为参考备份，但不建议双线开发。切换环境前应提交或明确备份未提交改动。

如果 Windows 上出现大量行尾差异，应先停止开发，确认 Git 的 `core.autocrlf=false` 和 `core.eol=lf` 后再继续。如果出现大量 file mode 差异，应确认仓库本地 `core.filemode=false`。

如果 `npm run verify` 只因为 `viewer/engine` 生成物不同而失败，应先确认 Rust 测试、WASM 构建和 JS 语法检查都通过，再决定是否提交生成物。

`.ccjz` 依赖浏览器 `CompressionStream` / `DecompressionStream`。现代 Chromium 可用；未来如果引入 Tauri/WebView2，需要确认 WebView2 支持情况，或者在 Tauri 后端提供压缩读写。

CDXML/SVG 保真仍应继续用 `tmp/` 和 `compare/` 下 fixture 做导入、保存、再导入和 SVG 对比，重点检查对象数、类型、颜色、文本 run、结构 label、箭头参数、相对位置、线宽、字体大小和 SVG primitive。

未来桌面壳推荐继续走 Tauri：第一阶段只做窗口、菜单、快捷键、文件打开/另存为、文件关联、最近文件、拖拽打开和打包安装；不要把化学编辑逻辑迁入壳层。第二阶段再考虑 Tauri 后端直接调用 Rust engine、原生压缩读写、缩略图和系统集成。
