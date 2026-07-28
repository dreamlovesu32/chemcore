# ChemSema 文档布局规则

## 1. 三个彼此独立的上下文

ChemSema 把文档布局拆成三个明确上下文，不用隐式回退互相代替：

1. **无限画布**：默认编辑视图，没有纸张边界；切换视图本身不改变对象坐标。
2. **纸张视图与分页输出**：以纸张尺寸为基元显示、分页和生成 PDF。
3. **Office/OLE 嵌入**：`FixInPlaceExtent` 和 `FixInPlaceGap` 仅描述嵌入编辑范围，不参与普通画布、分页或对象包围盒计算。

底栏固定为 40 px。右下角保留两个 32 × 32 px 的 CAD 风格按钮：模板入口和纸张视图入口。纸张按钮左键在无限画布/纸张视图之间切换，右键显示纸型、方向、绘图区模式和完整文档布局对话框。

## 2. 初次分页

默认纸型为 A4 纵向，尺寸为 `595.275590551 × 841.88976378 pt`。

第一次需要纸张布局时：

1. 取得全部可见文档对象的实际绘制包围盒。
2. 水平和垂直方向分别计算覆盖该包围盒所需的最少纸张数。
3. 页边距不参与纸张数量计算；页边距只是每张纸上的打印参考区域。
4. 在所得纸张并集内，将内容包围盒水平、垂直居中。
5. 把此时“原始页面网格左上角”的文档坐标保存为 `pageOrigin`。

空文档使用一张纸，原始页面坐标为 `(0, 0)`。

## 3. 编辑后的稳定锚点

一旦 `pageOrigin` 已建立，后续编辑不得重新按当前内容包围盒居中，也不得让原始页面漂移。

- 内容向右或向下越界：在右侧或下侧追加纸张。
- 内容向左或向上越界：在左侧或上侧前插纸张。
- 前插纸张会改变整个已解析网格的左上角，但保存的原始页面仍位于同一个文档坐标。
- 删除或缩小内容不会自动回收已经由 `widthPages` / `heightPages` 明确要求的最小页数。
- `autoPaginate=false` 时不自动增页，超出已配置页面的内容保持超出状态。

因此，“原内容与原页面的相对位置”在编辑、保存、关闭、重新打开和再次输出后均保持稳定。该规则也适用于从 CDX/CDXML 读取的已有页面：导入时直接以 `<page BoundingBox>` 左上角建立 `pageOrigin`。

## 4. Pages 与 Poster

- `DrawingSpace=pages`：相邻纸张步长等于完整纸张宽/高。
- `DrawingSpace=poster`：相邻纸张步长为 `纸张尺寸 - PageOverlap`。
- `PageOverlap` 只在 Poster 模式改变排布；Pages 模式保存该值但不使用它。
- 自动分页上限为横向、纵向各 256 张；文档模型拒绝超过该限制的显式页数。

## 5. 输出

- **PDF**：每个解析后的纸张基元生成一个物理 PDF 页面，按从左到右、从上到下排序；每页使用真实纸张尺寸。
- **CDX/CDXML**：写入解析后的总页面包围盒、页数、纸张尺寸、重叠、页眉页脚、裁切标记和文档视图/嵌入字段。
- **CCJS/CCJZ**：显式保存全部布局字段，包括空字符串、空数组和 `null`，不依赖省略字段推断语义。
- **SVG/EMF 与 Office 剪贴板预览**：仍是单画布/单对象输出，不伪造多页容器；分页语义由 PDF、CCJS、CDX 和 CDXML 承载。

页眉、页脚支持 `&l`、`&c`、`&r` 分区，以及 `&f` 文件名、`&p` 页码、`&d` 日期、`&t` 时间。纸张视图和 PDF 复用同一个页面装饰模块；裁切标记也由该模块给出每页 8 条位于物理页范围内的线段，避免屏幕预览画在纸外而 PDF 裁掉的分叉。

## 6. 参数与作用域

| CCJS 字段 | CDX/CDXML 字段 | 作用 |
|---|---|---|
| `drawingSpace` | `DrawingSpace` | Pages / Poster 排布 |
| `paper.width`, `paper.height` | `Width`, `Height` 与页数共同表达 | 单张纸物理尺寸 |
| `widthPages`, `heightPages` | `WidthPages`, `HeightPages` | 最小页数 |
| `autoPaginate` | ChemSema 原生字段 | 是否自动补页 |
| `pageOrigin` | `<page BoundingBox>` 左上角 | 原始页面锚点 |
| `margins` | `PrintMargins` | 打印参考边距 |
| `pageOverlap` | `PageOverlap` | Poster 重叠 |
| `printTrimMarks` | `PrintTrimMarks` | 裁切/套准标记 |
| `header`, `footer` | `Header`, `Footer` | 页眉页脚文本 |
| `headerPosition`, `footerPosition` | 同名字段 | 页眉页脚基线位置 |
| `magnificationPercent` | `Magnification` | 保存的文档视图缩放；CDX/CDXML 数值为百分数的十倍 |
| `pageDefinition` | `PageDefinition` | 页面格式枚举；缺省为 `Undefined` |
| `splitters[].id` | `<splitter id>` | Splitter 对象的全局唯一身份 |
| `splitters[].position` | `<splitter p>` | Splitter 对象的文档坐标 |
| `splitters[].pageDefinition` | `<splitter PageDefinition>` | Splitter 自身的格式枚举 |
| `legacySplitterPositionIds` | `SplitterPositions` | ChemDraw 6 旧式对象 ID 数组；只保真，不解释成坐标 |
| `fixInPlaceExtent` | `FixInPlaceExtent` | OLE 原位编辑尺寸 |
| `fixInPlaceGap` | `FixInPlaceGap` | OLE 原位编辑留白 |

所有几何数值均以文档点为单位，只有 `magnificationPercent` 使用百分数。
`SplitterPositions` 的官方二进制类型是 `CDXObjectIDArray`，其中的数字是对象 ID，
不是点值。该字段从 ChemDraw 7 起已由 Splitter 子对象取代；两种表示没有冲突时
可以同时保留。

## 7. 交互与保存

- 文档布局对话框由内核提供字段契约和纸型预设，前端只负责呈现和提交完整值。
- 纸型、方向、页数、原始页面坐标、页边距、重叠、页眉页脚、缩放、页面定义、
  Splitter 对象、旧式 Splitter ID 数组和 OLE 参数均可编辑。
- 打开文档时应用保存的 `magnificationPercent`；用户改变缩放后，在保存或导出前把当前缩放同步回文档。
- 纸张视图状态属于标签页视图状态，不进入 CDX/CDXML；文档的分页参数和页面锚点属于文档状态。
- 所有布局修改经过统一命令历史，可撤销/重做。

## 8. 回归门禁

布局门禁至少覆盖：

- 首次 A4 自动分页与居中；
- 向四个方向编辑后原页面锚点不漂移；
- Poster 重叠步长；
- CCJS 显式字段；
- CDXML 和 CDX 全字段往返；
- 多页 PDF 页数与每页 `MediaBox`；
- 40 px 底栏、两个按钮及纸张右键菜单的 GUI 路径。

NR-014 与 NR-016 的联合关闭门禁为 `npm run gate:native:biodraw-layout`；它依次运行
BioShape Rust 行为测试、参数化几何门禁、21 类绝对视觉门禁、布局单元门禁、
BioDraw/属性编辑浏览器回归和分页真实 GUI 回归。
