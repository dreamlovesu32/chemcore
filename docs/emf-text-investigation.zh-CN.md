# Packaged EMF 文字对齐调查

## 目标

找出 Office 打包 `EMF+ dual` 文字渲染与 ChemDraw 的差异来源，优先解决标题/条件块：

- `Cu(MeCN)4PF6 (5 mol%), L (7 mol%)`

当前问题已经确认：

- 不是内核文本内容问题
- 不是全局尺寸 / OLE 缩放问题
- 不是 bbox / 选择框问题
- 是 packaged `EMF` 文字链路的问题

## 当前问题描述

全局尺寸、键宽、右边界对象裁切等大问题已经基本收敛。  
现在剩下的是较窄的一条：

- packaged `EMF` 里，标题/条件块的 fallback `EMR_EXTTEXTOUTW` 链和 ChemDraw 不一致
- 差异集中在 `PF6` 边界附近
- 这种差异是 **上下文敏感** 的：
  - 同一个标题对象单独导出时是好的
  - 放回完整 payload 里会变坏

## 已确认事实

### 不是根因的项目

- 不是内核文本内容
- 不是内核 bbox
- 不是整体对象尺寸
- 不是简单 run chunking
- 不是普通 centered text 的通病
- 不是普通短标签通病
- 不是“没切出空格 token”这种粗粒度问题

### 已确认的核心事实

- 标题对象单独导出时，可以得到期望的 fallback：
  - `6`
  - `" "`
  - `"(5 "`
- 放进完整 payload 后，这颗 fallback 空格可能消失
- 所以问题依赖 **前置对象上下文**
- 问题点位于 packaged dual-EMF 链：
  - `GDI+ DrawString`
  - 自动生成的 fallback `EMR_EXTTEXTOUTW`

## 关键样本

### 真实文件

- `tmp/thiocyanation-source.cdxml`
- `tmp/current-thiocyanation.payload.json`
- `tmp/current-thiocyanation.emf`
- `tmp/current-thiocyanation.emf.records.json`
- ChemDraw 参考：
  - `tmp/thiocyanation-source.chemdraw.emf`
  - `tmp/thiocyanation-source.chemdraw.emf.records.json`

### 最小样本

- `tmp/word-text-fixtures/mixed-center-line.cdxml`
- `tmp/word-text-fixtures/mixed-center-two-line.cdxml`
- `tmp/word-text-fixtures/mixed-center-block.cdxml`
- `tmp/word-text-fixtures/plain-center-line.cdxml`
- `tmp/word-text-fixtures/right-edge-ph.cdxml`

### 子集 payload

- `tmp/title-only.payload.json`
- `tmp/title-only.emf`
- `tmp/subset-8.payload.json`
- `tmp/subset-8.emf`
- `tmp/subset-9.payload.json`
- `tmp/subset-9.emf`

## 最有价值的复现

### `title-only`

真实标题对象单独导出。

结果：

- fallback 中存在：
  - `6`
  - `" "`
  - `"(5 "`

结论：

- 标题对象本身是有能力生成目标 fallback 链的

### `subset-8`

包含：

- 标题对象
- `L:` 文本

结果：

- 同样有：
  - `6`
  - `" "`
  - `"(5 "`

### `subset-9`

包含：

- 标题对象
- `L:` 文本
- 分子对象

结果：

- 变成：
  - `6`
  - `"(5 "`

结论：

- 前面的矢量对象会改变后续标题块的 fallback 行为

### 新增：上下文触发矩阵（2026-05-15）

为了把问题拆成更窄的机制链，我基于 `subset-8` 人工拼了几组最小 payload：

- `subset-8-plus-free-ph`
- `subset-8-plus-free-6`
- `subset-8-plus-free-x`
- `subset-8-plus-8free-ph`
- `subset-8-plus-line`
- `subset-8-plus-unlabeled-molecule`
- `subset-8-plus-free-ph-unlabeled-molecule`
- `subset-8-plus-unlabeled-molecule-free-x`
- `subset-8-plus-unlabeled-molecule-line`

其中：

- `free-*`：插入一个额外 free text 对象
- `8free-ph`：插入多颗额外 free `Ph`
- `line`：插入普通 vector line 对象
- `unlabeled-molecule`：插入一个 **只有 molecule、没有任何节点 label** 的骨架对象

结论如下。

#### A. 任意额外 free text 会打坏试剂行，但不会打坏标题行

样本：

- `subset-8-plus-free-ph`
- `subset-8-plus-free-6`
- `subset-8-plus-free-x`
- `subset-8-plus-8free-ph`

共同结果：

- 标题第二行仍然保留：
  - `6`
  - `" "`
  - `"(5 "`
- 试剂行会退化成：
  - `Ph`
  - `"(3 "`

这说明：

- 试剂行 `Ph -> 空格 -> (3 ` 这一段的问题，不依赖具体 token 是 `Ph` 还是 `6` 还是 `X`
- 只要前面出现额外 free text，上下文就足以吞掉试剂行那颗 fallback 空格

#### B. 普通 vector line 不会触发任何一类空格丢失

样本：

- `subset-8-plus-line`

结果：

- 标题行正常
- 试剂行正常

这说明：

- 标题行问题不是“任意前置矢量对象”都会触发
- 试剂行问题也不是普通 vector 绘制链导致的

#### C. 无标签 molecule 会打坏标题行，但不会打坏试剂行

样本：

- `subset-8-plus-unlabeled-molecule`

结果：

- 标题第二行退化成：
  - `6`
  - `"(5 "`
- 试剂行仍然保留：
  - `Ph`
  - `" "`
  - `"(3 "`

这说明：

- 标题行 `6 -> 空格 -> (5 ` 的丢失，不需要节点 label 参与
- 仅仅 molecule 的骨架绘制链就足以触发

#### D. free text 可以“修复” molecule 对标题行的污染，但同时继续打坏试剂行

样本：

- `subset-8-plus-free-ph-unlabeled-molecule`
- `subset-8-plus-unlabeled-molecule-free-x`

结果：

- 标题第二行又恢复成：
  - `6`
  - `" "`
  - `"(5 "`
- 试剂行仍然退化成：
  - `Ph`
  - `"(3 "`

注意：

- 这些样本里，free text 在最终渲染顺序上位于 molecule 之后、标题对象之前
- 所以它更像是把“molecule -> title”的某个坏状态重置掉了

这说明：

- 标题行问题和试剂行问题不是同一个触发器
- free text 对象会带来一种新的 fallback 状态：
  - 它会让试剂行变坏
  - 但又能把无标签 molecule 对标题行造成的污染冲掉

#### E. line 不能起到同样的“重置”作用

样本：

- `subset-8-plus-unlabeled-molecule-line`

结果：

- 标题第二行仍然退化成：
  - `6`
  - `"(5 "`
- 试剂行仍然保留：
  - `Ph`
  - `" "`
  - `"(3 "`

这说明：

- 这个“重置器”是 text 特有的，不是任意对象都能做到

#### 当前最强结论

现在已经可以把问题明确拆成 **两条独立的状态链**：

1. `molecule path/state -> title second line`
   - 无标签 molecule 足以打坏标题行空格
   - free text 可以把它重置掉
   - 普通 line 不会触发，也不会重置

2. `free text state -> reagent line`
   - 任意额外 free text 都足以打坏试剂行空格
   - 普通 line 不会触发
   - 无标签 molecule 单独不会触发

所以 `subset-9` / 完整 payload 里看到的“标题和试剂都略坏”，更像是 **两条不同的 fallback 状态污染链叠加**，而不是一个统一 bug 的两种外观。

## 记录级观察

### ChemDraw 在 `6 -> 空格 -> (5 ` 这段的形态

ChemDraw 的 fallback 记录是：

- `EMR_EXTTEXTOUTW "6"`
- `EMR_EXTTEXTOUTW " "`
- `EMR_EXTTEXTOUTW "(5 "`

对应的 `EMR_GDICOMMENT / EmfPlusDrawString` comment 很小，接近“点锚定”：

- comment 内不重建 font / stringFormat
- `layoutRect = 0 x 0`

### 好例：`title-only` / `subset-8`

结果虽然保住了 fallback 空格，但结构仍然是 chemcore 这边的风格：

- `EmfPlusObject + EmfPlusObject + EmfPlusDrawString`
- `layoutRect` 非零

这说明：

- 想成功，不一定要完全复制 ChemDraw comment 的最小结构

### 坏例：`subset-9` / 完整 payload

坏例里，`EMF+ DrawString` 里的空格仍然存在；缺的是 fallback `EMR_EXTTEXTOUTW " "`。

这说明真实问题不是：

- “空格没生成”

而是：

- “dual fallback 没把这条 DrawString 落成 GDI 文本记录”

## 已证伪路径

### 1. 全局缩放 / bbox 调参

作用：

- 以前解决过整体尺寸问题

结论：

- 不是这条文字问题的根因

### 2. chunking 调整

结论：

- `preview_text_lines()` 本来就能切出目标 token
- 问题发生在更后面的 packaged 链

### 3. 特判空格 / NBSP / 手补单颗空格

结论：

- 要么无效
- 要么坐标不对
- 要么影响别处

### 4. packaged 全量 GDI fallback

结论：

- token 序列更像 ChemDraw
- 但几何坐标会大幅漂移
- 整图不可接受

### 5. 每个 Text primitive 单独 `Save/Restore`

结论：

- 不足以恢复缺失的 fallback 空格
- 不是简单 graphics state 泄漏

### 6. “GDI+ 布局 + 手写 GDI 文本”窄实验

实验结果：

- 在修正 `CHEMDRAW_EMF_PAGE_SCALE` 坐标换算后
- packaged fallback 链可以变成：
  - `Cu(MeCN)` `4` `PF` `6` `" "` `"(5 "`
- 而且整图视觉上相当接近 ChemDraw

但问题是：

- 这已经不是“让 dual 自动吐对 fallback”
- 而是我们自己替代了 fallback 生成

结论：

- 这是一个可行的保底方案
- 但当前还不把它视作“真正符合 ChemDraw 路径”的修复

## 当前最强假设

最强假设是：

> packaged `EMF+ dual` 中，前置对象上下文会改变后续 `DrawString`
> 被转换成 fallback `EMR_EXTTEXTOUTW` 的方式，而被抑制掉的正是
> `6` 与 `"(5 "` 之间那颗独立空格。

注意：

- 坏例里 `EMF+ DrawString " "` 仍然存在
- 消失的是 fallback `EMR_EXTTEXTOUTW " "`

## 下一步建议

### 1. 继续留在 packaged GDI+ 主路径

不要把整块标题文本切到纯 GDI。

原因：

- broad GDI fallback 的几何已经证伪

### 2. 对比 `title-only` vs `subset-9`

重点看：

- `6` 后那几条 `EMR_GDICOMMENT`
- object id、font id、format id、brush id
- comment 前后的 `GetDC / Save / Restore / object reuse`

### 3. 比较标题开始前的上下文

重点确认是什么前置对象使得：

- `title-only` 正常
- `subset-8` 正常
- `subset-9` 异常

### 4. 保留手写 fallback 作为备选

如果 dual fallback 的真实根因过于顽固，
目前最像可交付保底方案的是：

- 用 GDI+ 布局得到 token 位置
- 自己写 GDI 文本记录

### 5. 引入独立 `GDI+ / EMF+ dual` harness，隔离产品路径

原因：

- 之前直接在产品代码里试 packaged 文本路径，很多实验会同时受：
  - payload 上下文
  - 预览坐标变换
  - 录制器对象复用
  - 现有 text chunking
  影响
- 需要一个完全脱离产品代码的最小实验台，专门验证：
  - `DrawString` 调用形式
  - 字体大小
  - rect-style vs point-style
  - dual fallback 是否会吐出独立 `" "` 记录

现有 harness：

- `scripts/gdiplus-text-fallback-harness.ps1`

能力：

- 直接用 `System.Drawing` 生成 standalone `EMF+ dual`
- 可以单独切换：
  - `DrawString` 用 point-style 还是 rect-style
  - normal / subscript 字体大小
  - 前置 GDI/GDI+ 对象
  - token 序列

意义：

- 后续只要 packaged 文本有新猜测，先在 harness 里证真/证伪，再决定要不要动产品代码

## 实验记录模板

```md
### Experiment: <short name>

- Hypothesis:
- Code path touched:
- Fixtures used:
- Expected result:
- Actual result:
- Relevant files:
- Kept or reverted:
- Conclusion:
```

## 实验记录

### Experiment: 建立调查文档与最小样本基线

- Hypothesis:
  - 先把路径和已知事实系统化，避免后续重复踩坑
- Code path touched:
  - 无产品代码改动
- Fixtures used:
  - `title-only`
  - `subset-8`
  - `subset-9`
  - `mixed-center-*`
- Expected result:
  - 明确“标题对象单独正常、放回完整 payload 异常”的事实
- Actual result:
  - 已确认该事实成立
- Relevant files:
  - 本文档
  - `tmp/*records.json`
- Kept or reverted:
  - 保留文档
- Conclusion:
  - 当前问题是 packaged dual fallback 的上下文敏感问题

### Experiment: 增加可复用的 `EMF` 文本追踪脚本

- Hypothesis:
  - 先把关键 `EMR_GDICOMMENT / EMR_EXTTEXTOUTW` 的信息稳定拉平，后面比较 `title-only / subset-8 / subset-9 / ChemDraw` 才不会反复写临时命令
- Code path touched:
  - 无产品代码改动
  - 新增 `scripts/emf-text-trace.mjs`
- Fixtures used:
  - `tmp/title-only.emf`
  - `tmp/subset-9.emf`
  - `tmp/thiocyanation-source.chemdraw.emf`
- Expected result:
  - 能直接打印：
    - `EMR_EXTTEXTOUTW` 文本与坐标
    - `EMR_GDICOMMENT` 内部 `EmfPlusObject / EmfPlusDrawString`
    - `objectId / formatId / rect`
- Actual result:
  - 脚本可稳定输出关键段
  - 已确认：
    - `title-only`：`formatId=4`，fallback 空格存在
    - `subset-9`：`formatId=2`，`EMF+ DrawString " "` 存在，但 fallback 空格缺失
    - ChemDraw：使用更小的 comment 结构，`layoutRect = 0 x 0`
- Relevant files:
  - `scripts/emf-text-trace.mjs`
  - 本文档
- Kept or reverted:
  - 保留
- Conclusion:
  - 后续所有关键比较都应优先走这个脚本，不再靠一次性命令拼接

### Experiment: 增加 `EmfPlusObject` 历史追踪脚本

- Hypothesis:
  - 需要稳定看到同一个 `DrawString` 在触发时依赖了哪些 `font / string-format / brush` 对象定义，才能判断是否是 object history 影响了 dual fallback。
- Code path touched:
  - 无产品代码改动
  - 新增 `scripts/emf-object-history.mjs`
- Fixtures used:
  - `tmp/title-only.emf`
  - `tmp/subset-8.emf`
  - `tmp/subset-9.emf`
  - `tmp/thiocyanation-source.chemdraw.emf`
- Expected result:
  - 直接打印：
    - `DrawString` 的 `fontId / formatId / brushId`
    - 对应 `EmfPlusObject` 的最近定义历史
    - 对应 fallback `EMR_EXTTEXTOUTW`
- Actual result:
  - 已能稳定打印关键对象链
  - 证明了：
    - `subset-9` / 坏样本里，`EMF+ DrawString " "` 其实存在
    - 真正缺的是 fallback `EMR_EXTTEXTOUTW " "`
    - 这不是单纯“没切出空格 token”
- Relevant files:
  - `scripts/emf-object-history.mjs`
  - 本文档
- Kept or reverted:
  - 保留
- Conclusion:
  - 后续分析要把“可见 `DrawString`”和“fallback `EXTTEXTOUTW`”明确分开看

### Experiment: 重新导出当前代码基线，避免被旧 `v62` 误导

- Hypothesis:
  - 之前长期参考的 `v62` 可能已经不是当前代码状态；如果继续拿它做推理，会把已经修掉的问题当成现状。
- Code path touched:
  - 无产品代码改动
  - 仅重新导出：
    - `tmp/thiocyanation-source.analysis.payload.json`
    - `tmp/thiocyanation-source.analysis.emf`
- Fixtures used:
  - `tmp/thiocyanation-source.cdxml`
  - `tmp/thiocyanation-source.chemdraw.emf`
- Expected result:
  - 得到一份严格对应“当前代码”的 packaged `EMF`，再与 ChemDraw 做同口径比较
- Actual result:
  - 当前导出的 `analysis.emf` 与旧 `v62` 有实质差异：
    - 当前代码标题块 `StringFormat` 原始 flags 为 `0x6804`
    - 旧 `v62` 为 `0x5804`
  - 当前 `analysis.emf` 中标题第二行 fallback 空格已经存在：
    - `EMR_EXTTEXTOUTW " "` at `ref=(857,457)`
  - 说明“空格缺失”在当前基线已经不是主矛盾
- Relevant files:
  - `tmp/thiocyanation-source.analysis.emf`
  - `tmp/thiocyanation-source.analysis.emf.records.json`
  - `tmp/thiocyanation-source.chemdraw.emf`
- Kept or reverted:
  - 保留产物作为当前分析基线
- Conclusion:
  - 后续推理必须以 `analysis.emf` 为准
  - 旧 `v62` 只能当历史样本，不能再当当前代码基线

### Experiment: 让 packaged `EMF` 的 GDI+ 文本稳定复用 font / string-format 对象

- Hypothesis:
  - 当前 dual fallback 的不稳定，可能来自同一个 `fontId` 在 `EmfPlusObject` 里被反复重绑成 normal/subscript 两种字号；如果把对象身份稳定下来，fallback tokenization 可能会更像 ChemDraw。
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - 在 packaged / GDI+ text 路径中增加 `PreviewGdiplusTextCache`
  - 复用 `GpFont` / `GpStringFormat`
- Fixtures used:
  - `tmp/thiocyanation-source.analysis.payload.json`
  - `tmp/thiocyanation-source.analysis.emf`
  - `tmp/thiocyanation-source.chemdraw.emf`
- Expected result:
  - 标题第二行对象链从“同一个 `fontId` 来回重绑”变成稳定的 normal/subscript font id
  - fallback `EMR_EXTTEXTOUTW " "` 重新出现
- Actual result:
  - 对象身份确实稳定了：
    - 我们从原来的 `fontId=5` 反复重绑
    - 变成了更像 ChemDraw 的分离状态：
      - normal `fontId=4`
      - subscript `fontId=5`
      - `formatId=3`
  - 但标题第二行 fallback 空格仍然缺失：
    - `EMF+ DrawString " "` 仍然存在
    - `EMR_EXTTEXTOUTW " "` 仍然没有出现
- Relevant files:
  - `tmp/thiocyanation-source.analysis.emf`
  - `tmp/thiocyanation-source.analysis.emf.records.json`
  - `scripts/emf-object-history.mjs`
- Kept or reverted:
  - 当前先保留，作为下一步实验基线
- Conclusion:
  - “object id 稳定化”本身不是充分条件
  - 根因仍然更像在 `DrawString` 的 point/layout 语义或 dual fallback 合并策略上

### Experiment: 把 packaged `DrawString` 改成 `0 x 0` point-style `layoutRect`

- Hypothesis:
  - 既然 ChemDraw 的标题/条件 plain text `DrawString` 都是 `rect=(x,y,0,0)`，那把我们的 packaged `DrawString` 也改成同样的 point-style anchor，可能会让 dual fallback 正确落出独立空格。
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - 仅在 `transform.emf_recording` 时，把 `DrawString` 的 `RectF` 宽高改成 `0`
- Fixtures used:
  - `tmp/thiocyanation-source.analysis.payload.json`
  - `tmp/thiocyanation-source.analysis.emf`
  - `tmp/thiocyanation-source.chemdraw.emf`
- Expected result:
  - 标题第二行 token 变成：
    - `6`
    - `" "`
    - `"(5 "`
  - fallback `EMR_EXTTEXTOUTW " "` 重新出现
- Actual result:
  - `DrawString` 形态确实更像 ChemDraw：
    - `rect=(x,y,0,0)`
    - normal / subscript font id 仍然稳定分开
  - 但标题第二行 fallback 空格仍然没有出现：
    - 仍然只有 `6`
    - 直接跳到 `"(5 "`
  - 因此 point-style `layoutRect` 不是充分条件
- Relevant files:
  - `tmp/thiocyanation-source.analysis.emf`
  - `tmp/thiocyanation-source.analysis.emf.records.json`
- Kept or reverted:
  - 计划回退产品代码
  - 文档保留
- Conclusion:
  - “稳定 font id + zero-rect” 两条同时满足，依然不能让 fallback 空格回来
  - 真正根因更可能在 `StringFormat` 对象内容，或 dual fallback 更深层的生成策略

### Finding: 当前最小复现应切到 `mixed-center-two-line`

- Observation:
  - 用当前基线代码重新导样本后，`mixed-center-line` 不再是最小复现，因为它会正确落出：
    - `6`
    - `" "`
    - `"(5 "`
  - 当前最小复现是 `tmp/word-text-fixtures/mixed-center-two-line.cdxml`
- Current behavior on `mixed-center-two-line`:
  - second line fallback 变成：
    - `Cu(MeCN)`
    - `4`
    - `PF`
    - `6`
    - `"(5 "`
  - 中间独立 `" "` 丢失
- ChemDraw behavior on the same fixture:
  - second line fallback 是：
    - `Cu(MeCN)`
    - `4`
    - `PF`
    - `6`
    - `" "`
    - `"(5 "`
- Conclusion:
  - 问题已经能在一个极小的、当前有效的 fixture 上稳定复现
  - 后续分析和回归优先围绕 `mixed-center-two-line`

### Finding: 不是 `Center` 特有问题；拆成两个 text object 会恢复

- Temporary fixtures:
  - `tmp/word-text-temp/mixed-left-two-line.cdxml`
  - `tmp/word-text-temp/mixed-right-two-line.cdxml`
  - `tmp/word-text-temp/mixed-center-two-objects.cdxml`
- Results:
  - `mixed-left-two-line`：第二行仍然缺独立 `" "` fallback
  - `mixed-right-two-line`：第二行仍然缺独立 `" "` fallback
  - `mixed-center-two-objects`：把两行拆成两个独立 `<t>` 后，第二行 fallback 空格恢复
- Conclusion:
  - 问题不是 `Center` 对齐专属
  - 更像是“同一个多行 text object 中，后续 mixed-script 行”的 packaged dual fallback 行为

### Experiment: packaged EMF 文字按“每一行”加 `GDI+ Save/Restore`

- Hypothesis:
  - 既然把两行拆成两个独立 `<t>` 会恢复第二行空格，也许在 packaged `EMF` 里对每一行加一层 `GDI+ Save/Restore`，能模拟 object boundary 的 flush 行为。
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `draw_gdiplus_text()` 的按行循环中，`transform.emf_recording` 时对每一行包一层 `GdipSaveGraphics / GdipRestoreGraphics`
- Fixtures used:
  - `tmp/word-text-fixtures/mixed-center-two-line.cdxml`
  - `tmp/thiocyanation-source.cdxml`
- Expected result:
  - `mixed-center-two-line` 的第二行 fallback 恢复：
    - `6`
    - `" "`
    - `"(5 "`
- Actual result:
  - 第二行独立空格仍然没有回来
  - 只是在记录链上引入了新的：
    - `EmfPlusRestore (0x4026)`
    - `EmfPlusSave (0x4025)`
  - 但不改变核心现象
- Kept or reverted:
  - 计划回退产品代码
  - 文档保留
- Conclusion:
  - object boundary 的效果，并不能通过“按行 Save/Restore”简单模拟
  - 两行拆成两个独立 `<t>` 能成功，说明真正有效的分界还在更高层

### Experiment: packaged `GDI+ MeasureString` 改到离屏 graphics

- Hypothesis:
  - 当前 `gdiplus_text_layout()` 在 packaged `EMF` 录制时，直接拿 recording metafile 的 `GpGraphics` 去 `MeasureString`。
  - 这可能会污染 dual fallback 的状态，所以把测量改到离屏 `GDI+ graphics` 上，也许能让 `mixed-center-two-line` 第二行的独立空格恢复。
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `gdiplus_text_layout()`
  - `transform.emf_recording` 时：
    - 基于 `CreateCompatibleDC` + `GdipCreateFromHDC`
    - 构造专用 `measure_graphics`
    - 用它替代 recording metafile graphics 参与 `gdiplus_text_run_advance()`
- Fixtures used:
  - `tmp/word-text-fixtures/mixed-center-two-line.cdxml`
  - `tmp/thiocyanation-source.cdxml`
- Expected result:
  - `mixed-center-two-line` 第二行 fallback 恢复：
    - `6`
    - `" "`
    - `"(5 "`
- Actual result:
  - 第二行独立空格仍然没有回来
  - `mixed-center-two-line` 仍然是：
    - `6`
    - `"(5 "`
  - 完整 `thiocyanation` 标题第二行也同样没有恢复
- Kept or reverted:
  - 计划回退产品代码
  - 文档保留
- Conclusion:
  - `MeasureString` 使用 recording metafile graphics 不是这颗 fallback 空格丢失的主因
  - 问题仍然更像在 packaged dual fallback 的文本输出阶段，而不是测量阶段

### Experiment: 每个 text object 前重放 fresh-file GDI+ 初始化状态

- Hypothesis:
  - `title-only` 作为 fresh file 的第一组文本时，开头会有：
    - `EmfPlusSetPageTransform`
    - `EmfPlusSetAntiAliasMode`
    - `EmfPlusSetTextRenderingHint`
  - `subset-9` 中标题块位于文件中部，这组初始化状态不会在标题前重放。
  - 也许问题不是普通 Save/Restore，而是前置对象污染了这几个更高层的 graphics state。
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `draw_gdiplus_text()`
  - 当 `transform.emf_recording` 时，在每个 text object 开始前显式重放：
    - `GdipSetPageUnit(UnitPixel)`
    - `GdipSetPageScale(1.0)`
    - `GdipSetPageScale(CHEMDRAW_EMF_PAGE_SCALE)`
    - `GdipSetSmoothingMode(SmoothingModeAntiAlias)`
    - `GdipSetTextRenderingHint(TextRenderingHintAntiAlias)`
- Fixtures used:
  - `tmp/word-text-fixtures/mixed-center-two-line.cdxml`
- Expected result:
  - `6 -> " " -> "(5 "` 之间的独立 fallback 空格恢复
- Actual result:
  - 空格对应的 `EmfPlusDrawString " "` 仍然存在
  - 但 fallback 依然没有 `EMR_EXTTEXTOUTW " "`
  - 记录链仍然是：
    - `EMR_EXTTEXTOUTW "6"`
    - `EMR_GDICOMMENT DrawString " "`
    - `EMR_GDICOMMENT DrawString "(5 "`
    - `EMR_EXTTEXTOUTW "(5 "`
- Kept or reverted:
  - 计划回退产品代码
  - 文档保留
- Conclusion:
  - 仅仅重放 page transform / antialias / text rendering hint，不足以复现 fresh-file 行为
  - 说明问题不是简单的“少了一组初始化 graphics state”

### Experiment: standalone harness（小字号）对比 rect-style vs point-style

- Hypothesis:
  - 如果 dual fallback 的差异主要来自 `DrawString` 调用形式，那么脱离产品路径后也应该能复现：
    - rect-style 更容易丢独立空格
    - point-style 更像 ChemDraw
- Code path touched:
  - 无产品代码改动
  - 仅新增/使用 `scripts/gdiplus-text-fallback-harness.ps1`
- Fixtures used:
  - `tmp/gdiplus-harness-test.emf`
  - `tmp/gdiplus-harness-rect.emf`
- Expected result:
  - 至少在 standalone harness 中观察到 rect / point 差异
- Actual result:
  - 小字号（近似 27 / 20）下：
    - point-style 会丢 fallback `" "`
    - rect-style 反而能保住 fallback `" "`
- Kept or reverted:
  - 保留 harness
  - 无产品代码改动
- Conclusion:
  - “point-style 一定比 rect-style 更像 ChemDraw”在小字号条件下不成立
  - 问题还依赖别的量

### Experiment: standalone harness（产品级大字号）对比 rect-style vs point-style

- Hypothesis:
  - packaged `EMF` 与 standalone harness 的关键差异之一，可能是产品链使用了更大的 `EmfPlusFont emSize`（约 normal=99.9 / subscript=74.9）
  - 只有把字号口径抬到产品级，才能复现 packaged fallback 行为
- Code path touched:
  - 无产品代码改动
  - 仅使用 harness 生成不同组合：
    - rect-style + 产品级字号
    - point-style + 产品级字号
- Fixtures used:
  - `tmp/harness/rect-fresh-product-fontsize.emf`
  - `tmp/harness/point-product-fontsize.emf`
- Expected result:
  - 找到最接近 packaged / ChemDraw 差异的最小条件组合
- Actual result:
  - `rect-style + 产品级字号`：
    - `Cu(MeCN)` `4` `PF` `6` 后
    - fallback `EMR_EXTTEXTOUTW " "` **消失**
    - 与 packaged 坏例一致
  - `point-style + 产品级字号`：
    - fallback `EMR_EXTTEXTOUTW " "` **重新出现**
    - token 序列接近 ChemDraw：
      - `Cu(MeCN)` `4` `PF` `6` `" "` `"(5 "`
- Kept or reverted:
  - 保留 harness 产物与分析
  - 无产品代码改动
- Conclusion:
  - 当前最有力证据是：
    - **large font + rect-style** 会稳定复现 packaged 坏例
    - **large font + point-style** 会稳定恢复 ChemDraw 风格的 fallback 空格
  - 因此 point-style 需要重新回到产品路径里验证，但必须是“产品级字号 + 窄命中”的前提下

### Finding: `mixed-center-two-line` 的 packaged 问题与 harness 结果一致

- Observation:
  - `mixed-center-two-line` 是当前最小坏例
  - 它的问题形态与 `rect-style + 产品级字号` harness 完全同构：
    - `6`
    - `"(5 "`
    - 中间缺独立 fallback `" "`
- Conclusion:
  - harness 已经不只是旁证，而是能真实复现 packaged 坏例的最小实验台
  - 后续如果做 point-style 产品实验，应优先以：
    - `mixed-center-two-line`
    - 大文件标题第二行
    双样本共同验收

### Experiment: 用固定 `selectionBounds` 重新验证 subset 上下文问题

- Hypothesis:
  - 早先 `subset-8` / `subset-8-plus-*` 的一些“状态泄漏”结论，可能混入了 `EMF frame` 不同导致的整体缩放差。
  - 如果强制所有 subset payload 使用相同的 `svg viewBox` 作为 `clipboard.selectionBounds`，再重新导出 `EMF`，就能把“上下文导致的文字问题”和“frame/source bounds 差异”拆开。
- Code path touched:
  - 无产品代码改动
  - 仅对 `tmp/fixed-selection/*.payload.json` 做分析性补丁：
    - `document.meta.clipboard.selectionBounds = svg viewBox`
  - 重新生成 `tmp/fixed-selection/*.emf`
- Fixtures used:
  - `subset-8`
  - `subset-8-plus-free-x`
  - `subset-8-plus-line`
  - `subset-8-plus-unlabeled-molecule`
  - `subset-8-plus-unlabeled-molecule-free-x`
- Expected result:
  - 如果标题第二行空格缺失真的是上下文状态泄漏，那么即使 `selectionBounds` 固定，它也应继续稳定复现。
- Actual result:
  - 固定 `selectionBounds` 以后：
    - **标题第二行 `PF₆` 后空格不再是稳复现问题**
    - `subset-8-plus-unlabeled-molecule` 中标题空格恢复
    - `subset-8-plus-line` 标题空格也正常
  - 但 **试剂行 `Ph -> " " -> "(3 "` 的空格缺失仍能在 `subset-8-plus-free-x` 稳定复现**
- Kept or reverted:
  - 仅保留分析产物与文档
  - 无产品代码改动
- Conclusion:
  - 早先“标题第二行是主问题”的判断被证伪，至少不是稳健结论。
  - 当前最干净、最稳定的 packaged `EMF` 文字坏例，是：
    - `subset-8-plus-free-x`
    - 试剂行 `Ph -> " " -> "(3 "`

### Finding: 当前稳复现坏例是“试剂行空格丢失”，不是标题第二行

- Observation:
  - 在固定 `selectionBounds` 后：
    - `subset-8`
      - `Ph @ (806,409)`
      - `" " @ (840,409)`
      - `"(3 " @ (848,409)`
    - `subset-8-plus-free-x`
      - `Ph @ (806,409)`
      - **没有独立 fallback `" "`**
      - `"(3 " @ (848,409)`
    - `subset-8-plus-line`
      - `Ph @ (806,409)`
      - `" " @ (840,409)`
      - `"(3 " @ (848,409)`
    - `subset-8-plus-unlabeled-molecule`
      - `Ph @ (806,409)`
      - `" " @ (840,409)`
      - `"(3 " @ (848,409)`
- Conclusion:
  - 当前 packaged `EMF` 文字问题最应该围绕：
    - **前置 free text**
    - **后续试剂行 fallback `" "` 缺失**
    这条链继续往下查。
  - `line` 和 `unlabeled molecule` 并不会触发同样的问题，因此“任意前置对象都会污染后续文本”这个说法也不成立。

### Finding: free text 样本会把后续普通文本切到更大的 `EmfPlusFont emSize`

- Observation:
  - `subset-8`（好例）里，普通文字对象常见 `EmfPlusFont emSize`：
    - normal: `99.96807098388672`
    - subscript: `74.9760513305664`
  - `subset-8-plus-line`（好例）也沿用这组：
    - normal: `99.96807098388672`
    - subscript: `74.9760513305664`
  - `subset-8-plus-unlabeled-molecule`（好例）则是另一组很接近但仍正常的口径：
    - normal: `99.97856903076172`
    - subscript: `74.98392486572266`
  - `subset-8-plus-free-x`（坏例）会把后续普通文字切到更大的口径：
    - normal: `100.00094604492188`
    - subscript: `74.9760513305664`
- Conclusion:
  - 当前最像根因的量之一，是：
    - **前置 free text 会把后续 packaged plain text 切到一组更大的 normal-font emSize**
  - 这还不能单独解释全部现象（因为标题第二行在同一组字号下仍然正常），但它已经是当前最有力的“可量化差异”之一。

### Experiment: 只移动 free text `X` 的位置，区分 top 扩展和 right 扩展

- Hypothesis:
  - `subset-8-plus-free-x` 的坏例，也许不是因为 “free text 身份” 本身，而是因为它把整体 `EMF header bounds` 往上或往右撑开后，间接改变了 packaged GDI+ 文本的口径。
  - 如果只改 `X` 的位置、不改内容和样式，就能分离：
    - top 扩展
    - right 扩展
    哪一个才是真正触发量。
- Code path touched:
  - 无产品代码改动
  - 仅在 `tmp/fixed-selection/free-x-variants/*.payload.json` 中修改：
    - `root.objects[2].transform.translate`
- Fixtures used:
  - 基线坏例：`subset-8-plus-free-x`
  - 变体：
    - `move-left`: 只把 `X` 左移到原有内容框内，保留顶部外扩
    - `move-down`: 只把 `X` 下移回原有顶部以下，保留右侧外扩
    - `move-inside`: 同时移回原有顶部和右侧范围内
- Actual result:
  - `move-left`
    - `header bounds = {left:163, top:268, right:994, bottom:602}`
    - 试剂行仍然是：
      - `Ph`
      - **没有独立 fallback `" "`**
      - `"(3 "`
    - reagent normal font 仍为：
      - `emSize = 100.00094604492188`
  - `move-down`
    - `header bounds = {left:163, top:290, right:1253, bottom:602}`
    - 试剂行恢复为：
      - `Ph`
      - `" "`
      - `"(3 "`
    - reagent normal font 恢复为：
      - `emSize = 99.96807098388672`
  - `move-inside`
    - `header bounds = {left:163, top:290, right:994, bottom:602}`
    - 试剂行同样恢复：
      - `Ph`
      - `" "`
      - `"(3 "`
    - reagent normal font 同样恢复为：
      - `emSize = 99.96807098388672`
- Kept or reverted:
  - 仅保留分析产物与文档
  - 无产品代码改动
- Conclusion:
  - **触发问题的是顶部外扩（top bounds 变化），不是右侧外扩。**
  - right 扩展单独存在时（`move-down`），试剂行空格不会丢。
  - 只要 top 恢复正常，normal font `emSize` 也会从 `100.000946` 回落到 `99.968071`，同时试剂行 fallback 空格恢复。
  - 这说明当前最值得继续追的是：
    - `header bounds.top / frame.top`
    - packaged GDI+ font 口径（`EmfPlusFont emSize`）
    - 以及它们如何影响 dual fallback 是否落出独立 `" "`

### Finding: 关键分叉不是“状态污染”，而是 `PreviewTransform::scale` 的限幅轴切换

- Observation:
  - `renderer.rs` 中 packaged GDI+ 字体口径来自：
    - `gdiplus_text_scale(transform)`
    - `create_gdiplus_font()`
    - `em_size = font_size * gdiplus_text_scale(transform)`
  - `gdiplus_text_scale(transform)` 在 `emf_recording` 时等于：
    - `transform.scale / CHEMDRAW_EMF_PAGE_SCALE`
  - 而 `transform.scale` 又来自：
    - `PreviewTransform::from_bounds(draw_bounds, source_bounds)`
    - `scale = min(target_width / source_width, target_height / source_height)`
- Derived math:
  - baseline / good case 使用固定 `selectionBounds` 后，source 约为：
    - `width = 1364.0313 - 128.888676 = 1235.142624`
    - `height = 823.2053 - 270.96 = 552.2453`
  - 对应 ratio：
    - `width_ratio = round(1235.142624) / 1235.142624 = 0.9998845283`
    - `height_ratio = round(552.2453) / 552.2453 = 0.9995558133`
    - 因此 baseline 取 **height-limited**：
      - `transform.scale = 0.9995558133`
  - free-text 顶部上抬后（`top = 266.67`），source 变成：
    - `height = 823.2053 - 266.67 = 556.5353`
  - 对应 ratio：
    - `width_ratio = 0.9998845283`（不变）
    - `height_ratio = round(556.5353) / 556.5353 = 1.0008349875`
    - 因此 bad case 转成 **width-limited**：
      - `transform.scale = 0.9998845283`
- Consequence:
  - 这正好解释了 `EmfPlusFont emSize` 的实测变化：
    - good normal: `99.96807098388672`
    - bad normal: `100.00094604492188`
  - 也解释了为什么：
    - `move-left`（只保留 top 外扩）仍坏
    - `move-down`（只保留 right 外扩）恢复
    - `move-inside`（都恢复）也恢复
- Conclusion:
  - 当前 packaged `EMF` 文字问题的最直接根因，不再像“前置 free text 污染了状态”。
  - 更准确地说，是：
    - **free text 把 `source_bounds.top` 往上抬**
    - **导致 `PreviewTransform::scale` 从 height-limited 切到 width-limited**
    - **进而把 packaged GDI+ normal font emSize 推大约 0.03%**
    - **最终改变 dual fallback 是否输出独立 `" "`**
  - 后续真正应该继续追的，是：
    - 为什么这 0.03% 的 `emSize`/scale 变化，会刚好跨过 fallback 空格输出阈值
    - 以及是否应该避免 text preview 受“顶部自由文本”驱动的限幅轴切换

### Experiment: 扫描 free text `Y` 位置，定位空格消失阈值

- Hypothesis:
  - 如果根因真的是 `source_bounds.top` 引起的 `scale` 限幅轴切换，那么把 free text `X` 的 `translate.y` 沿竖直方向微调，空格是否存在应该会出现一个非常明确的阈值。
- Code path touched:
  - 无产品代码改动
  - 仅生成分析性 payload 变体：
    - `tmp/fixed-selection/free-x-y-sweep/*.payload.json`
- Sweep values:
  - `y = 266.67, 268.00, 269.00, 270.00, 270.50, 270.96, 271.00, 272.00, 275.00, 280.00, 290.00, 300.00, 320.00`
- Actual result:
  - 只有最上面的一个点会坏：
    - `y=266.67`
      - `header.bounds.top = 268`
      - reagent fallback：
        - `Ph`
        - **没有独立 `" "`**
        - `"(3 "`
      - reagent normal font raw：
        - `emSize = 100.00094604492188`
  - 从 `y=268.00` 开始往下，全都恢复：
    - `y=268.00`
      - `header.bounds.top = 269`
      - reagent fallback：
        - `Ph`
        - `" "`
        - `"(3 "`
      - reagent normal font raw：
        - `emSize = 99.96807098388672`
  - 之后 `y >= 268.00` 的所有样本都保持同样的“好”状态。
- Conclusion:
  - 这不是模糊的“有时会坏”，而是一个**几乎单像素级**的阈值问题。
  - 当前最稳定的经验规律是：
    - `header.bounds.top = 268` 时坏
    - `header.bounds.top >= 269` 时好
  - 这进一步支持上一条结论：
    - 问题由 `source/top -> scale -> emSize` 的极小切换触发
    - 而不是笼统的上下文或对象种类污染

### Experiment: zero-layout DrawString for standalone whitespace (packaged EMF)
- Hypothesis:
  - If dual fallback is dropping standalone `" "` because its nonzero layout rectangle is treated as a clipped/no-op trailing-space case, then forcing whitespace-only runs to use a point-style / zero-layout `DrawString` should restore the fallback `EMR_EXTTEXTOUTW " "`.
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `draw_gdiplus_text_run()`
  - For `transform.emf_recording && run.text.chars().all(char::is_whitespace)`, use `RectF { X: x, Y: top, Width: 0.0, Height: 0.0 }`.
- Validation sample:
  - bad threshold case: `tmp/fixed-selection/free-x-y-sweep/y-266_67.payload.json`
  - regenerated output: `y-266_67.zero-space.emf`
- Actual result:
  - The reagent-line standalone space still exists at the EMF+ layer as `DrawString text=" "`.
  - But fallback still skips the corresponding `EMR_EXTTEXTOUTW " "` on the bad threshold case.
  - The title-line standalone space still behaves normally.
  - The regenerated trace still shows reagent sequence as:
    - `Ph`
    - `DrawString " "` present
    - no fallback `EXTTEXTOUTW " "`
    - `"(3 "`
- Conclusion:
  - The presence/absence of the fallback space is **not controlled simply by the standalone whitespace token's layout-rect width/height**.
  - This weakens the “standalone space is dropped only because its own layoutRect is nonzero” hypothesis.
  - The remaining root cause is more likely tied to the broader packaged dual-fallback threshold behavior (font scale / context / fallback conversion), not the whitespace token's own rectangle alone.
- Status:
  - Experiment failed.
  - Product code should be reverted; keep only the finding.

### Experiment: clamp packaged normal `EmfPlusFont emSize` back to the good-case value
- Hypothesis:
  - Since the bad free-text-top case correlates with normal packaged font `emSize` jumping from `99.96807098388672` to `100.00094604492188`, maybe the standalone reagent-line fallback space disappears simply because the font crosses that threshold.
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `create_gdiplus_font()`
  - Add an experiment-only env var:
    - `CHEMCORE_OFFICE_EXPERIMENT_EMSIZE_CLAMP`
    - when `transform.emf_recording`, clamp `em_size = min(em_size, clamp)`
- Validation sample:
  - bad threshold case: `tmp/fixed-selection/free-x-y-sweep/y-266_67.payload.json`
  - run with:
    - `CHEMCORE_OFFICE_EXPERIMENT_EMSIZE_CLAMP=99.968071`
  - output:
    - `y-266_67.clamped.emf`
- Actual result:
  - The packaged EMF font objects for the affected normal runs were successfully clamped down to the good-case family:
    - reagent normal `DrawString` objects now use raw `...a7efc742...` (`f32 = 99.9681`)
    - instead of the bad-case `...7c00c842...` (`f32 = 100.0009`)
  - However, the reagent-line fallback still remained bad:
    - `Ph`
    - **no fallback `EMR_EXTTEXTOUTW " "`**
    - `"(3 "`
  - The standalone reagent space still exists at the EMF+ layer as `DrawString text=" "`.
- Conclusion:
  - `EmfPlusFont emSize` is **not by itself sufficient** to determine whether dual fallback emits the standalone reagent-line space.
  - The earlier `top -> scale-axis -> emSize` chain is still a strong correlation, but this experiment shows `emSize` is not the sole causal switch.
  - Whatever the real trigger is, it survives even after the packaged normal font is forced back into the “good” numeric bucket.
- Status:
  - Experiment failed as a fix.
  - Keep the finding, revert the temporary clamp hook.

### Experiment: packaged `DrawString` point-style / zero-layout for all text runs
- Hypothesis:
  - If ChemDraw is effectively using a point-anchored `DrawString` path, then forcing packaged GDI+ text to use `RectF { width=0, height=0 }` for every run might make the dual fallback preserve the standalone reagent-line space.
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `draw_gdiplus_text_run()`
  - Add experiment-only env var:
    - `CHEMCORE_OFFICE_EXPERIMENT_POINT_STYLE_TEXT`
    - when `transform.emf_recording`, force `RectF { X: x, Y: top, Width: 0.0, Height: 0.0 }`
- Validation samples:
  - bad threshold case:
    - `tmp/fixed-selection/free-x-y-sweep/y-266_67.payload.json`
    - output: `y-266_67.point.emf`
  - good threshold case:
    - `tmp/fixed-selection/free-x-y-sweep/y-268_00.payload.json`
    - output: `y-268_00.point.emf`
- Actual result:
  - The standalone reagent-line space still exists at the EMF+ layer in both files.
  - Good case still emits fallback:
    - `Ph`
    - `" "`
    - `"(3 "`
  - Bad case still drops fallback:
    - `Ph`
    - **no fallback `EMR_EXTTEXTOUTW " "`**
    - `"(3 "`
  - This is true even though the standalone space `DrawString` now uses a literal zero-layout rect:
    - bad: `rect=(3151.2849...,1443.3918...,0,0)`
    - good: `rect=(3151.1689...,1444.4235...,0,0)`
- Conclusion:
  - “Point-style / zero-layout rect” by itself is **not sufficient** to make the dual fallback keep the standalone reagent-line space.
  - This further narrows the problem: it is not just layout-rect width/height, and not just `emSize`; some broader packaged fallback state is still controlling the drop.
- Status:
  - Experiment failed as a fix.
  - Keep the finding, revert the temporary point-style hook.

### Experiment: packaged text top-nudge threshold
- Hypothesis:
  - Since the bad free-text-top case already correlates with a `source_bounds.top` lift and a `scale`-axis switch, maybe the real trigger is not `emSize` alone but the packaged text run's **vertical placement** (`RectF.Y` / top) crossing a fallback threshold.
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `draw_gdiplus_text_run()`
  - Add experiment-only env var:
    - `CHEMCORE_OFFICE_EXPERIMENT_TEXT_TOP_NUDGE`
    - when `transform.emf_recording`, add a positive delta to the packaged text `top`
- Validation sample:
  - bad threshold case:
    - `tmp/fixed-selection/free-x-y-sweep/y-266_67.payload.json`
  - generated outputs:
    - `y-266_67.top-0_05.emf`
    - `y-266_67.top-0_1.emf`
    - `y-266_67.top-0_2.emf`
    - `y-266_67.top-0_3.emf`
    - `y-266_67.top-0_4.emf`
    - `y-266_67.top-0_5.emf`
    - `y-266_67.top-1.emf`
    - `y-266_67.top-1_03.emf`
    - `y-266_67.top-1_1.emf`
    - `y-266_67.top-1_5.emf`
- Actual result:
  - The title-line standalone space (`PF₆` boundary) exists for every nudge value and is not the discriminator.
  - The reagent-line standalone space shows a **sharp threshold**:
    - `top_nudge = 0.05 / 0.1 / 0.2`
      - fallback remains bad:
        - `Ph`
        - **no fallback `EMR_EXTTEXTOUTW " "`**
        - `"(3 "`
    - `top_nudge = 0.3` and above
      - fallback becomes good:
        - `Ph`
        - `" "`
        - `"(3 "`
  - Summary table:
    - `0.05 -> bad`
    - `0.10 -> bad`
    - `0.20 -> bad`
    - `0.30 -> good`
    - `0.40 -> good`
    - `0.50 -> good`
    - `1.00 -> good`
    - `1.03 -> good`
    - `1.10 -> good`
    - `1.50 -> good`
- Conclusion:
  - The fallback-space bug is controlled by a **vertical-placement threshold** in packaged dual EMF.
  - This is stronger than the earlier `emSize` correlation:
    - clamping `EmfPlusFont emSize` back into the "good" bucket is **not sufficient**
    - forcing zero-layout rects is **not sufficient**
    - but nudging packaged text `top` by about `+0.3` page-space units **is sufficient**
  - The most accurate current statement is:
    - `source_bounds.top` / scale-axis switching matters because it perturbs packaged text `RectF.Y`
    - and the dual fallback converter has a sharp Y-threshold around that location
- Status:
  - Experiment produced a real narrowing result.
  - Product code should still be reverted; keep only the threshold finding.

### Experiment: global packaged-text top bias
- Hypothesis:
  - Since a packaged-text `top` nudge of about `+0.3` is sufficient to restore the missing reagent-line fallback space in the bad threshold case, maybe a small global packaged-text top bias can serve as a stable product fix.
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `draw_gdiplus_text_run()`
  - Apply a hardcoded `packaged_top_bias = 0.3` when `transform.emf_recording`
- Validation samples:
  - bad threshold case:
    - `tmp/fixed-selection/free-x-y-sweep/y-266_67.payload.json`
    - output: `y-266_67.topbias.emf`
  - full thiocyanation packaged output:
    - payload: `tmp/thiocyanation-source.chemcore.v62.payload.json`
    - output: `tmp/thiocyanation-source.topbias.emf`
    - docx: `tmp/thiocyanation-source.topbias.docx`
- Actual result:
  - The minimal bad case becomes good:
    - reagent sequence changes from
      - `Ph`
      - **no fallback `EMR_EXTTEXTOUTW " "`**
      - `"(3 "`
    - to
      - `Ph`
      - `" "`
      - `"(3 "`
  - The packaged `DrawString` rect Y values move exactly as expected, e.g. on the reagent line:
    - bad `top-0.2`: `rect.y = 1443.591796875`
    - good `top-0.3`: `rect.y = 1443.69189453125`
    - global top-bias output matches the good side of that threshold
  - However, on the full thiocyanation document, the global pixel overlap gets slightly worse:
    - direct top-left-aligned comparison against `tmp/thiocyanation-source.chemdraw.emf`
    - `ink_iou = 0.6213436096613667`
  - This is lower than the current packaged-text baseline from commit `0b11408` (`~0.6264`).
- Conclusion:
  - A **global** packaged-text top bias is too broad.
  - It fixes the narrow fallback-space bug, but it also nudges unrelated text that was already close to ChemDraw, reducing the full-document match slightly.
  - The useful lesson is:
    - the vertical-placement threshold is real
    - but the eventual fix must be **targeted**, not global
- Status:
  - Experiment is informative but not acceptable as the final product path.
  - Revert the code change; keep only the finding.
