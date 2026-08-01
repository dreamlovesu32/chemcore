# 公共 CDX/CDXML 视觉门禁身份规则

公共图集不是一组可以脱离内核版本长期复用的截图。每次候选图生成都必须记录并绑定：

- 仓库 `HEAD` 与完整工作区状态指纹；
- 实际执行的 `chemsema-cli` 二进制 SHA-256；
- CLI 编译时内嵌的仓库状态身份；该身份必须与当前仓库状态完全一致；
- `benchmarks/public-cdxml/manifest.json` 的 SHA-256；
- 每个固定公共语料仓库的实际 revision；
- 本轮三代往返报告的 SHA-256。

每个图集条目还单独记录生成它的仓库状态指纹和 CLI SHA-256。增量生成只能更新被重新绘制的条目，不能把旧条目冒充为当前内核产物。

## 强制行为

1. 规范全量图集默认拒绝脏工作区。开发中的临时验证必须显式传入 `--allow-dirty`。
2. 视觉门禁默认拒绝缺少身份信息、仓库状态变化、CLI 被替换、语料 revision 变化、语料清单变化或往返报告变化的图集。
3. 三代往返报告本身必须记录仓库身份、CLI 内嵌构建身份与 CLI SHA-256；图集生成会再次核对三者，不能把旧报告接到新 CLI 上。
4. 全量门禁要求所有选中条目都由当前仓库和当前 CLI 生成。只更新部分条目后，不能产生看似覆盖全部案例的绿色报告。
5. `--allow-stale-gallery` 仅用于调查历史产物，不能用于发布结论；报告仍保留图集身份，便于追溯。
6. 三代往返门禁强制关闭 CLI 导入缓存；每一代都必须由当前解析器重新读取，不能把历史 CCJS 当作当前解析结果。
7. 产品 CLI 的导入缓存键包含源码构建身份；同一版本号的不同源码构建不能共享解析缓存。
8. ChemDraw oracle 可以缓存，因为它由固定源文件和固定 ChemDraw 行为产生；ChemSema 候选图不能跨内核身份冒充当前结果。
9. 门禁报告必须同时匹配当前 report schema、case-metrics schema、cache identity 和完整策略哈希。旧报告不能通过“补盖”哈希继续使用，必须由当前门禁重新分析原候选图。
10. 严格下限迁移的冻结报告必须来自旧下限记录的确切仓库提交与仓库身份；默认不得减少旧的累计通过集合或最低通过数。门禁定义纠正导致的旧假阳性只能通过已提交的 gate-definition retirement 清单逐项退役，迁移同时记录该清单的内容哈希；清单不会参与渲染或日常判定。
11. 每个可比较案例都必须包含 coarse 与 detail 两层的压缩空间下限。字段缺失、损坏、尺度不符、重复或乱序均视为报告损坏，不能被当成零缺陷。
12. 门禁定义升级时，只能用 `--gate-definition-upgrade --report-only` 生成迁移诊断报告。该入口强制选择完整 original-338，禁止基线、缓存和部分筛选；一旦已提交下限与当前定义一致，该入口立即拒绝运行。它不能替代迁移脚本或发布门禁。
13. 已验证的渲染规则纠正若移动了红图中的残余差异，迁移仍须由已提交的 renderer-migration 审核精确绑定前后仓库身份、规则证据和每张候选图哈希。审核集合必须与连续回归集合完全相等；日常门禁不读取该文件，也不改变任何阈值。

## 空间回归身份

门禁的空间身份使用 ChemDraw 参考坐标，不使用画布百分比：

- coarse 窗口为 48×48、步长 24；detail 窗口为 24×24、步长 12；
- 通过判定仍使用重叠固定窗口；回归下限则只遍历每个 tile 的唯一 core 像素，把它按全局坐标唯一归入 24×24 的 coarse 缺陷格或 12×12 的 detail 缺陷格；所有格像素数之和必须等于该分析层的全局 mismatch 总数；
- 每个非空缺陷格保存 missing/extra 两个方向的精确占用位图，并以规范的小端记录压缩进 JSON；
- 历史缺陷像素消失属于改善；当前像素若无法在固定绝对容差内找到历史同类像素支持，就计入不可抵消的新增缺陷总量；因此跨格拆成许多小错误、同格内保持统计量不变的重排都不能逃逸；
- 原始记录保存 SHA-256、解码长度、压缩长度和两方向像素总数；解压有固定上限，并校验顺序、唯一性、分析尺度、步长、cell 边界、报告 domain 和分析层总数。zlib 字节只用于存储，不参与跨运行时的 canonical 身份。

因此，一处旧缺陷改善不能抵消另一处新增缺陷，同一个局部错误也不会因为外面增加空白或无关对象而更容易通过。

## 标准全量流程

```powershell
node scripts/build-public-cdxml-cli.mjs
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
