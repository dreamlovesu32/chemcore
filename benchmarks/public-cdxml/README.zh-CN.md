# 公共 CDXML/CDX 往返基准集

这个基准使用公开、许可证清楚的 ChemDraw CDXML/CDX 文件，避免让保密科研文档成为
公开测试和论文结论不可替代的依据。上游文件下载到 Git 忽略的 `tmp/` 目录，不直接
vendoring 到 ChemSema 仓库。

当前固定版本的清单包含五个上游项目、共 413 个文件：

| 来源 | 许可证 | CDXML | CDX | 主要覆盖 |
| --- | --- | ---: | ---: | --- |
| RDKit | BSD-3-Clause | 94 | 126 | 解析回归、query、模板和专利结构 |
| Indigo | Apache-2.0 | 123 | 28 | 分子、反应、渲染和异常输入测试 |
| cdxml-toolkit | MIT | 34 | 2 | 完整线性、换行和分支反应路线图 |
| SAMPL6 | MIT | 1 | 2 | 已发表的主客体结构 |
| SAMPL9 | MIT | 2 | 1 | 已发表的主客体结构 |

其中两个文件是故意构造的异常输入；另有四个 `.cdx` 实际保存 Base64 传输文本，并非
原始 CDX 字节，因此单独分类。其余 407 个文件作为正向往返案例。其中一个故意损坏坐标
的 fixture 分类为安全清洗，两个只移除未使用图形样式的 fixture 分类为无损归一化。

## 复现方法

```bash
npm run benchmark:cdxml-public:fetch
cargo build -p chemsema-cli
npm run benchmark:cdxml-public
```

如需为语料中的全部文件生成 ChemDraw 与 ChemSema 肉眼审图集，运行：

```bash
node scripts/render-public-cdxml-visual-review.mjs --all \
  --root tmp/public-corpus-pilot \
  --report tmp/public-cdxml-roundtrip-label-audit/report.json \
  --out tmp/public-cdxml-chemdraw-review-all
```

审图集把两侧图像统一映射到 ChemDraw 参考图坐标系。ChemDraw SVG 的全局 `matrix` 明确给出二十分之一点坐标到参考图像素的比例，因此门禁固定使用该等比缩放；绝对页面原点不是可移植语义，门禁以双方外层墨迹包围盒中心为起点，只搜索全局平移，不拟合缩放、旋转或非等比变换。基线建立后，同一 ChemDraw 参考图的后续候选必须沿用基线平移，局部绘制修改不能重新配准整页。门禁使用 SVG 声明的可含小数 `width`/`height`，不使用浏览器分别取整的 `naturalWidth`/`naturalHeight`，避免候选图发生隐含的非等比拉伸。只有缺少矢量尺度的栅格参考图才明确进入同时搜索缩放和平移的墨迹配准分支。
判定、备注、当前图片、显示模式、透明度和框选模式都会随操作实时保存到浏览器本地存储。在任一侧
框选的区域均以参考图坐标保存并同步显示到两侧，同时立即把该图片标记为“有问题”。切换图片或
重新打开审图集后，框选模式仍保持开启。

审图集只用于定位和解释差异，不是发布门禁。自动像素门禁直接使用其中缓存的 ChemDraw oracle
和已经配准的 ChemSema 渲染：

```bash
npm run benchmark:cdxml-public:visual-gate
# 只生成当前基线报告，不以非零退出码阻断命令：
npm run benchmark:cdxml-public:visual-gate:report
```

门禁对每个可比较文档等权计票，不受画布或文件尺寸影响，空白画布像素完全不进入评分。粗粒度阶段检查固定尺寸局部窗口以及缺失/多余墨迹的连通分量；细粒度阶段继续检查连通对象数量、细小符号尺寸和重复的紧凑微缺陷（例如虚线键端点斜接断开）。复杂多对象图的连通分量数量和归一化位置分布只作为归因信息，不能覆盖局部空白窗口、固定跨度缺陷或覆盖率失败。所有阈值都使用 ChemDraw 参考坐标或归一化结构坐标，因此小标签、正负号或键细节缺失不会被大分子、反应路线图或大页面稀释。JSON 报告会给出参考坐标中的缺陷框和明确的原因码；没有真实 ChemDraw oracle 的案例单独报告，不进入通过率分母。每次运行门禁还会在完整审图集旁
生成只含通过案例的 `passed.html`；使用 `--reuse-report report.json` 可以直接从已有报告重建
该页面，无需重新执行像素分析。

### 增量视觉门禁

日常渲染修复不再默认重跑全部 413 个文件。增量门禁参考 OCR 仓库的 affected-gate 约定，先把当前代码改动映射到视觉规则族，再从机器生成的 `tmp/public-cdxml-feature-index.json` 选择同类文件和历史回归样例。选择计划写入 `tmp/public-cdxml-affected-gate-plan.json`，不能用手写编号列表替代；额外诊断样例通过 `--extra` 进入计划并保留理由。

第一次把既有全量报告用作缓存基线时，只需为它记录参考图和候选 SVG 的内容哈希：

```bash
node scripts/public-cdxml-visual-gate.mjs \
  --gallery tmp/public-cdxml-chemdraw-review-all \
  --stamp-report tmp/public-cdxml-chemdraw-review-all/gate-report.json
```

普通开发循环先检查计划，再运行受影响门禁：

```bash
npm run benchmark:cdxml-public:visual-gate:affected -- --dry-run
npm run benchmark:cdxml-public:visual-gate:affected
```

规划器会增量更新完整图集的对应条目。像素门禁按“ChemDraw 参考图哈希 + ChemSema SVG 哈希 + 门禁策略版本”复用未变化案例，只分析真正改变的图片；最终报告仍包含完整基线的全部案例，并在 `cache.reused`/`cache.analyzed` 中记录复用和重算数量。基线模式允许历史红图继续留待后续修复，但任何旧绿转红都会写入 `delta.regressions` 并让命令失败。代码路径到特征族及历史回归样例的映射保存在 `benchmarks/public-cdxml/visual-impact-map.json`。未登记的生产代码改动会保守地强制全量，门禁算法本身变化也必须全量验证。

严格的原始 338 张门禁还会读取 `benchmarks/public-cdxml/strict-pass-floor.json`。这份受版本控制的通过下限保存所有已验收通过图片的累计并集，因此即使误选了一份已经退化的基线报告，也不能把更早的绿图从历史中洗掉。严格门禁会先拒绝已经丢失受保护绿图的基线，并独立检查本次结果是否丢失任一受保护绿图。一次干净的严格门禁新增通过且没有任何回退后，只提升新增项：

```bash
npm run benchmark:cdxml-public:visual-gate:promote -- \
  --report tmp/public-cdxml-visual-gate-current-strict338.json
```

提升命令会核对完整 338 张集合、干净且当前一致的仓库和 CLI 来源、零分析错误、零即时回退以及零累计回退。它只会把新增通过项并入下限，绝不会删除已经受保护的路径。

v12 门禁不再允许等价分支绕过固定坐标的细节检查：整页覆盖率和拓扑分布都不能掩盖局部空白窗口或大块局部缺陷，粗粒度像素相似也不能删除细粒度连通组件不一致。细节层使用零膨胀，保留亚像素键交汇差异；重复缺陷阈值使用参考图坐标单位，不随画布大小稀释。SVG 使用声明的固定矢量比例与基线锁定平移，局部 glyph 或键型变化不能再推动整页重新拟合缩放与位移。正式门禁会拒绝仓库、CLI、语料、往返报告或逐图候选来源不一致的图集；`--allow-stale-gallery` 只供诊断，不能把旧图变成正式结果。

使用 `--cohort original-338` 可严格运行 `benchmarks/public-cdxml/failure-ledger.json` 中登记的原始 338 张审查集合。只要图库缺少其中一个路径，门禁会在像素分析前失败；报告会记录集合名称、清单路径、期望数量和实际选择数量。

如果后续全量门禁发现增量计划漏掉的回归，应先补充影响映射或特征提取，让同类样例以后能被自动选中，再修绘制规则；不能只把漏网文件手工加进一次性命令。

可用 `CHEMSEMA_PUBLIC_CDXML_DIR` 修改下载目录。详细报告写入未跟踪的
`tmp/public-cdxml-roundtrip/report.json`。默认会对每个正向案例连续保存并重新打开三代，
每一代同时检查分子、箭头身份、括号几何、原子标签和自由文本的语义指纹，以及对象、
资源、样式和对象类型计数。文本门禁会比较源文本与显示文本、行结构、样式段、对齐、
锚点、换行宽度、行高和标签/文本几何。语义漂移和非幂等始终会让命令失败；传入
`--strict-counts` 后，已分类的计数漂移也会失败。

当前 ChemSema 1.0.0-beta.1 源码基线没有未预期失败、语义漂移、非幂等或未分类计数
漂移。413 个文件中，404 个连续三代完全一致，1 个是预期的安全清洗，2 个是预期的无损
归一化，2 个按预期拒绝导入，4 个传输编码文件跳过。语义门禁覆盖元素身份与电荷、分子
连接关系、无头箭头身份、括号分组与几何、原子标签实现和自由文本布局；计数门禁则独立
捕获对象和资源增长。

清单固定每个上游 commit，并记录许可证链接。语料变化时应更新清单、重新运行基准并提交
新的版本化 summary，而不是静默覆盖旧基线。

表中的许可证是各上游仓库公开声明的仓库许可证。下载器让文件继续留在原上游仓库中，
适合做可复现的外部基准；如果以后要把这些文件重新打包成独立数据集，还应逐文件复核
来源和署名要求，尤其是 RDKit 中源自专利的 fixtures。
