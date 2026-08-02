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

审图集把两侧图像统一映射到 ChemDraw 参考图坐标系。ChemDraw SVG 的全局 `matrix` 明确给出二十分之一点坐标到参考图像素的比例，因此门禁固定使用该等比缩放；绝对页面原点不是可移植语义，门禁以双方外层墨迹包围盒中心为起点，只搜索全局平移，不拟合缩放、旋转或非等比变换。平移搜索固定在文档世界坐标格点上：候选 SVG 的 `viewBox` 只是导出裁剪框，改变其原点或范围只能改变显示坐标平移，不能改变文档世界配准。分块和局部窗口也固定锚定在 ChemDraw 参考坐标，而不是跟随候选画布边缘移动。每个当前候选都依据自身像素重新配准，历史平移不会覆盖当前证据。门禁使用 SVG 声明的可含小数 `width`/`height`，不使用浏览器分别取整的 `naturalWidth`/`naturalHeight`，避免候选图发生隐含的非等比拉伸。只有缺少矢量尺度的栅格参考图才明确进入同时搜索缩放和平移的墨迹配准分支。
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

需要建立新的正式图集目录时，应复用已经审定的 ChemDraw 输出，不要把全部源文件再次
静默另存：

```bash
node scripts/render-public-cdxml-visual-review.mjs --all \
  --oracle-gallery tmp/public-cdxml-previous-clean \
  --out tmp/public-cdxml-current-clean
```

`--oracle-gallery` 要求语料清单和各上游 revision 完全一致，并要求每个可比较输入都有
保留的参考图。参考图集不完整或不兼容时命令会明确失败，不会混用新旧 ChemDraw
基准继续运行。

门禁对每个可比较文档等权计票，不受画布或文件尺寸影响，空白画布像素完全不进入评分。粗粒度阶段检查固定尺寸局部窗口以及缺失/多余墨迹的连通分量；细粒度阶段继续检查连通对象数量、细小符号尺寸和重复的紧凑微缺陷（例如虚线键端点斜接断开）。复杂多对象图的连通分量数量和归一化位置分布只作为归因信息，不能覆盖局部空白窗口、固定跨度缺陷或覆盖率失败。所有阈值都使用 ChemDraw 参考坐标或归一化结构坐标，因此小标签、正负号或键细节缺失不会被大分子、反应路线图或大页面稀释。候选 SVG 还要独立执行画布自洽检查：ChemSema 导出器规定墨迹外有 8 pt 边距，门禁以固定 4 pt 下限检查四边实际墨迹距离；任何贴边或被根 viewport 裁切的内容都会给出 `candidate-viewport-ink-margin`，不能被整图相似度或大画布稀释。JSON 报告会给出参考坐标中的缺陷框和明确的原因码；没有真实 ChemDraw oracle 的案例单独报告，不进入通过率分母。每次运行门禁还会在完整审图集旁
生成只含通过案例的 `passed.html`；使用 `--reuse-report report.json` 可以直接从已有报告重建
该页面，无需重新执行像素分析。

### 增量视觉门禁

日常渲染修复不再默认重跑全部 413 个文件。增量门禁参考 OCR 仓库的 affected-gate 约定，先把当前代码改动映射到视觉规则族，再从机器生成的 `tmp/public-cdxml-feature-index.json` 选择同类文件和历史回归样例。选择计划写入 `tmp/public-cdxml-affected-gate-plan.json`，不能用手写编号列表替代；额外诊断样例通过 `--extra` 进入计划并保留理由。

基线必须由当前门禁定义重新分析生成；不能通过替换哈希或来源信息让旧判定重新可信：

```bash
node scripts/public-cdxml-visual-gate.mjs \
  --gallery tmp/public-cdxml-chemdraw-review-all \
  --out tmp/public-cdxml-chemdraw-review-all/gate-report.json
```

普通开发循环先检查计划，再运行受影响门禁：

```bash
npm run benchmark:cdxml-public:visual-gate:affected -- --dry-run
npm run benchmark:cdxml-public:visual-gate:affected
```

规划器会增量更新完整图集的对应条目。像素门禁按“ChemDraw 参考图哈希 + ChemSema SVG 哈希 + 门禁策略版本”复用未变化案例，只分析真正改变的图片；最终报告仍包含完整基线的全部案例，并在 `cache.reused`/`cache.analyzed` 中记录复用和重算数量。基线模式允许历史红图继续留待后续修复，但任何旧绿转红都会写入 `delta.regressions` 并让命令失败；仍为红图的案例会逐坐标保护 coarse 与零容差 detail 的 missing/extra 精确占用掩膜。当前缺陷像素只有在固定绝对容差内得到历史同类像素支持才不算新增；所有格中的未支持像素会累计，因此任何旧位置的改善都不能抵消新位置错误。新增失败原因、受保护指标或掩膜消失、旧缺陷超容差换位置都会写入 `delta.continuousRegressions`。确有取舍时必须人工复核并显式提升基线，不能静默放行。固定参考单位窗口、占用掩膜和绝对容差保证结果不会被画布大小稀释。代码路径到特征族及历史回归样例的映射保存在 `benchmarks/public-cdxml/visual-impact-map.json`。未登记的生产代码改动会保守地强制全量，门禁算法本身变化也必须全量验证。

严格的原始 338 张门禁还会读取 `benchmarks/public-cdxml/strict-pass-floor.json`。这份受版本控制的全案例下限绑定一个精确门禁定义，保存全部 338 条路径：既包含所有已验收通过图片的累计并集，也包含每张剩余红图的状态、ChemDraw 原图哈希、失败原因和非退化指标。严格模式直接与这份仓库内基线比较；外部报告只能用作分析缓存，不能替换回归历史。因此，退化后的当前报告无法把旧回退洗掉，ChemDraw 原图变化、指标消失、脏/旧/局部图库以及“某处改善掩盖另一处恶化”也都会被拒绝。一次干净的严格门禁新增通过或改善红图且没有任何回退后，再提升全案例下限：

```bash
npm run benchmark:cdxml-public:visual-gate:promote -- \
  --report tmp/public-cdxml-visual-gate-current-strict338.json
```

提升命令会核对完整 338 张集合、干净且当前一致的仓库和 CLI 来源、零分析错误、零即时回退以及零累计回退。它会把新增通过项并入下限，并把仍为红图的案例收紧到本次已改善指标；绝不会删除已经受保护的路径。

门禁定义本身被纠正时，不能静默沿用旧 verdict。必须用新定义分别重算一份冻结的旧候选图集和当前图集；两份报告都必须精确覆盖相同 338 条路径并使用相同 ChemDraw oracle。若新定义证明旧通过项是假阳性，每条被退役路径都必须写入已提交、排序稳定且带统一原因的审核清单；清单以外仍不允许同口径旧绿转红：

```bash
node scripts/public-cdxml-visual-gate.mjs \
  --gallery tmp/frozen-candidate-gallery \
  --gate-definition-upgrade --report-only --allow-stale-gallery --jobs 8 \
  --out tmp/frozen-candidate-gate-report.json
```

`--gate-definition-upgrade` 是跨越旧定义下限的唯一引导入口：它强制选择完整 original-338，只生成诊断报告，并禁止基线、缓存和局部筛选；一旦已提交下限与当前定义一致，该入口就会拒绝运行。

```bash
npm run benchmark:cdxml-public:visual-gate:migrate-floor -- \
  --previous-report tmp/frozen-candidate-gate-report.json \
  --current-report tmp/current-candidate-gate-report.json \
  --reviewed-retirements benchmarks/public-cdxml/gate-definition-retirements-v22.json \
  --reviewed-renderer-migration benchmarks/public-cdxml/renderer-migrations/hash-bond-v2.json
```

迁移会把冻结报告绑定到旧下限记录的确切仓库身份，并记录两份报告哈希、同口径变化和退役清单哈希。未列出的旧通过项和清单之外的下限缩减仍会被拒绝。该清单只审计门禁定义纠正，渲染器和普通通过判定不会读取它，因此不是绘制特例。

经过探针验证的渲染规则纠正，可能让仍为红图的案例内部残余差异像素发生移动；普通门禁仍必须报告这些变化。若要让新下限接管，只能再提供一份已提交的渲染迁移审核：它绑定前后仓库身份、统一规则和探针证据，以及每张受影响红图的新旧候选图哈希。每张图还必须在历史固定配准下，分别保证 coarse 和 detail 两层的 missing+extra 差异总量不增加；另一层或另一张图的改善不能抵消这一层哪怕一个新的未解决像素。排序后的案例集合必须与连续回归完全相等，不能藏住无关的第七张回退、不能允许旧绿转红，也不会放宽后续门禁。该文件是迁移审计记录，不是运行时文件特例。

v22 对每一个可比较案例都运行 coarse 和零容差 detail 两层分析，包括本来已经严重失败的红图。每个 core 像素只按全局 ChemDraw 坐标分桶一次，门禁把固定缺陷格中 missing/extra 的精确占用掩膜压缩写入受保护下限。当前像素必须在固定绝对容差内由历史同类像素支持；所有格的未支持像素统一累计，所以旧缺陷修好不能补偿任何位置的新缺陷，同一格中保持数量、质心和外框不变的重排也无法逃逸。原始掩膜记录用 SHA-256 校验完整性，zlib 只负责存储，不把不同运行时的压缩字节当成逻辑身份。通过判定只有一条路径：固定窗口 coarse 阈值与 detail 规则必须同时通过；旧的整页覆盖率和拓扑等价宽松分支已退役，同一个局部缺陷不会因为画布变大而更容易通过。细节层使用零膨胀，保留亚像素键交汇差异；重复缺陷阈值使用参考图坐标单位，不随画布大小稀释。只有缺失块与多余块同时位于一个固定细节窗口的距离内，才会被认定为同一细节发生位移；画布两端外形相似但互不相关的字形或键边缘不能互相配对。SVG 使用声明的固定矢量比例；每个当前候选图都独立执行文档世界坐标上的多分辨率最大整体重叠平移，分块和局部窗口锚定在固定参考格点，根 `viewBox` 改变不会移动配准或采样窗口。历史通过保护只比较同一门禁定义的结果，不会把旧候选图的平移量注入新图。正式门禁会拒绝仓库、CLI、语料、往返报告或逐图候选来源不一致的图集；`--allow-stale-gallery` 只供诊断，不能把旧图变成正式结果。图库生成器和影响面规划器默认使用 `build-public-cdxml-cli.mjs` 生成的发布版 CLI，不会静默选用旧调试版。`--allow-dirty` 只用于明确的开发诊断：它会记录脏仓库，并允许以此前干净的往返报告为起点做增量重绘；正式门禁仍会拒绝该结果，除非同时显式使用仅供诊断的脏/旧图集开关。

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
