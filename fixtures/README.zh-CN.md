# 公开 Fixture

这个目录只放可公开、版权边界清楚的回归测试资产。

- `cdxml/` 保存 synthetic CDXML 源文件。
- `expected/svg/` 保存从这些 CDXML 文件生成的 ChemCore SVG golden snapshots。

Fixture 应尽量小，每个文件只覆盖一两个行为，例如标签裁剪、箭头几何、对象堆叠或文本布局。提交进仓库的 fixture 应可共享，并适合作为开源回归测试资产。

重新生成单个 SVG 快照：

```bash
cargo run -p chemcore-engine --example cdxml_to_svg -- fixtures/cdxml/synthetic-reaction.cdxml fixtures/expected/svg/synthetic-reaction.svg
```
