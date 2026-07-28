# 主绘制栏与生物辅助绘制栏设计

本文定义 ChemSema 左侧绘制栏的两种工作状态，以及 ChemDraw `BioDraw`
对象进入内核、编辑器和交换格式时的统一规则。

## 1. 名称与边界

产品中只使用以下两个名称：

- **主绘制栏**（Main Drawing Rail）：分子、键、文字、箭头、括号、符号、普通
  图形、表格、色谱、电泳、轨道、模板和链工具。
- **生物辅助绘制栏**（Biology-Assisted Drawing Rail）：用于给化学结构图增加酶、
  受体、蛋白、核酸、膜、细胞器和质粒图等生物示意对象。

“化学工具栏”和“生物工具栏”不作为产品术语。生物对象是化学文档的辅助绘制
对象，不代表编辑器切换成独立的生物学文档模式。

## 2. 左侧绘制栏

左侧绘制栏分为三个固定区域：

1. 顶部选择工具。两个绘制栏中始终存在，位置和图标不变。
2. 中部专业工具。切换绘制栏时整体替换，不混排两个绘制栏的工具。
3. 底部切换器。始终固定在最底部，不随内容滚动。

切换器不是绘图工具，不进入内核 `Tool`，不修改文档，不进入撤销历史。切换时：

- 当前存在未提交文本编辑时先按正常工具切换规则提交；
- 取消尚未落地的拖拽预览；
- 保留当前对象选择；
- 激活选择工具，避免隐藏的旧工具继续响应下一次画布点击；
- 记住两种绘制栏各工具族上次选择的二级工具；
- 对每个标签页保持同一个应用级绘制栏状态，避免切换标签时左栏闪烁或意外改变。

切换器的 tooltip 和无障碍名称明确写出目标状态：

- `切换到生物辅助绘制栏`
- `切换到主绘制栏`

## 3. 生物辅助绘制栏的工具族

生物辅助绘制栏中每个中部按钮都是工具族。普通点击复用该族上次选中的二级工具；
点击按钮角标、长按或再次点击已激活按钮时打开二级选项。顶部上下文栏同时显示该族
的完整二级选项，保证鼠标、触屏和键盘均可操作。

| 工具族 | 二级工具 |
| --- | --- |
| 酶 | 单底物酶、双底物酶 |
| 受体与通道 | 受体、离子通道 |
| 抗体 | 免疫球蛋白 |
| G 蛋白 | α 亚基、β 亚基、γ 亚基 |
| 螺旋与核酸 | DNA、螺旋蛋白、tRNA |
| 核糖体 | 核糖体 A、核糖体 B |
| 生物膜 | 直线膜、弧形膜、椭圆膜、胶束 |
| 细胞器 | 内质网、高尔基体、线粒体 |
| 生物示意 | Cloud |
| 质粒图 | 质粒图 |

一级按钮图标显示该族当前二级工具。二级选择是工具状态，不产生文档变更。

## 4. 内核模型

`Tool::BioDraw` 只表示当前使用生物辅助绘制工具。具体工具由强类型
`BioDrawKind` 保存，其中质粒图继续复用 `PlasmidMapData`，其余官方 `bioshape`
枚举使用 `BioShapeData`。

`BioShapeData` 必须显式保存：

- `kind`：官方 BioShape 类型；
- 中心、长轴端点和短轴端点；
- 填充类型、线型、颜色、线宽、粗线宽、渐隐比例；
- 对应类型允许的酶、受体、G 蛋白、DNA、螺旋蛋白、膜和 Golgi 参数。

字段不放入 `payload.extra`，不保存成图片或不透明 SVG。`SceneObject` 可以继续使用
`shape` 这一通用场景类别，但必须携带 `payload.bioShape`；普通 shape 携带
`bioShape` 或 BioShape 缺少该字段均视为文档校验错误。

官方类型与 CCJS 值一一对应，不使用猜测式默认分支。未知类型在 CDX/CDXML
交换层明确报告 unsupported；不能静默改成 Cloud、普通图形或任意已知类型。

## 5. 创建与编辑

- 空白处点击：使用该二级工具的已验证默认尺寸创建，并立即选中新对象。
- 拖拽：主轴由起点到终点确定；对象自身的默认纵横规则或短轴参数确定另一轴。
- 创建期间使用内核 render primitive 预览；释放后预览与落地几何必须一致。
- 选择工具支持移动、缩放、旋转、复制粘贴、跨标签页粘贴、删除、分组和 Link。
- 生物对象参与通用 `link`，但 `auto` 不凭空间接近关系推断生物学语义。
- 参数化对象使用专属控制柄；固定模板至少支持边界框缩放和旋转。
- 专属控制柄的位置必须由当前参数反算；按下后原地释放是无操作，不能把参数跳回
  某个写死的显示位置。对象正文不冒充控制柄，使用 BioDraw 工具点击已有对象正文
  仍按当前二级工具创建新对象。
- 右键菜单提供 `生物对象属性…`，字段定义、范围、混合值和 mutation 来自内核。
- 创建和首次属性确认合并为一个撤销步骤；取消创建对话框时完整回滚。

## 6. 绘制与交换

GUI、SVG、PNG、EMF 和 Office 预览共用 Rust render primitive，不在 viewer
复制几何。CDXML 映射官方 `bioshape`、`BioShapeType` 和全部相关字段；CDX 通过
相同语义模型及已复核的对象/属性 tag 往返。

`xyz`、`MajorAxisEnd3D` 和 `MinorAxisEnd3D` 是原生几何权威；
`BoundingBox` 是由当前几何与线宽得到的输出边界，不作为另一套编辑几何。

## 7. 图标

工具图标由内核按相同生物几何生成，再作为 SVG primitive 交给 Web、桌面和
Harmony 使用。viewer 只负责展示与状态同步，不维护另一套手写生物对象图形。

绘制栏切换器属于产品界面而非文档对象，可使用专用的“主绘制/辅助绘制切换”图标；
它不冒充任何一个 BioShape。

图标取景也属于内核规则：先生成真实 BioShape render primitive，再取实际可见图元
边界，以较长边为基准增加 9% 四周留白并形成正方形 viewBox。这样细长的 DNA、膜线
与近圆形的酶、核糖体使用相同视觉重量，又不会复制第二套缩略图坐标。

## 8. 专属控制柄

专属控制柄严格采用 ChemDraw 21 手册的对象边界，不从“看起来可以调”推断功能：

| 对象 | 专属控制柄 |
| --- | --- |
| Receptor | width |
| GProteinGamma | shape |
| HelixProtein | height、strand width、cylinder width、cylinder spacing |
| DNA | height、spacing、strand width、second-strand offset |
| MembraneLine | unit size；长度由通用选择框修改 |
| MembraneArc | start、end、unit size |
| MembraneEllipse、MembraneMicelle | unit size |

IonChannel、GProteinAlpha、GProteinBeta、Immunoglobulin、Golgi、
EndoplasmicReticulum、Mitochondrion、Cloud、酶、tRNA 与 Ribosome
没有手册定义的专属控制柄，只使用通用移动、八向缩放和旋转。选择工具与创建该对象的
BioDraw 工具都能命中同一组内核控制柄；拖动只修改对应类型字段并形成单个撤销步骤。

## 9. 验收门禁

每个二级工具至少覆盖：

1. 点击创建、拖拽创建、取消、撤销和重做；
2. 选择、聚焦、移动、八向缩放、旋转、复制粘贴、分组和 Link；
3. CCJS 明确字段序列化和文档校验；
4. CDXML、CDX 二次解析往返；
5. GUI、SVG、PNG、EMF 和 Office 预览；
6. 默认参数和至少一个非默认参数的 ChemDraw 对照；
7. Web、Windows 桌面和 Harmony 的绘制栏切换与二级工具状态；
8. 未知枚举、非法尺寸和不适用字段的明确错误，不允许 fallback。

可复跑的 ChemDraw 静默探针入口是：

```text
npm run probe:chemdraw-bioshapes
npm run probe:chemdraw-bioshape-geometry
npm run benchmark:bioshapes:geometry-gate
npm run benchmark:bioshapes:visual-gate
npm run gate:native:biodraw-layout
```

探针缓存写入 `tmp/chemdraw-bioshape-probe`，不进入版本库。
视觉门禁复用该缓存，不会重复启动 ChemDraw；逐类输出 montage 与 JSON 报告到
`tmp/chemdraw-bioshape-probe/visual-gate`。视觉门禁把 ChemDraw 与 ChemSema 固定到
同一文档单位比例，只允许整体平移寻找最大重叠，禁止按各自内容边界独立缩放；除前景
交并比外，还计算双向前景距离的 99%、99.9% 分位和最大值。几何门禁单独核对路径
拓扑与文档单位坐标，因此局部缺口、错位和尺寸误差不会被大对象或空白画布稀释。

已复核的动态规则包括：

- 单底物酶由 `EnzymeReceptorSize` 先得到开口比，再由三次曲线导数根确定水平极值；
- DNA 的两条链分别从自身零相位计数，`DNAWaveWidth` 下限为 `BondLength / 10`，
  前后交织按遮挡顺序绘制；
- 螺旋蛋白的圆柱数量、上下连接、两端延伸和四个专属参数均由闭式几何生成；
- 胶束尾部段数为 `round(1.6 * MembraneElementSize)`，每段径向步长为
  `3e / (3e + 2)`，波幅为 `BoldWidth / 2`。ChemDraw 旧 BioDraw 内核的尾部方向还
  存在不超过基线一文档单位的离散化抖动；门禁把该抖动按尾长传播为绝对坐标包络，
  不使用图片尺寸或样例编号放宽；
- 线粒体外层使用 BioShape 渐变和细轮廓，内部嵴固定填充 `#d9d9d9`，并使用
  `LineWidth` 描边。
- BioShape 曲线渐变先把每段三次曲线分成 21 份采样并合并共线直线段；渐变层
  对这组采样点做仿射缩放。裁剪轮廓不是法线偏移，而是在该轮廓包围盒的四边各
  内缩 `0.1` 文档单位后，分别按 X、Y 缩放同一组采样点。该规则同时用于 GUI、
  SVG、PNG、EMF 与 Office 输出。
- 缺省参数只由 `BioShapeParameters::defaults_for` 一处定义；创建、导入后绘制、
  控制柄和属性面板都通过同一组显式默认值解析，不能各自保留另一套 `unwrap_or`
  常量。

视觉门禁同时执行两层判断：固定的绝对验收上限决定是否达到关闭质量，版本库基线只用于
阻止已审查结果回退。绝对层要求 21 类对象的文档宽高比例落在 `0.95–1.05`、对齐
IoU 不低于 `0.89`、总差异比例不高于 `0.32`，双向前景距离的 99%、99.9% 分位和
最大值分别不超过 `0.75`、`1.0`、`1.1` 文档单位。它不把抗锯齿和渐变像素差异
误称为像素等价，但会阻止尺寸、轮廓、重复单元和局部缺口超出已验收的 ChemDraw
对照范围。
