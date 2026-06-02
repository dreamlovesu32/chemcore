# ChemDraw 聚焦/选中行为审计（2026-06-02）

## 目的

建立一条可重复的 `ChemDraw` 实机观测链，用真实桌面窗口来核对：

- 选中态长什么样
- 局部对象是否有单独聚焦态
- 后续右键菜单、拖动手柄、旋转行为怎么测

本次先完成“截图链打通 + 第一轮顶层对象选中态采样”。

## 采样链

新增脚本：

- [scripts/chemdraw-focus-capture.ps1](D:/Projects/chemcore/scripts/chemdraw-focus-capture.ps1:1)

调用方式：

```powershell
npm run chemdraw:focus-capture -- `
  -InputPath tmp/orbital.cdxml `
  -OutputPath tmp/chemdraw-focus/orbital-selected.png `
  -Collection Graphics `
  -Index 0 `
  -Mode Selected
```

这条链当前做的事情是：

1. 通过 `ChemDraw COM` 打开样本文件。
2. 对支持的对象直接设置 `Selected` / `Highlighted`，或走点击模式。
3. 将窗口拉到前台并抓取真实屏幕像素。
4. 输出 `png` 到 `tmp/chemdraw-focus/`。

## 当前已确认行为

### 1. Orbital（顶层 graphic）选中态

样本：

- [orbital-selected.png](D:/Projects/chemcore/tmp/chemdraw-focus/orbital-selected.png:1)

观察结果：

- `s orbital` 被选中后，出现浅蓝色矩形框。
- 角点和边中点有小方块手柄。
- 中心有一个浅蓝色十字锚点。

这说明 orbital 不是“只有外框”，而是典型的图形对象选中态。

### 2. 波浪键选中态

样本：

- [rest-wavy-selected.png](D:/Projects/chemcore/tmp/chemdraw-focus/rest-wavy-selected.png:1)

观察结果：

- 波浪键不是沿路径逐段高亮。
- `ChemDraw` 给它的是一个紧包裹的小型浅蓝矩形框。
- 没有看到 orbital 那种中心十字。

### 3. 空心锲形键选中态

样本：

- [rest-hollow-wedge-selected.png](D:/Projects/chemcore/tmp/chemdraw-focus/rest-hollow-wedge-selected.png:1)

观察结果：

- 空心锲形键同样是紧包裹矩形框。
- 当前看到的选中框语义和波浪键一致，属于 bond/graphic 风格的小外框。

### 4. 田字表格选中态

样本：

- [rest-table-selected.png](D:/Projects/chemcore/tmp/chemdraw-focus/rest-table-selected.png:1)

观察结果：

- 田字表格当前看到的是整对象外框，而不是单元格逐格高亮。
- 这更像“表格对象整体被选中”。

## 当前未完全打通的部分

### 1. TLC plate 顶层选中态

样本：

- [tlcplate-only-normalized-selected.png](D:/Projects/chemcore/tmp/chemdraw-focus/tlcplate-only-normalized-selected.png:1)

观察结果：

- `TLC plate` 顶层对象可以被整体选中。
- 选中后出现浅蓝色外接矩形。
- 顶边中央有一个额外的小手柄。
- 内部的 spot 不会跟着出现单独的蓝色框。

### 2. TLC lane / TLC spot 局部聚焦态

当前发现：

- `TLCSpot` 在 `COM` 里没有可写的 `Selected` / `Highlighted` 属性。
- `TLCLane` 也没有这两个可写属性。
- 所以 spot/lane 的聚焦行为不能只靠 `COM` 属性切换测出来，必须走“真实点击”。

当前进展：

- 已经通过 `TLC plate` 的选中框反推出一版视图缩放/偏移。
- 基于这版映射对首个 `spot` 做了真实单击抓图：
  - [tlcspot-normalized-click-mapped.png](D:/Projects/chemcore/tmp/chemdraw-focus/tlcspot-normalized-click-mapped.png:1)
- 基于同一条映射链，又补了两种交互：
  - 右键：
    - [tlcspot-normalized-rightclick.png](D:/Projects/chemcore/tmp/chemdraw-focus/tlcspot-normalized-rightclick.png:1)
  - 短距离竖直拖动：
    - [tlcspot-normalized-drag.png](D:/Projects/chemcore/tmp/chemdraw-focus/tlcspot-normalized-drag.png:1)
- 另外还做了两张“直接按屏幕黑点中心点”的手工验证：
  - 单击：
    - [tlcspot-manualclick-678-548.png](D:/Projects/chemcore/tmp/chemdraw-focus/tlcspot-manualclick-678-548.png:1)
  - 拖动：
    - [tlcspot-manualdrag-678-548-to-500.png](D:/Projects/chemcore/tmp/chemdraw-focus/tlcspot-manualdrag-678-548-to-500.png:1)
- 这几轮里都没有出现明显局部焦点框，`ShowRf` 也没有自动变为 `true`。

当前判断：

- 在默认选择工具下，`ChemDraw` 至少没有表现出“spot 单击就出现独立浅蓝焦点框”这种行为。
- 在默认选择工具下，`spot` 的右键和短拖动目前也没有出现我们能稳定捕获到的局部 UI。
- 需要继续测：
  - 是否只有拖动时才出现局部反馈
  - 是否必须切到特定 TLC 相关工具
  - 是否是右键/双击/拖动起手才会触发局部 UI

## 当前判断

这条观测链已经足够支持后续工作：

- 顶层对象的选中态可以系统采样。
- orbital / bond / table 这批可以先据此对齐。
- TLC spot/lane 这种局部聚焦对象，还需要补一层视图映射或专门小样本。

## 后续建议顺序

1. 为 `TLC` 提取单独样本，先把 plate 顶层选中态采清楚。
2. 补视图映射，稳定点中 `spot`，采它的真实聚焦态。
3. 在选中态链稳定后，再测右键菜单和拖动手柄行为。
