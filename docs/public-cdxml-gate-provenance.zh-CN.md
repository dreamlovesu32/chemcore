# 公共 CDX/CDXML 视觉门禁身份规则

公共图集不是一组可以脱离内核版本长期复用的截图。每次候选图生成都必须记录并绑定：

- 仓库 `HEAD` 与完整工作区状态指纹；
- 实际执行的 `chemsema-cli` 二进制 SHA-256；
- `benchmarks/public-cdxml/manifest.json` 的 SHA-256；
- 每个固定公共语料仓库的实际 revision；
- 本轮三代往返报告的 SHA-256。

每个图集条目还单独记录生成它的仓库状态指纹和 CLI SHA-256。增量生成只能更新被重新绘制的条目，不能把旧条目冒充为当前内核产物。

## 强制行为

1. 规范全量图集默认拒绝脏工作区。开发中的临时验证必须显式传入 `--allow-dirty`。
2. 视觉门禁默认拒绝缺少身份信息、仓库状态变化、CLI 被替换、语料 revision 变化、语料清单变化或往返报告变化的图集。
3. 全量门禁要求所有选中条目都由当前仓库和当前 CLI 生成。只更新部分条目后，不能产生看似覆盖全部案例的绿色报告。
4. `--allow-stale-gallery` 仅用于调查历史产物，不能用于发布结论；报告仍保留图集身份，便于追溯。
5. ChemDraw oracle 可以缓存，因为它由固定源文件和固定 ChemDraw 行为产生；ChemSema 候选图不能跨内核身份冒充当前结果。

## 标准全量流程

```powershell
cargo build --release -p chemsema-cli
node scripts/public-cdxml-roundtrip.mjs `
  --root tmp/public-corpus-pilot `
  --out-dir tmp/public-cdxml-roundtrip-current `
  --strict-counts `
  --cli target/release/chemsema-cli.exe
node scripts/render-public-cdxml-visual-review.mjs `
  --all --jobs 4 `
  --root tmp/public-corpus-pilot `
  --report tmp/public-cdxml-roundtrip-current/report.json `
  --out tmp/public-cdxml-chemdraw-review-all `
  --cli target/release/chemsema-cli.exe
node scripts/public-cdxml-visual-gate.mjs `
  --jobs 4 `
  --gallery tmp/public-cdxml-chemdraw-review-all `
  --out tmp/public-cdxml-chemdraw-review-all/gate-current.json
```

发布结论必须来自未使用 `--allow-dirty`、`--allow-stale-gallery` 或 `--report-only` 绕过失败状态的完整运行。
