# 渲染对比与回归资产

ChemCore 把渲染保真度作为工程目标处理。公开测试资产分成两类：

- `fixtures/cdxml/` 中的 synthetic fixtures，用于可复现、适合 CI 的回归测试；
- 仓库根目录下由维护者绘制的真实论文图 benchmark：`figure1.cdxml` 和 `figure2.cdxml`，用于高信号视觉对比。

这些 benchmark CDXML 文件由维护者本人绘制，并随仓库一起提供，用于可复现的渲染对比。新的自动化测试默认应优先使用 synthetic fixtures，因为它们更小、版权边界清楚，也更容易在发现回归时化简。

## Golden SVG 快照

Golden SVG 快照放在 `fixtures/expected/svg/`。Rust 测试会读取每个
`fixtures/cdxml/*.cdxml`，通过内核导入后导出 SVG，并与同名 expected SVG 文件比较。

运行快照测试：

```bash
cargo test -p chemcore-engine public_cdxml_fixture_svg_golden_snapshots_match --test render_document
```

当渲染规则有意变化并影响 fixture 时，用下面的命令重新生成对应 SVG：

```bash
cargo run -p chemcore-engine --example cdxml_to_svg -- fixtures/cdxml/synthetic-reaction.cdxml fixtures/expected/svg/synthetic-reaction.svg
```

提交前需要人工查看 diff。SVG 快照是文本资产，diff 应该清楚显示被修改的 primitive、坐标、颜色和文本输出。

## ChemDraw Oracle 对比

ChemDraw oracle 脚本是可选的本地工具。它们需要 Windows，以及本机可通过 COM 调用的 ChemDraw 安装。

为一个或多个 CDXML 文件生成 ChemDraw SVG/EMF：

```bash
npm run emf:chemdraw-oracle -- --out tmp/chemdraw-oracle figure1.cdxml figure2.cdxml
```

同时生成 ChemDraw 与 ChemCore 的 SVG/EMF、EMF 检查报告和 EMF raster preview：

```bash
npm run emf:compare-oracle -- --out tmp/emf-oracle figure1.cdxml figure2.cdxml
```

README 中的对比资产位于 `docs/assets/readme/comparison/`，由这些输出以及 ChemCore 的 `cdxml_to_svg` 和 Office EMF 写出路径生成。默认 GitHub Actions CI 使用开源、无专有依赖的回归检查；ChemDraw 和 Office oracle 检查保留为本地 Windows 工作流。

## 添加公开 Fixture

请使用 synthetic chemistry、化简布局，或维护者本人绘制且权利边界清楚的文件。公开 fixture 应可共享、足够小，并且权利边界清楚。

推荐 fixture 名称直接描述被测试行为，例如 `label-clipping-basic.cdxml`、`equilibrium-arrow-geometry.cdxml` 或 `orbital-stacking.cdxml`。每个 fixture 都应配套一个 expected SVG 快照后再提交 PR。
