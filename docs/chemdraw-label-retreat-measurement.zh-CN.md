# ChemDraw 字形裁剪与键端退让测量

本文记录真实 ChemDraw 的受控测量方法和当前已经得到的规律。这里的目标不是按
文件名或字符名单修补图像，而是建立可重复、可留出验证的几何模型。

## 测量方法

`scripts/chemdraw-label-retreat-probe.mjs` 自动生成 CDXML 探针页，通过后台 COM
调用 ChemDraw 导出 SVG 和规范化 CDXML，然后直接读取 SVG 中键身路径的可见端点。
测量不依赖截图、画布大小或 computer-use。

每个探针记录：

- 字形、字体、字号和 face；
- `MarginWidth`、`LineWidth`；
- 键相对标签节点的角度；
- 原始键长、可见键长和端点退让量，单位均为 pt。

可执行的测量档位为：

```powershell
npm run probe:label-retreat -- --profile smoke
npm run probe:label-retreat -- --profile survey
npm run probe:label-retreat -- --profile fine
npm run probe:label-retreat -- --profile thin
npm run probe:label-retreat -- --profile holdout
npm run probe:label-retreat -- --profile comprehensive --out tmp/chemdraw-label-retreat-comprehensive
npm run probe:label-retreat -- --profile directional --out tmp/chemdraw-label-retreat-directional
npm run probe:label-retreat -- --profile anchored-directional --out tmp/chemdraw-label-retreat-anchored-directional
npm run probe:label-retreat:analyze
```

`thin` 使用 `0.05pt` 键宽，以尽量分离字形排除区和有限宽键身效应；`holdout`
使用训练阶段没有出现过的 margin、线宽和字号。

## 当前样本

基础留出分析包含 33,480 个 ChemDraw 矢量端点：

- survey：9,720；
- 5° 角度扫描：9,720；
- 0.05pt 超细键扫描：6,480；
- 留出验证：7,560。

调查覆盖 Arial 全部 ASCII 字母、数字和常用标点，并对重点字形扫描多个字号、
margin、线宽、Times New Roman、Calibri、粗体和斜体。

扩展覆盖矩阵另含 110,640 个矢量端点。加上逐度方向和字符锚点扫描后，全部档位
合计 213,240 个端点。它不是完整笛卡尔积，
而是由下列相互独立又有交叉的层组成：

- 7 个字号：`6/8/10/12/14/18/24pt`；
- 7 种字体：Arial、Times New Roman、Calibri、Cambria、Segoe UI、Courier New、Symbol；
- 4 种 face：regular、bold、italic、bold italic；
- 12 个 `MarginWidth`：`0–4pt`，包含小数和未参与基础拟合的中间值；
- 8 个 `LineWidth`：`0.05–4pt`；
- 常规矩阵中的 48 个方向角和 120 种标签，包括完整拉丁字母、数字、标点、科学符号以及 `Cl`、`Br`、
  `NH2`、`Fe3+`、`CO2H`、`t-Bu`、希腊字母等化学标签。

扩展档位共 92 个实验。规范化 CDXML 已复核：ChemDraw 确实保留了所请求的字体、
字号、face、margin、line width 和 Unicode 文本，并非只在输入元数据中声明。

同一参数和标签下直接拿其他字体与 Arial 比较，端点退让的平均绝对差为：
Segoe UI `0.36pt`、Times New Roman `0.53pt`、Cambria `0.55pt`、Calibri `0.67pt`、
Symbol `0.93pt`、Courier New `0.97pt`。95 分位最高达到 `3.36pt`；极大值来自某个
笔画组件是否被射线命中的拓扑切换。因此字体不能被简化成只改变文本包围盒。

在 Arial、Times New Roman、Calibri、Cambria 四个字体家族中，bold 相对 regular
的平均差为 `0.12–0.21pt`，italic 为 `0.30–0.40pt`，bold italic 为 `0.32–0.40pt`。
斜体造成的方向性变化明显大于粗体，证明 face 必须进入几何裁剪核，而不能只参与文字绘制。

方向档位另含 43,200 个端点，对 20 个代表性单字符和多字符标签逐 `1°` 扫描完整
`0–359°`，并交叉 `MarginWidth=0/0.75/2.5` 与 `LineWidth=0.05/1`。字符锚点档位
另含 25,920 个端点，对 `NH2`、`Fe3+`、`CO2H`、`(PhO)2POH` 分别设置左端、
内部和右端 `EndAttach` 字符索引，再做相同的逐度扫描。ChemDraw 归一化输出已验证
保留全部 `EndAttach`，因此这不是用文本对齐方式模拟的伪锚点。

字符内部连接不能只报告“退让量”：ChemDraw 会根据目标字符重新定位标签并让键轴转向。
测量结果因此同时保存主轴拟合后的实际线长、相对输入方向的角偏转和固定节点坐标下的
有效端点位移。相对左端锚点，中间锚点的有效端点位移平均绝对差为 `5.62pt`，右端为
`7.16pt`；这证明字符锚点必须在布局和裁剪之前进入模型，事后仅移动文字或修改包围盒不成立。

归一化 CDXML 还给出了稳定的字符锚点布局分支：`EndAttach=0` 使用普通终端标签布局，
来键位于节点右侧时为 `LabelJustification=Right`，来键位于左侧或恰好竖直时为 `Left`；
所有非零字符索引均为 `LabelJustification=Center`，整段文本以节点居中，然后键轴转向目标
字符并对该字符的排除轮廓裁剪。左端档位的 8,640 个样本各有 4,320 个 Left/Right，
中间和右端档位各 8,640 个样本全部为 Center。字符索引和文本 justification 是两个独立输入，
不能互相替代。

## 已确认规律

### 1. 无量纲相似律

对于同一字体、字形和 style，ChemDraw 的退让满足：

```text
R(size, margin, lineWidth, angle)
  = size * F(margin / size, lineWidth / size, angle)
```

用 10pt 数据预测未参与拟合的 14pt 样本，180 个共同角度样本的平均绝对端点误差
为 0.099pt，P95 为 0.483pt。此前对 6/8/12pt 的预测平均误差分别为
0.034/0.031/0.027pt。

因此，运行时模型应先在 `margin/fontSize` 与 `lineWidth/fontSize` 的无量纲空间选择
裁剪核，再整体乘字号。不能先把固定 10pt 多边形映射到字框，再分别缩放所谓
“内部点”和“外部点”。

### 2. margin 不是简单的统一端点加法

圆形字形（如 `O`、`0`）非常接近自然轮廓外扩；但开放笔画、狭窄笔画和分离组件
会发生拓扑切换：小 margin 时键可以穿过空隙，超过临界值后会突然命中远侧组件。
这是真实几何的不连续选择，不能用画布归一化误差或单一包围盒掩盖。

在从未参与拟合的 margin `0.25/0.75/1.25/2.5/3.5pt` 上，对无量纲方向核做分段
插值，2,700 个样本的平均绝对误差为 0.064pt，P95 为 0.286pt。最大误差仍集中在
上述拓扑临界角度，说明正式内核需要保存组件几何或更密的临界区，而不是只保存
稀疏径向数值。

### 3. LineWidth 是有限键身效应，不是第二个 margin

线宽变化主要影响键身擦过笔画边缘或字形空隙的方向；正对外轮廓的端点往往不变。
因此不能完全忽略线宽，也不能把 `lineWidth/2` 统一加到 margin。

留出线宽插值结果：

- `MarginWidth=0`：平均 0.058pt，P95 0.220pt；
- `MarginWidth=2`：平均 0.048pt，P95 0.190pt。

最大误差同样发生在“刚好碰到另一个组件”的阈值处。内核最终应让完整有限宽键身
与排除区求交；三条射线可作为快速近似，但必须用这些阈值案例验证。

### 4. 字体和 style 必须进入裁剪核

Times New Roman、Calibri、Arial 粗体和 Arial 斜体的方向分布均与 Arial regular
不同。只把 Arial 多边形塞入不同字体的 ink box 不能保持 ChemDraw 退让。

### 5. 当前运行时 margin 重映射不是同一几何操作

当前 manifest 只保存 10pt、1pt margin 下已经合并的多边形。运行时再以点到字框
中心的距离，在“自然 outset”和“圆缩放”之间逐点选择；`circle_radius_pt` 并没有
作为独立几何量参与重建。

在 6,480 个超细键样本上，当前 ChemSema 与 ChemDraw 的总体平均端点误差约
0.91pt，P95 约 3.29pt。`MarginWidth=0` 最严重，平均约 2.72pt；非零 margin 的
平均误差约 0.51–0.58pt。根因是已经合并的 canonical polygon 无法可靠反演成
其他 margin 下的组件与临界拓扑。

### 6. 轴向接触是独立于字形膨胀的固定分支

1° 方向扫描显示，水平和垂直方向两侧各存在固定的窄扇区：与坐标轴夹角小于约
`10°` 时，ChemDraw 使用位于字形四个轴向极值处的接触点；离开扇区后立即切换回
真实字形排除区。这个边界不随 `MarginWidth` 改变，也不是文字位置、BoundingBox
或 justification 改变造成的。

轴向接触点分别为：

```text
(xmax + margin, 0)
(xmin - margin, 0)
(0, ymax + margin)
(0, ymin - margin)
```

多字符标签不能先合并成一个总包围框再取上述极值。每个轴向分支只使用与连接轴正交
方向上覆盖该轴的字形排除带：竖轴按 X 区间筛选字形，横轴按 Y 区间筛选字形，区间
筛选和最终极值都包含 `MarginWidth`。公开的 `3060_iso_butane.cdxml` 给出了一条
独立实证：`CH3` 右侧下标 `3` 比 `C` 更低，但位于 `C` 下方的竖键仍在
`191.09pt` 结束；若错误使用整段标签的 `ymax`，端点会退到约 `193.19pt`。
`scripts/chemdraw-multiglyph-axis-contact-probe.mjs` 又做了 31 组静默 ChemDraw
配对探针，覆盖 Arial、Times New Roman、Calibri，8/10/14pt，
`MarginWidth=0/1.6/3pt`、`LineWidth=0.6/1.5pt`、四个轴向以及远处下标/上标。
单独 `C` 与增加远处字形后的标签轴向端点最大差 `0.10pt`，等于 ChemDraw SVG
在这些模板上的坐标量化级别，远小于错误使用整段包围框造成的数 pt 偏差。

扇区内退让是接触点在键方向上的投影。把该分支加到连续 Arial 轮廓模型后，独立
1° 档位的 MAE 从 `0.287pt` 降到 `0.204pt`，P95 从 `1.213pt` 降到
`0.812pt`。因此轴向接触必须作为独立几何层保留，不能靠扩大整个字形模拟。

### 7. 无字符表的凸包特征层

旧模型的固定 `0.22 × glyphHeight` 内移只在默认字号附近成立。Arial 10pt 下该值
约等于常用的 `MarginWidth=1.59pt`；跨 8/14/24pt 后，固定字高内移的最佳倍率会
漂移，不能进入运行时。

通过大 margin 和曲线字体门禁后的规则完全由轮廓、字号和 margin 构造。令
`m = MarginWidth`、`em = fontSize`、`q = min(m, 0.25 × em)`：

1. 真实字形轮廓以欧氏距离自然外扩 `MarginWidth`；
2. 展平真实字形的所有外轮廓，取整体凸包的顶点，不依赖字体轮廓顺序或
   on-curve/off-curve 编码；
3. 顶点沿指向字形中心的方向内移 `0.5 × q`；
4. 以内移点为圆心加入半径 `1.5 × q` 的圆；
5. 与轴向接触分支取并集，再与完整有限宽键身求交。

`q` 的上限很重要：不设上限时，8pt 字号配 2.5/3pt margin 会过度填充；设为
`0.25em` 后，端点特征在大 margin 下自然落入完整的轮廓外扩，不需要字符或字号分支。

该规则没有字符名称、轮廓索引或角度查表。在 Arial 10pt、0.05pt 键宽的独立 1°
档位中，MAE 为 `0.145pt`，P95 为 `0.421pt`；基础“欧氏外扩 + 轴向接触”分别为
`0.204pt` 和 `0.812pt`。在 Arial、Times New Roman、Calibri、Segoe UI、
Courier New 的 8/14/24pt 共 15 个固定参数切片中，MAE 和 P95 全部下降。在 Arial、
Times New Roman、Calibri、Cambria 的 regular/bold/italic/bold italic 共 16 个
切片中，MAE 全部下降；P95 仅 Times New Roman italic/bold italic 有
`0.001/0.019pt` 的轻微波动。

多字符标签不建立另一套裁剪核。先由 `EndAttach`、justification、sourceText/visible
reversal 和化学标签重排得到实际 glyph placements，再对每个 glyph 独立生成上述
分层排除区并取并集。`(PhO)2POH` 的左端锚点会被 ChemDraw 重排显示为
`HOP2(OPh)`，说明不能按源码字符顺序猜测 glyph 位置。

### 8. 已排除的简单候选

以下模型已在完整方向档位上直接比较，不能作为 ChemDraw 规则：

- 整体或分组件凸包；
- bbox 椭圆、超椭圆、菱形和矩形安全区；
- 支撑函数或角度外包络；
- 所有尖角、所有凸角统一加圆；
- 圆角、斜接或削角三种标准路径 join；
- 低分辨率 Windows GDI hinted bitmap；
- 方形（chessboard）或菱形（taxicab）位图膨胀核；
- 旧 `ANCHOR_MAP` 字符索引表及其 `2 × MarginWidth` 圆的直接复现。

真实基础核仍最接近“实际字体轮廓 + 欧氏 margin + 有限宽键身”，误差主要来自轴向
接触分支、外露端点以及擦边时的拓扑切换，而不是缺少一张 360° 数值表。

## 下一版模型应满足的约束

1. 以字体、style 和字形的真实轮廓/组件为输入，不使用字符名字分支。
2. 在无量纲参数空间生成裁剪核，字号只做最后的整体相似缩放。
3. 自然轮廓、分离组件、特征补强和节点最小排除区必须保持为独立几何层，不能提前
   烘焙成一个 canonical polygon 后再反向缩放。
4. 键端退让使用有限宽键身与排除区的真实相交；快速路径必须与精确路径共享门禁。
5. 训练样本和留出样本分开，门禁报告 pt 单位的 MAE、P95、最大误差和角度连续性。

目前数据已经足以否定“Arial 单一合并多边形 + 运行时点级径向缩放”的方案。下一步
应把真实字体轮廓、自然欧氏外扩、受限凸包特征层和轴向接触点作为独立数据层接入
Rust 精确路径；旧 manifest 只能作为迁移期快照，不能继续承担其他字体、style 和
任意 margin 的反向重建。
