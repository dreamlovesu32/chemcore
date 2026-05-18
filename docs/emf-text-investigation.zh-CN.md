## 2026-05-17 ???`frame-global3 + same-shell` ?????? centered text ?? bbox ??????

???????? molecule label ?????????

- `scripts/compare-full-text-boxes.py`

??????

- ? `chemcoreDocumentJson` ?? top-level `text` object ?? `payload.box + transform`
- ?? same-shell Word `CopyAsPicture` ????
- ???? box ???? ours / ChemDraw ??? ink bbox

???????

- ours: `tmp/frame-word-ab/frame-global3-shellchem.wordcopy.png`
- ref: `tmp/v28-wrapper-ablate10/v28-rerun.shape2.png`
- shift: `dx = -1`, `dy = 0`
- output: `tmp/frame-word-ab/frame-global3-text-object-compare.json`

### ???? text object ???

1. `obj_text_004`????? + ????
- `deltaDims = [0, -1]`
- `deltaTopLeft = [0, +1]`

2. `obj_text_005`?CH3CN / 420 nm ...?
- `deltaDims = [-1, 0]`
- `deltaTopLeft = [0, 0]`

3. `obj_text_006`??? 2 ??
- `deltaDims = [0, 0]`
- `deltaTopLeft = [0, 0]`

### ???????

? `frame-global3 + same-shell` ???????? free/centered text ??? ink bbox ???????

- `obj_text_004` ?? `1 px` ??????
- `obj_text_005` ?? `1 px` ????????
- `obj_text_006` ??? ChemDraw ?? bbox ????

????????????????????

1. molecule ?? label replay ????
2. ????? knockout ????
3. ????? replay residual?? `top_left_substrate` ????????

??????????????????????

- `top_right_product`
- `bottom_right_catalyst`
- molecule ?? `Ph / CN / NC / S`
- `top_left_substrate`


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

### Experiment: targeted packaged-text top bias for centered mixed-script lines
- Hypothesis:
  - Since the global packaged top bias fixed the synthetic bad case but hurt the full document slightly, maybe the right scope is narrower:
    - only packaged `EMF`
    - only `text_anchor == middle`
    - only lines that actually mix normal and sub/superscript runs
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - Add `preview_line_has_mixed_script(line_runs)`
  - In packaged `draw_gdiplus_text_run()`, apply `+0.3` top bias only when the enclosing line is both centered and mixed-script
- Validation samples:
  - synthetic bad threshold case:
    - `tmp/fixed-selection/free-x-y-sweep/y-266_67.payload.json`
    - output: `y-266_67.targeted.emf`
  - full thiocyanation packaged output:
    - payload: `tmp/thiocyanation-source.chemcore.v62.payload.json`
    - output: `tmp/thiocyanation-source.targeted.emf`
    - docx: `tmp/thiocyanation-source.targeted.docx`
- Actual result:
  - The synthetic bad threshold case becomes good:
    - reagent sequence becomes
      - `Ph`
      - `" "`
      - `"(3 "`
  - But the real document still does **not** improve in the area we actually care about.
  - Whole-document direct overlay against ChemDraw:
    - targeted: `ink_iou = 0.6213436096613667`
    - current baseline (`v62`): better than targeted
  - Local title/conditions region (`x=480..1450`, `y=220..560`) also does not improve:
    - `v62`: `iou = 0.2918534206767473`
    - targeted: `iou = 0.29002007737133345`
- Conclusion:
  - The synthetic top-threshold phenomenon is real, but it is **not** the dominant remaining production difference in the real thiocyanation packaged EMF.
  - Narrowing the top bias to centered mixed-script lines still does not improve the real title/conditions block.
  - Therefore the top-threshold path should be treated as:
    - a valid **mechanistic clue**
    - but **not** a final or even locally correct production fix
- Status:
  - Experiment failed as a production path.
  - Revert the code change and return to token-level analysis on the real document.

### Production fallback token baseline (v62 vs ChemDraw)
- Goal:
  - Stop overfitting to the synthetic `standalone " "` threshold case.
  - Re-anchor the investigation on the real packaged production file:
    - `tmp/thiocyanation-source.v62.emf.records.json`
    - `tmp/thiocyanation-source.chemdraw.emf.records.json`
- New reusable tool:
  - `scripts/emf-text-compare.mjs`
  - Purpose:
    - compare `EMR_EXTTEXTOUTW` token sequences between two `.records.json`
    - region-filtered
    - LCS-aligned, so missing standalone spaces stay visible instead of shifting every later row
- Generated reports:
  - title / conditions / reagent / `CH3CN` block:
    - `tmp/v62-chemdraw-title-conditions.md`
  - catalyst lower-right labels:
    - `tmp/v62-chemdraw-catalyst.md`
  - yield / `d.r.` block:
    - `tmp/v62-chemdraw-yield.md`
- Hard findings from the real production file:
  - Title / conditions / reagent block is **systematically low by `+2 px`** in our packaged fallback text.
  - Representative rows:
    - `4DPAIPN<sp>`: `(652,316)` vs `(650,314)` => `dx=+2`, `dy=+2`
    - `Cu(MeCN)`: `(541,347)` vs `(541,345)` => `dx=0`, `dy=+2`
    - `PF`: `(681,347)` vs `(681,345)` => `dx=0`, `dy=+2`
    - missing fallback space after `6`: ChemDraw has standalone `<sp>` at `(726,345)`; ours has no matching `EMR_EXTTEXTOUTW`
    - reagent line `PhthNCO / SCH / Ph / <sp> / (3<sp>` is also uniformly `dy=+2`
  - `CH3CN` line is a special case:
    - `CH`: `(670,457)` vs `(677,455)` => `dx=-7`, `dy=+2`
    - `CN<sp>`: `(720,457)` vs `(727,455)` => `dx=-7`, `dy=+2`
    - `(0.2<sp>)`: `(766,457)` vs `(773,455)` => `dx=-7`, `dy=+2`
    - following standalone `<sp>`: `(857,457)` vs `(864,455)` => `dx=-7`, `dy=+2`
  - Yield block is much closer:
    - first line mostly `dy=+1`
    - second line mostly `dy=+2`, `dx` near `0`
  - Catalyst lower-right structure labels are already very close:
    - average `dx ≈ 0.1`
    - average `dy ≈ 0.2`
- Interpretation:
  - The current dominant production mismatch is **not** the synthetic standalone-space threshold alone.
  - In the real packaged document, the remaining production error is primarily:
    - a broad fallback text baseline shift (`dy ≈ +2 px`) on the central title / reagent block
    - plus a line-specific anchor shift on `CH3CN` (`dx ≈ -7 px`)
  - Therefore the next production experiments should target:
    - packaged fallback baseline / anchor positioning on the real document
    - not more synthetic top-bounds-only fixes

### Production `EmfPlusDrawString` baseline (v62 vs ChemDraw)
- Goal:
  - Check whether the remaining production shift exists only in fallback `EMR_EXTTEXTOUTW`,
    or whether packaged `GDI+ DrawString` itself is already offset before dual fallback.
- New reusable tool:
  - `scripts/emf-drawstring-compare.mjs`
  - Purpose:
    - decode `EmfPlusDrawString` from the binary `EMF`
    - align by token text
    - compare `rect.x / rect.y` directly between ours and ChemDraw
- Generated reports:
  - title / conditions / reagent / `CH3CN` block:
    - `tmp/v62-chemdraw-drawstring-title-conditions.md`
  - catalyst + yield area:
    - `tmp/v62-chemdraw-drawstring-catalyst-yield.md`
- Hard findings:
  - The production shift is **already present in packaged `EmfPlusDrawString` geometry**.
  - Title / conditions / reagent block:
    - `4DPAIPN<sp>`: ours `(2444.110,1093.119)` vs ChemDraw `(2439.935,1087.000)` => `dx=+4.175`, `dy=+6.119`
    - `Cu(MeCN)`: `dx=+1.713`, `dy=+5.994`
    - `PF`: `dx=+1.275`, `dy=+5.994`
    - standalone `<sp>` after `6`: `dx=+1.133`, `dy=+5.994`
    - reagent line `PhthNCO / SCH / Ph / <sp> / (3<sp>`: mostly `dy≈+5.743`
  - `CH3CN` line is a distinct sub-case:
    - `CH`: ours `(2513.093,1621.923)` vs ChemDraw `(2540.469,1616.300)` => `dx=-27.376`, `dy=+5.623`
    - `CN<sp>`: `dx=-27.532`, `dy=+5.623`
    - `(0.2<sp>)`: `dx=-27.676`, `dy=+5.623`
    - trailing standalone `<sp>`: `dx=-27.964`, `dy=+5.623`
  - Yield area:
    - first line (`76%<sp>`, `yield,<sp>`, `94%<sp>`, `ee`) is only `dy≈+2.547`
    - second line (`d.r.<sp>`, `><sp>`, `20:1`) is `dy≈+2.449`
  - Catalyst structure labels are already close:
    - most `Ph` labels differ by only about `dx≈-1`, `dy≈-1`
- Interpretation:
  - The remaining production mismatch is **not fallback-only**.
  - For the main title / reagent block, packaged `DrawString` is already too low by about `+5.5 .. +6.1` in page space.
  - The fallback `EMR_EXTTEXTOUTW` `dy≈+2` is therefore downstream of an earlier packaged `DrawString` placement difference, not an isolated fallback bug.
  - `CH3CN` is not just a vertical issue; it is a separate line-specific anchor/placement problem with a large negative `dx`.
  - This narrows the next real target to:
    - packaged `DrawString` anchor / baseline placement
    - especially for the centered title / reagent block and `CH3CN`
    - rather than only trying to “repair” fallback tokenization

### Experiment: trim end-of-line trailing spaces from centered packaged line-width only
- Hypothesis:
  - The `CH3CN (0.2 M)  ` line is horizontally shifted because our packaged centering width includes end-of-line trailing spaces.
  - ChemDraw seems to draw those trailing spaces, but not count them when computing the centered line anchor.
  - If we keep token drawing unchanged, and only subtract the width of **line-end trailing spaces** from the centered line width, the `CH3CN` line should move back into place without perturbing other lines.
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - Add `gdiplus_line_trailing_space_trim(...)`
  - In `draw_gdiplus_text(...)`, for packaged `EMF` + centered/end-anchored text only:
    - `width = line_layout.width - trailing_trim`
    - token rendering itself remains unchanged
- Validation samples:
  - packaged production payload:
    - `tmp/thiocyanation-source.chemcore.v62.payload.json`
  - outputs:
    - `tmp/thiocyanation-source.v69.emf`
    - `tmp/thiocyanation-source.chemcore.v69.docx`
    - `tmp/thiocyanation-source.v69.emf.records.json`
  - token compare reports:
    - `tmp/v69-chemdraw-title-conditions.md`
    - `tmp/v69-chemdraw-drawstring-title-conditions.md`
  - whole-image pixel compare:
    - `tmp/v69-chemdraw-direct-compare/metrics.json`
    - `tmp/v69-chemdraw-direct-compare/overlay-top-left-aligned.png`
- Actual result:
  - The `CH3CN` line horizontal mismatch is essentially eliminated.
  - Fallback `EMR_EXTTEXTOUTW`:
    - before (`v62`):
      - `CH`: `(670,457)` vs `(677,455)` => `dx=-7`, `dy=+2`
      - `CN<sp>`: `dx=-7`, `dy=+2`
      - `(0.2<sp>)`: `dx=-7`, `dy=+2`
      - final `<sp>`: `dx=-7`, `dy=+2`
    - after (`v69`):
      - `CH`: `(678,457)` vs `(677,455)` => `dx=+1`, `dy=+2`
      - `CN<sp>`: `dx=0`, `dy=+2`
      - `(0.2<sp>)`: `dx=0`, `dy=+2`
      - final `<sp>`: `dx=+1`, `dy=+2`
  - `EmfPlusDrawString`:
    - before (`v62`):
      - `CH`: `dx=-27.376`, `dy=+5.623`
      - `CN<sp>`: `dx=-27.532`, `dy=+5.623`
      - `(0.2<sp>)`: `dx=-27.676`, `dy=+5.623`
      - final `<sp>`: `dx=-27.964`, `dy=+5.623`
    - after (`v69`):
      - `CH`: `dx=+0.383`, `dy=+5.623`
      - `CN<sp>`: `dx=+0.228`, `dy=+5.623`
      - `(0.2<sp>)`: `dx=+0.084`, `dy=+5.623`
      - final `<sp>`: `dx=-0.204`, `dy=+5.623`
  - The main title / reagent block remains unchanged; this experiment does **not** address its vertical offset.
  - Whole-image direct overlap vs ChemDraw improves significantly:
    - previous baseline (`v56`) `ink_iou = 0.5790806951869038`
    - `v69` `ink_iou = 0.6530264147832091`
- Conclusion:
  - The `CH3CN` line misalignment is a separate, line-width problem, not a baseline or tokenization problem.
  - ChemDraw-like behavior is better matched by:
    - drawing the trailing spaces as before
    - but excluding line-end trailing space width from centered packaged line-width computation
  - This is a valid production improvement and should be kept.
- Remaining problem after this experiment:
  - The title / conditions / reagent block is still too low in packaged `DrawString` by about `+5.7 .. +6.1` page-space units.
  - So the next investigation target remains:
    - packaged `DrawString` anchor / baseline placement for the centered title block
    - not the `CH3CN` line anymore

### Experiment: carry primitive baselineOffset directly into packaged DrawString top (all runs)
- Hypothesis:
  - Since `RenderPrimitive::Text.y` is generated as a baseline (`ty + baselineOffset + index * lineHeight`), packaged `GDI+ DrawString` should use the original text-object `baselineOffset` instead of a hard-coded `0.88` ascent factor.
- Code path touched:
  - `RenderPrimitive::Text` gains optional `baselineOffset`
  - text-object renderers populate it from payload (`baselineOffset` or `fontSize * 0.82`)
  - packaged `draw_gdiplus_text_run()` uses that value for `top`
- Result:
  - normal text runs became much closer to ChemDraw
  - but subscript runs were pushed too far upward
  - representative failures:
    - `4` in `PF6`: `dy = +3.063` vs ChemDraw when compared at `DrawString` level
    - `6` in `PF6`: `dy = +3.063`
    - reagent subscript `2`: `dy = +2.813`
    - `CH3CN` subscript `3`: `dy = +2.693`
  - naive whole-image top-left-crop IoU collapsed, so this is not an acceptable product path by itself
- Conclusion:
  - primitive `baselineOffset` is valuable for normal runs
  - but applying the same top reconstruction to sub/superscript runs is wrong
  - sub/superscript still need their own smaller-font ascent model

### Experiment: hybrid packaged DrawString top = normal uses primitive baselineOffset, sub/superscript uses font ascent
- Hypothesis:
  - The previous experiment suggests the right split is:
    - normal runs: use primitive `baselineOffset`
    - sub/superscript runs: use run-local font ascent (smaller font) + existing script baseline shift
- Code path touched:
  - keep `RenderPrimitive::Text.baselineOffset`
  - packaged `draw_gdiplus_text_run()`:
    - normal runs use `baselineOffset * gdiplus_text_scale(transform)`
    - sub/superscript runs use `font_px * ascent_ratio`
    - ascent ratio comes from `GdipGetFamily + GdipGetCellAscent + GdipGetEmHeight`
- Validation samples:
  - packaged production payload:
    - `tmp/thiocyanation-source.chemcore.v62.payload.json`
  - outputs:
    - `tmp/thiocyanation-source.v71.emf`
    - `tmp/thiocyanation-source.chemcore.v71.docx`
    - `tmp/thiocyanation-source.v71.emf.records.json`
  - reports:
    - `tmp/v71-chemdraw-title-conditions.md`
    - `tmp/v71-chemdraw-drawstring-title-conditions.md`
    - `tmp/v71-chemdraw-direct-compare/metrics.json`
- Actual result:
  - packaged `DrawString` title/reagent normal runs improve substantially:
    - `4DPAIPN<sp>`: `dy` from `+6.119` -> `+1.622`
    - `Cu(MeCN)`: `dy` from `+5.994` -> `+1.497`
    - `PF`: `dy` from `+5.994` -> `+1.497`
    - reagent `PhthNCO`: `dy` from `+5.743` -> `+1.246`
  - subscript runs are no longer wildly wrong:
    - `4` in `PF6`: `dy` from `-21.521` (failed all-run baselineOffset variant) -> `+3.063`
    - `6` in `PF6`: `dy` from `-21.521` -> `+3.063`
    - reagent `2`: `dy` from `-21.772` -> `+2.813`
    - `CH3CN` subscript `3`: `dy` from `-21.892` -> `+2.693`
  - `CH3CN` line keeps the earlier horizontal centering fix.
  - Important metric nuance:
    - top-left-cropped IoU is misleading here because changing topmost text also changes the crop anchor
    - on fixed-canvas whole-page compare, `v71` is better than `v69`:
      - `v69` canvas IoU = `0.27131386629888554`
      - `v71` canvas IoU = `0.2831567292741713`
- Conclusion:
  - carrying primitive `baselineOffset` is directionally correct, but only for normal runs
  - hybrid normal-baselineOffset + subscript-ascent is materially better than both:
    - the old constant-`0.88` packaged path
    - the failed all-run baselineOffset path
  - remaining main gap is still the centered title/conditions block, but the vertical error band is now much smaller and more structured

### Experiment: packaged centered DrawString upward bias (v72)
- Hypothesis:
  - After `v71`, the remaining packaged `DrawString` gap is a structured vertical bias:
    - normal centered runs sit about `+1.1 .. +1.6` page units too low
    - subscript runs sit about `+2.7 .. +3.1` page units too low
  - A narrow packaged-only top correction should help if it is applied only to centered text and scales with `font_px`.
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `draw_gdiplus_text_run()` now receives `text_anchor`
  - when `transform.emf_recording && text_anchor == middle`:
    - all runs get `font_px * 0.012` upward bias
    - sub/superscript runs get an extra `font_px * 0.02`
- Validation samples:
  - outputs:
    - `tmp/thiocyanation-source.v72.emf`
    - `tmp/thiocyanation-source.chemcore.v72.docx`
    - `tmp/thiocyanation-source.v72.emf.records.json`
  - reports:
    - `tmp/v72-chemdraw-title-conditions.md`
    - `tmp/v72-chemdraw-drawstring-title-conditions.md`
    - `tmp/v72-chemdraw-drawstring-catalyst-yield.md`
    - `tmp/v72-chemdraw-catalyst-yield-fallback.md`
- Actual result:
  - packaged `DrawString` title / conditions become much tighter vertically:
    - `4DPAIPN<sp>`: `dy +1.622 -> +0.423`
    - `Cu(MeCN)`: `+1.497 -> +0.298`
    - `4` in `PF6`: `+3.063 -> +0.666`
    - `6` in `PF6`: `+3.063 -> +0.666`
    - reagent `PhthNCO`: `+1.246 -> +0.047`
    - reagent subscript `2`: `+2.813 -> +0.415`
    - `CH3CN` normal runs: `+1.126 -> -0.073`
    - `CH3CN` subscript `3`: `+2.693 -> +0.295`
  - yield / catalyst block also improves or stays flat:
    - `76%<sp>`: `dy +1.047 -> -0.152`
    - `d.r.<sp>`: `+0.949 -> -0.250`
    - catalyst `4DPAIPN`: `+0.534` unchanged in the acceptable range
  - fixed-canvas pixel overlap improves:
    - full page IoU: `0.2831567292741713 -> 0.287132406025894`
    - title region IoU: `0.4285628526833954 -> 0.43884717849358196`
    - yield region IoU: `0.24084778420038536 -> 0.24883540372670807`
    - catalyst / ligand regions remain effectively unchanged
  - Known side effect:
    - fallback token compare now loses the standalone trailing `<sp>` after reagent `Ph` and after `M)` on the `CH3CN` line.
    - Despite that token-level regression, whole-image fixed-canvas overlap still improves.
- Conclusion:
  - The residual `v71` gap was indeed dominated by a packaged centered top-bias problem.
  - A narrow packaged-only `font_px`-scaled correction improves the real image more than it harms it.
  - This is a valid new baseline, but the fallback token side effect means the next step should be a follow-up cleanup, not the final stop.

### Experiment: packaged centered DrawString zero-layout / point-style on top of v72
- Hypothesis:
  - Since ChemDraw's packaged title/conditions `EmfPlusDrawString` records use `layoutRect = 0 x 0`, applying the same zero-layout shape to our already-improved `v72` centered packaged text might further reduce the residual difference.
- Code path touched:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - Only for `transform.emf_recording && text_anchor == middle`, set `RectF.Width = 0` and `RectF.Height = 0` in `draw_gdiplus_text_run()`.
- Validation samples:
  - outputs:
    - `tmp/thiocyanation-source.v73.emf`
    - `tmp/thiocyanation-source.v73.emf.records.json`
    - `tmp/v73-chemdraw-title-conditions.md`
    - `tmp/v73-chemdraw-drawstring-title-conditions.md`
- Actual result:
  - Record shape changes as expected:
    - `v72` title `DrawString` records have nonzero `layoutRect` (`w ~= 816`, `h ~= 144` for `4DPAIPN `)
    - `v73` corresponding records become `layoutRect = 0 x 0`
  - But geometry does **not** change:
    - `DrawString` `x/y` for compared title / reagent / `CH3CN` runs remain identical to `v72`
    - fixed-canvas image IoU is identical to `v72`
      - full page: `0.287132406025894`
      - title region: `0.43884717849358196`
      - yield region: `0.24883540372670807`
  - Therefore this experiment only changes record shape, not visible placement.
- Conclusion:
  - Matching ChemDraw's `layoutRect = 0 x 0` is **not sufficient** once our own `x/y` anchor computation is already fixed to the current packaged path.
  - The remaining gap is not in rect size anymore; it is in the computed anchor positions / fallback behavior.
  - Revert the code change and keep only the finding.
### Experiment: packaged centered trailing-space advance blend (v74/v75)
- Hypothesis:
  - The remaining horizontal drift in `v72` is dominated by a few centered packaged tokens whose measured advance is too small, especially tokens that end with a visible character plus trailing space (for example `4DPAIPN `, `L `, `3W, `).
  - Replacing all centered packaged layout widths with GDI extents (`v74`) over-corrects and integerizes the whole line.
  - A narrower variant that only blends GDI extents into trailing-space tokens (`v75`) may keep GDI+ geometry while fixing the worst under-measured steps.
- Code paths touched (experiment only, reverted):
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `v74`: packaged centered `gdiplus_text_layout()` used GDI extents for all centered runs.
  - `v75`: packaged centered layout kept GDI+ widths by default, but for tokens ending with whitespace and containing visible characters, blended `advance = gdiplus + 0.5 * (gdi - gdiplus)`.
- Validation samples:
  - outputs:
    - `tmp/thiocyanation-source.v74.emf`
    - `tmp/thiocyanation-source.v75.emf`
  - reports:
    - `tmp/v74-chemdraw-drawstring-title-conditions.md`
    - `tmp/v74-chemdraw-title-conditions.md`
    - `tmp/v75-chemdraw-drawstring-title-conditions.md`
    - `tmp/v75-chemdraw-title-conditions.md`
- Actual result:
  - `v74` significantly tightens packaged DrawString x positions:
    - `4DPAIPN `: `dx +4.175 -> +0.065`
    - `Cu(MeCN)`: `+1.713 -> -0.363`
    - `3W, `: `+3.000 -> +0.121`
    - `10 `: `-2.685 -> -0.446`
  - But fixed-canvas image IoU gets worse than `v72`:
    - full page: `0.28713 -> 0.28261`
    - title region: `0.35999 -> 0.33549`
    - yield region: `0.24730 -> 0.25541`
  - `v75` partially recovers from that over-correction:
    - `4DPAIPN `: `+4.175 -> +2.183`
    - `Cu(MeCN)`: `+1.713 -> +1.011`
    - `3W, `: `+3.000 -> +1.668`
    - `10 `: `-2.685 -> -1.457`
  - But `v75` still does not beat `v72` in actual pixels:
    - full page: `0.28713 -> 0.28450`
    - title region: `0.35999 -> 0.35074`
    - yield region: `0.24730 -> 0.24865`
- Conclusion:
  - GDI extents are useful as a diagnostic: they prove the centered packaged horizontal residual is tied to a few under-measured trailing-space tokens.
  - But even a narrow trailing-space blend does not outperform the `v72` packaged baseline in real pixels.
  - Therefore the remaining issue is not solved by simply switching the width source from GDI+ to GDI; this line should stay reverted and be treated as measurement evidence rather than product logic.

### Experiment: packaged trailing-space width via GDI+ MeasureCharacterRanges (v76)
- Hypothesis:
  - The remaining centered packaged horizontal residual is concentrated in tokens that end with visible glyphs plus trailing spaces.
  - Replacing the packaged trailing-space width path with `GDI+ MeasureCharacterRanges`, while staying on the GDI+ path rather than switching to GDI extents, may preserve ChemDraw-like geometry and improve the record chain at the same time.
- Code paths touched (experiment only, reverted):
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - Added a packaged-only branch inside `gdiplus_text_run_advance()`:
    - for tokens ending with whitespace and containing visible glyphs, first try `gdiplus_measure_text_width_ranges(...)`
    - otherwise fall back to the existing `gdiplus_measure_text_width(...)`
  - Added `GdipMeasureCharacterRanges` / `GdipSetStringFormatMeasurableCharacterRanges` / region-bound helper code
- Validation samples:
  - outputs:
    - `tmp/thiocyanation-source.v76.emf`
    - `tmp/thiocyanation-source.v76.emf.records.json`
  - reports:
    - `tmp/v76-chemdraw-drawstring-title-conditions.md`
    - `tmp/v76-chemdraw-title-conditions.md`
- Actual result:
  - Record-chain level comparisons become much tighter:
    - packaged `DrawString` title/conditions average residual shrinks to roughly `avg dx = 0.091`, `avg dy = 0.139`
    - fallback `EMR_EXTTEXTOUTW` title/conditions average residual shrinks to roughly `avg dx = 0.552`, `avg dy = 0.103`
    - `PF6 -> " " -> "(5 "` sequence is present and nearly aligned:
      - `" "` at `727` vs ChemDraw `726`
      - `"(5 "` at `734` vs ChemDraw `733`
  - But the visible pixels do **not** improve at all:
    - fixed-canvas whole-page IoU stays exactly the same as `v72`: `0.287132406025894`
    - title region IoU stays `0.359989856843173`
    - yield region IoU stays `0.24729616386350595`
    - catalyst / ligand regions also remain unchanged
- Conclusion:
  - `MeasureCharacterRanges` can change packaged text record geometry so that it *looks* much closer to ChemDraw at the `DrawString` / `EXTTEXTOUTW` level.
  - However, those record-level improvements do **not** translate into any visible pixel improvement in the rendered EMF.
  - Therefore this path is not a winning product fix; keep it reverted and treat it as another negative result that separates record-chain similarity from actual image similarity.

### Experiment: packaged centered text via GDI+ DrawDriverString (v77)
- Hypothesis:
  - The earlier harness evidence suggested that a true point-style / driver-style GDI+ text API may preserve the critical fallback spaces better than `DrawString(RectF, ...)`, while still staying on a GDI+ path.
  - A narrow packaged-only experiment that switches only centered packaged text from `DrawString` to `DrawDriverString` may therefore move us closer to ChemDraw.
- Code paths touched (experiment only, reverted):
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - Added a packaged-only `CHEMCORE_OFFICE_EXPERIMENT_PACKAGED_DRIVER_TEXT` branch:
    - only active for `transform.emf_recording && text_anchor == middle`
    - build per-code-unit positions from cumulative GDI+ prefix widths
    - draw the run with `GdipDrawDriverString(...)`
  - Added `GdipDrawDriverString` imports and a temporary helper `draw_gdiplus_driver_text_run(...)`
- Validation samples:
  - outputs:
    - `tmp/thiocyanation-source.v77.emf`
    - `tmp/thiocyanation-source.v77.emf.records.json`
  - reports:
    - `tmp/v77-chemdraw-title-conditions.md`
    - packaged image compare against `tmp/thiocyanation-source.chemdraw.emf`
- Actual result:
  - The packaged centered text is no longer recorded as `EmfPlusDrawString`; it becomes `EmfPlus_0x4036` / `DrawDriverString`.
  - Fallback `EMR_EXTTEXTOUTW` tokenization becomes much more ChemDraw-like:
    - `PF6 -> " " -> "(5 "` is present again
    - `Ph -> " " -> "(3 "` also appears again
    - title/conditions fallback compare stays near-perfect in `x`, with a consistent `dy = +1 px`
  - But visible pixels get dramatically worse:
    - fixed-canvas whole-page IoU drops from the `v72` baseline `0.28713` to `0.24992`
    - title region drops to `0.24706`
    - yield region stays only around `0.24801`
    - catalyst / ligand regions remain effectively unchanged because they do not use this path
- Conclusion:
  - `DrawDriverString` can restore the desired fallback token chain, but the visible EMF+ rendering it produces is substantially worse than the current `DrawString` baseline.
  - Therefore this is **not** a shippable product path in its current form.
  - Keep the investigation tooling and harness support, but revert the packaged product experiment.

### Follow-up tooling: line-level fallback comparison
- Added:
  - `scripts/emf-text-line-compare.mjs`
- Reason:
  - token-level `dx/dy` summaries had become misleading;
  - we needed a metric closer to what the user actually sees, namely:
    - per-line `left`
    - per-line `right`
    - per-line `width`
    - per-line `center`
- For the title/conditions block in the current baseline, the line-level picture is:
  - `4DPAIPN (2 mol%)`:
    - `left +2`, `right +0`, `width -2`, `center +1`
  - `Cu(MeCN)₄PF₆ (5 mol%), L (7 mol%)`:
    - effectively aligned
  - `CH₃CN (0.2 M)`:
    - `left +1`, `right -7`, `width -8`, `center -3`
  - `420 nm 3W, 10 °C, 24 h`:
    - `left +2`, `right +0`, `width -2`, `center +1`
- This shows the remaining packaged residual is no longer “all centered text is wrong”.
- It is concentrated in a few centered **plain-text** lines, while the mixed normal/subscript lines are already much closer.

### Follow-up analysis: local rendered text still prefers a small translation
- On rendered PNGs from the current baseline, local text-only crops still get a visibly better IoU after a small translation:
  - `title_text_only`: best around `(dx=+5, dy=-2)` in the scale-2 rendered PNG
  - `yield_text_only`: best around `(dx=+6, dy=-1)`
  - `catalyst_text_only`: best around `(dx=+6, dy=-1)`
  - `ligand_label_only`: best around `(dx=+4, dy=-2)`
- Important interpretation:
  - this is **not** a remaining global scaling problem;
  - it is a packaged text placement / ink-placement residual that survives after the larger size and OLE sizing fixes.

### Experiment platform additions
- Added investigation-only toggles so we can run narrow A/B tests without repeatedly rewriting product logic:
  - `CHEMCORE_EMF_PACKAGED_TEXT_DISABLE_TRAILING_TRIM`
  - `CHEMCORE_EMF_PACKAGED_TEXT_DISABLE_NOFITBLACKBOX`
  - `CHEMCORE_EMF_PACKAGED_TEXT_GRIDFIT`
  - `CHEMCORE_EMF_PACKAGED_PIXEL_OFFSET_HIGHQUALITY`
  - `CHEMCORE_EMF_PACKAGED_CENTERED_PLAIN_GDI_WIDTH`
- These do not change default behavior.

### Experiment: disable packaged `NoFitBlackBox`
- Variant:
  - `tmp/thiocyanation-source.v81-nofitoff.emf`
- Hypothesis:
  - the remaining title drift might come from our glyph ink extending further outside the fallback/layout bounds than ChemDraw’s.
- Result:
  - no measurable change in the title/yield text-only crops
  - no measurable change in the line-level fallback metrics
- Conclusion:
  - `NoFitBlackBox` is **not** the active lever for the current packaged residual.

### Experiment: disable packaged centered trailing-trim
- Variant:
  - `tmp/thiocyanation-source.v82-trimoff.emf`
- Hypothesis:
  - the centered-layout trailing-space trim might be responsible for the apparent centered drift.
- Result:
  - the problematic title/yield crops did not improve
  - `CH₃CN (0.2 M)` became dramatically worse at fallback line level:
    - `left -7`, `right -14`, `center -10.5`
- Conclusion:
  - centered trailing-trim is **not** the source of the current visible drift;
  - disabling it actively damages the `CH₃CN` line.

### Experiment: packaged `TextRenderingHintAntiAliasGridFit`
- Variant:
  - `tmp/thiocyanation-source.v84-gridfit.emf`
- Hypothesis:
  - the remaining difference might be primarily a packaged ink rasterization / hinting issue.
- Result:
  - title and yield crops both got worse
  - the title second-line standalone space regressed again
- Conclusion:
  - packaged `AntiAliasGridFit` is **not** ChemDraw’s winning path for this problem.

### Experiment: packaged `PixelOffsetModeHighQuality`
- Variant:
  - `tmp/thiocyanation-source.v85-pixeloffset.emf`
- Hypothesis:
  - ChemDraw emits `EmfPlusSetPixelOffsetMode flags=2`; perhaps our packaged text drift is caused by not recording the same state.
- Result:
  - title and yield crops both got worse
  - the best local shifts moved from roughly `(5,-2)/(6,-1)` to `(6,-1)/(7,0)`
- Conclusion:
  - simply copying ChemDraw’s global pixel-offset mode into our packaged recording path does **not** solve the remaining text mismatch.

### Experiment: centered plain-text lines use GDI widths
- Variant:
  - `tmp/thiocyanation-source.v86-plaingdi.emf`
- Hypothesis:
  - since the remaining line-level errors are concentrated in centered **plain-text** lines, a narrow version of the old `v74` idea might help:
    - packaged only
    - centered only
    - only lines whose runs are all plain (no script)
    - use GDI run widths there
- Result:
  - in the current code structure, this variant produced **no measurable change** in:
    - title/yield/catalyst text-only crops
    - title line-level fallback metrics
- Conclusion:
  - the historical `v74` effect cannot be recovered by simply reapplying “plain-line GDI widths” inside the current packaged path.

### Current interpretation after these follow-up experiments
- We have now ruled out several obvious visible-path levers:
  - `NoFitBlackBox`
  - packaged centered trailing-trim
  - packaged `AntiAliasGridFit`
  - packaged `PixelOffsetModeHighQuality`
  - a narrow “plain centered line uses GDI widths” replay
- The strongest remaining interpretation is:
  - the residual is concentrated in a few centered plain-text lines;
  - mixed normal/subscript lines are already close enough that broad fixes tend to over-correct them;
  - the next worthwhile step is to keep targeting those centered plain lines, rather than re-opening all packaged text behavior.

### Follow-up tooling: rendered PNG line-level ink comparison
- Added:
  - `scripts/png-text-line-compare.py`
- Reason:
  - record-level `DrawString` / `EMR_EXTTEXTOUTW` diffs had become insufficient;
  - we needed to compare the **actual visible ink boxes** that the user sees in rendered PNGs.
- On the current packaged baseline (`v80`) versus ChemDraw, title/conditions region (`650,120,1850,560`) shows:
  - row 0: `left -5`, `right -6`, `center_x -5.5`, `center_y +1.0`
  - row 1: `left -5`, `right -6`, `center_x -5.5`, `center_y +2.0`
  - row 2: `left -5`, `right -11`, `center_x -8.0`, `center_y -0.5`
  - row 3: `left -5`, `right -5`, `center_x -5.0`, `center_y +1.0`
  - row 4: `left -4`, `right -7`, `center_x -5.5`, `center_y +1.5`
- This is the first direct metric that matches the user's eye:
  - the visible centered title block is consistently a few pixels **left** of ChemDraw,
  - even when record-level token chains already look nearly aligned.

### Finding: rendered-ink residual is not only a text problem
- Sanity-check region bboxes on rendered PNGs show the same leftward residual on non-text content:
  - top-left substrate region:
    - ours bbox `163,160 -> 899,326`
    - ChemDraw bbox `168,160 -> 899,324`
    - left edge is `-5 px`
  - central arrow region:
    - ours bbox `770,240 -> 1799,359`
    - ChemDraw bbox `775,240 -> 1799,359`
    - left edge is again `-5 px`
  - top-right product region, by contrast, is essentially aligned in bbox
- Conclusion:
  - the remaining packaged-vs-ChemDraw gap is **not purely text-layout-specific**
  - at least part of it lives at the lower rendered-ink / rasterization level.

### Finding: the visible leftward residual is robust to threshold choice
- On the title/conditions region, repeating the rendered-ink bbox compare with RGB thresholds
  - `740`
  - `700`
  - `660`
  - `620`
  - `580`
  produces essentially the same row-level left drift:
  - rows remain about `-5 px` left of ChemDraw
  - vertical drift remains around `+1 ~ +2 px`
- Conclusion:
  - this is **not** just a loose anti-aliased fringe being counted by an overly permissive threshold.
  - The residual survives into darker ink thresholds and should be treated as a real visible-placement / rasterization difference.

### Finding: packaged `DrawString` token anchors are already close
- Using `scripts/emf-drawstring-compare.mjs` on the title/conditions block of `v80` versus ChemDraw:
  - `4DPAIPN `: `dx +4.175`, `dy +0.423`
  - `Cu(MeCN)`: `dx +1.713`, `dy +0.298`
  - `PF` / `6` / standalone `" "` / `"(5 "` are all within roughly `1 px`
  - `TMSCN (4 eq.)`, `PhthNCO₂SCH₂Ph (3 eq.)`, and `CH₃CN (0.2 M)` tokens are similarly close in `DrawString` space
- Important contrast:
  - visible rendered line boxes are still about `5 px` left of ChemDraw
  - therefore the remaining problem is **not explained by token anchor positions alone**
- This sharply narrows the issue to:
  - `layoutRect` semantics
  - ink placement inside the `DrawString` layout
  - or lower-level rasterization state

### Experiment matrix: packaged `SetAntiAliasMode`
- Added investigation-only env override:
  - `CHEMCORE_EMF_PACKAGED_SMOOTHING_MODE_VALUE`
- Variants tested:
  - `mode=3`
  - `mode=4`
  - `mode=5`
  - `mode=6`
- Emitted `EmfPlusSetAntiAliasMode` flags:
  - `mode=3 -> flags=6`
  - `mode=4 -> flags=9`
  - `mode=5 -> flags=11`
  - `mode=6 -> no useful improvement and no visible recovery`
- Actual visible result on title/conditions:
  - all four variants preserve essentially the same rendered-ink line boxes as baseline
  - no meaningful recovery toward ChemDraw
- Conclusion:
  - packaged anti-alias mode, at least via direct `GdipSetSmoothingMode(...)` numeric overrides, is **not** the missing lever for the current residual.

### Experiment matrix: packaged `SetPixelOffsetMode`
- Added investigation-only env override:
  - `CHEMCORE_EMF_PACKAGED_PIXEL_OFFSET_MODE_VALUE`
- Variants tested:
  - `mode=1`
  - `mode=2`
  - `mode=3`
  - `mode=4`
- Emitted `EmfPlusSetPixelOffsetMode` flags:
  - `mode=1 -> flags=1`
  - `mode=2 -> flags=2` (matches ChemDraw's recorded value)
  - `mode=3 -> flags=3`
  - `mode=4 -> flags=4`
- Actual visible result:
  - `mode=1` and `mode=3` are effectively indistinguishable from baseline
  - `mode=2` and `mode=4` actually make the title/conditions left drift slightly worse (e.g. row centers move from about `-5.5 px` to `-6.5 px`)
- Conclusion:
  - even matching ChemDraw's `EmfPlusSetPixelOffsetMode flags=2` **does not** recover the remaining visible packaged-vs-ChemDraw difference.
  - Therefore pixel-offset mode is not the missing explanation either.

### Current interpretation after rendered-ink investigation
- We now have a stronger picture than before:
  - record-level token placement for packaged text is already very close in many key lines
  - but visible rendered ink still drifts left/down slightly
  - and a similar left-edge drift appears in at least some non-text regions (substrate, arrow)
- This means the remaining problem has moved below the previous text-token / fallback layer.
- The most likely remaining classes are now:
  - packaged `DrawString` layout-rect semantics versus ChemDraw's zero-layout / point-like recording
  - or deeper EMF+/GDI+ rasterization behavior that affects both text and vector ink placement.

### New tooling: direct SVG-vs-EMF raster comparison
- Added:
  - `scripts/render-svg-preview.py`
  - `scripts/png-ink-bbox-compare.py`
  - `scripts/png-best-shift.py`
- Purpose:
  - compare **our own current SVG** against **our own packaged EMF** for the same payload;
  - isolate whether the residual is introduced before EMF export or by packaged EMF replay/rasterization itself.

### Finding: packaged EMF already diverges from current chemcore SVG on pure-vector fixtures
- Three self-comparison fixtures were run:
  - `molecule`
  - `arrows-acs`
  - `assets-acs`
- Method:
  - export payload JSON with current engine
  - render packaged EMF to PNG with `render-emf-preview.mjs`
  - extract payload `svg`, rasterize with `render-svg-preview.py`
  - compare visible ink bbox with `png-ink-bbox-compare.py`

#### `molecule`: packaged EMF vs current chemcore SVG
- EMF bbox:
  - `left=9 top=25 right=917 bottom=309`
- current SVG bbox:
  - `left=16 top=16 right=923 bottom=329`
- Difference:
  - `left -7`
  - `right -6`
  - `center_x -6.5`
  - `center_y -5.5`
  - `height -29`

#### `arrows-acs`: packaged EMF vs current chemcore SVG
- EMF bbox:
  - `left=5 top=30 right=488 bottom=855`
- current SVG bbox:
  - `left=17 top=16 right=499 bottom=919`
- Difference:
  - `left -12`
  - `right -11`
  - `center_x -11.5`
  - `center_y -25.0`
  - `height -78`

#### `assets-acs`: packaged EMF vs current chemcore SVG
- EMF bbox:
  - `left=7 top=14 right=648 bottom=1305`
- current SVG bbox:
  - `left=15 top=16 right=655 bottom=1340`
- Difference:
  - `left -8`
  - `right -7`
  - `center_x -7.5`
  - `center_y -18.5`
  - `height -33`

### Interpretation: this is no longer a ChemDraw-only mismatch
- These fixtures show that the packaged EMF residual already exists **before** bringing ChemDraw into the loop.
- In other words:
  - current chemcore SVG and current packaged EMF for the same payload do **not** place visible ink identically;
  - the residual is therefore not just a ChemDraw oracle difference;
  - it is at least partly introduced somewhere in the packaged preview chain (preview geometry, EMF recording, or EMF replay), not solely by ChemDraw-vs-chemcore document semantics.
- This shifts the search strategy:
  - further text-only fallback work will not be sufficient on its own;
  - we need a lower-level packaged EMF visible-ink harness that can reproduce the same shift on pure vector scenes.

### Negative control: removing right-side preview padding does not explain the shift
- Added investigation-only env override:
  - `CHEMCORE_PREVIEW_SOURCE_RIGHT_PADDING_PT`
- Hypothesis:
  - perhaps the `16 pt` right padding workaround was shrinking the entire scene and causing the visible leftward residual.
- Result on `molecule` with `padding=0`:
  - baseline packaged EMF vs current SVG:
    - `left -7`, `right -6`, `center_x -6.5`
  - `padding=0` packaged EMF vs current SVG:
    - `left -7`, `right -25`, `center_x -16.0`
- Result on `arrows-acs` with `padding=0`:
  - identical to baseline:
    - `left -12`, `right -11`, `center_x -11.5`, `center_y -25.0`
- Conclusion:
  - the right-padding workaround is **not** the primary cause of the pure-vector residual;
  - on `molecule` it actually makes the horizontal mismatch worse,
  - and on `arrows-acs` it changes essentially nothing.

### Added direct packaged-preview geometry introspection
- Added a debug command in `chemcore-office`:
  - `--write-preview-bounds-payload <payload.json> <output.json>`
- Purpose:
  - dump the packaged preview geometry chain directly from product code instead of inferring it from EMF headers.
- Report fields:
  - `visibleBoundsSvgPx`
  - `svgViewBoxBoundsSvgPx`
  - `sourceBoundsSvgPx`
  - `frameBoundsHimetric`
  - `drawBoundsLogical`
  - `sourceBoundsMode`
  - `rightPaddingPt`

### Finding: `preview_source_bounds()` is asymmetric relative to the SVG viewBox
- On all three pure-vector fixtures (`molecule`, `arrows-acs`, `assets-acs`), the packaged source bounds are **not** the same as the SVG viewBox.
- Example (`molecule`):
  - `visibleBoundsSvgPx = [266.84, 806.46, 721.13, 962.99]`
  - `svgViewBoxBoundsSvgPx = [258.84, 798.46, 729.13, 970.99]`
  - `sourceBoundsSvgPx = [266.84, 806.46, 745.13, 962.99]`
- This means the default packaged source bounds are:
  - cropped on the left by about `8 px`
  - cropped on the top by about `8 px`
  - extended on the right by the padding workaround
  - cropped on the bottom by about `8 px`
- So the packaged preview is **not** replaying the same source rectangle as the current chemcore SVG.

### Source-bounds mode matrix (`current / visible / svg / union`)
- Added investigation-only env override:
  - `CHEMCORE_PREVIEW_SOURCE_BOUNDS_MODE`
  - values:
    - `current`
    - `visible`
    - `svg`
    - `union`
- Goal:
  - test whether the packaged EMF residual is mainly caused by the source-bounds rectangle choice.

#### Pure-vector fixtures against current chemcore SVG
- `molecule`
  - `current`: `best_iou = 0.841335`
  - `visible`: `best_iou = 0.880177`
  - `svg`: `best_iou = 0.853187`
  - `union`: `best_iou = 0.853187`
- `arrows-acs`
  - `current`: `best_iou = 0.950161`
  - `visible`: `best_iou = 0.949947`
  - `svg`: `best_iou = 0.917337`
  - `union`: `best_iou = 0.917337`
- `assets-acs`
  - `current`: `best_iou = 0.870225`
  - `visible`: `best_iou = 0.894418`
  - `svg`: `best_iou = 0.874157`
  - `union`: `best_iou = 0.874157`

Interpretation:
- `svg` / `union` do **not** magically restore alignment with current chemcore SVG.
- `visible` is often slightly better than `current`, but not consistently enough to call it the final answer.
- So `source_bounds` choice matters, but it is **not the only layer** creating the visible residual.

### Word real replay vs `System.Drawing` EMF replay
- A direct Word COM `CopyAsPicture` path was re-established for simple fixtures.
- For `molecule`:
  - the copied Word image is much smaller in raw pixels than the `System.Drawing`-rendered EMF PNG,
  - but after scaling to match content width, it aligns **much better with the packaged EMF** than with the current chemcore SVG.
- Approximate normalized comparison:
  - `word-copy` vs packaged EMF:
    - `best_iou ≈ 0.531`
  - `word-copy` vs current chemcore SVG:
    - `best_iou ≈ 0.510`

Interpretation:
- `render-emf-preview.mjs` is not a perfect stand-in for Word,
- but for simple packaged-preview geometry it is still a **useful proxy**;
- the packaged EMF and Word replay seem to share the same class of residual more than either matches the current SVG.

### `right-edge-ph` minimal sample: current mode is over-padding
- A new mode matrix was run on the minimal `right-edge-ph` fixture.
- Packaged EMF raster results:
  - `current`: right margin `22 px`
  - `visible`: right margin `3 px`
  - `svg`: right margin `0 px`
  - `union`: right margin `0 px`
- Then the same fixture was replayed through **Word COM CopyAsPicture**:
  - `visible` mode still kept the right-side `Ph` visible in the minimal sample.

Interpretation:
- `current` is **not** the minimal safe source-bounds policy for this right-edge text case.
- It leaves substantially more right breathing room than the minimal sample needs.

### Full `thiocyanation-source`: visible mode is still too aggressive
- The same mode experiment was then lifted back to the full production document.
- Result:
  - `visible` mode still reintroduced the right-side clipping in Word live replay / `CopyAsPicture`.
- So:
  - `visible` is good enough for the tiny `right-edge-ph` sample,
  - but **not** good enough for the full mixed-content document.

### Full `thiocyanation-source`: right-padding sweep (`4 / 8 / 12 / 16`)
- We then held `sourceBoundsMode = current` and only swept:
  - `CHEMCORE_PREVIEW_SOURCE_RIGHT_PADDING_PT = 4 / 8 / 12 / 16`
- Comparing packaged EMF against the existing ChemDraw EMF:
  - `pad=4`: `best_iou = 0.779106`
  - `pad=8`: `best_iou = 0.763541`
  - `pad=12`: `best_iou = 0.785945`
  - `pad=16`: `best_iou = 0.773699`

Interpretation:
- `16 pt` is **not** obviously optimal.
- In this experiment, `12 pt` gave the best packaged-EMF overlap against ChemDraw.
- `4 pt` was also competitive.
- So the current `16 pt` workaround likely contains some real over-padding.

### Current synthesis
- Two things now look simultaneously true:
  1. the packaged preview chain does **not** use the same source rectangle as the current chemcore SVG;
  2. the current right-side padding workaround is **larger than necessary**, at least in some cases.
- But the experiments also show:
  - simply switching to `svg`/`union` source bounds is not enough;
  - simply switching to `visible` source bounds breaks the full production document.
- So the next search should not be “pick one global rectangle and hope”.
- The more promising direction is:
  - keep investigating the packaged preview geometry,
  - but separate:
    - the **base source rectangle policy**
    - from the **text-overhang compensation policy**,
  - instead of baking both into one global `current` rule.

### Additional right-padding sweep (`0 / 2 / 4 / 6 / 8 / 10 / 12 / 14 / 16`)
- The full production document was then swept more densely with:
  - `sourceBoundsMode = current`
  - `rightPaddingPt = 0, 2, 4, 6, 8, 10, 12, 14, 16`
- Against the current ChemDraw EMF, the packaged-EMF overlap came out as:
  - `pad=0`: `best_iou = 0.781718`
  - `pad=2`: `best_iou = 0.764830`
  - `pad=4`: `best_iou = 0.779087`
  - `pad=6`: `best_iou = 0.781622`
  - `pad=8`: `best_iou = 0.763541`
  - `pad=10`: `best_iou = 0.792161`
  - `pad=12`: `best_iou = 0.785943`
  - `pad=14`: `best_iou = 0.792083`
  - `pad=16`: `best_iou = 0.773699`

Interpretation:
- `16 pt` is not merely “suboptimal”; it is clearly **off the local optimum**.
- The best values in this sweep were around:
  - `10 pt`
  - `14 pt`
- Even `0 pt` and `6 pt` were competitive with or better than the old `16 pt`.

### Word real replay check: clipping threshold is lower than expected
- To avoid overfitting to `System.Drawing` replay, the full production document was also exported as `.docx` and checked through **Word COM + CopyAsPicture** at multiple paddings:
  - `pad=0`
  - `pad=2`
  - `pad=4`
  - `pad=10`
  - `pad=14`
- In the focused right-bottom catalyst crop, the problematic right-side `Ph` remained visually present across all of these sampled paddings.

Interpretation:
- The “current” source-bounds policy already has one important protection even at `pad=0`:
  - it still uses `max(visible_right, svg_right)` on the right edge.
- That suggests the **inclusion of the SVG right edge** may be the truly necessary part,
  while the extra `+16 pt` constant is likely larger than required.

### Refined hypothesis
- We now have stronger evidence that the current rule is mixing two separate ideas:
  1. use a source rectangle that is not as tight as `visible_bounds`,
  2. add an additional right-side compensation constant.
- The experiments suggest:
  - `visible` alone is too aggressive for the full production document,
  - but `current + 16 pt` is over-padded,
  - and a large part of the safety may already come from including `svg_right`.

This suggests a more faithful future rule should likely be:
- keep the **base rectangle policy** explicit (e.g. include `svg_right` when needed),
- then add only the **minimum extra right compensation** needed for real replay,
- instead of carrying a single oversized global constant.

### New experiment modes: `svgpad` / `unionpad`
- Two additional investigation-only source-bounds modes were added:
  - `svgpad`
    - use the full SVG bounds on left/top/bottom,
    - but still allow extra right-side padding.
  - `unionpad`
    - use the union of visible/svg bounds,
    - then still allow extra right-side padding.

Purpose:
- test whether the real win comes from:
  - keeping the full SVG frame,
  - while still preserving enough right-side breathing room for text.

### Result: `svgpad` is much more promising than the old `current` rule
#### Pure-vector fixtures against current chemcore SVG
- `molecule`
  - `current`: `best_iou = 0.841335`
  - `svgpad`: `best_iou = 0.833295`
- `arrows-acs`
  - `current`: `best_iou = 0.950161`
  - `svgpad`: `best_iou = 0.915239`
- `assets-acs`
  - `current`: `best_iou = 0.870225`
  - `svgpad`: `best_iou = 0.859204`

Interpretation:
- `svgpad` is **not** closer to the current chemcore SVG.
- This strengthens the earlier conclusion that **current chemcore SVG is not the right gold standard** for packaged-preview geometry decisions.

#### Full `thiocyanation-source` against ChemDraw EMF
- `current`: `best_iou = 0.773699`
- `visible`: `best_iou = 0.780436`
- `svgpad`: `best_iou = 0.803110`
- `unionpad`: `best_iou = 0.803095`

Interpretation:
- Against the actual ChemDraw target, `svgpad`/`unionpad` are **substantially better** than the existing `current` rule.
- So the direction “keep the full SVG frame, then compensate the right side only as needed” now looks more plausible than the old “visible left/top/bottom + extra right padding” rule.

### `svgpad` right-padding sweep
- A focused sweep was then run for:
  - `sourceBoundsMode = svgpad`
  - `rightPaddingPt = 0 / 4 / 8 / 10 / 12 / 16`
- Against ChemDraw EMF:
  - `pad=0`: `best_iou = 0.808369`
  - `pad=4`: `best_iou = 0.801002`
  - `pad=8`: `best_iou = 0.798227`
  - `pad=10`: `best_iou = 0.795584`
  - `pad=12`: `best_iou = 0.790117`
  - `pad=16`: `best_iou = 0.803095`

Interpretation:
- The best result in this sweep was actually:
  - `svgpad + pad=0`
- This is extremely important, because it suggests:
  - the **full SVG frame itself** is already providing the needed right-side safety,
  - and the extra `+16 pt` compensation may not be necessary once we stop cropping left/top/bottom to the visible bounds.

### Word real replay check for `svgpad + pad=0`
- The full production document was exported as `.docx` using:
  - `sourceBoundsMode = svgpad`
  - `rightPaddingPt = 0`
- Then replayed through:
  - Word COM
  - `CopyAsPicture`
- In the focused right-bottom catalyst crop:
  - the problematic right-side `Ph` remained visible.

Interpretation:
- `svgpad + pad=0` is not just a packaged-EMF numeric win;
- it also survives the Word live replay sanity check for the previously problematic right edge.

### Current best candidate hypothesis
- The strongest candidate so far is now:
  - **`svgpad + pad=0`**

Why it matters:
- It beats the old `current` rule against ChemDraw.
- It still survives Word replay on the previously problematic right edge.
- It avoids the large global over-padding that the old `current + 16 pt` rule introduced.

What this suggests structurally:
- The truly necessary part may be:
  - “do not crop away the SVG frame on left/top/bottom”
- rather than:
  - “crop tightly to visible bounds and then re-add a large constant on the right”.
## Post-`svgpad + pad=0` re-check on the new default baseline

After switching the product default to:
- `sourceBoundsMode = svgpad`
- `rightPaddingPt = 0`

the most important old text findings need to be re-evaluated rather than assumed to still hold.

### Fallback `EMR_EXTTEXTOUTW` on the title / conditions block is now very close
- Region:
  - `650,120,1850,560`
- Comparison:
  - `tmp/default-svgpad-analysis/title-fallback.md`

Key rows:
- `4DPAIPN (2 mol%)`
  - `dleft = +2`
  - `dwidth = -2`
- `PF (5 mol%), L (7 mol%)`
  - `dleft = 0`
  - `dwidth = 0`
- `TMSCN (4 eq.)`
  - `dleft = 0`
  - `dwidth = +1`
- `SCHPh (3 eq.)`
  - `dleft = +1`
  - `dwidth = 0`
- `CH3CN (0.2 M)`
  - this line is still the loosest in the central block:
    - `dleft = +1`
    - `dright = -7`
    - `dwidth = -8`
- `420 nm 3W, 10 °C, 24 h`
  - `dleft = +2`
  - `dwidth = -2`

Interpretation:
- On the new default baseline, the old `PF6 -> " " -> "(5 "` fallback-space issue is no longer the dominant residual.
- The packaged fallback text for the main title / conditions block is now mostly within `0..2 px` of ChemDraw.

### Yield / `d.r.` fallback is also near-aligned
- Region:
  - `1070,440,1335,525`
- Comparison:
  - `tmp/default-svgpad-analysis/yield-fallback.md`

Key rows:
- `76% yield, 94% ee`
  - `dy = 0`
  - `dleft = 0`
  - `dright = +1`
  - `dwidth = +1`
- `d.r. > 20:1`
  - `dy = +1`
  - `dleft = +1`
  - `dright = 0`
  - `dwidth = -1`

Interpretation:
- The upper-right plain-text block is also no longer the primary problem.

### Rendered-ink region checks now point more strongly at a global placement residual
- Region-level rendered bbox report:
  - `tmp/default-svgpad-analysis/region-bbox-shift.json`

Observed best shifts:
- `title_text`
  - `dx = 10`
  - `dy = 20`
- `product_top_right`
  - `dx = 10`
  - `dy = 20`
- `ligand_lower_left`
  - `dx = 11`
  - `dy = 20`
- `catalyst_lower_right`
  - `dx = 10`
  - `dy = 20`

Observed bbox deltas:
- `title_text`
  - `left +10`
  - `top +15`
  - `width -10`
  - `height -26`
- `product_top_right`
  - `top +29`
  - `height -29`
- `ligand_lower_left`
  - `left +11`
  - `top +25`
  - `width -11`
  - `height -21`

Interpretation:
- Once the default is changed to `svgpad + pad=0`, the remaining large visible mismatch is much less text-specific.
- Across title, product, and ligand regions, the rendered result still behaves like:
  - the whole visible ink is slightly too far right
  - and consistently too low
- This pattern is much more consistent with:
  - frame/source placement,
  - replay anchoring,
  - or a packaged preview canvas offset
  than with local fallback tokenization.

### Updated working hypothesis
- Before the `svgpad + pad=0` switch, text-token and fallback behavior dominated the investigation.
- After the switch, the packaged fallback text is already close enough that it no longer explains the bulk of the visible delta.
- The new primary target should be reclassified as:
  - **global packaged preview placement / top-left anchoring**
  - not the old `PF6 -> " " -> "(5 "` fallback-space issue.

### `sourceBounds` side-sweep v2：full SVG 四边仍然最好

为了验证“剩下的 `(10,25)` 级别统一位移，是否只是因为 `sourceBounds` 左/上取错了”，做了一轮**四边分别从 `visible/svg` 取值**的 sweep。

有效结果在：
- `tmp/source-side-sweep-v2/summary.json`

结论：
- 最好的一组仍然是：
  - `left=svg, top=svg, right=svg, bottom=svg`
  - `best_iou = 0.8083659744`
  - `dx = 10`
  - `dy = 25`
- 把 `top` 改成 `visible` 以后，`dy` 会从 `25` 掉到 `1` 左右，但整体 IoU 明显变差
- 也就是说：
  - **不能把当前主残差解释成“只要把 top/left 改成 visible 就好了”**

这一步非常重要，因为它基本排除了：
- 继续在 `sourceBounds` 左/上/下的裁切策略上内耗

接下来的研究重点应该从：
- `what source bounds to draw from`
转向：
- `what frame to advertise around that source`

## 2026-05-16：`frame-only patch` 把主残差从 `(10,25)` 级别压到 `(1,1)`

在当前默认基线：
- `sourceBoundsMode = svgpad`
- `rightPaddingPt = 0`

之上，做了一组**不改任何绘制记录，只 patch `EMR_HEADER.frame`** 的对照实验。

实验方法：
- 基线文件：
  - `tmp/default-svgpad-check/thiocyanation-source.default.emf`
- ChemDraw 参考：
  - `tmp/thiocyanation-source.chemdraw.emf`
- patch 版本：
  - 只把 chemcore `EMF` 头里的 `frame` 改成 ChemDraw 的 `frame`
  - `bounds`、所有 `EMF+/GDI` 绘制记录都保持不变

产物目录：
- `tmp/frame-patch-tests`

### Full document 结果

原始基线：
- `best_iou = 0.8083659744`
- `dx = 10`
- `dy = 25`

只改 `frame`：
- `best_iou = 0.8364475215`
- `dx = 1`
- `dy = 1`

只改 `bounds`：
- 与原始基线完全相同
  - `best_iou = 0.8083659744`
  - `dx = 10`
  - `dy = 25`

`frame + bounds`：
- 与“只改 frame”几乎完全相同
  - `best_iou = 0.8364532814`
  - `dx = 1`
  - `dy = 1`

结论非常强：
- **当前主残差几乎肯定主要来自 `EMR_HEADER.frame` 这一层**
- `header.bounds` 基本不影响当前离线渲染对比结果
- 也就是说，先前看到的 `(10,25)` 级别统一位移，并不是绘制记录本身错了，而是 preview frame/origin 语义错了

### `frame` 与当前 chemcore / ChemDraw 的量级差异

chemcore 当前默认：
- `frame = { left: 1364, top: 2868, right: 14267, bottom: 8712 }`
- 换回 `svg px`：
  - `[128.882, 270.992, 1348.063, 823.181]`

ChemDraw：
- `frame = { left: 1413, top: 2993, right: 14403, bottom: 8649 }`
- 换回 `svg px`：
  - `[133.512, 282.803, 1360.913, 817.228]`

对应差异：
- left：`+4.63 px`
- top：`+11.81 px`
- right：`+12.85 px`
- bottom：`-5.95 px`

注意：
- 这不是简单的“全体平移”
- 也不是单纯 `visible/svg` 二选一能表达出来的东西
- 它同时包含：
  - 左/上裁掉一部分
  - 右侧再放出更多余量
  - 底部略收回

### 最小样本上的 `frame-only` 复查

为了判断这是不是 full document 偶然命中，还对最小样本做了同样的“只改 frame”实验：
- `tmp/frame-patch-fixtures`

结果：
- `mixed-center-line`
  - 原始：`iou = 0.847609`, `dx = 8`, `dy = -3`
  - frame-only：`iou = 0.855361`, `dx = 1`, `dy = 3`
- `plain-center-line`
  - 原始：`0.682499`, `dx = 13`, `dy = -3`
  - frame-only：`0.694272`, `dx = 0`, `dy = 3`
- `mixed-center-block`
  - 原始：`0.784522`, `dx = 8`, `dy = -2`
  - frame-only：`0.855769`, `dx = 1`, `dy = 3`

但也不是所有样本都单调变好：
- `mixed-center-two-line`
  - 原始：`0.883024`
  - frame-only：`0.867768`
- `right-edge-ph`
  - 原始：`0.226190`
  - frame-only：`0.138632`

所以当前能下的最稳结论是：
- **`frame` 的确是一个一级因素**
- 它能解释 full 文档里那种统一位移级别的大残差
- 但它还不是“改了就所有 fixture 都更好”的万能公式

### 由此得到的新假设

这组结果把问题进一步收窄成了两个层次：

1. `sourceBounds` 选择
- `svgpad + 0` 仍然是当前最好的 source 口径
- side-sweep-v2 已经证伪了“只要把 left/top 改成 visible 就会更好”

2. `frame` 语义
- 现在更像是：
  - `source` 口径基本已经对了
  - 但 **preview `frame` 应该独立于 `source` 再做一次 ChemDraw 风格的重映射**
- 换句话说：
  - 主问题不再是 “what to draw from”
  - 而更像 “what frame to advertise around what we draw”

当前最值得继续打的方向：
- 不再继续在 `sourceBounds` 上内耗
- 转而做 **frame-only research path**
- 看能不能在真实导出路径里引入一个分析用 `frame override`，
  验证“只改 frame 不改 draw”是否就能稳定吃到这波提升

### analysis hook：只改 frame，不改 source/draw

为了在**真实导出路径**里复现这件事，而不是每次手工 patch 二进制，现在增加了一个分析用环境变量：

- `CHEMCORE_PREVIEW_FRAME_OFFSETS_SVG_PX=left,top,right,bottom`

语义：
- 它只影响 `office_preview_frame_bounds(...)`
- 不改 `sourceBounds`
- 不改 `drawBounds`
- 不改任何 `EMF+/GDI` 图元记录的几何

这让后续可以在产品路径里直接做：
- `source` 保持当前最佳 `svgpad + 0`
- 只研究 `frame` 偏移到底是不是根因

### 重要副结论：record-time frame override 和 post-hoc header patch 不是同一回事

拿 full 文档做了一个对照：

1. **post-hoc binary patch**
- 先正常导出 chemcore `EMF`
- 再把 header `frame` 改成 ChemDraw 那组值
- 结果：
  - `best_iou ≈ 0.83645`
  - `dx ≈ 1`
  - `dy ≈ 1`
  - `System.Drawing.Metafile` 可正常打开

2. **record-time frame override**
- 用上面的 `CHEMCORE_PREVIEW_FRAME_OFFSETS_SVG_PX`
- 直接在生成时带着同一组 target frame 去录制
- 结果：
  - `EMF inspect` 能正常读 header
  - 但 `System.Drawing.Metafile` 会报 `A generic error occurred in GDI+`

这说明：
- “最终 header 数值” 和 “录制路径是否被 GDI+/replay 接受”
- 是两件不同的事

所以当前不能直接把：
- “binary patch 成功”
等价成：
- “产品代码只要把 frame 设成那组数就一定成立”

这反而进一步支持：
- 后续要么继续研究 ChemDraw 的**真实 frame 生成逻辑**
- 要么接受“post-process 修 frame”是一条单独的工程路径

### `frame` 分解实验：`x-origin`、`y-origin`、`height` 才是主分量

为了继续把 `frame` 从黑箱拆开，full document 又做了一组二进制 patch 分解实验：
- `tmp/frame-decompose-tests`
- 结果汇总：
  - `tmp/frame-decompose-tests/summary.json`

实验项：
- `origin-only`
- `size-only`
- `x-only`
- `y-only`
- `width-only`
- `height-only`
- `x-plus-height`
- `x-plus-size`
- `origin-x-y-plus-height`
- `origin-x-y-plus-width`

关键结果：

基线：
- `original`
  - `iou = 0.808366`
  - `dx = 10`
  - `dy = 25`

完整 ChemDraw frame：
- `chem`
  - `iou = 0.836453`
  - `dx = 1`
  - `dy = 1`

单分量：
- `x-only`
  - `iou = 0.810327`
  - `dx = 1`
  - `dy = 25`
- `y-only`
  - `iou = 0.803123`
  - `dx = 10`
  - `dy = 1`
- `width-only`
  - `iou = 0.805905`
  - `dx = 10`
  - `dy = 25`
- `height-only`
  - `iou = 0.823427`
  - `dx = 10`
  - `dy = 24`

组合：
- `origin-only`
  - `iou = 0.794240`
  - `dx = 1`
  - `dy = 2`
- `size-only`
  - `iou = 0.805467`
  - `dx = 10`
  - `dy = 24`
- `x-plus-height`
  - `iou = 0.810632`
  - `dx = 1`
  - `dy = 24`
- `x-plus-size`
  - `iou = 0.831902`
  - `dx = 1`
  - `dy = 24`
- `origin-x-y-plus-height`
  - `iou = 0.828944`
  - `dx = 1`
  - `dy = 1`
- `origin-x-y-plus-width`
  - `iou = 0.809180`
  - `dx = 1`
  - `dy = 1`

从这组结果可以非常清楚地看出：

1. `x-origin` 单独负责把 `dx` 从 `10` 压到 `1`
2. `y-origin` 单独负责把 `dy` 从 `25` 压到 `1`
3. 真正最值钱的单个尺寸分量是：
   - **`height`**
4. `width` 单独几乎没有贡献
5. 最接近完整 ChemDraw patch 的“最小解释集合”目前是：
   - **`origin(x+y) + height`**

这意味着：
- 下一步如果要找一条更可解释的 frame 规则，
- 应该优先研究：
  - 为什么 ChemDraw 的 preview frame 在 `x/y origin` 上会换到那组值
  - 以及为什么 `height` 会显著更短
- 而不是先盯 `width`

### Word `CopyAsPicture` 口径也验证了：`frame` 是主因，但敏感分量和离线 patch 不完全相同

为了避免再次被离线 `System.Drawing.Metafile` 的行为误导，这一轮把对比口径切到了 **Word 自己的实时回放**：

- 用当前分支重新导出：
  - `tmp/frame-word-ab/current.docx`
- 然后只 patch 其中 `word/media/image1.emf` 的 `EMR_HEADER.frame`
- 最后用 Word COM：
  - 选中第一个 inline shape
  - `CopyAsPicture`
  - 保存成 `PNG`

为此新增了两个可复用脚本：

- `scripts/word-copy-inline-shape.ps1`
  - 用 Word COM 打开 `docx`
  - 对指定 `InlineShape` 执行 `CopyAsPicture`
  - 从剪贴板保存为 `PNG`
- `scripts/patch-docx-image1-frame.py`
  - 只 patch `docx` 包内 `word/media/image1.emf` 的 `frame`
  - 不改 `document.xml` / `oleObject1.bin` / 显示尺寸

这次对 full document 做的 patch 变体：

- `current`
- `x-only`
- `y-only`
- `width-only`
- `height-only`
- `size-only`
- `origin-only`
- `origin-height`
- `frame-chem`

ChemDraw 参照口径：

- 直接使用 `tmp/thiocyanation-source.chemcore.v28.docx`
- 通过同一条 Word COM `CopyAsPicture` 链导出第二个对象：
  - `tmp/frame-word-ab/v28-shape2.png`

得到的 Word 回放结果（`best_shift` 对 `v28-shape2.png`）：

- `current`
  - `iou = 0.311686`
  - `dx = 3`
  - `dy = 2`
- `x-only`
  - `iou = 0.295366`
  - `dx = 2`
  - `dy = 2`
- `y-only`
  - `iou = 0.409181`
  - `dx = 3`
  - `dy = -2`
- `width-only`
  - `iou = 0.267287`
  - `dx = 0`
  - `dy = 2`
- `height-only`
  - `iou = 0.344130`
  - `dx = 3`
  - `dy = 3`
- `size-only`
  - `iou = 0.287548`
  - `dx = 0`
  - `dy = 3`
- `origin-only`
  - `iou = 0.376939`
  - `dx = 2`
  - `dy = -2`
- `origin-height`
  - `iou = 0.439630`
  - `dx = 2`
  - `dy = -1`
- `frame-chem`
  - `iou = 0.408697`
  - `dx = -1`
  - `dy = -1`

对应的 Word 回放可见 ink bbox：

- `current`
  - `left=5 top=8 right=552 bottom=232`
  - `width=548 height=225`
- `x-only`
  - `left=2 top=8 right=552 bottom=232`
  - `width=551 height=225`
- `y-only`
  - `left=5 top=3 right=552 bottom=232`
  - `width=548 height=230`
- `width-only`
  - `left=4 top=8 right=550 bottom=232`
  - `width=547 height=225`
- `height-only`
  - `left=5 top=8 right=552 bottom=235`
  - `width=548 height=228`
- `origin-only`
  - `left=3 top=3 right=552 bottom=232`
  - `width=550 height=230`
- `origin-height`
  - `left=3 top=3 right=552 bottom=235`
  - `width=550 height=233`
- `frame-chem`
  - `left=3 top=3 right=550 bottom=235`
  - `width=548 height=233`
- `v28-shape2`（ChemDraw Word 参照）
  - `left=2 top=2 right=551 bottom=239`
  - `width=550 height=238`

这一轮最重要的结论：

1. `frame` 不只是离线 patch 的主因，在 **Word 自己的实时回放** 里同样是主因。
2. 但 Word 口径下的敏感分量和离线 patch 不完全一样：
   - `y-only` 的价值明显高于 `x-only`
   - `height-only` 也有稳定收益
   - `width-only` 依然几乎没有价值
3. 在 Word 口径里，当前最强的简化候选不是完整 ChemDraw frame，而是：
   - **`origin + height`**
4. `record-time frame override` 仍然是可行产品路径：
   - 同一组 `frame` 数字直接通过环境变量写入录制过程
   - Word 仍能正常打开并 `CopyAsPicture`
   - 而且和“事后 patch `image1.emf` header”的 Word 回放结果几乎一致
5. 这也说明前面那个“record-time frame override 不可用”的结论，至少对 **Word 回放** 来说已经过期了；
   - 当时真正不兼容的是 `System.Drawing.Metafile`
   - 不是 Word 本身

当前更准确的 working hypothesis：

- 对 packaged preview 来说，
- `EMR_HEADER.frame` 不是一个“无关紧要的 metadata”，
- 它实质上参与了 Word 对整张 preview 的放置和缩放解释。

而在 full document 上，
最值得继续逆向的 frame 规则已经进一步收敛成：

- 先解释 `top / bottom / height`
- 再解释 `left`
- `right / width` 暂时不是主矛盾

### 最小 Word fixture 说明：`origin+height` 不是通用 frame 规则

为了验证 `origin+height` 是否只是 full document 偶然成立，这一轮把已有最小样本也搬进了 **Word COM `CopyAsPicture`** 口径。

样本：

- `mixed-center-block`
- `mixed-center-line`
- `mixed-center-two-line`
- `plain-center-line`
- `right-edge-ph`

方法：

1. 用当前代码从 `.payload.json` 生成 `*.current.docx`
2. 以同一个 `docx` 壳为基底，替换 `word/media/image1.emf`
   - `chemref`
     - 直接替换成对应的 `*.chemdraw.emf`
   - `origin-height`
     - 只 patch `frame.left/top/bottom`
     - 保持 `frame.right` 为当前值
   - `frame-chem`
     - 整组 `frame` 直接抄成 ChemDraw header
3. 用 `scripts/word-copy-inline-shape.ps1`
   - 对第一个 inline shape 执行 `CopyAsPicture`
   - 保存为 `PNG`
4. 用 `scripts/png-best-shift.py` 对 `chemref.wordcopy.png` 做比较

结果（相对各自的 `chemref`）：

#### `mixed-center-block`
- `current`
  - `iou = 0.449363`
  - `dx = 14`
  - `dy = 3`
- `origin-height`
  - `iou = 0.586382`
  - `dx = 7`
  - `dy = -2`
- `frame-chem`
  - `iou = 0.798127`
  - `dx = 1`
  - `dy = -2`

#### `mixed-center-line`
- `current`
  - `iou = 0.411039`
  - `dx = 14`
  - `dy = -2`
- `origin-height`
  - `iou = 0.525624`
  - `dx = 7`
  - `dy = -5`
- `frame-chem`
  - `iou = 0.678733`
  - `dx = 1`
  - `dy = -5`

#### `mixed-center-two-line`
- `current`
  - `iou = 0.416014`
  - `dx = 14`
  - `dy = -8`
- `origin-height`
  - `iou = 0.620329`
  - `dx = 7`
  - `dy = -5`
- `frame-chem`
  - `iou = 0.679595`
  - `dx = 1`
  - `dy = -5`

#### `plain-center-line`
- `current`
  - `iou = 0.315641`
  - `dx = 15`
  - `dy = 0`
- `origin-height`
  - `iou = 0.772355`
  - `dx = 9`
  - `dy = -5`
- `frame-chem`
  - `iou = 0.580243`
  - `dx = -1`
  - `dy = -5`

#### `right-edge-ph`
- `current`
  - `iou = 0.194174`
  - `dx = 6`
  - `dy = -12`
- `origin-height`
  - `iou = 0.0`
  - `dx = -20`
  - `dy = -20`
- `frame-chem`
  - `iou = 0.0`
  - `dx = -20`
  - `dy = -20`

结论：

1. `frame` 在 Word 口径下对最小样本同样是强影响因子。
2. 但 `origin+height` 不是通用规则：
   - 对 full document 很强
   - 对 centered text fixtures 也有收益
   - 但对 `right-edge-ph` 这类窄右侧样本直接失败
3. 对 centered fixtures 来说，完整 `frame-chem` 反而比 `origin+height` 更接近参考。
4. 这说明：
   - full document 上的 `origin+height` 最优，不应该被过度推广成产品全局规则
   - frame 语义很可能仍然和：
     - 内容类型
     - 显示 extents
     - 或 Word/OLE 外层显示框
     存在耦合

当前更稳的表述应该是：

- `frame` 仍然是主问题；
- 但它不是“统一改成 `origin+height`”就能结束；
- 后续应该继续找：
  - 为什么 full document 更像 `origin+height`
  - 为什么 centered fixtures 更像 `frame-chem`
  - 为什么 `right-edge-ph` 会对这两条都失稳
## 2026-05-16：`docx` 外层尺寸 patch 工具与 `dxaOrig/style` 解耦结论

这轮先把 `frame` 和 `docx` 外层尺寸拆开做成可控实验。

新增工具：

- `scripts/patch-docx-object-size.py`

它直接 patch `word/document.xml` 里第一个 OLE 对象的：

- `w:dxaOrig / w:dyaOrig`
- `v:shape style="width:...;height:..."`

支持：

- 绝对值 patch
- `--display-scale`
- `--natural-scale`

### 观察 1：`dxaOrig/dyaOrig` 对 Word `CopyAsPicture` 基本无影响

对 `right-edge-ph` 做对称实验：

- current / chemref 两边同时改 `natural-scale = 0.6 ~ 1.2`
- `v:shape width/height` 保持不变

结果：

- 渲染后真实 ink bbox 完全不变
- best-shift IoU 基本固定在 `0.563 ~ 0.575`

结论：

- Word 实时回放几乎不吃 `dxaOrig / dyaOrig`
- 这两个字段更像“原始大小/属性对话框”元数据
- 真正控制 `CopyAsPicture` 可见结果的，是 `v:shape style width/height`

### 观察 2：`v:shape width/height` 会显著改变像素 IoU

继续对 `right-edge-ph` 做对称实验：

- current / chemref 两边同时改 `display-scale = 0.6 ~ 1.1`
- `dxaOrig / dyaOrig` 保持不变

结果：

- `0.60 -> 0.5771`
- `0.70 -> 0.5856`
- `0.80 -> 0.5804`
- `0.90 -> 0.5494`
- `1.00 -> 0.5632`
- `1.10 -> 0.5100`

也就是说，只改 shell 的显示尺寸，就能明显改动逐像素 IoU。

### 观察 3：centered fixture 也同样对 display-size 敏感

对 `mixed-center-line` 做同样的对称 `display-scale` 扫描：

- `0.60 -> 0.4995`
- `0.70 -> 0.4781`
- `0.80 -> 0.4723`
- `0.90 -> 0.4619`
- `1.00 -> 0.4598`
- `1.10 -> 0.4765`

这说明：

- `display-size` 对 Word `CopyAsPicture` 的像素重叠有一阶影响
- 这种影响里混有明显的栅格化/采样效应

### 这轮的解释

这批实验不能直接解读成：

- “把显示尺寸调小就更几何正确”

更合理的理解是：

- Word 的 `CopyAsPicture` 对最终显示分辨率高度敏感
- shell 显示尺寸变化会改变栅格化采样口径
- 所以 IoU 变好，不一定代表底层 `frame` / `bounds` 语义更对

### 当前可用结论

1. `dxaOrig/dyaOrig` 不是 Word 实时回放的主控制量。
2. `v:shape width/height` 才是主控制量。
3. 因为 `display-size` 本身会显著改动 IoU，后续如果继续研究 `frame` 语义：
   - 必须尽量固定 `display width/height`
   - 不能再把“缩小后更像”直接当成 geometry 修复成功
4. 这也解释了为什么 `right-edge-ph` 和 centered fixtures 在 Word 口径下会显得更不稳定：
   - 它们不仅受 `frame` 影响
   - 也明显受 shell 显示尺寸和最终采样口径影响
## 2026-05-16：same-shell full-doc 对照把 `origin+height` 假象拆掉

这轮补了一个很关键的 full-doc 对照：

- 以前 full 文档是拿 current docx 去对真实 `v28` ChemDraw 对象
- 那个 target 同时包含：
  - ChemDraw 的 `image1.emf`
  - ChemDraw 自己的 `document.xml` shell

这会把两层问题混在一起：

1. `EMF frame`
2. `docx` 外层 shell（`dxaOrig` / `style width-height`）

于是此前才会出现一个误导现象：

- full doc 看起来 `origin+height` 比 `frame-chem` 更优

### 这轮做法

直接用 current shell 造一个 **same-shell chemref**：

- 基底：`tmp/frame-word-ab/current.docx`
- 只替换 `word/media/image1.emf` 为 `tmp/thiocyanation-source.chemdraw.emf`
- 得到：`tmp/frame-word-ab/chemref-sameshell.docx`

然后再用 Word `CopyAsPicture` 比较：

- `current.wordcopy.png`
- `frame-chem.wordcopy.png`
- `frame-origin-height.wordcopy.png`

against

- `chemref-sameshell.wordcopy.png`

### 结果

- `current`
  - `iou = 0.355754`
  - `dx = -4`
  - `dy = -4`
- `frame-chem`
  - `iou = 0.724490`
  - `dx = 0`
  - `dy = 0`
- `origin-height`
  - `iou = 0.447824`
  - `dx = -3`
  - `dy = 0`

### 结论

这条结果非常关键：

- **同一个 shell** 下，full doc 也明确是 `frame-chem` 更对
- 之前 full doc 上“`origin+height` 更优”的现象，主要是因为 target 还混进了 ChemDraw 自己的 shell 语义

也就是说，现在可以把问题正式拆成两层：

1. **EMF frame 语义**
   - 在 same-shell 对照里，ChemDraw 风格完整 frame 是明显正确方向

2. **docx shell 语义**
   - `dxaOrig`
   - `v:shape width/height`
   - 这层会改变 Word `CopyAsPicture` 的像素结果
   - 但它不该再反过来误导我们判断 `frame` 本身

### 当前解释更新

之前那句：

- “full doc 更像 `origin+height`，centered fixtures 更像 `frame-chem`”

现在应该更新成：

- **在 same-shell 口径下，full doc 和 centered fixtures 都更支持 `frame-chem`**
- `origin+height` 更像是“current shell 去对真实 ChemDraw shell”时出现的折中假象

因此后续研究应继续坚持：

- 先固定 shell
- 再判断 frame

而不是把两层继续混在一起。

## 2026-05-16：`v28` 内嵌 ChemDraw `image2.emf` 与 shell 影响量级

这轮又核了一个容易混淆的前提：

- `tmp/thiocyanation-source.chemdraw.emf`
- `tmp/thiocyanation-source.chemcore.v28.docx` 里真正嵌入的 `word/media/image2.emf`

它们不是同一份字节：

- oracle `tmp/thiocyanation-source.chemdraw.emf`
  - `sha256 = e251be78...`
  - `size = 101100`
- `v28` 内嵌 `image2.emf`
  - `sha256 = d22b974f...`
  - `size = 100964`

但它们的几何非常接近：

- oracle frame = `(1413, 2993, 14403, 8649)`
- `v28 image2` frame = `(1410, 2993, 14403, 8651)`
- bounds 也一样：`(133, 280, 1334, 806)`

### 同一张 ChemDraw `EMF`，只换 shell，会掉多少

为了量 shell 的纯影响，我做了：

1. 从 `v28.docx` 里抽出真正嵌入的 `image2.emf`
2. 用 current shell 重新组一个 same-shell chemref：
   - `tmp/frame-word-ab/chemref-v28embed-sameshell.docx`
3. 再和真实 `v28` shape2 的 Word `CopyAsPicture` 对比

结果：

- `chemref-v28embed-sameshell.wordcopy` vs `v28-shape2`
  - `iou = 0.410894`
  - `dx = 2`
  - `dy = 2`

这个值非常关键，因为它说明：

- **就算 EMF 本身已经和 v28 里嵌入的那张完全一致**
- 只要 shell 还用 current 这套
- Word 回放出来的像素结果仍然会掉到 `~0.41`

也就是说，shell 本身就是一个非常强的一阶因素。

### 在 same-shell 口径下重新比较 full doc frame

以 `chemref-v28embed-sameshell.wordcopy` 为参考：

- `current`
  - `iou = 0.355815`
  - `dx = -4`
  - `dy = -4`
- `frame-chem`
  - `iou = 0.743112`
  - `dx = 0`
  - `dy = 0`
- `origin-height`
  - `iou = 0.442946`
  - `dx = -3`
  - `dy = 0`

这进一步强化了上一节的结论：

- **在 shell 固定后，full doc 的正确 frame 方向明显是 `frame-chem`**
- `origin+height` 只是“拿 current shell 去对真实 ChemDraw shell”时，被 shell 差异污染出来的折中解

### 当前总判断

到这一步可以比较有把握地把问题正式拆成两层：

1. `EMF frame`
   - same-shell 口径下，ChemDraw 完整 frame 明显更对

2. `docx shell`
   - 就算 EMF 自己对了
   - shell 不同也会把 Word 回放结果拉到很低的 IoU

所以后续研究顺序应该是：

1. 先在 same-shell 口径下把 `frame` 搞对
2. 再单独研究 ChemDraw 的 shell 语义

而不是再把两层混着看。
## 2026-05-16：`CopyAsPicture` 口径确认与 shell/EMF 交互更新

### 1. `CopyAsPicture` 确实吃 packaged preview image，不是 live OLE

之前一个潜在风险是：

- 如果 Word `CopyAsPicture` 走的是 live OLE 渲染
- 那么只替换 `word/media/image1.emf`
- 理论上不该显著改变复制出来的像素结果

这轮做了一个极端替换实验：

- 基底：`tmp/frame-word-ab/current.docx`
- 不动 `oleObject1.bin`
- 只把 `word/media/image1.emf` 换成一个完全不同的 fixture：
  - `mixed-center-line.chemref` 的 `image1.emf`
- 得到：
  - `tmp/frame-word-ab/current-swapfixturepreview.docx`

Word `CopyAsPicture` 后的结果立刻完全变样：

- `current.wordcopy.png`
  - bbox = `[5, 8, 552, 232]`
  - ink = `11938`
- `current-swapfixturepreview.wordcopy.png`
  - bbox = `[7, 54, 543, 200]`
  - ink = `26890`

结论：

- `CopyAsPicture` 这条研究口径，确实主要吃的是 packaged preview image
- 不是 live OLE object 的实时渲染

这条非常重要，因为它说明：

- 现在围绕 `image1.emf / frame / shell` 做的实验是有意义的
- 没有被 live OLE 路径污染

### 2. exact `v28` shell 对 chemcore 自己的 EMF 几乎没有额外影响

我又做了一个对称性检查：

- `current-shellchem.wordcopy` vs `current-in-v28shell.shape2`
- `frame-chem-shellchem.wordcopy` vs `frame-chem-in-v28shell.shape2`
- `frame-origin-height-shellchem.wordcopy` vs `frame-origin-height-in-v28shell.shape2`

结果：

- `current-shellchem` vs `current-in-v28shell`
  - `iou = 1.0`
  - `dx = 0`
  - `dy = 0`
- `frame-chem-shellchem` vs `frame-chem-in-v28shell`
  - `iou = 1.0`
  - `dx = 0`
  - `dy = 0`
- `frame-origin-height-shellchem` vs `frame-origin-height-in-v28shell`
  - `iou = 0.999915`
  - `dx = 0`
  - `dy = 0`

也就是说：

- 对 **chemcore 自己生成的这些 EMF** 而言
- 一旦 `dxaOrig + style width/height` patch 到 ChemDraw 那组值
- 剩下那些 `v28` shell 细节（额外 rels、styles/theme、第二段结构、复杂 root 命名空间等）
- 对 `CopyAsPicture` 结果几乎没有可见影响

### 3. 但 exact ChemDraw image 对 shell 仍然敏感

对 exact `v28 image2.emf`：

- `chemref-v28embed-shellchem.wordcopy` vs `v28-shape2`
  - `iou = 0.639602`
  - `dx = 1`
  - `dy = 1`

所以：

- 对 ChemDraw 自己那张 preview image
- current shell 只 patch `dxaOrig + width/height` 还不够

### 这轮的更新结论

到这一步，比较准确的判断是：

1. `CopyAsPicture` 研究的是 preview image，不是 live OLE。
2. 对 chemcore 当前生成的 EMF：
   - shell 的主效应几乎已经收敛到
     - `dxaOrig / dyaOrig`
     - `v:shape width/height`
3. 对 ChemDraw 自己那张 image：
   - shell 还有额外影响量
   - 说明 shell 与 preview image 之间存在交互，不是单一常数项

因此下一步更合理的方向是：

- 不再抽象地说“shell 还有很多神秘差异”
- 而是更精确地说：
  - **shell 对 ChemDraw 风格紧 frame / 紧内容的 image 更敏感**
  - 对 chemcore 当前这类 preview image，额外 shell 细节基本已经不重要

## 2026-05-16：`ProgID / ObjectID / ShapeID` 任一单字段变化都足以触发同一条 Word 回放分支

这一轮继续在 **exact `v28` shell + exact `v28 image2.emf`** 这条最干净的口径下做 identity ablation。

### 1. 先补齐 `objectid-only / shapeid-only`

之前已经知道：

- `size-current`：只改 `dxaOrig + style width/height`
  - `iou = 0.410956`
- `progid-only`
  - `iou = 0.639556`
- `identity-current`
  - `iou = 0.639556`

我补量了两条之前缺的：

- `objectid-only.shape2`
  - `iou = 0.639556`
  - `dx = -1`
  - `dy = -1`
- `shapeid-only.shape2`
  - `iou = 0.639556`
  - `dx = -1`
  - `dy = -1`

其中：

- `objectid-only`
  - 只改 `o:OLEObject@ObjectID`
- `shapeid-only`
  - 同时改 `o:OLEObject@ShapeID`
  - 以及 `<v:shape id>`

这说明：

- 不只是 `ProgID` 特殊
- `ObjectID` 单独变化也足够
- `ShapeID` 组变化也足够

### 2. 再把 `ShapeID` 组拆细：OLE ShapeID 和 VML shape id 各自都能单独触发

为了确认 `shapeid-only` 不是“必须两个字段一起变”，我又做了两条更细的变体：

- `oleshapeid-only`
  - 只改 `o:OLEObject@ShapeID`
- `vshapeid-only`
  - 只改 `<v:shape id>`

结果：

- `oleshapeid-only.shape2`
  - `iou = 0.639556`
  - `dx = -1`
  - `dy = -1`
- `vshapeid-only.shape2`
  - `iou = 0.639556`
  - `dx = -1`
  - `dy = -1`

所以：

- `o:OLEObject@ShapeID` 单独变化，足以触发同一条 Word 回放分支
- `<v:shape id>` 单独变化，也足以触发同一条 Word 回放分支

### 3. 再确认“不是字段类型，而是精确值变化本身”

为了排除“Word 只是识别到从 ChemDraw 变成了 Chemcore 风格字符串”，我又做了同家族/同格式但不同值的变体：

- `progid-samefamily`
  - `ChemDraw.Document.6.0 -> ChemDraw.Document.6.1`
- `objectid-bump`
  - `_1840302152 -> _1840302153`
- `oleshapeid-bump`
  - `_x0000_i1026 -> _x0000_i1099`
- `vshapeid-bump`
  - `_x0000_i1026 -> _x0000_i1099`

结果四条完全一样：

- `iou = 0.639556`
- `dx = -1`
- `dy = -1`

这说明：

- Word 不只是看“这是不是 ChemDraw 家族的值”
- 而是在乎 **这些 identity 字段是否精确等于原值**
- 任一单字段只要偏离原值，就会掉到同一条回放模式

### 4. 当前最准确的判断

到这一步，可以把 shell identity 的结论说得更硬：

- 在 exact `v28 shell + exact v28 image2.emf` 口径下
- `ProgID`
- `ObjectID`
- `o:OLEObject@ShapeID`
- `<v:shape id>`

这四类 identity 字段，**任意一个单独偏离原值**，都足以把 Word `CopyAsPicture` 的像素结果从：

- 基线 `v28-shape2`

拉到同一个较差台阶：

- `iou ≈ 0.639556`

而且这个台阶高度完全一致，不像是“各字段各自带来一点可叠加误差”，更像：

- Word 内部对这组 identity 做了某种二值化路径选择
- 只要其中任何一个 key 不匹配，就切到另一条预览/回放分支

这说明下一步如果继续研究 shell，重点不该再放在：

- “这些字段哪一个更重要”

而应该放在：

- Word 是否把这几项当成同一组 identity key
- 以及还有没有其他字段也属于这组 key（例如 `Type / DrawAspect / r:id` 等）

## 2026-05-16：旧 `v28-shape2.png` 参考图失效，先前 shell/identity 结论被污染

这一轮最关键的不是某个新 patch，而是发现：

- 我们一直拿来当 Word 基线的
  - `tmp/frame-word-ab/v28-shape2.png`
- 已经不是当前这套 `word-copy-inline-shape.ps1 + Word COM` 口径下可复现的结果

### 1. 原始 `v28.docx` 重新跑一次 `CopyAsPicture`，结果就已经不是旧参考图

我直接对原文件重新导了一次：

- `tmp/v28-wrapper-ablate10/v28-rerun.shape2.png`

拿它和旧参考图比：

- 旧参考：`tmp/frame-word-ab/v28-shape2.png`
- 新 rerun：`tmp/v28-wrapper-ablate10/v28-rerun.shape2.png`

旧参考是：

- size = `(557, 244)`
- bbox = `(2, 2, 551, 239)`

而新 rerun 是：

- size = `(555, 242)`
- bbox = `(2, 2, 549, 237)`

也就是说：

- **不用改任何字段**
- **只是现在重新跑一次 Word `CopyAsPicture`**
- 结果就已经落到了先前那个 `~0.639556` 的台阶上

### 2. 纯文件系统拷贝、纯 rezip-identical，也都与新 rerun 完全一致

我又做了两个极端对照：

- `filesystem-copy.docx`
  - 只是对 `v28.docx` 做文件系统拷贝
- `rezip-identical.docx`
  - 逐 entry 原字节重打包，内部文件内容不改

拿它们和新 rerun 比：

- `filesystem-copy.shape2` vs `v28-rerun.shape2`
  - `iou = 1.0`
  - `dx = 0`
  - `dy = 0`
- `rezip-identical.shape2` vs `v28-rerun.shape2`
  - `iou = 1.0`
  - `dx = 0`
  - `dy = 0`

这说明：

- 之前那套“identity 单改就掉到 0.639556”的现象
- 本质上不是 identity patch 的专有后果
- 而是因为我们把这些变体拿去和一个**已经失效的旧参考图**比了

### 3. 之前那些 identity / 非 identity 变体，彼此其实都和新 rerun 完全一致

我把下面这些变体，全都重新改拿 `v28-rerun.shape2` 当参考：

- `progid-samefamily`
- `objectid-bump`
- `oleshapeid-bump`
- `vshapeid-bump`
- `anchorid-bump`
- `imagetitle-nonempty`

结果全部一样：

- `iou = 1.0`
- `dx = 0`
- `dy = 0`

所以现在可以明确收回前面的误判：

- `ProgID / ObjectID / ShapeID` 不是当前口径下的主因
- `anchorId / o:title` 当然也不是
- 我们之前看到的统一 `0.639556`，只是因为参考图错了

### 4. shell 影响也被这个新基线大幅收缩了

最关键的一个回算是：

- `chemref-v28embed-shellchem.wordcopy.png` vs `v28-rerun.shape2.png`
  - `iou = 0.999912`
  - `dx = 0`
  - `dy = 0`

也就是说：

- **exact ChemDraw image**
- 放到 current shell 但把 `dxaOrig + style width/height` patch 成 ChemDraw 值后
- 在当前 Word `CopyAsPicture` 口径下，已经几乎和新 rerun 一样

这和之前“shell 对 exact ChemDraw image 仍然强敏感”的结论相冲突。  
现在更准确的说法是：

- 旧结论主要是被旧参考图污染了

### 5. 纠偏后的稳定结论

用新 rerun 基线重算后，当前还成立的硬结论只剩这些：

1. 对我们自己的 EMF：
   - `current-in-v28shell.shape2`
     - `iou = 0.342309`
     - `dx = 4`
     - `dy = 3`
   - `frame-chem-in-v28shell.shape2`
     - `iou = 0.744865`
     - `dx = 0`
     - `dy = 0`
   - `frame-origin-height-in-v28shell.shape2`
     - `iou = 0.474935`
     - `dx = 3`
     - `dy = 0`

2. 同样地，在 current shell + ChemDraw size 的那组文档里：
   - `current-shellchem.wordcopy`
     - `iou = 0.342309`
   - `frame-chem-shellchem.wordcopy`
     - `iou = 0.744865`
   - `frame-origin-height-shellchem.wordcopy`
     - `iou = 0.474871`

所以：

- **真正还站得住的主结论是：`frame-chem` 仍然明显更对**
- 而不是 shell identity / shell metadata

### 6. 下一步研究方向需要回正

这次纠偏以后，后面的重点应该回到：

- `EMR_HEADER.frame` 的语义
- 以及我们怎样在 record-time 生成更接近 ChemDraw 的 frame

而不是继续深挖：

- `ProgID`
- `ObjectID`
- `ShapeID`
- `anchorId`
- `o:title`

因为在当前有效基线下，这些都已经被证伪为主因。

## 2026-05-16：基于新 rerun 基线重新做 `frame` 分解与局部搜索

在把旧 `v28-shape2.png` 这个失效参考图排除掉之后，我重新用：

- `tmp/v28-wrapper-ablate10/v28-rerun.shape2.png`

作为唯一有效的 Word `CopyAsPicture` 基线，重新检查 `frame` 四个分量。

### 1. 重新分解 `frame` 四个分量，`frame-chem` 仍然是正确主方向

基于：

- current frame：`(1364, 2868, 14267, 8712)`
- chem frame：`(1410, 2993, 14403, 8651)`

在 current shell + ChemDraw size 的同一壳上重新 patch，得到：

- `frame-chem`
  - `iou = 0.740374`
  - `dx = 0`
  - `dy = 0`
  - `bbox = (3, 3, 551, 237)`
- `size-only`
  - `iou = 0.681619`
  - `dx = 2`
  - `dy = 6`
- `origin-height`
  - `iou = 0.560322`
  - `dx = 2`
  - `dy = 0`
- `height-only`
  - `iou = 0.529369`
  - `dx = 4`
  - `dy = 6`

而差的几项是：

- `current`
  - `iou = 0.342309`
  - `dx = 4`
  - `dy = 3`
- `x-origin-only`
  - `iou = 0.342797`
- `left-only`
  - `iou = 0.325379`

这说明当前稳定成立的规律是：

- 主问题仍然是 `frame`
- 而且比起 `origin`，`size` 尤其是 **height / vertical extent** 更重要
- `left / x-origin` 并不是主要驱动项

### 2. 围绕 `frame-chem` 做一维局部扫描

我又直接在 `frame-chem` 周围做了单变量扫描：

- `left += {-80,-40,0,+40,+80}`
- `top += {-80,-40,0,+40,+80}`
- `right += {-80,-40,0,+40,+80}`
- `bottom += {-80,-40,0,+40,+80}`

结果：

#### left

- `-80` -> `iou = 0.548803`
- `-40` -> `0.672135`
- `0` -> `0.740374`
- `+40` -> `0.646461`
- `+80` -> `0.574257`

#### top

- `-80` -> `0.549209`
- `-40` -> `0.668022`
- `0` -> `0.740374`
- `+40` -> `0.662105`
- `+80` -> `0.545683`

#### right

- `-80` -> `0.606252`
- `-40` -> `0.715031`
- `0` -> `0.740374`
- `+40` -> `0.642893`
- `+80` -> `0.524438`

#### bottom

- `-80` -> `0.540389`
- `-40` -> `0.662164`
- `0` -> `0.740374`
- `+40` -> `0.641590`
- `+80` -> `0.558969`

这说明：

- `frame-chem` 本身是一个真正的局部峰，不是偶然
- 但这个峰附近仍然有可能通过**多轴联动**再往上推
- 单轴看时，四个边单独偏离都会降分

### 3. 在 `frame-chem` 附近做联合小网格，找到了一个更高的局部最优

我进一步做了一个窄联合搜索：

- `left += {0, 20, 40}`
- `top += {0, 20, 40}`
- `right += {0, 20, 40}`
- `bottom = 0`

最优结果不是 `frame-chem` 本身，而是：

- `left +40, top +0, right +20`
  - `iou = 0.779287`
  - `dx = -1`
  - `dy = 0`
  - `bbox = (1, 3, 550, 237)`

它明显优于：

- `frame-chem`
  - `iou = 0.740374`
  - `bbox = (3, 3, 551, 237)`

这个新峰说明：

- ChemDraw 原始 frame 不是当前 Word 口径下对 **chemcore 这张 EMF** 的最优 frame
- 但它给出了非常正确的 basin
- 在这个 basin 里，通过继续收窄
  - `left` 更大一些
  - `right` 也略大一些
  - `top` 维持不变

还可以进一步逼近 Word rerun 的像素结果

### 4. 当前最可靠的判断

到这一步，最可靠的研究结论已经更新成：

1. shell identity 不是主因（之前被旧参考图污染）
2. `EMR_HEADER.frame` 才是主因
3. `frame-chem` 是非常好的方向，但不是最终最优
4. 目前最优局部点已经提升到：
   - `left +40`
   - `top +0`
   - `right +20`
   - `bottom +0`
   - `iou = 0.779287`

下一步最值得做的是：

- 围绕这个新局部峰再做更细的局部搜索
- 尤其是：
  - `left` 的 `20~60`
  - `right` 的 `0~40`
  - `top` 的 `0~20`

先确认这个峰是真峰，还是还能继续往上爬。

## 2026-05-16：围绕 `l+40 / t+0 / r+20` 的细网格继续上爬到 `0.8076`

上一节找到的粗峰是：

- `left +40`
- `top +0`
- `right +20`
- `bottom +0`
- `iou = 0.779287`

这一轮我围绕这个点继续做细搜索：

- `left += {20, 30, 40, 50, 60}`
- `top += {0, 10, 20}`
- `right += {0, 10, 20, 30, 40}`
- `bottom = 0`

对每个变体都重新：

- patch `word/media/image1.emf` 的 `EMR_HEADER.frame`
- 用 `word-copy-inline-shape.ps1` 让 Word 自己 `CopyAsPicture`
- 再和新的基线 `v28-rerun.shape2.png` 做像素 IoU

### 1. 新的最优局部峰

当前最佳点已经提升到：

- `left +30`
- `top +10`
- `right +30`
- `bottom +0`
- `iou = 0.807582`
- `dx = -1`
- `dy = 0`
- `bbox = (1, 2, 550, 237)`

这比上一轮的粗峰：

- `left +40 / top +0 / right +20`
- `iou = 0.779287`

又明显前进了一步。

### 2. Top 结果排行（前 10）

前 10 个结果里，最值得看的有：

1. `l30_t10_r30`
   - `iou = 0.807582`
   - `bbox = (1, 2, 550, 237)`

2. `l40_t10_r20`
   - `iou = 0.791274`

3. `l30_t00_r30`
   - `iou = 0.789592`

4. `l40_t00_r20`
   - `iou = 0.779287`

5. `l60_t10_r40`
   - `iou = 0.777449`

6. `l30_t10_r20`
   - `iou = 0.774446`

7. `l20_t10_r00`
   - `iou = 0.773184`

8. `l40_t10_r30`
   - `iou = 0.771099`

9. `l20_t10_r30`
   - `iou = 0.765034`

10. `l20_t00_r00`
    - `iou = 0.763839`

### 3. 这轮结果透露出的规律

和上一轮相比，现在更清楚的规律是：

- `top` 不是完全无关
  - 从 `0` 提到 `+10`，在最优 basin 里是有益的
- `left` 和 `right` 看起来要一起联动
  - 单看 bbox，最优点并不是“左边越接近 2、右边越接近 549 就一定最好”
  - 例如某些 bbox 更接近 ref，但 IoU 反而更低
- 所以当前 Word `CopyAsPicture` 口径下，
  - `frame` 不只是简单的可见 bbox 对齐
  - 它还会影响更底层的可见 ink 栅格化

### 4. 当前最好的 frame 候选

如果把这个局部最优直接换算成实际 `EMF frame`，就是：

- base chem frame:
  - `(1410, 2993, 14403, 8651)`

加上当前最佳增量：

- `left +30`
- `top +10`
- `right +30`
- `bottom +0`

得到候选：

- `(1440, 3003, 14433, 8651)`

（这里只是作为当前最优候选记录下来，**还没有**把它写进产品代码。）

### 5. 当前阶段的判断

到这一步，已经可以很明确地说：

- `frame` 研究不是原地空转
- 我们已经从
  - `current = 0.342309`
  - `frame-chem = 0.740374`
  - 粗峰 `0.779287`
  - 继续推进到细峰 `0.807582`

这说明：

- `EMR_HEADER.frame` 的局部搜索确实能持续提高 Word `CopyAsPicture` 与 ChemDraw 的像素重合度
- 而且当前增益不是噪声量级，是实打实的台阶提升

### 6. 下一步

下一步如果继续，我建议只做一件事：

- 围绕 `l30_t10_r30`
- 再做一轮更小步长（例如 `±10`、甚至 `±5`）的局部搜索

目标不是再大范围扫，而是确认：

- `0.807582` 是不是已经接近这个 basin 的真正峰值
- 还是还能再往上抬一点

## 2026-05-16：继续细化到 `0.841267`，局部峰基本钉住

基于上一节的粗细搜索结果，我又继续做了两轮更小步长的联合搜索。

### 1. 第一轮细搜索：`left/right/top` 以 `5~10` 为步长

搜索空间：

- `left += {20, 30, 40, 50, 60}`
- `top += {0, 10, 20}`
- `right += {0, 10, 20, 30, 40}`
- `bottom = 0`

这一轮把峰值从 `0.807582` 进一步推到了：

- `l30_t10_r30`
  - `iou = 0.807582`
- 之后新的最好点是：
  - `l35_t05_r25`
  - `iou = 0.827429`
  - `dx = -1`
  - `dy = 0`
  - `bbox = (1, 3, 550, 237)`

前几名大致集中在：

- `l35_t05_r25`
- `l30_t05_r30`
- `l35_t05_r30`
- `l30_t05_r25`

这说明：

- 之前的 `l30_t10_r30` 还不是峰顶
- basin 继续向
  - `top` 更小
  - `left/right` 更接近对称加大
  的方向移动

### 2. 第二轮微搜索：围绕 `l35_t05_r25` 做 `±2` 搜索

接着我又围绕：

- `left = 35`
- `top = 5`
- `right = 25`

做了一个最小邻域搜索：

- `left += {33, 35, 37}`
- `top += {3, 5, 7}`
- `right += {23, 25, 27}`

结果新的峰值是：

- `l33_t03_r27`
  - `iou = 0.841267`
  - `dx = -1`
  - `dy = 0`
  - `bbox = (1, 3, 550, 237)`

前几名已经非常密集：

1. `l33_t03_r27`
   - `0.841267`
2. `l33_t02_r27`
   - `0.839272`
3. `l33_t03_r28`
   - `0.839085`
4. `l33_t04_r27`
   - `0.836593`

可以看到：

- 峰值附近已经进入一个很平的高原
- `left=33`
- `top≈3`
- `right≈27`
这一带都非常接近

### 3. 当前最优 frame 候选

把这个最优点换算回实际 `EMF frame`：

- base chem frame:
  - `(1410, 2993, 14403, 8651)`

加上：

- `left +33`
- `top +3`
- `right +27`
- `bottom +0`

得到当前最佳候选：

- `(1443, 2996, 14430, 8651)`

### 4. 这轮的实际意义

从当前有效基线回头看，整个 `frame` 研究的台阶已经是：

- `current`
  - `0.342309`
- `frame-chem`
  - `0.740374`
- 第一轮粗峰
  - `0.779287`
- 第二轮细峰
  - `0.807582`
- 当前微峰
  - `0.841267`

这说明：

- `frame` 这条线不是“微调 noise”
- 而是确实在持续、稳定地拉近 Word `CopyAsPicture` 和 ChemDraw 的像素重合

### 5. 当前阶段的判断

到这里我会把结论写得更明确：

- 当前 Word 口径下，`EMR_HEADER.frame` 的最优解并不等于 ChemDraw 原始 frame
- 但它和 ChemDraw frame 有明显的连续关系
- 这个最优解可以通过局部搜索稳定逼近

而且现在已经出现一个很重要的信号：

- 峰值附近很多点给出的 `bbox` 完全相同：
  - `(1, 3, 550, 237)`
- 但 `iou` 仍然能继续小幅变化

这说明再往后如果还要继续优化，就不能只盯 bbox 了，必须承认：

- `frame` 同时影响可见 ink 的更细栅格化行为

### 6. 下一步

如果继续，我建议后面不再做更大面积 brute-force，而是转成两条更像工程解的问题：

1. 这个最优 `frame` 是否能从现有 geometry/bounds 直接推导出来，而不是搜索出来？
2. 在不改 preview shell 的情况下，record-time 能否直接生成这一类更接近最优点的 frame？
## 2026-05-16：same-shell 最小 fixture 说明 `frame` 至少有两类家族

### 1. 新增可复用工具：直接替换 `docx` 里的 `image1.emf`

- 新增脚本：
  - `scripts/patch-docx-image1.py`
- 作用：
  - 直接把任意 `EMF` 塞进同一个 Word shell，
  - 避免每次手工 unzip / rezip，
  - 方便做真正的 **same-shell** A/B。

### 2. `current-swapfixturepreview.docx` 实际上是一个很好的 same-shell fixture 壳

验证结果：

- `tmp/frame-word-ab/current-swapfixturepreview.docx`
  - 里面的 `word/media/image1.emf`
  - 与 `tmp/word-text-fixtures-compare/mixed-center-line.chemdraw.emf`
  - **字节完全一致**

所以它可以直接作为：
- centered text 类最小样本的 same-shell ChemDraw 参考壳。

### 3. same-shell `mixed-center-line`：`frame-chem` 强烈优于 current

构造：

- `mixed-center-line.chemref.docx`
  - 用 `current-swapfixturepreview.docx` 壳
  - 替换 `image1.emf = mixed-center-line.chemdraw.emf`
- `mixed-center-line.current.docx`
  - 同壳
  - 替换 `image1.emf = mixed-center-line.chemcore.emf`
- `mixed-center-line.frame-chem.docx`
  - 在 `current.docx` 基础上
  - 再 patch `frame = ChemDraw frame`

Word `CopyAsPicture` 结果：

- `current`
  - `IoU = 0.419537`
  - `dx = 2`
  - `dy = 0`
- `frame-chem`
  - `IoU = 0.904823`
  - `dx = 1`
  - `dy = 9`

结论：

- 对 centered mixed-script 单行文本，
- same-shell 口径下，
- **ChemDraw frame 明显是正确方向**。

### 4. same-shell `plain-center-line`：`frame-chem` 仍然有效，但提升幅度比 mixed-script 小

Word `CopyAsPicture` 结果：

- `current`
  - `IoU = 0.382959`
  - `dx = 8`
  - `dy = 2`
- `frame-chem`
  - `IoU = 0.680721`
  - `dx = -1`
  - `dy = 9`

结论：

- centered plain text 这类，
- 也支持 `frame-chem`，
- 但不像 mixed-script 那样几乎一锤定音。

### 5. same-shell `right-edge-ph`：`frame-chem` 完全不是正确家族

Word `CopyAsPicture` 结果：

- `current`
  - `IoU = 0.255300`
  - `dx = -16`
  - `dy = 3`
- `frame-chem`
  - `IoU = 0.0`
  - `dx = -12`
  - `dy = -12`

也就是说：

- 右缘窄标签类样本，
- **不能直接套 centered text 那套 ChemDraw frame**。

### 6. `right-edge-ph` 的 same-shell 小搜索给出了另一条家族

在 `visible bbox + padding` 这条族里做 12 个点的小搜索后，
当前最优点是：

- `left +0`
- `top +0`
- `right +2`
- `bottom +1`

对应 same-shell `CopyAsPicture`：

- `IoU = 0.951067`
- `dx = 0`
- `dy = 10`

比：

- `current = 0.255300`
- `frame-chem = 0.0`

都高得多。

这说明现在已经可以比较明确地收敛成：

- `frame` 的最优解 **不是单一家族**
- 至少有两类：
  1. centered text 家族：更像 `frame-chem`
  2. right-edge 窄标签家族：更像 `visible-tight + 小右/下余量`

### 7. 当前最值得继续的方向

到这里，下一步不该再把所有样本混在一起搜一个统一 `frame`。

更合理的路线是：

1. 先把样本按几何家族分开：
   - centered text
   - right-edge narrow label
   - full mixed graphic/text
2. 分别研究每类家族的最优 `frame` 规则
3. 再看 full doc 更像哪种组合，而不是强行套 centered 或 right-edge 的单一规则

## 2026-05-16：full doc 的 `frame-best` 不是折中误伤，而是在所有主区域都更优

为了判断 full doc 的 `frame-best` 是否只是“牺牲一部分区域、换来全局 IoU 更高”的折中，我补了一个区域级 IoU 工具：

- `scripts/png-region-iou.py`

并用同一个 oracle：

- `tmp/v28-wrapper-ablate10/v28-rerun.shape2.png`

对三份 full doc 候选做了同口径比较：

- `tmp/frame-word-ab/current-shellchem.wordcopy.png`
- `tmp/frame-word-ab/frame-chem-shellchem.wordcopy.png`
- `tmp/frame-word-ab/frame-best-shellchem.wordcopy.png`

区域定义：

- `top_left_substrate = (20,25,165,85)`
- `title_block = (170,0,360,55)`
- `conditions_block = (165,45,360,112)`
- `arrow = (150,50,430,80)`
- `top_right_product = (420,0,555,105)`
- `bottom_left_ligand = (0,120,175,242)`
- `bottom_center_reagent = (185,120,385,235)`
- `bottom_right_catalyst = (380,120,555,242)`

区域报告：

- `tmp/frame-word-ab/current-shellchem.regions.json`
- `tmp/frame-word-ab/frame-chem-shellchem.regions.json`
- `tmp/frame-word-ab/frame-best-shellchem.regions.json`

结果：

- `current`
  - global best：`dx=4, dy=3, IoU=0.342309`
- `frame-chem`
  - global best：`dx=0, dy=0, IoU=0.744865`
- `frame-best`
  - global best：`dx=-1, dy=0, IoU=0.841267`

更重要的是，`frame-best` 并不是靠牺牲某几块换来的，而是在所有主区域里都优于 `frame-chem`：

- `top_left_substrate`
  - `frame-chem = 0.760`
  - `frame-best = 0.872`
- `title_block`
  - `0.811 -> 0.865`
- `conditions_block`
  - `0.782 -> 0.925`
- `arrow`
  - `0.763 -> 0.972`
- `top_right_product`
  - `0.764 -> 0.830`
- `bottom_left_ligand`
  - `0.762 -> 0.850`
- `bottom_center_reagent`
  - `0.750 -> 0.858`
- `bottom_right_catalyst`
  - `0.635 -> 0.721`

结论：

- full doc 的 `frame-best` 不是“对某个局部家族偏心”的偶然折中
- 它在当前 full doc 上，确实统一改善了主要区域
- 所以下一步不能再简单假设“full doc 只是 centered 家族和 right-edge 家族的加权平均”

## 2026-05-16：centered 家族内部还要再细分，至少分成 mixed-script 和 plain 两个亚类

为了判断 full doc 的最优微偏移：

- `left +33`
- `top +3`
- `right +27`
- `bottom +0`

是不是 centered 家族的通用修正，我在 same-shell fixture 上做了一个很小的 targeted 检查。

基线仍然是：

- `mixed-center-line.chemref`
- `plain-center-line.chemref`

在 `frame-chem` 基础上只试 4 个点：

- `chem = (+0,+0,+0,+0)`
- `quarter = (+8,+1,+7,+0)`
- `half = (+16,+1,+13,+0)`
- `full = (+33,+3,+27,+0)`

结果：

### `mixed-center-line`

- `chem`：`IoU = 0.904823`
- `quarter`：`0.894295`
- `half`：`0.902575`
- `full`：`0.872650`

结论：

- mixed-script centered line 明显更喜欢原始 `frame-chem`
- full doc 那套微偏移会把它带坏

### `plain-center-line`

- `chem`：`IoU = 0.680583`
- `quarter`：`0.681116`
- `half`：`0.695755`
- `full`：`0.719335`

结论：

- plain centered line 则反过来，更吃 full-doc 风格的正向微偏移
- 而且偏移越大越好（至少在这 4 个点上如此）

这说明：

- centered 家族本身也不是单一家族
- 至少还要分成：
  1. `mixed-script centered`：更像 `frame-chem`
  2. `plain centered`：更像 `frame-chem` 再加一层正向微偏移

因此当前最稳的判断是：

- `right-edge narrow label` 是一类
- `mixed-script centered` 是一类
- `plain centered` 是另一类

也就是说，已经不适合继续用“一个统一 frame 规则”去解释所有样本。下一步更像应该做：

- full doc 的对象分族
- 再研究这些亚类在同一 Word replay 口径下如何组合成最终最优 frame

### 补充：`mixed-center-two-line` 和 `mixed-center-block` 把 centered 家族从二分法推成了连续梯度

为了避免只凭两个样本就下结论，我又把：

- `mixed-center-two-line`
- `mixed-center-block`

也塞进了同一个 same-shell 壳里做对照：

- 产物目录：`tmp/frame-word-ab/sameshell-fixtures-extra/`

先看 `current -> frame-chem`：

- `mixed-center-two-line`
  - `current = 0.469240`
  - `frame-chem = 0.876091`
- `mixed-center-block`
  - `current = 0.450178`
  - `frame-chem = 0.854650`

说明它们仍然明确属于 centered 家族。

再看 4 点小矩阵：

- `chem = (+0,+0,+0,+0)`
- `quarter = (+8,+1,+7,+0)`
- `half = (+16,+1,+13,+0)`
- `full = (+33,+3,+27,+0)`

结果：

- `mixed-center-two-line`
  - `chem = 0.875907`
  - `quarter = 0.882337`
  - `half = 0.858742`
  - `full = 0.846681`
- `mixed-center-block`
  - `chem = 0.854617`
  - `quarter = 0.856661`
  - `half = 0.870833`
  - `full = 0.834889`

把 4 个 centered 样本并在一起看，会得到一条更像连续谱的规律：

- `mixed-center-line`：最喜欢 `chem`
- `mixed-center-two-line`：最喜欢 `quarter`
- `mixed-center-block`：最喜欢 `half`
- `plain-center-line`：最喜欢 `full`

这里还有一个重要细节：`mixed-center-two-line` 这个 fixture 名字有误导性。  
它在 payload 里其实是：

- 两行 centered text
- `runs = 5`
- `scripts = ['normal']`

也就是说，它是 **两行 plain centered**，不是 mixed-script。

对应的 4 个样本特征表：

- `mixed-center-line`
  - `lines = 1`
  - `width = 960`
  - `height = 66.67`
  - `scripts = ['normal', 'subscript']`
- `mixed-center-two-line`
  - `lines = 2`
  - `width = 960`
  - `height = 133.33`
  - `scripts = ['normal']`
- `mixed-center-block`
  - `lines = 4`
  - `width = 720`
  - `height = 133.33`
  - `scripts = ['normal', 'subscript']`
- `plain-center-line`
  - `lines = 1`
  - `width = 880`
  - `height = 66.67`
  - `scripts = ['normal']`

这说明 centered 家族的最优偏移，至少不是单纯由某一个维度决定的：

- 不是只看是不是 mixed-script
- 也不是只看 line count
- 也不是只看 box height

更像是一个由多个对象特征共同决定的连续函数。

因此后面的正确方向更像是：

- 继续收集 centered 样本
- 从对象特征出发拟合它们对 `chem -> quarter -> half -> full` 这条轴的位置

而不是继续用“mixed vs plain”这种过粗的二分法。 

### 补充：full doc 的 centered 文本对象，本身就落在不同原型附近

为了不只盯最小样本，我补了一个对象特征工具：

- `scripts/dump-text-object-features.py`

并导出了：

- `tmp/current-thiocyanation.text-features.json`
- `tmp/word-text-fixtures-compare/*.text-features.json`

先看 full doc 里真正居中的 3 个文本对象：

1. `obj_text_004`
   - `4DPAIPN (2 mol%) / Cu(MeCN)4PF6 ... / TMSCN ... / PhthNCO2SCH2Ph ...`
   - `lines = 4`
   - `runs = 10`
   - `scripts = ['normal', 'subscript']`
   - `width = 681.36`
   - `height = 125.33`

2. `obj_text_005`
   - `CH3CN (0.2 M) / 420 nm 3W, 10 °C, 24 h`
   - `lines = 2`
   - `runs = 3`
   - `scripts = ['normal', 'subscript']`
   - `width = 434.72`
   - `height = 62.67`

3. `obj_text_006`
   - `76% yield, 94% ee / d.r. > 20:1`
   - `lines = 2`
   - `runs = 1`
   - `scripts = ['normal']`
   - `width = 331.76`
   - `height = 61.33`

用一个很粗的对象特征距离去找最接近的 fixture 原型后，得到：

- `obj_text_004` 最接近 `mixed-center-block`
- `obj_text_006` 最接近 `plain-center-line`
- `obj_text_005` 则不太像现有任何一个 fixture
  - 勉强最近的是 `mixed-center-line`
  - 但它实际上是：
    - 两行
    - mixed-script
    - 比 `mixed-center-line` 短很多
    - 又比 `plain-center-line` 更复杂

这很重要，因为它解释了为什么：

- full doc 的最佳 `frame-best`
- 不会刚好等于某一个 centered fixture 的最优点

更准确地说：

- `obj_text_004` 会把 full doc 往 `half` 那边拉
- `obj_text_006` 会把 full doc 往 `full` 那边拉
- `obj_text_005` 是一个当前 **缺 oracle 原型** 的中间态，它很可能在决定最终 full doc 最优点时起了关键作用

所以现在最有信息量的下一步，不是继续只在现有 4 个 centered fixture 上做离散推理，而是：

- 明确承认 `obj_text_005` 这一类还缺最小 oracle
- 后续如果能拿到更多 ChemDraw 样本，优先补这类：
  - 两行
  - mixed-script
  - 较窄 centered text

在没有新 oracle 之前，我们也至少知道：

- full doc 之所以偏向 `frame-best`
- 不是因为它“整体像某一个 fixture”
- 而是因为它内部本来就是多种 centered 子原型的混合。 

## 2026-05-16：centered 家族不是二分，而更像一条从 `chem` 到 `full` 的连续梯度

为了避免只凭 `mixed-center-line / plain-center-line` 两个样本就下过头的结论，我又把：

- `mixed-center-two-line`
- `mixed-center-block`

也拉进了同一个 same-shell 壳：

- `tmp/frame-word-ab/sameshell-fixtures-extra/`

先看 `current -> frame-chem`：

### `mixed-center-two-line`

- `current`：`IoU = 0.469240`
- `frame-chem`：`0.876091`

### `mixed-center-block`

- `current`：`IoU = 0.450178`
- `frame-chem`：`0.854650`

所以这两类仍然明确属于 centered 家族，而不是 right-edge 家族。

然后我继续用和前面一样的 4 个点去试：

- `chem = (+0,+0,+0,+0)`
- `quarter = (+8,+1,+7,+0)`
- `half = (+16,+1,+13,+0)`
- `full = (+33,+3,+27,+0)`

结果：

### `mixed-center-two-line`

- `chem`：`0.875907`
- `quarter`：`0.882337`
- `half`：`0.858742`
- `full`：`0.846681`

### `mixed-center-block`

- `chem`：`0.854617`
- `quarter`：`0.856661`
- `half`：`0.870833`
- `full`：`0.834889`

把 centered 样本放在一起看，会出现一个比“二分法”更像真实规律的梯度：

- `mixed-center-line`：最喜欢 `chem`
- `mixed-center-two-line`：最喜欢 `quarter`
- `mixed-center-block`：最喜欢 `half`
- `plain-center-line`：最喜欢 `full`

这说明：

- centered 家族内部不是简单分成 “mixed-script” 和 “plain” 两个完全离散亚类
- 更像是从 `chem` 向 `full-doc` 微偏移逐步过渡的一条连续谱

至少当前样本下：

1. mixed-script 越强、越“尖”的 centered 对象，越靠近 `chem`
2. centered 文本块越大/越块状，越能接受一点正向微偏移
3. 纯 plain centered text 最吃 `full-doc` 那种偏移

所以接下来最值得研究的，不再是“到底属于 mixed 还是 plain”，而是：

- centered 家族的最优微偏移，能不能用某个对象级特征连续预测

例如：

- 文本块高度
- 行数
- plain/mixed run 比例
- 上下标 run 占比
- 可见 bbox 的长宽比

也就是说，我们下一步更像要从“分家族”迈到“按对象特征拟合 centered frame 偏移”。 


## 2026-05-16???? full doc ? `obj_text_005` ?????????? `frame-best/full`

?????? `obj_text_005`?`CH3CN (0.2 M) / 420 nm 3W, 10 ?C, 24 h`?????? standalone proxy fixture?

- `tmp/word-text-fixtures/ch3cn-two-line.cdxml`

????????? shell ??? full doc oracle?

???????????????????

- standalone `current.docx` ?? full doc ????????
- ??? `ch3cn-two-line` ?? full-doc shell??? full doc ??? `obj_text_005` ??????????????
- ??? proxy ?? centered ???????????????

???????????????????

- ??? full doc oracle `tmp/v28-wrapper-ablate10/v28-rerun.shape2.png` ?? `obj_text_005` ????????
- ???? full doc ???? frame ????????? IoU?

???????????

- `ch3cn = (220,70,360,108)`

?????????? tight oracle??????????? centered ??? `obj_text_005` ??????????

### ??

- `current`?`IoU = 0.757832`
- `frame-chem`?`IoU = 0.820333`
- `frame-quarter`?`IoU = 0.845196`
- `frame-half`?`IoU = 0.755257`
- `frame-best`?`IoU = 0.875336`

### ??

1. `obj_text_005` ????? `current` ???
2. ?????? `frame-chem`???? `frame-best/full` ??????????
3. ?????????
   - `frame-best/full`
   - `quarter`
   - `frame-chem`
   - `current ~= half`

????? Word replay ??????? `mixed-center-line` ???? `chem`?????? full-doc ?? frame ????

### ? centered ???????

??????????`obj_text_005` ????????????????????

????????????

- `obj_text_005` ?????????? fixture ???
- ??? full doc ??? Word ??????????? `chem -> quarter -> half -> full` ??????
- ???? region ??????????? `full/frame-best` ???

?????????????????????????

- ????????? `full` ? centered ????
- ?? `obj_text_004`??? `mixed-center-block`??`obj_text_006`??? `plain-center-line`????? full doc ?????? `frame-best`?


## 2026-05-16?`obj_text_005` ? centered ?????????`half` ?????? trough

?? `obj_text_005` ??? oracle ????? `ch3cn = (220,70,360,108)` ??????? full doc ????

- `frame-quarter-shellchem`
- `frame-half-shellchem`

?? `current / chem / quarter / half / full(best)` ??????????? full-doc same-shell ????????

### `obj_text_005 / ch3cn` ????

- `current`?`0.757832`
- `frame-chem`?`0.820333`
- `frame-quarter`?`0.845196`
- `frame-half`?`0.755257`
- `frame-best`?`0.875336`

### ????

??? `obj_text_005` ?????? `full/frame-best` ????????????
`chem -> quarter -> half -> full` ??????????

?????

- `quarter` ? `chem` ?????
- ??? `half`??? IoU ??????? `current`?
- ?? `full(frame-best)`????????

??????? centered frame ?????? `obj_text_005` ?????

- ??????
- ???????
- ?????? trough

### ????????

?? centered ????????????????????

- `mixed-center-line` ? `chem`
- `mixed-center-block` ? `half`
- `plain-center-line` ? `full`
- ?? `obj_text_005` ??????????????????????

???? region ?????????????

- `obj_text_005` ???????? full ???????
- ????? centered ????? frame ??????????????????

### ????? full doc ?????

???? `current / chem / quarter / half / best` ????`half` ??? `ch3cn` ????
??????????????

- `title_block`?`0.719285`
- `conditions_block`?`0.720636`
- `arrow`?`0.695868`
- `top_right_product`?`0.683086`
- `bottom_center_reagent`?`0.660435`

? `quarter` ???? `chem` ???????? `best`?

????????????

- `frame-best` ????? `chem -> full` ?????????
- `quarter` ???????? basin?
- `half` ?????? Word replay ?????????
- `best` ?????? basin?

???????????????? centered ?????????????????

- ????? frame ??? `quarter` ? `half` ?????
- ??? `obj_text_005` ???????????????


## 2026-05-16?`obj_text_005` ??????? `left`?? full-doc ??????? `top/bottom`

??? `obj_text_005` ???? `frame-best/full` ?????? `quarter -> half -> full` ???????????????

- ?????? frame ????? `CH3CN / 420 nm` ???
- ??? `half` ????? trough?
- ???? `full(frame-best)` ?????????

### 1. `quarter -> half` ???????????

? `quarter` ? `half`????????????

- `quarter-left8`
- `quarter-right6`

?? `ch3cn` ?????

- `quarter-base`?`0.845196`
- `quarter-left8`?`0.833043`
- `quarter-right6`?`0.830009`
- `half`?`0.755257`

????

- `left` ? `right` ??????? `obj_text_005` ???????????
- ????? `half` ???? IoU ????????
- ?? `half` ????????????????????????????

### 2. `half -> full` ??`obj_text_005` ?? `left +14`?????? `top/bottom +2`

??? `half-plus-right` ? `full` ?????????

- `half-plus-y`??? `top/bottom +2`
- `hpr-plus-left14`??? `left +14`
- `full`?????

?? `ch3cn` ?????

- `half-plus-right`?`0.810692`
- `half-plus-y`?`0.807356`
- `hpr-plus-left14`?`0.879605`
- `full`?`0.875336`

?????????

- ? `obj_text_005` ??????????? `left +14`?
- ??? `top/bottom +2` ???????
- ?? `hpr-plus-left14` ??? IoU ???? `full`?

?????`obj_text_005` ?????? full-doc ??? `y` ?????????????/?????????

### 3. ? full-doc ???????? `top/bottom +2`

???????????????????????

- `title_block`
- `conditions_block`
- `arrow`

`hpr-plus-left14` vs `full`?

- ???`0.793335 -> 0.841267`
- `title_block`?`0.850177 -> 0.864653`
- `conditions_block`?`0.853150 -> 0.924503`
- `arrow`?`0.843542 -> 0.971687`

????

- `obj_text_005` ?????? `left +14`
- ? full-doc ????????? `full(frame-best)`?????
  - `top/bottom +2` ??????
  - ??? `conditions_block` ? `arrow`
  ????????

### ?????????

????? centered/full-doc ? frame ?????????

- full-doc ?? `frame-best` ????????????
- ?????????????????????
  - `obj_text_005` ????/left ??
  - ?????????? vertical/top-bottom ??

?????????? frame ?????????????????????????

- ??? `frame` ???????????
- ?????
  - `left/right` ??? centered ?????
  - `top/bottom` ???????/?????


## 2026-05-16?`top/bottom` ??????? `conditions_block + arrow`???? `obj_text_005`

?? `obj_text_005` ?????? full-doc ?????????????????????

- `title_block`
- `conditions_block`
- `arrow`

??????????????????????? `top/bottom +2`?

?????????

- `half-plus-y = (1429,2996,14416,8651)`
- `half-plus-right = (1429,2994,14430,8649)`
- `hpr-plus-left14 = (1443,2994,14430,8649)`
- `full = (1443,2996,14430,8651)`

### ??

#### `half-plus-y`
- `title_block`?`0.731805`
- `conditions_block`?`0.796805`
- `arrow`?`0.825028`

#### `half-plus-right`
- `title_block`?`0.765321`
- `conditions_block`?`0.766751`
- `arrow`?`0.744534`

#### `hpr-plus-left14`
- `title_block`?`0.850177`
- `conditions_block`?`0.853150`
- `arrow`?`0.843542`

#### `full`
- `title_block`?`0.864653`
- `conditions_block`?`0.924503`
- `arrow`?`0.971687`

### ??

????????????????????

1. `obj_text_005 / ch3cn`
- ?? `left`
- ? `top/bottom +2` ?????????????

2. `conditions_block`
- ??? `y` ??????`0.796805`?
- ?????????? `left + y` ??????`0.924503`?

3. `arrow`
- ????? `y` ?????
- `half-plus-right` ???`0.744534`?
- `half-plus-y` ???? `0.825028`
- `full` ???? `0.971687`

### ???????????

????? full-doc ? frame ????????

- `left/right` ???????? centered ???
  ?? `obj_text_005` ?????????????
- `top/bottom` ???????
  - `conditions_block`
  - `arrow`
  ????????? ink ??????????

?????`frame-best` ???????????????????????? centered fixture???????

- ?????? `obj_text_005` ??????
- ???? `conditions_block / arrow` ??????

????????????frame ??????????????????

- ??????????????? `left/right` ? `top/bottom` ???????????
- ?????????????? centered ?????

## 2026-05-16：centered text 家族可以直接拟合成 `visible -> frame` pad 规则

这轮不再只盯 `frame-best/full` 的搜索结果，而是把 4 个 same-shell centered fixture 的
`ChemDraw frame` 统一改写成相对 `visible frame` 的 pad：

- `mixed-center-line`
- `plain-center-line`
- `mixed-center-two-line`
- `mixed-center-block`

分析产物：

- `tmp/frame-word-ab/centered-family-fit.json`
- `scripts/fit-centered-frame-family.py`

### 1. 相对 `visible frame` 的 centered 家族 pad 很规整

4 个 fixture 的 pad（单位：px，相对 `visible frame`）：

- `mixed-center-line`
  - `left = +3.686`
  - `top = -3.024`
  - `right = +18.239`
  - `bottom = +0.095`
- `plain-center-line`
  - `left = +6.897`
  - `top = -3.118`
  - `right = +14.549`
  - `bottom = +0.094`
- `mixed-center-two-line`
  - `left = +3.779`
  - `top = -3.118`
  - `right = +17.763`
  - `bottom = +0.661`
- `mixed-center-block`
  - `left = +3.686`
  - `top = -3.024`
  - `right = +18.239`
  - `bottom = +1.796`

直接能看出 3 条规律：

1. 水平总额外量几乎恒定：
   - `left + right = 21.446 ~ 21.925 px`
2. `top` 基本是常数：
   - 大约 `-3.1 px`
3. `bottom` 会随块高增长：
   - 一行时接近 `0.1 px`
   - 两行时约 `0.66 px`
   - 四行时约 `1.80 px`

### 2. 拟合结果

对这 4 个 centered fixture 做线性拟合后：

- `left_px ≈ 10.3741 - 0.0145064 * visible_width_px`
  - `R² = 0.9964`
- `right_px ≈ 10.7087 + 0.0160568 * visible_width_px`
  - `R² = 0.9751`
- `top_px ≈ -3.1058 + 0.0004981 * visible_height_px`
  - `R² = 0.166`
  - 更像常数项，实际可直接近似为 `-3.1 px`
- `bottom_px ≈ -0.6054 + 0.0180983 * visible_height_px`
  - `R² ≈ 1.0`

这意味着 centered text 家族已经不再只是“`chem / quarter / half / full` 哪个更像”的口头描述，
而是第一次出现了可直接从 `visible width/height` 推 `frame` 分量的经验规则。

### 3. 套到 full doc 的 3 个 centered text object 上

将上面的拟合直接代入 full doc 的 3 个 centered 文本对象：

- `obj_text_004`（四行标题块，`width=681.36`, `height=125.33`）
  - 预测：
    - `left = +0.490`
    - `right = +21.649`
    - `top = -3.043`
    - `bottom = +1.663`
- `obj_text_005`（`CH3CN ... / 420 nm ...`，`width=434.72`, `height=62.67`）
  - 预测：
    - `left = +4.068`
    - `right = +17.689`
    - `top = -3.075`
    - `bottom = +0.529`
- `obj_text_006`（产率两行，`width=331.76`, `height=61.33`）
  - 预测：
    - `left = +5.561`
    - `right = +16.036`
    - `top = -3.075`
    - `bottom = +0.505`

这和前面的 qualitative 观察是能对上的：

- `obj_text_004` 宽、行数多，所以天然要求：
  - 更小的 `left`
  - 更大的 `right`
  - 更大的 `bottom`
- `obj_text_005 / obj_text_006` 更窄，所以 `left` 会更大、`right` 更小
- 这解释了为什么 full doc 内部不同 centered 子原型会把最优 `frame` 往不同方向拉

### 4. 为什么这还不能直接等于 full-doc `frame-best`

虽然 centered 家族已经能拟合出一条像样规则，但 full doc 的全局最优点：

- `frame-best = (1443, 2996, 14430, 8651)`

仍然不能只靠 centered text 解释。原因前面已经量过：

- `top/bottom` 对 `conditions_block` 和 `arrow` 很值钱
- `left/right` 对 `obj_text_005` 这类窄 centered text 很值钱

所以现在更准确的模型是：

- **centered text 家族**
  - 提供一条“基准 frame pad 规则”
- **non-text / arrow / conditions family**
  - 会在这个基准上继续推 `top/bottom`
  - 甚至推 `right`

也就是说，`frame-best/full` 更像：

- `centered-family visible-based rule`
- 再叠加
- `conditions + arrow` 的额外修正项

### 当前结论

这是到目前为止最有价值的新进展之一：

- 我们第一次拿到了可参数化的 centered family frame 规则
- 而不是只靠 brute-force 搜索 `frame-best`
- 下一步就可以继续研究：
  - full doc 里 `conditions_block + arrow` 到底贡献了哪些额外分量
  - 看能不能把 `frame-best` 分解成：
    - `centered rule`
    - `non-text correction`

### 5. `frame-best` 相对 full visible 的 pad，离 `obj_text_004` 最近

把 full doc 的 `frame-best` 也改写成相对 full visible 的 pad：

- `frame-best(full visible)`
  - `left = -0.567`
  - `top = +0.378`
  - `right = +23.432`
  - `bottom = +2.173`

再和 3 个 centered text object 的 family 预测值直接做差：

- 相对 `obj_text_004`（四行标题块）
  - `Δleft = -1.057`
  - `Δtop = +3.421`
  - `Δright = +1.783`
  - `Δbottom = +0.510`
- 相对 `obj_text_005`
  - `Δleft = -4.635`
  - `Δtop = +3.453`
  - `Δright = +5.743`
  - `Δbottom = +1.644`
- 相对 `obj_text_006`
  - `Δleft = -6.128`
  - `Δtop = +3.453`
  - `Δright = +7.396`
  - `Δbottom = +1.668`

这说明：

- full doc 的最终最优 `frame-best`
  - 在**水平 pad 结构**上最接近 `obj_text_004`
  - 而不是 `obj_text_005/006`
- 也就是说，full doc 的 frame 基准更像：
  - 先由四行标题块确定 centered family 的主方向
  - 再由 `conditions_block + arrow` 把 `top/bottom` 和少量 `right` 往外推

这一步很重要，因为它把 full doc 的最优 frame 分成了两层：

1. **水平主基准**
   - 主要由 `obj_text_004`（四行标题块）决定
2. **非文本修正**
   - 主要由 `conditions_block + arrow` 决定
   - 其效果更像：
     - 往下补一点
     - 往右再补一点
     - 同时把顶部也略微往下压

## 2026-05-16 新进展：`frame-quarter -> frame-best` 的残差几乎都落在非标题区

为了避免再靠手工口算区域提升，新增了一个通用脚本：

- `scripts/compare-region-reports.py`

它可以直接比较两份 `png-region-iou.py` 报告，输出每个区域的：

- `base_iou`
- `target_iou`
- `delta_iou`
- `delta_intersection`
- `delta_only_ours`
- `delta_only_reference`

对应的 full-doc same-shell 对比结果在：

- `tmp/frame-word-ab/quarter-to-best.delta.json`

这里把 `frame-quarter-shellchem` 当作“centered-family 工程基准”，再看它到 `frame-best-shellchem` 的提升。结果很清楚：

- 全局：`0.770339 -> 0.841267`，`+0.070929`
- 最大增益区域按 `delta_iou` 排序：
  1. `arrow`: `+0.165292`
  2. `conditions_block`: `+0.104738`
  3. `bottom_right_catalyst`: `+0.087969`
  4. `bottom_center_reagent`: `+0.078198`
  5. `top_left_substrate`: `+0.060337`
  6. `bottom_left_ligand`: `+0.049916`
  7. `top_right_product`: `+0.043866`
  8. `ch3cn`: `+0.030141`
  9. `title_block`: `+0.013597`

这说明一件很重要的事：

- 从 `quarter` 到 `best` 的修正，**不是主要为了标题块**
- 标题区和 `CH3CN` 区都还有收益，但已经很小
- 真正的主增益来自：
  - `arrow`
  - `conditions_block`
  - 底部 `catalyst / reagent`
  - 以及若干非标题骨架区

也就是说，`frame-quarter` 已经可以视为一个比 `obj_text_004` 拟合值更好的 **centered-family 工程基准**；
而 `frame-best` 则更像是在这个基准上，再叠加一层面向 **非标题 / 非纯 centered 文本** 的修正。

## 2026-05-16 新进展：full-doc 的 frame 修正不能直接套回 centered-only / right-edge fixture

为了验证“第二家族修正”是不是某种 centered 通用规律，我做了一个反证实验：

- 取 full doc 上 `frame-chem -> frame-best` 的 header frame 增量
- 直接套回 same-shell fixture 的 `frame-chem` 版本
- 再用 Word `CopyAsPicture` 和对应 ChemDraw oracle 做逐像素比较

full-doc 的这组增量是（HIMETRIC）：

- `left +30`
- `top +3`
- `right +27`
- `bottom +2`

结果保存在：

- `tmp/frame-word-ab/full-delta-on-fixtures.json`

关键结果如下：

- `mixed-center-line`
  - `chem = 0.904823`
  - `delta = 0.856653`
  - 明显变差
- `plain-center-line`
  - `chem = 0.680721`
  - `delta = 0.694470`
  - 只小幅变好
- `mixed-center-two-line`
  - `chem = 0.876091`
  - `delta = 0.871995`
  - 微幅变差
- `mixed-center-block`
  - `chem = 0.854650`
  - `delta = 0.812987`
  - 明显变差
- `right-edge-ph`
  - `current = 0.203061`
  - `chem = 0.000000`
  - `delta = 0.000000`
  - 说明这组 centered/full-doc 修正对右缘窄标签家族完全不适用

这一步的意义很大：

- full-doc 的 `frame-best` 修正 **不是** 一个可以跨家族复用的统一规则
- 它对 `plain-center-line` 这类样本可能有一点帮助
- 但对 `mixed-center-line / mixed-center-block / right-edge-ph` 都不成立

所以当前更靠谱的结构化理解是：

1. **centered-family 基准**
   - 当前工程上最像的是 `frame-quarter`
2. **full-doc 专属修正**
   - 主贡献集中在 `arrow + conditions + bottom catalyst/reagent`
   - 不能直接拿去套 centered-only fixture
   - 更不能拿去套 `right-edge-ph` 这类窄右缘标签家族

### 当前结论（更新版）

到这里为止，frame 语义至少已经可以稳定拆成三层：

1. `current`
   - 我们现在产品里的默认 frame
2. `centered-family baseline`
   - 当前工程上最像 `frame-quarter`
3. `full-doc residual correction`
   - 主增益来自 `arrow / conditions / bottom catalyst/reagent`
   - 不是 centered-only 可复用规则

因此下一步最合理的方向，不是继续强找“单一全局 frame 公式”，而是：

- 继续研究 `quarter -> best` 这一层 residual
- 把它和 `arrow / conditions / bottom catalyst/reagent` 的几何量联系起来
- 看它能否被表述成第二套、仅在 full-doc 这类混合图上启用的修正规则

## 2026-05-16 新进展：`quarter -> best` 只是一组 ~2px 级别的微修正

我把 full doc 的几组关键 frame 放到同一口径里重新换算了一次。基于：

- `tmp/default-svgpad-analysis/thiocyanation.preview-bounds.json`

得到 full doc 当前可见宽度口径下的 `HIMETRIC -> visible px` 换算后：

- `quarter -> best`
  - HIMETRIC: `[+22, +2, +20, +2]`
  - 约等于像素: `[+2.051, +0.187, +1.865, +0.187]`
- `chem -> best`
  - HIMETRIC: `[+30, +3, +27, +2]`
  - 约等于像素: `[+2.797, +0.280, +2.518, +0.187]`

这说明一个很关键的事实：

- `frame-quarter -> frame-best` 不是大尺度重排
- 而是一组 **2px 左右的水平微修正 + 0.2px 左右的垂直微修正**

也就是说，full doc 剩下的大部分同壳差异，其实是 Word replay 对 `EMR_HEADER.frame` 的一个很敏感的、近阈值级别的解释差。

## 2026-05-16 新进展：`quarter -> best` 可以拆成水平项和垂直项，而且垂直项更值钱

为了把这组微修正继续分解，我做了两个中间点：

- `frame-quarter-lr-shellchem`
  - 只加水平项：`left +22, right +20`
- `frame-quarter-tb-shellchem`
  - 只加垂直项：`top +2, bottom +2`

并分别和 `frame-quarter-shellchem`、`frame-best-shellchem` 做区域增益对比。  
新增通用脚本：

- `scripts/compare-region-reports.py`

产物：

- `tmp/frame-word-ab/quarter-to-lr.delta.json`
- `tmp/frame-word-ab/quarter-to-tb.delta.json`
- `tmp/frame-word-ab/tb-to-best.delta.json`

### 1. `quarter -> lr-only`

全局：

- `0.770339 -> 0.793335`
- `+0.022996`

增益最大的区域：

1. `bottom_right_catalyst`: `+0.057831`
2. `arrow`: `+0.037147`
3. `ch3cn`: `+0.034409`
4. `conditions_block`: `+0.033385`
5. `top_right_product`: `+0.021985`

解释：

- **水平微修正** 对 `catalyst / ch3cn / top-right product` 这类区域最值钱
- 对标题块本身帮助很小

### 2. `quarter -> tb-only`

全局：

- `0.770339 -> 0.815914`
- `+0.045576`

增益最大的区域：

1. `arrow`: `+0.141665`
2. `conditions_block`: `+0.084225`
3. `top_left_substrate`: `+0.058574`
4. `bottom_center_reagent`: `+0.051423`
5. `bottom_left_ligand`: `+0.040417`

解释：

- **垂直微修正** 带来的收益明显大于水平项
- 它主要在修：
  - `arrow`
  - `conditions_block`
  - 左侧和中部的非文本骨架区

### 3. `tb-only -> best`

全局：

- `0.815914 -> 0.841267`
- `+0.025353`

剩余增益最大的区域：

1. `bottom_right_catalyst`: `+0.070883`
2. `ch3cn`: `+0.029388`
3. `bottom_center_reagent`: `+0.026775`
4. `arrow`: `+0.023627`
5. `top_right_product`: `+0.023448`

解释：

- 在垂直项补完后，**剩下最值得水平项继续修的**，是：
  - `bottom_right_catalyst`
  - `ch3cn`
  - `top_right_product`

### 这一步带来的结构化结论

现在 `quarter -> best` 已经可以拆成两个近乎正交的分量：

1. **垂直微修正（主增益项）**
   - 只需要大约 `top/bottom +0.19 px`
   - 但对 `arrow + conditions_block` 非常值钱
2. **水平微修正（次增益项）**
   - 大约 `left +2.05 px`, `right +1.86 px`
   - 更偏向 `catalyst / ch3cn / top-right product`

也就是说，第二家族不应该再被笼统叫成“full-doc residual correction”，而更像：

- `vertical residual family`
  - 主修 `arrow / conditions / left-middle non-text`
- `horizontal residual family`
  - 主修 `catalyst / ch3cn / right-side product`

这比之前“centered 基准 + 一团 residual”更进一步，因为 residual 自己也能继续拆。


## 2026-05-16 ????full doc ?????top-level object / molecule component?

?????
- `scripts/dump-document-geometry-features.py`

???
- `tmp/current-thiocyanation.geometry.json`

### 1. top-level object ????

`current-thiocyanation.payload.json` ???? wrapper??????????? `chemcoreDocumentJson`?

full doc ? top-level object ???? 8 ??
- `obj_cdxml_merged_molecule`
- `obj_line_001`
- `obj_text_001 .. obj_text_006`

?????????????
- ??
- ??
- ??
- ???
- ??

?????????? **?? merged molecule object** ??????? 6 ??????? top-level object?

### 2. merged molecule ?????? 5 ?????

? `nodes + bonds` ???????5 ? component ? role guess ?????

- `top_right_product_component`
  - `worldBox = [1084.00, 283.76, 1316.80, 411.10]`
  - `labelTexts = ['CN', 'S', 'Ph']`

- `top_left_substrate_component`
  - `worldBox = [217.44, 363.04, 450.24, 439.84]`
  - `labelTexts = []`

- `bottom_right_catalyst_component`
  - `worldBox = [1085.71, 585.60, 1336.35, 800.27]`
  - `labelTexts` ???? `Ph / CN / NC / N`

- `bottom_left_ligand_component`
  - `worldBox = [138.32, 597.42, 460.59, 789.84]`
  - `labelTexts = ['O', 'N', 'O', 'N']`

- `bottom_center_reagent_component`
  - `worldBox = [576.40, 606.46, 932.43, 784.70]`
  - `labelTexts = ['O', 'N', 'O', 'O', 'O', 'S']`

????????????????? residual ???? payload ?????????

### 3. ?????????????

top-level object ??
- `leftmost`: `obj_cdxml_merged_molecule`
- `topmost`: `obj_cdxml_merged_molecule`
- `rightmost`: `obj_text_006`
- `bottommost`: `obj_cdxml_merged_molecule`

?????
- **??????????????? `obj_text_006` ???**
- ???????????? doc ???

molecule component ??
- `leftmost`: `bottom_left_ligand_component`
- `topmost`: `top_right_product_component`
- `rightmost`: `bottom_right_catalyst_component`
- `bottommost`: `bottom_right_catalyst_component`

??? merged molecule ???
- ????????????
- ???????????
- ???????????????

### 4. merged molecule ? top-level bbox ????????? label ink

????????
- top-level molecule `payload.bbox` ?? transform ?? `worldBox`
- 5 ? component ? `worldBox` union

???
- `topLevelMoleculeWorldBox = [136.88, 283.63, 1335.41, 800.14]`
- `componentUnionWorldBox = [138.32, 283.76, 1336.35, 800.27]`

component union ?? top-level molecule box ????
- `left = +1.44`
- `top = +0.13`
- `right = +0.94`
- `bottom = +0.13`

???
- molecule top-level bbox ???????? component union **???** `1.44 px`
- ?? `top/right/bottom` ?? **???** ?? component union
- ??? molecule ??? bbox ???? label glyph ?????????
  - ???????
  - ???????/???

### 5. ? `quarter -> best` residual family ???

???????? `lr/tb residual` ????????????????

- `tb residual` ????? `top/bottom +0.19 px`
  - ? merged molecule ??? component union ?? top-level bbox ? `top/bottom` ?????? `+0.13 px`
  - ??????
  - ?? vertical residual ????????? molecule bbox ? `product/catalyst` label ink ???

- `lr residual` ??????
  - molecule component union ???? top-level bbox ?? `+0.94 px`
  - ????????????????????
  - ?????????? `obj_text_006`
  - ?? full-doc ??? residual ??? **???? + molecule ??????????**

- ???????
  - top-level molecule bbox ??? component union ????? `1.44 px`
  - ???? `left` ? residual **??** molecule label overflow ?????
  - ?? centered baseline / Word replay ????????

### ??????

????full-doc `frame` ???????????

1. `centered baseline`
   - ??????? `frame-quarter`

2. `molecule box correction`
   - ????
     - `product` ??
     - `catalyst` ?/??

3. `doc-level right-edge text driver`
   - ??? `obj_text_006`

??????????????????????????????????? `frame`????
- centered baseline
- molecule box ??
- ???????

??????????


## 2026-05-16 ????component union overflow ????? frame ???

??? `current-thiocyanation.geometry.json` ????????????????

- ?? merged molecule ? component union ?? top-level bbox ?
  - `top +0.13 px`
  - `right +0.94 px`
  - `bottom +0.13 px`
- ????????????? `frame-quarter`????????????????????
  - `frame-geom-qplus-r10-tb1`
  - `frame-geom-qplus-r10-tb2`
  - `frame-geom-no-left`

??? frame ????
- `qplus-r10-tb1 = (1421, 2995, 14420, 8650)`
- `qplus-r10-tb2 = (1421, 2996, 14420, 8651)`
- `no-left      = (1421, 2996, 14430, 8651)`

Word same-shell ???
- `frame-quarter-shellchem`: `IoU = 0.770339`
- `frame-best-shellchem`: `IoU = 0.841267`
- `frame-geom-qplus-r10-tb1`: `IoU = 0.730468`
- `frame-geom-qplus-r10-tb2`: `IoU = 0.734627`
- `frame-geom-no-left`: `IoU = 0.734627`

### ??

????
- `component union` ?? top-level molecule bbox ?????**???? residual ???**
- ?**?????? frame ???**
- ?????`quarter -> best` ???? molecule bbox ???? label union?????

??????????
- molecule overflow ???????
- ???? `frame-best` ???????? Word replay / centered baseline / ????????????
- ?????????????????????????????? overflow ????? frame


## 2026-05-16 ????molecule box correction ?????? `quarter -> best` ???

?? `current-thiocyanation.geometry.json` ?? component-union ???????????????????? molecule bbox ???????????????????

- `frame-geom-mol-a = (1436, 2995, 14420, 8650)`

???? `frame-quarter` ????????
- ?? inward ??
- ?? outward ??
- ??????????

Word same-shell ???
- `frame-quarter-shellchem`: `IoU = 0.770339`
- `frame-geom-mol-a`: `IoU = 0.816512`
- `frame-best-shellchem`: `IoU = 0.841267`

?????
- ?? `molecule box correction`?????? `quarter -> best` ??????
- ??? `0.8165 -> 0.8413` ???????? centered/text/left-side residual

### `quarter -> geom-mol-a` ??????

??????
1. `arrow`: `+0.140503`
2. `bottom_right_catalyst`: `+0.086638`
3. `conditions_block`: `+0.075660`
4. `top_right_product`: `+0.049574`
5. `bottom_center_reagent`: `+0.038893`

???????
- `ch3cn`: `+0.004229`

?????
- `bottom_left_ligand`: `-0.006111`
- `title_block`: `-0.021815`

??? molecule-box ??????????????????????? centered residual ?????????
- ??
- ?????
- ????
- ???
- ????

### `geom-mol-a -> best` ??????

? `geom-mol-a` ??? `frame-best`???????????
1. `bottom_left_ligand`: `+0.056028`
2. `top_left_substrate`: `+0.042167`
3. `bottom_center_reagent`: `+0.039304`
4. `title_block`: `+0.035413`
5. `conditions_block`: `+0.029078`
6. `ch3cn`: `+0.025912`

?????? `geom-mol-a` ???????????

### ???????????

?? `quarter -> best` ???????????

1. `quarter -> geom-mol-a`
   - ?? **molecule box correction**
   - ?????
     - `arrow`
     - `top_right_product`
     - `bottom_right_catalyst`
     - `conditions_block`
     - `bottom_center_reagent`

2. `geom-mol-a -> best`
   - ?? **left-side / centered residual**
   - ?????
     - `bottom_left_ligand`
     - `top_left_substrate`
     - `title_block`
     - `ch3cn`

??????????
- centered baseline
- molecule box correction
- right-edge text driver

???????????
- centered baseline (`frame-quarter`)
- molecule-driven correction????????
- left-side / centered follow-up residual?????????

??????frame-best ????? residual?????????????? `ligand + substrate + title/ch3cn` ???????


## 2026-05-16 ????`geom-mol-a` ??????????? full-doc ??

??????????????
- `scripts/search-word-frame-regions.py`

???
- ??? docx shell ???
- patch `word/media/image1.emf` ? `EMR_HEADER.frame`
- ? Word `CopyAsPicture`
- ???? IoU ??? region IoU ???

?????????????
- ??? `--region` ????? `global_best.iou` ??

### 1. ?? `geom-mol-a -> best` ??? residual ?? left-family / right-family

? `frame-geom-mol-a = (1436, 2995, 14420, 8650)` ????

- left-family ?????
  - `bottom_left_ligand`
  - `top_left_substrate`
  - `title_block`
  - `ch3cn`

? `top/bottom = +1/+1` ?????????
- `left +6, top +1, right +10, bottom +1`
- frame = `(1442, 2996, 14430, 8651)`

??? 4 ???? score ??? `frame-best`???????
- left-family best global IoU = `0.833610`
- old `frame-best-shellchem` global IoU = `0.841267`

???? `top/bottom` ?????left-family ??????
- `left +6, top +1, right +10, bottom +2`
- frame = `(1442, 2996, 14430, 8652)`

### 2. right-family ?????? `frame-best` ???

right-family ?????
- `conditions_block`
- `arrow`
- `top_right_product`
- `bottom_center_reagent`
- `bottom_right_catalyst`

?? `top/bottom = +1/+1` ???????????????????
- `left +7, top +1, right +10, bottom +1`
- frame = `(1443, 2996, 14430, 8651)`

???? `frame-best` ??? right-family ?????????? left-family ?????

### 3. ??? full-doc ?? IoU ????????

?? `frame-geom-mol-a` ????????????????

- `frame-global2 = (1442, 2995, 14430, 8653)`
  - global IoU = `0.848013`

??????????????????
- `frame-global3 = (1441, 2994, 14431, 8656)`
  - global IoU = `0.861882`

??? `frame-best-shellchem`?
- old best = `0.841267`
- new `frame-global3` = `0.861882`
- ????? `+0.020615`

### 4. `frame-global3` ?????

?? `frame-geom-mol-a`?
- `bottom_left_ligand`: `+0.105288`
- `bottom_center_reagent`: `+0.068582`
- `title_block`: `+0.056373`
- `top_left_substrate`: `+0.040334`
- `bottom_right_catalyst`: `+0.040860`
- `conditions_block`: `+0.033493`
- `arrow`: `+0.026597`
- `ch3cn`: `+0.025912`
- `top_right_product`: `-0.019067`

??? `frame-best-shellchem`?
- `bottom_left_ligand`: `+0.049260`
- `bottom_center_reagent`: `+0.029278`
- `title_block`: `+0.020960`
- `bottom_right_catalyst`: `+0.039529`
- `conditions_block`: `+0.004415`
- `arrow`: `+0.001807`
- `ch3cn`: `+0.000000`
- `top_left_substrate`: `-0.001833`
- `top_right_product`: `-0.013359`

?????
- `frame-global3` ??????? left-family????? right-family?
- ????????????????????????
  - `ligand`
  - `title`
  - `bottom_center_reagent`
  - `bottom_right_catalyst`

????????
- `top_right_product`
- ???? `top_left_substrate`

### 5. ??????????

???????????????
- `quarter -> molecule correction -> old best`

???
1. `frame-quarter`
2. `molecule-driven correction` (`geom-mol-a`)
3. `right-family` ??? `frame-best`
4. ?????? IoU ?????????? `frame-global3`
   - ?????? right-family ?????
   - ??????????? left/title/reagent residual

??? `frame-best-shellchem` ????????? full-doc ????????????????


## 2026-05-17 DocumentKnockout 可见性与“标签被圈出来”现象

这轮先不继续调 `frame`，而是把用户指出的另一类现象单独拆开：

- 我们的标签在对比图里像被一圈 halo/clip 壳包住
- 这看起来不像单纯的 `frame` 或全局位移问题
- 需要先判断：是不是内部 label knockout 几何被错误地显示出来了

### 1. 代码语义确认：SVG 隐藏 node-specific knockout，但 Office preview 目前会显示

代码层面对比：

- `crates/chemcore-engine/src/render_svg.rs`
  - `visible_in_document_svg()` 会隐藏
    - `RenderPrimitive::Rect { role: DocumentKnockout, .. }`
    - `RenderPrimitive::Polygon { role: DocumentKnockout, node_id: Some(_), .. }`
- `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - `office_preview_primitive_visible()` 当前会把 `RenderRole::DocumentKnockout` 当成可见 primitive

这说明：

- 文档 SVG 语义：`DocumentKnockout` 是内部退让/遮挡几何，不应该被看见
- Office preview 语义：当前会把这层内部几何 replay 到可见结果里

这本身就是一个明确的语义不一致。

### 2. full doc 里所有 knockout 都是 node-specific label knockout

新增分析例子：

- `crates/chemcore-engine/examples/render_role_report.rs`

运行：

- `cargo run -q -p chemcore-engine --example render_role_report -- tmp/thiocyanation-source.payload.json > tmp/thiocyanation-source.render-roles.json`

结果：

- `primitiveCount = 217`
- `roleCounts`
  - `DocumentBond = 138`
  - `DocumentGraphic = 2`
  - `DocumentKnockout = 39`
  - `DocumentText = 38`
- `knockout.nodeSpecificCount = 39`
- `knockout.plainCount = 0`

所以当前 full doc 里：

- 所有 knockout 都是节点级的 label knockout
- 没有“整对象背景框”那种 plain knockout

同时：

- `visibleBoundsWithKnockout == visibleBoundsNoKnockout`

也就是说，在这份 full doc 里：

- knockout 会影响像素可见结果
- 但不会影响 `visible bounds / source bounds / frame`

### 3. direct generated docx：隐藏 knockout 后，Word 结果变化很大

我加了一个分析用环境变量：

- `CHEMCORE_EMF_HIDE_DOCUMENT_KNOCKOUT`

用 built `chemcore-office.exe` 生成两份 docx：

- 默认：
  - `tmp/thiocyanation-source.current-docx.docx`
- 隐藏 knockout：
  - `tmp/thiocyanation-source.no-knockout.docx`

再用 Word `CopyAsPicture` 导出 PNG：

- `tmp/thiocyanation-source.current-docx.png`
- `tmp/thiocyanation-source.no-knockout.png`

二者直接像素对比：

- `intersection = 8984`
- `only_current = 4824`
- `only_no_knockout = 4838`
- `IoU = 0.481819`

说明：

- 在 direct generated docx 这条路径里，隐藏 knockout 会显著改变 Word 可见结果
- 所以“标签像被圈出来”不是错觉，`DocumentKnockout` 这层确实在干扰可见 replay

### 4. 但在 same-shell patched replay 里，它不是当前主矛盾

我把两份 raw EMF 都 patch 进同一个 shell：

- `tmp/frame-word-ab/frame-global3-shellchem.docx`

得到：

- `frame-global3-currentknockout.docx`
- `frame-global3-noknockout.docx`

Word `CopyAsPicture` 后：

- `tmp/frame-word-ab/frame-global3-currentknockout.wordcopy.png`
- `tmp/frame-word-ab/frame-global3-noknockout.wordcopy.png`

这两张图之间：

- 只有 `9` 个像素不同

而且分别与 same-shell ChemDraw 参考图对齐后：

- `currentknockout` 与参考的 best-shift IoU：`0.345603`
- `noknockout` 与参考的 best-shift IoU：`0.345603`

数值完全一样。

所以这轮可以得出一个比较强的结论：

- `DocumentKnockout` 被错误显示出来，确实是一个真实语义问题
- 但在当前 `same-shell patched replay` 主线里，它并不是剩余 ChemDraw 差异的主驱动项

换句话说：

- 它解释了“为什么看起来像每个标签被圈出来”
- 但还不能解释我们现在 same-shell 主线的剩余像素差

### 5. 当前判断

现在可以把问题更清楚地拆成两层：

1. `DocumentKnockout` 可见性泄漏
   - 是明确 bug
   - direct generated docx 上影响很大

2. same-shell ChemDraw 对齐残差
   - 当前主因仍更像 `EMR_HEADER.frame / Word replay` 语义
   - knockout 在这条主线上不是 dominant factor

下一步更合适的研究方向是：

- 继续做“文本本体 vs knockout”隔离
- 但 same-shell 主线不应因为这条发现而完全转向 `glyph clipping`
- 更像是把它作为一条并行 bug 线索保留


## 2026-05-17 文本本体 vs knockout 隔离：`knockout-only` 基本是 `text-only` 的胖超集

在上面的可见性结论基础上，这轮继续做 role-isolation，不再直接拿 full figure 猜。

### 1. 分析开关扩展

在 `office_preview_primitive_visible()` 上继续加了三个分析开关：

- `CHEMCORE_EMF_HIDE_DOCUMENT_TEXT`
- `CHEMCORE_EMF_HIDE_DOCUMENT_BOND`
- `CHEMCORE_EMF_HIDE_DOCUMENT_GRAPHIC`

这样可以导出 3 类隔离视图：

- `text+knockout`
  - 隐藏 `DocumentBond / DocumentGraphic`
- `text-only`
  - 隐藏 `DocumentBond / DocumentGraphic / DocumentKnockout`
- `knockout-only`
  - 隐藏 `DocumentBond / DocumentGraphic / DocumentText`

对应产物：

- `tmp/thiocyanation-source.text-plus-knockout.docx`
- `tmp/thiocyanation-source.text-only.docx`
- `tmp/thiocyanation-source.knockout-only.docx`

以及 Word `CopyAsPicture`：

- `tmp/thiocyanation-source.text-plus-knockout.png`
- `tmp/thiocyanation-source.text-only.png`
- `tmp/thiocyanation-source.knockout-only.png`
- 总拼图：
  - `tmp/thiocyanation-source.role-isolation-montage.png`

### 2. 关键视觉结论

直接看拼图时，最关键的观察是：

- `knockout-only` 不是空白
- 它也不是一些与文字无关的矩形块
- 它看起来像一套 **更胖、更壳状的文字**

也就是说，当前 node-specific `DocumentKnockout` 的可见形态，本质上就是：

- 以 label 文字为中心
- 外扩/肥化过一层的字形壳

这和用户看到的“每个标签像被圈出来”是完全一致的。

### 3. 定量结论：`knockout-only` 几乎是 `text-only` 的严格超集

我又直接对 `text-only` 和 `knockout-only` 做了一张红蓝叠加：

- `tmp/thiocyanation-source.text-vs-knockout-overlay.png`
- 指标：
  - `tmp/thiocyanation-source.text-vs-knockout.json`

结果：

- `intersection = 7274`
- `only_text = 0`
- `only_knockout = 644`
- `IoU = 0.918666`

这个数值非常关键，因为它说明：

- `text-only` 的所有黑字像素，全部都落在 `knockout-only` 里面
- 没有任何 “text-only 独有、而 knockout-only 没覆盖到” 的像素
- `knockout-only` 只是比 `text-only` 多出了一圈额外外扩

也就是：

- `text-only ⊂ knockout-only`

从几何上讲，这已经非常接近“文字本体 + 一圈 halo”。

### 4. 这条证据和 same-shell 主线如何并存

这轮结论和上一节并不冲突，反而把问题拆得更清楚了：

1. `DocumentKnockout` 可见性泄漏
   - 现在已经有强证据证明：
   - 它看起来就是文字壳/外扩字形
   - 是用户肉眼看到“标签被圈出来”的直接来源

2. same-shell ChemDraw 对齐主线
   - 当前剩余 same-shell 残差的 dominant factor 仍更像 `EMR_HEADER.frame / Word replay`
   - 但这不意味着 `DocumentKnockout` 无关
   - 它更像是一个独立的、局部视觉错误

所以当前更合理的总判断是：

- `frame` 主导全局对齐差
- `DocumentKnockout` 可见性主导“标签外面有壳”的局部错误

下一步如果进入修复，不应该把两者混成一个问题。应该：

- `frame` 继续沿 same-shell Word replay 主线研究
- `DocumentKnockout` 作为单独 bug 线，目标是让 Office preview 回到 SVG 语义：
  - 内部退让几何参与计算
  - 但不在最终可见结果中留下字壳

### 6. 把 `current / no-knockout` 都补到 `frame-global3` 后，same-shell 主指标仍完全一致

为了彻底排除“只是 frame 没补齐”的干扰，又做了一步：

1. 先把
   - `tmp/thiocyanation-source.current.emf`
   - `tmp/thiocyanation-source.no-knockout.emf`
   patch 进同一个 `frame-global3` shell
2. 再把两者的 `EMR_HEADER.frame` 都 patch 成：
   - `(1441, 2994, 14431, 8656)`

得到：

- `tmp/frame-word-ab/frame-global3-currentknockout-fg3.docx`
- `tmp/frame-word-ab/frame-global3-noknockout-fg3.docx`

再用 Word `CopyAsPicture` 和真正的 ChemDraw 参考图
- `tmp/v28-wrapper-ablate10/v28-rerun.shape2.png`
做 best-shift 比较，结果三者完全一样：

- `global3_candidate`: `IoU = 0.861882`
- `current_fg3`: `IoU = 0.861882`
- `noknockout_fg3`: `IoU = 0.861882`

所以在 current 主线上可以更强地下结论：

- `DocumentKnockout` 可见性泄漏虽然是真的
- 但就算把 frame 对齐到 `frame-global3`
- same-shell 与 ChemDraw 的主指标仍完全不受它影响

也就是说，它现在更像：

- 一个明确的局部显示错误
- 但不是 `frame-global3 -> ChemDraw` 这条主线的瓶颈

### 7. `frame-global3` 残差热点：主要仍集中在标题/标签文字，但已不再是“纯文字问题”

新增工具：

- `scripts/png-hotspot-components.py`

它会对两张渲染后的 PNG 做：

- best-shift 对齐后 XOR
- 连通分量提取
- 输出热点 bbox 和叠加图

当前 `frame-global3` 对 `v28-rerun.shape2.png` 的结果：

- `tmp/frame-word-ab/frame-global3-hotspots/overlay-hotspots-topn.png`
- `tmp/frame-word-ab/frame-global3-hotspots/hotspots-topn.json`

主热点确实大多挂在：

- 上部标题/条件文字
- 右上产物标签
- 右下催化剂标签
- 左下配体标签

但我又把这些残差像素投到全文本 bbox 邻域里做了一次量化：

- 使用 `render_role_report` 导出的 `DocumentText` bbox
- 每个 bbox 外扩 `3 px`

结果：

- `residual_inside_text_boxes = 961`
- `residual_outside_text_boxes = 767`
- `inside_ratio = 0.5561`

也就是：

- 约 `55.6%` 的残差确实落在文本附近
- 但还有 `44.4%` 的残差并不在文本邻域内

这说明：

- “普通文本/标签文本对齐”仍然是主矛盾之一
- 但当前 same-shell 剩余差异已经不能再被简化成纯文字问题
- `frame-global3` 之后，还同时有一层非文本/非标签的 replay 残差

### 8. `DocumentKnockout` 在 Office preview 里确实泄漏成可见 halo，但它不是当前 `frame-global3` 主线瓶颈

为了把“标签外面像被圈出来”这件事和 same-shell 主线分开，我补了一个 role 报告工具：

- `crates/chemcore-engine/examples/render_role_report.rs`

对 `tmp/thiocyanation-source.payload.json` 运行后得到：

- `primitiveCount = 217`
- `DocumentBond = 138`
- `DocumentGraphic = 2`
- `DocumentKnockout = 39`
- `DocumentText = 38`
- `DocumentKnockout` 全部都是 `nodeSpecific`，`plainCount = 0`
- `visibleBoundsWithKnockout == visibleBoundsNoKnockout`

这说明：

- knockout 并不改变当前 full doc 的可见/source bounds
- 但它在像素层确实参与了绘制

随后在 Office preview 里加了 4 个分析开关：

- `CHEMCORE_EMF_HIDE_DOCUMENT_KNOCKOUT`
- `CHEMCORE_EMF_HIDE_DOCUMENT_TEXT`
- `CHEMCORE_EMF_HIDE_DOCUMENT_BOND`
- `CHEMCORE_EMF_HIDE_DOCUMENT_GRAPHIC`

并导出了 4 组 Word 复制图：

- `text-only`
- `knockout-only`
- `text+knockout`
- `current`

关键图：

- `tmp/thiocyanation-source.role-isolation-montage.png`
- `tmp/thiocyanation-source.text-vs-knockout-overlay.png`
- `tmp/thiocyanation-source.text-vs-knockout.json`

量化结果：

- `intersection = 7274`
- `only_text = 0`
- `only_knockout = 644`
- `IoU = 0.918666`

这说明对 full doc 而言：

- `knockout-only` 基本就是 `text-only` 的“胖超集”
- 也就是说，用户肉眼看到的“每个标签像被圈出来”，并不是错觉，而是内部 knockout 几何真的泄漏成了可见外壳

但把 `current.emf` 和 `no-knockout.emf` 都 patch 进同一个 `frame-global3` shell 之后，再和 ChemDraw 做 same-shell 对比：

- `global3_candidate = 0.861882`
- `current_fg3 = 0.861882`
- `noknockout_fg3 = 0.861882`

三者完全一致。

所以现在可以更准确地把它定性成：

- `DocumentKnockout` 可见性泄漏是一个**独立的显示 bug**
- 它解释“标签外面有一圈壳”
- 但它**不是**当前 `frame-global3 -> ChemDraw` same-shell 主线残差的 dominant factor

### 9. `frame-global3` 剩余误差里，分子内部标签/组件比外部 centered text 还重

为了继续拆 residual，我补了两个脚本：

- `scripts/attribute-word-residual.py`
- `scripts/attribute-word-residual-labels.py`

前者把 `frame-global3` 与 ChemDraw 的残差像素投到：

- top-level object
- merged molecule component

后者再把分子内部标签单独投影出来，直接数每个 label box 里的 residual 像素。

#### 9.1 几何/组件归因

输出：

- `tmp/frame-word-ab/frame-global3-geometry-attribution.json`

关键结果：

- `component` 总 residual = `1202`
- `top:text` 总 residual = `691`
- `top:line` 总 residual = `30`

也就是：

- 分子组件 residual 比 top-level centered text residual 还大
- arrow 之类的 line 残差只占很小一层

组件排名前几位：

- `bottom_right_catalyst_component = 538`
- `top_right_product_component = 240`
- `bottom_center_reagent_component = 191`
- `bottom_left_ligand_component = 170`
- `top_left_substrate_component = 63`

这说明当前 same-shell 主线里，最重的非全局残差其实已经偏到：

- 右下催化剂
- 右上产物
- 中下试剂

#### 9.2 分子内部标签归因

输出：

- `tmp/frame-word-ab/frame-global3-label-attribution.json`

最重的标签 residual 基本都落在分子内部标签，而不是普通 centered title：

- 黑色 `Ph`：`69 / 64 / 53 ...`
- 蓝色 `CN`：`66`
- 橙色 `Ph`：`61`
- 黑色 `NC`：`51`
- 橙色 `S`：`44`
- 试剂中的 `S`：`33`

这一步很重要，因为它把当前 same-shell 主线的剩余误差进一步收窄成：

1. 外部 centered text（标题、`CH3CN`、产率）
2. **分子内部标签 replay**（催化剂/产物/试剂上的 `Ph/CN/NC/S`）
3. 少量非文本 replay

也就是说，接下来如果继续死磕：

- 不能再把所有问题统称成“普通文本没对齐”
- 分子内部标签已经是独立且更重的一条主线

### 10. `right-edge-ph` 最小样本上的 knockout 泄漏不像 full doc 那样稳定

我还对 `tmp/word-text-fixtures/right-edge-ph.cdxml` 做了同样的 role isolation：

- `right-edge-ph.current.docx/png`
- `right-edge-ph.text-only.docx/png`
- `right-edge-ph.knockout-only.docx/png`
- `right-edge-ph.role-isolation-montage.png`
- `right-edge-ph.text-vs-knockout.json`

结果和 full doc 很不一样：

- `intersection = 951`
- `only_text = 3264`
- `only_knockout = 2151`
- `IoU = 0.149387`

也就是说，`right-edge-ph` 这个最小 fixture 上：

- `knockout-only` 并不是 `text-only` 的稳定胖超集
- “knockout 泄漏成字壳”这个现象在 full doc 上非常强，但并不是所有 fixture 的统一行为

因此后面修这个 bug 时要注意：

- 它很可能和完整文档上下文 / merged molecule label replay 有关
- 不能简单拿 `right-edge-ph` 一类最小样本就当作 full doc 的直接替身

### 11. 给 Office preview 加了 `object_id` 分析过滤，开始把“外部 centered text”和“分子内部标签 replay”拆开

为了不再只靠整张 full doc 猜，我在 Office preview 里补了一个纯分析开关：

- `CHEMCORE_EMF_INCLUDE_OBJECT_IDS`

语义是：

- 只回放指定 top-level `object_id`
- 其他 primitive 一律不画

这样可以直接做：

- `obj_cdxml_merged_molecule`
- `obj_text_004/005/006`
- `obj_line_001`

这类“同一 payload、同一 Word 壳、不同对象子集”的对照。

当前这只是分析工具，不是产品行为。

### 12. `frame-global3` 剩余误差里，分子组件 residual 已经大于外部 centered text

新增脚本：

- `scripts/attribute-word-residual.py`

它会把 `frame-global3` 对 ChemDraw 的 residual 像素投到：

- top-level object
- merged molecule component

输出：

- `tmp/frame-word-ab/frame-global3-geometry-attribution.json`

结果很关键：

- `component` 总 residual = `1202`
- `top:text` 总 residual = `691`
- `top:line` 总 residual = `30`

这说明现在的主问题已经不能再概括成“普通 centered text 没对齐”。  
在 `frame-global3` 之后：

- 分子组件 replay 残差比外部 centered text 残差还更重
- arrow / line 只占一小层

组件内残差排名前几位：

- `bottom_right_catalyst_component = 538`
- `top_right_product_component = 240`
- `bottom_center_reagent_component = 191`
- `bottom_left_ligand_component = 170`
- `top_left_substrate_component = 63`

这一步把 same-shell 主线重新拆成了：

1. 外部 centered text
2. **分子组件/分子内部标签 replay**
3. 少量 line/arrow replay

### 13. `frame-global3` 的标签级归因：当前最重的是催化剂/产物/试剂里的 `Ph/CN/NC/S`

新增脚本：

- `scripts/attribute-word-residual-labels.py`

它直接读取 payload 中 merged molecule 的 node label `glyphPolygons / box`，把 residual 投到每个 label box 上。

输出：

- `tmp/frame-word-ab/frame-global3-label-attribution.json`

当前最重的标签残差集中在：

- 黑色 `Ph`
- 蓝色 `CN`
- 黑色 `NC`
- 橙色 `Ph`
- 橙色 `S`

典型数值：

- 黑色 `Ph`: `69 / 64 / 53 ...`
- 蓝色 `CN`: `66`
- 橙色 `Ph`: `61`
- 黑色 `NC`: `51`
- 橙色 `S`: `44`
- 试剂中的 `S`: `33`

这一步很关键，因为它说明：

- 当前 same-shell 主线剩余误差里，**分子内部标签**已经是独立且更重的一条支线
- 而且主要集中在：
  - 右下催化剂
  - 右上产物
  - 中下试剂

也就是说，后面不能再把所有 residual 都叫“文字问题”：

- 外部 centered text 是一条线
- 分子内部 `Ph/CN/NC/S` 标签 replay 是另一条线

### 14. 当前更合理的主线分解

结合上面的 role isolation、geometry attribution、label attribution，现在 `frame-global3` 之后的 mainline 已经可以更准确地表述成：

1. `DocumentKnockout` 可见性泄漏  
   - 真 bug  
   - 解释“标签外面像被圈出来”  
   - 但不是 same-shell `frame-global3 -> ChemDraw` 的 dominant factor

2. 外部 centered text replay  
   - 仍然有 residual  
   - 但已经不是唯一主矛盾

3. **分子内部标签 replay**  
   - 当前权重更大  
   - 尤其是催化剂/产物/试剂中的 `Ph/CN/NC/S`

4. 少量非文本 replay  
   - arrow / 一些非标签分子几何

后面的研究不该再只沿着“普通文本对齐”一条线往下打，而应该至少把：

- 外部 centered text
- 分子内部标签 replay

分成两条并行主线。

### 15. 分子 residual 里大约 `81%` 真的是 label residual，不是骨架 replay

为了把“分子内部标签 replay”和“分子骨架 replay”再拆开，我补了第三个脚本：

- `scripts/attribute-word-residual-molecule.py`

它会在 `frame-global3` same-shell 主线下，把 residual 再分成：

- 落在任意 molecule label box 内
- 落在 molecule component box 内、但不在任何 label box 内

输出：

- `tmp/frame-word-ab/frame-global3-molecule-partition.json`

当前结果：

- `residualPixelCount = 1728`
- `componentUnionResidualCount = 1202`
- `labelUnionResidualCount = 978`
- `componentNonLabelResidualCount = 224`

也就是说：

- 在所有 molecule-component residual 里
- 大约 `978 / 1202 = 81.4%`
- 是**直接落在分子标签 box 内**的

这一步很关键，因为它把“分子组件 residual 更重”进一步推进成了：

- **分子主线的 dominant factor 已经不是骨架本身，而是分子内部标签 replay**

按 component 看会更明显：

- `bottom_right_catalyst_component`
  - `residualCount = 538`
  - `labelResidualCount = 527`
  - `nonLabelResidualCount = 11`
- `top_right_product_component`
  - `240 / 171 / 69`
- `bottom_center_reagent_component`
  - `191 / 162 / 29`
- `bottom_left_ligand_component`
  - `170 / 118 / 52`
- `top_left_substrate_component`
  - `63 / 0 / 63`

这说明当前 molecule 侧 residual 可以进一步分成两类：

1. **标签主导型**
   - `bottom_right_catalyst`
   - `top_right_product`
   - `bottom_center_reagent`
   - `bottom_left_ligand`（虽有一部分非标签 residual，但标签仍占大头）

2. **纯几何型**
   - `top_left_substrate`
   - 这块几乎没有标签，残差都来自分子骨架 replay

因此，`frame-global3` 之后更准确的 residual 模型应该更新成：

- 外部 centered text replay
- 分子内部标签 replay（当前最重）
- 少量分子骨架 replay（尤其 `top_left_substrate`）
- 少量 line/arrow replay

### 16. 新增 `node_id` 过滤：可以把同一 merged molecule 里的单个标签节点单独回放

为了把“分子内部标签 replay”继续拆细，我在 Office preview 的分析开关里又补了一个：

- `CHEMCORE_EMF_INCLUDE_NODE_IDS`

语义是：

- 只回放指定 `node_id` 的 primitive
- 和现有 `CHEMCORE_EMF_INCLUDE_OBJECT_IDS` 可叠加使用

典型用法：

- `CHEMCORE_EMF_INCLUDE_OBJECT_IDS=obj_cdxml_merged_molecule`
- `CHEMCORE_EMF_INCLUDE_NODE_IDS=f4_32333`

这样可以在不改 payload 的前提下，把 merged molecule 里的某一个标签单独抽出来生成 Word docx / `CopyAsPicture`，适合做：

- 单标签 vs ChemDraw 的局部 crop 对照
- `text-only / knockout + text` 的对照
- 同一家族标签（`Ph/CN/NC/S`）之间的 replay 形状归一化

### 17. 单标签 `f4_32333`（黑色 `Ph`）局部对照：knockout 不是唯一来源，标签回放本身就不对

我用新加的 `node_id` 过滤，单独抽了催化剂区最高残差的黑色 `Ph`：

- `node_id = f4_32333`

生成了：

- `tmp/frame-word-ab/label-f4_32333.docx`
- `tmp/frame-word-ab/label-f4_32333.wordcopy.png`
- `tmp/frame-word-ab/label-f4_32333-text-only.docx`
- `tmp/frame-word-ab/label-f4_32333-text-only.wordcopy.png`
- `tmp/frame-word-ab/label-f4_32333.preview-bounds.json`

以及对应的局部对照：

- `tmp/frame-word-ab/label-f4_32333-compare/overlay-ours-vs-chemdraw.png`
- `tmp/frame-word-ab/label-f4_32333-compare/overlay-text-only-vs-chemdraw.png`
- `tmp/frame-word-ab/label-f4_32333-compare/metrics.json`

关键数值：

- `ours_vs_chemdraw.iou = 0.20879`
- `text_only_vs_chemdraw.iou = 0.17021`
- `ours_vs_text_only.iou = 0.37815`

这一步说明两件事：

1. 这条高残差黑色 `Ph` 的局部错位/形状偏差是真的存在，不是 full-doc 聚合假象。
2. 把 `DocumentKnockout` 去掉后局部结果并没有自动变对；相反，`with-knockout` 版本还略好一点。

因此对这类标签来说：

- knockout 泄漏是一个真实 bug
- 但当前 same-shell 主线里，**标签文本/标签 replay 本身也在错**

### 18. patch `image1.emf` 生成的 same-shell layer-doc 不能当 role partition oracle

我尝试过把不同 role-layer 的 EMF（例如：

- `molecule-text-only.emf`
- `molecule-knockout-only.emf`
- `molecule-nontext-only.emf`

）分别 patch 到同一个 `frame-global3` 壳里，再让 Word `CopyAsPicture`。

结果很反直觉：

- 三份输出的 `wordcopy.png` 哈希完全一致
- bbox 也完全一致

同时我又做了一个 `blank-preview.emf` 的对照：

- 把 `text / knockout / bond / graphic` 全部隐藏
- patch 进同一个壳里后，Word 输出确实会变

这说明：

- `image1.emf` 本身并不是完全不起作用
- 但“只靠 patch `image1.emf` 再让 Word 回放”这条路，不足以稳定地做 role-layer same-shell partition

因此，后面的 role 分层归因不能把这类 patched layer-doc 当成权威 oracle。

### 19. 当前 dominant label family：`attached-group + start + 短标签`

为了继续收紧 label 主线，我把 `label attribution` 结果又做了一层聚合，新增：

- `scripts/summarize-label-attribution.py`
- 输出：`tmp/frame-word-ab/frame-global3-label-summary.json`

聚合后最强的规律是：

- `layout = attached-group`
- `anchor = start`
- 高残差短标签：
  - `Ph`
  - `CN`
  - `NC`
  - `S`

关键汇总：

- `attached-group`: `count = 25`, `residual = 950`, `avgResidual = 38.0`
- `attached-group-above`: `count = 2`, `residual = 58`, `avgResidual = 29.0`
- `Ph`: `count = 9`, `residual = 426`, `avgResidual = 47.33`
- `CN`: `count = 2`, `residual = 104`, `avgResidual = 52.0`
- `NC`: `count = 1`, `residual = 51`
- `S`: `count = 2`, `residual = 77`, `avgResidual = 38.5`

按填充色再拆：

- `Ph|#000000`: `8 / 365 / 45.625`
- `CN|#0000ff`: `1 / 66`
- `Ph|#ff8000`: `1 / 61`
- `NC|#000000`: `1 / 51`
- `S|#ff8000`: `1 / 44`
- `CN|#000000`: `1 / 38`
- `S|#000000`: `1 / 33`

所以目前最值得继续做节点级局部对照的，不是任意标签，而是这组 dominant family：

- 黑色 `Ph`
- 蓝色 `CN`
- 黑色 `NC`
- 橙色 `Ph`
- 橙色 `S`

这也意味着后面的 same-shell 主线，已经可以从“分子内部标签 replay”进一步细化成：

- `attached-group/start/short-label` replay family
- 少量非该家族的标签
- 极少量纯骨架几何 residual

### 20. 节点级单标签对照：dominant label family 不是单一“胖/瘦”规律，而是至少分成几种局部 replay 形态

为了继续验证上一节的 dominant family，我补了一个小脚本：

- `scripts/compare-single-label-wordcopy.py`

它做的事是：

1. 从 `frame-global3-label-attribution.json` 里取某个 `nodeId` 的 `pixelBox`
2. 在 full-doc ChemDraw 参考图里，用该 `pixelBox` 量真实 label ink bbox
3. 再把同一个 `nodeId` 用 `CHEMCORE_EMF_INCLUDE_NODE_IDS` 单独导成 Word docx
4. 量它在 `CopyAsPicture` 里的单标签 ink bbox
5. 比较：
   - `with knockout`
   - `text-only`

这里一个关键修正是：  
**对单标签对照，不能再用带大 padding 的局部 crop；应该直接以 full-doc 里该 label 自己的 `pixelBox` 为参考口径。**  
否则会把周边骨架或别的标签一起卷进来，误判成单标签 replay 差异。

本轮重点看了这些节点：

- `f4_32333` 黑色 `Ph`
- `f4_32321` 黑色 `NC`
- `f5_2784` 蓝色 `CN`
- `f5_2794` 橙色 `Ph`
- `f5_2788` 橙色 `S`
- 控制样本：
  - `f4_32347` 另一处黑色 `Ph`
  - `f4_32323` 黑色 `CN`

用 `pixelBox` 口径量出的结论很有价值：

#### A. 黑色催化剂标签：`Ph/NC` 往往靠 knockout 才接近 ChemDraw，但也不是完全一致

`f4_32333` 黑色 `Ph`

- ChemDraw：`17 x 10`
- with knockout：`17 x 11`
- text-only：`14 x 10`

`f4_32321` 黑色 `NC`

- ChemDraw：`20 x 10`
- with knockout：`19 x 11`
- text-only：`17 x 10`

这两条说明：

- 对这组催化剂黑色短标签来说，knockout 不是纯副作用
- 它反而在宽度上帮我们补回了一部分 text-only 的“过瘦”问题

但它们也不是完全对齐：

- `with knockout` 版本普遍仍会多出 `+1 px` 的高度
- 说明同一标签里同时存在：
  - 文本本体偏瘦
  - knockout 外壳偏胖

#### B. 蓝色/橙色产物标签是另一类

`f5_2784` 蓝色 `CN`

- ChemDraw：`16 x 10`
- with knockout：`19 x 10`
- text-only：`17 x 9`

它和上面的黑色 `Ph/NC` 不一样：

- text-only 已经接近参考
- knockout 反而把宽度推得更宽

`f5_2794` 橙色 `Ph`

- ChemDraw：`20 x 12`
- with knockout：`17 x 10`
- text-only：`14 x 10`

`f5_2788` 橙色 `S`

- ChemDraw：`10 x 15`
- with knockout：`9 x 10`
- text-only：`6 x 10`

这两条说明：

- 橙色产物标签的问题不是单纯横向胖/瘦
- 而是**高度明显不足**
- knockout 只能略微补宽，补不了缺掉的竖向 ink

#### C. 同是黑色 `Ph` 也不全是一种错法

控制样本 `f4_32347` 黑色 `Ph`：

- ChemDraw：`23 x 16`
- with knockout：`17 x 10`
- text-only：`14 x 9`

它和 `f4_32333` 完全不是同一个量级。  
这说明即使限制到：

- `layout = attached-group`
- `anchor = start`
- `fill = #000000`
- `text = Ph`

也还不能把它们视为同一个完全统一的 replay 子类。

#### D. 当前最准确的节点级结论

dominant family `attached-group/start/short-label` 仍然成立，但它内部至少已经能看出三种局部 replay 形态：

1. **黑色催化剂短标签**
   - 典型如 `Ph / NC`
   - text-only 偏瘦
   - knockout 会把宽度补回来一些，但同时略带额外厚度

2. **蓝色 `CN`**
   - text-only 已经接近
   - knockout 会把宽度推过头

3. **橙色 `Ph / S`**
   - 即使带 knockout 也明显偏矮
   - 主问题更像竖向 ink / replay 缺失，而不是单纯外壳宽窄

所以现在不能再把 molecule 内部标签问题粗略概括成：

- “所有标签都被 knockout 壳圈胖了”

更准确的表述应该是：

- `DocumentKnockout` 泄漏是独立 bug
- 但 dominant label family 内部，至少还混着多种 label-text replay 子问题
- 后面如果继续死磕 molecule 主线，应该按：
  - 黑色催化剂标签
  - 蓝色 `CN`
  - 橙色 `Ph/S`
  
  三组分别研究，而不是再假设一条统一修正规则

### 21. 更稳的口径：直接在 same-shell full-doc 对齐图里逐个 label box 量 ours/ref 的局部 ink bbox

单标签 doc 的价值主要在“证明局部现象存在”。  
但它也有一个天然问题：

- shell/canvas 和 full doc 不同
- 即使只剩一个 label，Word 仍可能给它不同的对象回放口径

因此本轮又补了一个更稳的分析脚本：

- `scripts/compare-full-label-boxes.py`

它直接在：

- `tmp/frame-word-ab/frame-global3-shellchem.wordcopy.png`
- `tmp/v28-wrapper-ablate10/v28-rerun.shape2.png`

这对已经 same-shell 对齐的 full-doc 图上，逐个 label box 去量：

- `oursLocalBbox`
- `refLocalBbox`
- `deltaDims`
- `deltaTopLeft`

这一步的好处是：

- 附着键、上下文、对象壳都在同一口径里
- 不再受单标签 doc 自己的 shell 差异影响

#### 21.1 这组 full-doc 局部 box 结果，比单标签 doc 更一致

用这个口径后，dominant family 的模式一下子规整了很多。

例如：

`f4_32333` 黑色 `Ph`

- ours：`18 x 11`
- ref：`17 x 10`
- `delta = (+1, +1)`
- `deltaTopLeft = (-1, 0)`

`f4_32321` 黑色 `NC`

- ours：`21 x 10`
- ref：`20 x 10`
- `delta = (+1, 0)`
- `deltaTopLeft = (-1, 0)`

`f5_2784` 蓝色 `CN`

- ours：`19 x 10`
- ref：`16 x 10`
- `delta = (+3, 0)`
- `deltaTopLeft = (-1, +1)`

`f5_2794` 橙色 `Ph`

- ours：`22 x 11`
- ref：`20 x 12`
- `delta = (+2, -1)`
- `deltaTopLeft = (0, +1)`

`f5_2788` 橙色 `S`

- ours：`11 x 16`
- ref：`10 x 15`
- `delta = (+1, +1)`
- `deltaTopLeft = (-1, 0)`

也就是说，在真正 same-shell full-doc 口径下：

- 大多数 dominant label 并不是“过瘦”
- 而更像：
  - 稍微偏宽 `(+1~+3 px)`
  - 轻微偏高或持平
  - 左边界常常早 `1 px` 吃进去

#### 21.2 black `Ph` 家族现在终于能看出稳定平均值

把 top residual 的 8 个黑色 `Ph` 放在一起后：

- `avgDw = +1.75`
- `avgDh = +0.25`
- `avgDx = -1.125`
- `avgDy = 0.0`

这说明对黑色 `Ph` 这组来说，same-shell full-doc 主线里的 replay 偏差，已经可以比较稳定地概括成：

- 宽度大约多 `1~2 px`
- 高度基本持平或略高
- 左边略早 `1 px`

而不是之前单标签 doc 给人的那种“有时偏瘦很多”的印象。

#### 21.3 颜色/文本子类仍然不同，但现在差异方向更清楚

按 `text|fill` 聚合：

- `Ph|#000000`
  - `avgDw = +1.75`
  - `avgDh = +0.25`
  - `avgDx = -1.125`
- `CN|#0000ff`
  - `avgDw = +3`
  - `avgDh = 0`
  - `avgDx = -1`
  - `avgDy = +1`
- `Ph|#ff8000`
  - `avgDw = +2`
  - `avgDh = -1`
  - `avgDy = +1`
- `NC|#000000`
  - `avgDw = +1`
  - `avgDh = 0`
  - `avgDx = -1`
- `S|#ff8000`
  - `avgDw = +1`
  - `avgDh = +1`
  - `avgDx = -1`

所以现在的 dominant family 比之前更像：

1. 黑色催化剂短标签  
   - 轻微偏宽
   - 左边略早 `1 px`

2. 蓝色 `CN`  
   - 偏宽最明显
   - 还带一点向下偏

3. 橙色 `Ph`  
   - 稍宽
   - 但高度反而略低
   - 整体更像向下沉 `1 px`

4. 橙色 `S`  
   - 稍宽且稍高

#### 21.4 `no-knockout` same-shell full-doc 对这组局部 box 几乎没有帮助

我还用：

- `frame-global3-noknockout-fg3.wordcopy.png`

跑了同一个 `compare-full-label-boxes.py`。  
结果和 `current frame-global3` 几乎一模一样。

这再次说明：

- `DocumentKnockout` 泄漏是独立 bug
- 但 dominant label family 在当前 mainline 里的局部 replay 偏差，并不是去掉 knockout 就能自然消失的

#### 21.5 当前更可靠的判断

到这里，dominant molecule-label 主线已经可以更准确地描述成：

- 不是“标签被 knockout 壳单纯圈胖”
- 也不是“文字本体单纯偏瘦”
- 而是在 same-shell full-doc 回放里，不同颜色/文本子类的局部 ink bbox 本身就已经和 ChemDraw 有系统性偏差：
  - 黑色 `Ph/NC/CN` 更像轻微偏宽、略左
  - 蓝色 `CN` 偏宽最重
  - 橙色 `Ph` 有宽度和垂直方向的混合偏差
  - 橙色 `S` 则更像宽高一起略胀

所以如果继续死磕 molecule label replay，最值得做的已经不是再看“有没有壳”，而是：

- 用 same-shell full-doc label-box 口径
- 继续按 `text|fill` / `layout|anchor` 家族化
- 找这些局部 bbox 偏差到底是来自：
  - label text 本体 replay
  - label-adjacent bond/context replay
  - 还是两者耦合后的局部栅格化差异

### 22. 在真实 full-doc 输出路径里，`current -> no-knockout` 仍然能拆出每类标签的“壳贡献”

虽然 patched same-shell 主线里去不去 knockout 指标几乎一样，但这不等于 knockout 对局部形状没有贡献。  
为了看“真实输出路径里这层壳到底改了多少”，我又做了一组更直接的比较：

- 参考：`tmp/frame-word-ab/current.wordcopy.png`
- 对照：`tmp/frame-word-ab/frame-global3-noknockout-raw.wordcopy.png`

然后还是用同一个脚本：

- `scripts/compare-full-label-boxes.py`

去量每个 label box 内：

- `current` 的局部 ink bbox
- `no-knockout` 的局部 ink bbox

这一组结果不能直接替代 same-shell ChemDraw 主线，但很适合回答一个更窄的问题：

- **在我们自己的真实 Office 输出里，knockout 对不同标签家族到底贡献了多少局部宽高？**

#### 22.1 dominant family 的 `current -> no-knockout` 差异很清楚

以高残差标签举例：

`f4_32333` 黑色 `Ph`

- current：`14 x 11`
- no-knockout：`13 x 10`
- 也就是 knockout 给它补了大约：
  - `+1 px` 宽
  - `+1 px` 高

`f5_2784` 蓝色 `CN`

- current：`16 x 6`
- no-knockout：`15 x 6`
- knockout 主要给它补的是：
  - `+1 px` 宽

`f5_2794` 橙色 `Ph`

- current：`15 x 8`
- no-knockout：`12 x 8`
- knockout 对它的主要贡献是：
  - `+3 px` 宽

`f4_32321` 黑色 `NC`

- current：`17 x 11`
- no-knockout：`16 x 11`
- knockout 贡献：
  - `+1 px` 宽

#### 22.2 按 `text|fill` 聚合后，当前最可靠的壳贡献是：

- `Ph|#000000`
  - `avgDw = -0.375`
  - `avgDh = +0.125`
  - `avgDx = +0.375`
  - `avgDy = +1.0`
- `CN|#0000ff`
  - `avgDw = -1`
  - `avgDh = 0`
- `Ph|#ff8000`
  - `avgDw = -3`
  - `avgDh = 0`
- `NC|#000000`
  - `avgDw = -1`
  - `avgDh = 0`

这里的 `delta` 是：

- `no-knockout - current`

所以负的 `avgDw` 就表示：

- knockout 在 `current` 里把这类标签**撑宽了**

由此可以读出：

1. 黑色催化剂标签  
   - knockout 贡献一般不大
   - 更像 `+1 px` 级别的补边

2. 蓝色 `CN`  
   - knockout 也主要是 `+1 px` 宽度

3. 橙色 `Ph`  
   - knockout 的宽度贡献明显更大，约 `+3 px`
   - 这和上一节里它在 ChemDraw 主线下“横向更胖”的观察是一致的

#### 22.3 这一步把 molecule label 主线再拆成了两层

现在可以更准确地把 dominant family 的误差来源写成：

1. **label text replay 本体**
   - 即使没有 knockout，局部 bbox 也已经和参考不同

2. **knockout 壳的额外增量**
   - 通常再给标签加 `+1 px` 左右的宽度
   - 对某些家族（尤其橙色 `Ph`）会更大

所以我们前面那句“knockout 不是唯一来源”现在可以更细化成：

- 对 dominant label family 来说，当前错误不是
  - 纯 text
  - 或纯 knockout

而是：

- **text replay 本体先偏了**
- **knockout 再按标签家族不同程度地把它撑得更偏**

这对后面修复策略很重要，因为它意味着：

- 单纯隐藏 knockout 不是解法
- 单纯调 text advance 也不是解法
- 更像要同时考虑：
  - text replay 家族差异
  - knockout 增量家族差异


## 2026-05-17 `component ? family ? knockout` ???????????? replay ??? knockout ??

??????

- `scripts/summarize-knockout-contribution-by-component.py`
- ???`tmp/frame-word-ab/frame-global3-knockout-component-summary.json`

?????????????????

1. `frame-global3` same-shell ?????? `component ? label family` ? residual ??
2. `current -> no-knockout` ??? Word ?????? family ????? bbox ??

?? knockout ?????????

- `avgKnockoutDw / Dh / Dx / Dy = current - no-knockout`
- ???? `avgKnockoutDw` ?? knockout ??????????
- ?? `avgKnockoutDx` ?? knockout ?????????????????

### 1. `bottom_right_catalyst_component`???????????? replay?????? knockout

- component residual?`538`
- label residual?`527`
- non-label residual?`11`

?? family?

- `Ph|#000000`
  - `sumResidual = 365`
  - `avgKnockoutDw = +0.375`
  - `avgKnockoutDh = -0.125`
  - `avgKnockoutDx = -0.375`
  - `avgKnockoutDy = -1.0`
- `N|#000000`
  - `sumResidual = 103`
  - knockout ???????`avgKnockoutDw = -0.25`, `avgKnockoutDh = 0`
- `NC|#000000`
  - `sumResidual = 51`
  - knockout ?????`+1 px` ??`-1 px` ???`-2 px` ??
- `CN|#000000`
  - `sumResidual = 38`
  - knockout ?????`+3 px` ??`+2 px` ??`-3 px` ???`-2 px` ??

???????????

- ???????? residual ???? `Ph` ?????
- ??? `Ph` ? knockout ???????????????
- ?? `bottom_right_catalyst` ? dominant factor ??? **label text replay ??**??? knockout
- ?? `CN/NC` ? knockout ?????????????? residual ??????

?? `f4_32339` ???? `Ph` ??????
- same-shell residual ???????? `deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`?
- ? `current -> no-knockout` ? `0`
- ??????????? replay ??????? knockout ??

### 2. `top_right_product_component`??? `CN` ??? `Ph` ???? knockout ????? `S` ???? replay

- component residual?`240`
- label residual?`171`
- non-label residual?`69`

family?

- `CN|#0000ff`
  - `sumResidual = 66`
  - `avgKnockoutDw = +1`
  - `avgKnockoutDx = -1`
- `Ph|#ff8000`
  - `sumResidual = 61`
  - `avgKnockoutDw = +3`
  - `avgKnockoutDx = -3`
- `S|#ff8000`
  - `sumResidual = 44`
  - knockout ?? `0 / 0 / 0 / 0`

????

- ?? `CN`?text replay ??????knockout ???? `+1 px` ??
- ?? `Ph`?????????text replay ? + knockout ??????
- ?? `S`????????? **? replay** ??

### 3. `bottom_center_reagent_component`?????????knockout ?? vertical nudge

- component residual?`191`
- label residual?`162`
- non-label residual?`29`

family?

- `O|#000000`
  - `sumResidual = 99`
  - `avgKnockoutDw = +0.5`
  - `avgKnockoutDh = +0.5`
  - `avgKnockoutDy = -1.5`
- `S|#000000`
  - `sumResidual = 33`
  - `avgKnockoutDh = +1`
  - `avgKnockoutDy = -3`
- `N|#000000`
  - `sumResidual = 30`
  - knockout ??? 0

???????

- `O / S` ? residual ??knockout ????????????/??
- `N` ????? replay ????
- ??????????????????? **?? replay + ?? knockout vertical nudge**

### 4. `bottom_left_ligand_component`?label residual ? non-label residual ????? knockout ?????

- component residual?`170`
- label residual?`118`
- non-label residual?`52`

family?

- `N|#000000`
  - `sumResidual = 60`
  - `avgKnockoutDh = -2`
  - ? knockout ????? `N` ? no-knockout ??
- `O|#000000`
  - `sumResidual = 58`
  - `avgKnockoutDw = +1.5`
  - `avgKnockoutDh = +2`
  - `avgKnockoutDx = -1`
  - `avgKnockoutDy = -2`

???????????????? label-family?

- `O` ???knockout ?? + replay ???????
- `N` ?????????????
- ??? `nonLabelResidual = 52` ???
- ????? **label + ????????** ??

### 5. ?????????????

????????????????????????????????

1. `bottom_right_catalyst_component / Ph|#000000`
   - residual ?? (`365`)
   - knockout ??????
   - ?????????? replay ??
2. `top_right_product_component / Ph|#ff8000` ? `CN|#0000ff`
   - residual ??
   - knockout ???????
   - ?????replay ?? + knockout ??????
3. `bottom_center_reagent_component / O|#000000 + S|#000000`
   - ?? vertical residual ??
4. `bottom_left_ligand_component`
   - ????????????????? label

?? molecule ???????????

- `A.` ???????? replay ???????
- `B.` ???/??? replay + knockout ??
- `C.` ???? residual ??
- `D.` ????? residual

???????molecule ????????????????????????


## 2026-05-17 same-shell `no-knockout` ???????????? dominant family ????? replay ??

?????????

- ? `scripts/compare-full-label-boxes.py` ?? box ????????? label ? same-shell ?????
- ? **??? `frame-global3 + same-shell` ??** ??? `no-knockout` ???? ChemDraw ???? label box ??

?????

- `tmp/frame-word-ab/frame-global3-noknockout-label-box-compare.json`
- `tmp/frame-word-ab/frame-global3-label-component-summary-noknockout.json`
- `tmp/frame-word-ab/frame-global3-label-context-summary.json`
- ???`scripts/summarize-label-context-by-family.py`

### 1. ?? `current` ? `same-shell no-knockout` ????????

? `frame-global3` ???????? family??????

#### 1.1 `bottom_right_catalyst_component`

- `Ph|#000000`
  - current: `avgDw = +1.75`, `avgDh = +0.25`, `avgDx = -1.125`, `avgDy = 0`
  - same-shell no-knockout: `avgDw = +1.75`, `avgDh = 0`, `avgDx = -1.125`, `avgDy = 0`
- `NC|#000000`
  - current: `(+1, 0, -1, 0)`
  - no-knockout: `(+1, 0, -1, 0)`
- `CN|#000000`
  - current: `(+2, 0, 0, 0)`
  - no-knockout: `(+2, 0, 0, 0)`

?????

- ????? `Ph` ??? same-shell ????knockout ??????? `+0.25 px` ?????
- ?????????????
- `NC/CN` ??????

????????????????

- `bottom_right_catalyst` ? dominant residual????? **text replay ??**
- knockout ??????????? same-shell ???????????

#### 1.2 `top_right_product_component`

- `CN|#0000ff`
  - current: `(+3, 0, -1, +1)`
  - no-knockout: `(+3, -1, -1, +1)`
- `Ph|#ff8000`
  - current: `(+2, -1, 0, +1)`
  - no-knockout: `(+2, -1, 0, +1)`
- `S|#ff8000`
  - current: `(+1, +1, -1, 0)`
  - no-knockout: `(+1, 0, -1, 0)`

????

- ?? `Ph` ? same-shell ?????? **? replay** ??
- ?? `CN` ???????????? `1 px` ???
- ?? `S` ???? replay???? already ?? `+1 px` ?? `-1 px` ??

#### 1.3 `bottom_center_reagent_component`

- `O|#000000`
  - current ? no-knockout ?????`(+1.25, +0.5, -0.25, -0.25)`
- `S|#000000`
  - current ? no-knockout ?????`(0, +1, 0, 0)`
- `N|#000000`
  - current: `(0, +1, 0, 0)`
  - no-knockout: `(0, 0, 0, 0)`

???????

- `O/S` ???? replay ??
- ?? `N` ???? `+1 px` ??? knockout ??

#### 1.4 `bottom_left_ligand_component`

- `N|#000000`
  - current ? no-knockout ???`(+1.5, +0.5, -0.5, 0)`
- `O|#000000`
  - current ? no-knockout ???`(0, 0, 0, 0)`

????????? same-shell ????

- label residual ????? replay ??
- ??? raw current/no-knockout ??????????????

### 2. ????????????

???????
- raw `current -> no-knockout` ??? family-specific knockout ??
- ??????? shell / frame / raw replay

?? same-shell ??????????

1. **raw ??????? knockout ???????**
2. **?? `frame-global3 + same-shell` ??????? dominant family ? bbox ????????**
3. ????????????????knockout ??????????
   - **replay ?????**
   - knockout ???? family ???????/??

????????? `knockout-only` ????????
- ???????????? bug
- ? same-shell ?? residual ? dominant factor?????

### 3. `component ? context`?????? `Ph` ????????????????? ChemDraw ????

? `scripts/summarize-label-context-by-family.py` ??
- family
- ?? `cdxml labelAlignment / labelJustification`
- `nodeType`
- ??????? `sideX / sideY`

??????`bottom_right_catalyst_component / Ph|#000000` ????????

- `Left / Left / Nickname / side(pos,pos)`
  - `count = 3`
  - `sumResidual = 106`
  - `avgDw = +3.0`, `avgDx = -2.0`
- `Right / Right / Nickname / side(neg,pos)`
  - `count = 2`
  - `sumResidual = 97`
  - `avgDw = +0.5`, `avgDx = -0.5`
- `Right / Right / Nickname / side(neg,neg)`
  - `count = 1`
  - `sumResidual = 69`
  - `avgDw = +1`, `avgDh = +1`, `avgDx = -1`
- `Left / Left / Nickname / side(pos,neg)`
  - `count = 1`
  - `sumResidual = 53`
- `Left / Left / Nickname / side(neg,pos)`
  - `count = 1`
  - `sumResidual = 40`

???????????????

- ????? `Ph` ???????? +1.75 px ????????
- ???? **?? ChemDraw ???? + ???????** ????
- ??? `f4_32339` ?????? `Left/Left` ? `Ph`??? knockout ??? `0`?? same-shell residual ??????????? replay ?????

### 4. ???????????????

???? `frame-global3 + same-shell` ??????????????

1. `bottom_right_catalyst_component / Ph|#000000`
   - ??
   - ? same-shell no-knockout ???????
   - ???????? replay ??
2. `top_right_product_component / Ph|#ff8000` ? `CN|#0000ff`
   - ??????
   - ? same-shell ? knockout ?????????? replay ??
3. `bottom_center_reagent_component`
   - ??? `O/S` ??????? replay ??
4. `bottom_left_ligand_component`
   - ???? label residual ?????????

????????molecule ?????????????

- ???? **label replay ??**
- ???????????? knockout ???


## 2026-05-17 `same-shell no-knockout` ??????????????? replay ????

??????????

- ? `frame-global3-noknockout-fg3.wordcopy.png` ? ChemDraw ???? same-shell label-box ??
- ? `cdxml labelAlignment / labelJustification / nodeType / ??????` ?????? dominant family ?????????????

??/?????

- `scripts/compare-full-label-boxes.py`????? box ????
- `scripts/summarize-label-context-by-family.py`

?????

- `tmp/frame-word-ab/frame-global3-noknockout-label-box-compare.json`
- `tmp/frame-word-ab/frame-global3-label-component-summary-noknockout.json`
- `tmp/frame-word-ab/frame-global3-label-context-summary.json`

### 1. `same-shell no-knockout` ???? dominant family ??????

? `frame-global3` ?????? `current` ? `no-knockout` ???????????????????

#### 1.1 `bottom_right_catalyst_component`

- `Ph|#000000`
  - current: `avgDw = +1.75`, `avgDh = +0.25`, `avgDx = -1.125`, `avgDy = 0`
  - same-shell no-knockout: `avgDw = +1.75`, `avgDh = 0`, `avgDx = -1.125`, `avgDy = 0`
- `NC|#000000`
  - current = no-knockout = `(+1, 0, -1, 0)`
- `CN|#000000`
  - current = no-knockout = `(+2, 0, 0, 0)`

?????

- ????? `Ph` ???? same-shell ???????????????????
- knockout ??????????????? `+0.25 px`?
- ?? `NC/CN` ?????? knockout

????????????

- `bottom_right_catalyst` ???? **replay ??**
- knockout ????? bug????????? dominant factor

#### 1.2 `top_right_product_component`

- `CN|#0000ff`
  - current: `(+3, 0, -1, +1)`
  - no-knockout: `(+3, -1, -1, +1)`
- `Ph|#ff8000`
  - current = no-knockout = `(+2, -1, 0, +1)`
- `S|#ff8000`
  - current: `(+1, +1, -1, 0)`
  - no-knockout: `(+1, 0, -1, 0)`

?????

- ?? `Ph` ???? replay ??
- ?? `CN` ???????????? `1 px` ????
- ?? `S` ????? replay???? already ? `+1 px` ?? `-1 px` ??

#### 1.3 `bottom_center_reagent_component`

- `O|#000000`
  - current = no-knockout = `(+1.25, +0.5, -0.25, -0.25)`
- `S|#000000`
  - current = no-knockout = `(0, +1, 0, 0)`
- `N|#000000`
  - current: `(0, +1, 0, 0)`
  - no-knockout: `(0, 0, 0, 0)`

???????

- `O/S` ???????? replay ??
- ?? `N` ???? `+1 px` ????

#### 1.4 `bottom_left_ligand_component`

- `N|#000000`
  - current = no-knockout = `(+1.5, +0.5, -0.5, 0)`
- `O|#000000`
  - current = no-knockout = `(0, 0, 0, 0)`

?????? same-shell ????

- label residual ???? replay ??
- ?? raw `current -> no-knockout` ??????????? same-shell ???????

### 2. `bottom_right_catalyst / Ph|#000000` ??????????????? `cdxml ?? ? ??` ??

? `scripts/summarize-label-context-by-family.py` ??

- `cdxml labelAlignment`
- `cdxml labelJustification`
- `nodeType`
- `sideX / sideY`???????????

????????????? `Ph` ????????

- `Left / Left / Nickname / side(pos,pos)`
  - `count = 3`
  - `sumResidual = 106`
  - `avgDw = +3.0`, `avgDx = -2.0`
- `Right / Right / Nickname / side(neg,pos)`
  - `count = 2`
  - `sumResidual = 97`
  - `avgDw = +0.5`, `avgDx = -0.5`
- `Right / Right / Nickname / side(neg,neg)`
  - `count = 1`
  - `sumResidual = 69`
  - `avgDw = +1`, `avgDh = +1`, `avgDx = -1`
- `Left / Left / Nickname / side(pos,neg)`
  - `count = 1`
  - `sumResidual = 53`
- `Left / Left / Nickname / side(neg,pos)`
  - `count = 1`
  - `sumResidual = 40`

????

- ?????? `Ph` ????? +1.75 px ????????
- residual ?? **?? ChemDraw ?????** ?????
- ???????? `f4_32339` ???????
  - same-shell residual ??
  - knockout ??? `0`
  - ?????? replay ????

### 3. ?? molecule ????????????

???? `frame-global3 + same-shell` ??????????????

1. `bottom_right_catalyst_component / Ph|#000000`
   - residual ??
   - same-shell no-knockout ???????
   - ?????????? replay ??
2. `top_right_product_component / Ph|#ff8000` ? `CN|#0000ff`
   - ??????
   - ???????? replay???? knockout
3. `bottom_center_reagent_component`
   - `O/S` ????????? replay ??
4. `bottom_left_ligand_component`
   - ???????????

????????molecule ??????????????????????

- ????? attached-group / nickname / fragment ??? Word replay ??????? family-level ??
- ???????? `Ph` ??????????????


## 2026-05-17 ? family ??????`layout / nodeType / cdxml alignment` ???????????

? `frame-global3-label-context-summary.json` ???????????????????????????? `Ph/CN/S` ???????

### 1. `nodeType`?????? replay ???????? `Nickname / Fragment / attached-group`

? `nodeType ? text ? fill` ???

- `Nickname / Ph / #000000`
  - `count = 8`
  - `sumResidual = 365`
  - `avgDw = +1.75`, `avgDh = +0.25`, `avgDx = -1.12`
- `None / N / #000000`
  - `count = 7`
  - `sumResidual = 193`
  - `avgDw = +0.43`, `avgDh = +1.0`
- `None / O / #000000`
  - `count = 6`
  - `sumResidual = 157`
  - `avgDw = +0.83`, `avgDh = +0.33`
- `Fragment / CN / #0000ff`
  - `sumResidual = 66`
- `Nickname / Ph / #ff8000`
  - `sumResidual = 61`
- `Fragment / NC / #000000`
  - `sumResidual = 51`

????

- ?? replay ????????????? `O/N` ??????
  - `Nickname`???? `Ph`?
  - `Fragment`???? `CN/NC`?
  - ????? attached ????

### 2. `layout`?`attached-group-above` ??? `attached-group` ?????????

? `layout ? text ? fill` ??

- `attached-group / Ph / #000000`
  - `sumResidual = 365`
  - `avgDw = +1.75`, `avgDh = +0.25`, `avgDx = -1.12`
- `attached-group / N / #000000`
  - `sumResidual = 193`
  - `avgDh = +1.0`
- `attached-group / O / #000000`
  - `sumResidual = 99`
  - `avgDw = +1.25`, `avgDh = +0.5`
- `attached-group-above / O / #000000`
  - `count = 2`
  - `sumResidual = 58`
  - `avgDw = 0`, `avgDh = 0`, `avgDx = 0`, `avgDy = 0`

?????????

- **??? `O`?`attached-group-above` ?????????? `attached-group` ???**
- ???????????????? **attached-group ????/?????? replay ??**?

### 3. `cdxmlLabelAlignment`?`Above` ???????????????

? `cdxmlLabelAlignment ? text ? fill`?

- `Left / Ph / #000000`
  - `count = 5`
  - `sumResidual = 199`
  - `avgDw = +2.4`, `avgDx = -1.4`
- `Right / Ph / #000000`
  - `count = 3`
  - `sumResidual = 166`
  - `avgDw = +0.67`, `avgDx = -0.67`
- `Above / N / #000000`
  - `count = 3`
  - `sumResidual = 83`
  - `avgDh = +1.67`
- `None / N / #000000`
  - `count = 3`
  - `sumResidual = 90`
  - `avgDw = +1.0`, `avgDh = +0.67`
- `None / O / #000000`
  - `count = 5`
  - `sumResidual = 132`
  - `avgDw = +1.0`, `avgDh = +0.4`

?????

- ?? `Ph` ???????????????????
- `Above` ? `N` ??????? residual ??
- ? `attached-group-above / O` ?????????????

### 4. ???????????????????

?? replay ???????

- **lateral attached-group labels**
  - ??? `Nickname/Fragment` ??????? `Ph/CN/NC`
- ????? `attached-group` ??? `N/O/S`

????

- ????????
- ?????????
- ???????/?????

?????????????????

- `layout = attached-group`
- `nodeType in {Nickname, Fragment}`
- ??? `cdxml alignment / sideX sideY`

???????? `Ph` ?????????????????? replay ???


## 2026-05-17 node-attached short label `zero-layout` ???? `attached-group` ????

### 1. packaged GDI+ `node-label zero-layout` ? same-shell Word replay ????

????????? packaged GDI+ ???

- ???`node_id.is_some()`
- `text_anchor == "start"`
- ??????
- ?? packaged `EMF` ????
- ? `DrawString` ? `layoutRect` ?? `0x0`????? ChemDraw ? point-like label ??

?????

- `CHEMCORE_EMF_PACKAGED_NODE_LABEL_ZERO_LAYOUT=1`

? full-doc `frame-global3 + same-shell` ??????

- ?? best-shift `IoU`?**????**
  - baseline: `0.8618815442410679`
  - nodezero: `0.8618815442410679`
- ???????? `dx = -1, dy = 0`
- label-box compare ?????????
  - `f4_32333 Ph`: `deltaDims = [2, 1]`, `deltaTopLeft = [-2, 0]`
  - `f5_2784 CN`: `deltaDims = [3, 0]`, `deltaTopLeft = [-2, 1]`
  - `f5_2794 orange Ph`: `deltaDims = [1, -1]`, `deltaTopLeft = [0, 1]`

???

- **molecule ?? lateral label residual ?????? packaged GDI+ `RectF` / zero-layout ???**
- ?? same-shell ??????????? **fallback GDI / HDC ?** ???? anchor ?????

### 2. `attached-group` ? `attached-group-above` ??????

?? `tmp/frame-word-ab/frame-global3-label-context-summary.json` ???

- `attached-group`
  - `count = 25`
  - `sumResidual = 950`
  - `avgDw = +1.24`
  - `avgDh = +0.48`
  - `avgDx = -0.56`
  - `avgDy = +0.04`
- `attached-group-above`
  - `count = 2`
  - `sumResidual = 58`
  - `avgDw = 0`
  - `avgDh = 0`
  - `avgDx = 0`
  - `avgDy = 0`

???

- `attached-group-above` ????????????
- ?? dominant family ??? **lateral attached-group**?

### 3. `nodeType` ?????? `Nickname / Fragment`

? `(layout, nodeType)` ???

- `attached-group + Nickname`
  - `count = 9`
  - `sumResidual = 426`
  - `avgDw = +1.778`
  - `avgDh = +0.111`
  - `avgDx = -1.0`
  - `avgDy = +0.111`
- `attached-group + Fragment`
  - `count = 3`
  - `sumResidual = 155`
  - `avgDw = +2.0`
  - `avgDh = 0`
  - `avgDx = -0.667`
  - `avgDy = +0.333`
- `attached-group + None`
  - `count = 13`
  - `sumResidual = 369`
  - `avgDw = +0.692`
  - `avgDh = +0.846`
  - `avgDx = -0.231`
  - `avgDy = -0.077`

???

- `Nickname / Fragment` ???????????????
- `nodeType = None` ?? residual??????????????

### 4. `text` ?????

? `(layout = attached-group, text)` ????

- `Ph`
  - `count = 9`
  - `avgLeft = -1.0`
  - `avgRight = +0.778`
  - `avgTop = +0.111`
  - `avgBottom = +0.222`
  - `avgWorldW ? 34.51`
- `CN`
  - `count = 2`
  - `avgLeft = -0.5`
  - `avgRight = +2.0`
  - `avgTop = +0.5`
  - `avgBottom = +0.5`
- `NC`
  - `count = 1`
  - `left = -1.0`
  - `right = 0.0`
- `S`
  - `count = 2`
  - `left = -0.5`
  - `right = 0.0`
- `N`
  - `count = 7`
  - `left = -0.143`
  - `right = +0.286`
  - `bottom = +1.0`
- `O`
  - `count = 4`
  - `left = -0.25`
  - `right = +1.0`

???

- ???? `Ph` ??? `CN` ??????? family?
- ?????????????????? **lateral attached-group + ??? + Nickname/Fragment ??**?

### 5. ????????

??????????????

- ???? label ????? packaged GDI+ `DrawString` ? `RectF` ?????
- ????? full-doc same-shell ????`node-label zero-layout` ???????
- ????????????
  - fallback GDI / HDC ?
  - ???? node-attached label anchor ??
  - ?????? packaged GDI+ `layoutRect`?

## 2026-05-17 attached-group �ֲ��������ս��primaryNeighborBucket��

�����ű���
- `scripts/analyze-attached-label-local-geometry.py`
- ���`tmp/frame-word-ab/frame-global3-attached-label-geometry.json`

���ְ� full-doc same-shell �� `attached-group` label �ٲ���һ�㡰�ڵ㱾�ؼ��Ρ�������
- `componentQuadrant`��label center ������ bbox ������
- `primaryNeighborBucket`���� label �����ڵ���ڽӼ���������`east / west / north / south`��
- `overhangToComponent`
- `labelOffsetFromNode`

### 1. `f4_32339` ����һ��ĺ�ɫ `Ph`
�� `bottom_right_catalyst_component` �� 8 ����ɫ `Ph` �У�

- `f4_32339`
  - `text = Ph`
  - `fill = #000000`
  - `cdxmlLabelJustification = Left`
  - `componentQuadrant = RB`
  - `primaryNeighborBucket = north`
  - `deltaDims = [7, 1]`
  - `deltaTopLeft = [-6, 0]`

��ͬ�����������ɫ `Ph`��
- `Left + RB + west`��`f4_32337`, `f4_32345`
- `Left + LB + north`��`f4_32343`
- `Right + LB/LT + east`��`f4_32333`, `f4_32341`, `f4_32347`
- `Left + RT + west`��`f4_32335`

���� `f4_32339` �Ѿ��ս��һ����խ�����壺
- **black `Ph`**
- **Left justification**
- **componentQuadrant = RB**
- **primaryNeighborBucket = north**

### 2. `primaryNeighborBucket` �������н�������
��ȫ�� `attached-group` label ���ܣ�

- `north`
  - `count = 7`
  - `sumResidual = 221`
  - `avgDw = +1.857`
  - `avgDh = +0.571`
  - `avgDx = -1.143`
- `south`
  - `count = 4`
  - `sumResidual = 141`
  - `avgDw = +1.5`
- `west`
  - `count = 7`
  - `sumResidual = 293`
  - `avgDw = +1.143`
- `east`
  - `count = 7`
  - `sumResidual = 295`
  - `avgDw = +0.571`

Ҳ����˵��
- **north/south ����������**��ƽ��������ͱ� east/west ����
- `f4_32339` ��������ɵ� `north` Ͱ�����żȻ

### 3. ���������ǡ����� north ����ը��
ͬ�� `primaryNeighborBucket = north` �Ļ��У�
- `f4_32343`��black `Ph`, Left, LB��
- `f5_2788`��orange `S`, Left, RB��
- ��� `N/O`

����Ҳ�� residual������û�� `f4_32339` ��ô���ˡ�
��˵����
- `north` ����Ч����
- ������Ҫ�� `text/fill/componentQuadrant` ���Ͽ�

### 4. ��ǰ��խ����ֵǮ�� replay ����
Ŀǰ��ֵ�ü������Ĳ��ǡ����� molecule labels�������ǣ�

- **black `Ph`, Left, RB, north**��`f4_32339`��
- �Լ����ڱȽ��飺
  - black `Ph`, Left, LB, north��`f4_32343`��
  - black `Ph`, Left, RB, west��`f4_32337`, `f4_32345`��
  - orange `S`, Left, RB, north��`f5_2788`��

��һ��Ӧ���ȱȽ��⼸���ֲ���� replay ���죬�������ٰ����� `Ph` ����һ�𿴡�
## 2026-05-17 attached-group ��������ϸ����replay vs knockout vs path sensitivity

�����ű���
- `scripts/summarize-attached-knockout-geometry.py`
- `scripts/summarize-attached-path-sensitivity.py`
- ���
  - `tmp/frame-word-ab/frame-global3-attached-knockout-geometry.json`
  - `tmp/frame-word-ab/frame-global3-attached-path-sensitivity.json`

### 1. `f4_32339` ���ڿ�����ȷ����Ϊ **knockout �Ŵ��� outlier**

�� `current` �� `no-knockout` ֱ�������

- `f4_32339`
  - `current deltaDims = [7, 1]`
  - `current deltaTopLeft = [-6, 0]`
  - `replay(no-knockout) deltaDims = [1, 0]`
  - `replay(no-knockout) deltaTopLeft = [0, 0]`
  - `knockout contribution = [+6, +1] / [-6, 0]`

���������ǡ�text replay �����ը�� 7 px�������ǣ�
- replay ����ֻ����΢ƫ��
- **knockout ������������ֶ���Ŵ���Լ 6 px�����Ѿֲ� bbox �������� 6 px**

��һ���ܹؼ�����Ϊ���� `f4_32339` �ʹ���� molecule label �Ļ��Ʒֿ��ˡ�

### 2. ������˵�������������� `Ph` ���� knockout ����

- `f4_32347`��black `Ph`, Right, LB, east��
  - `current = [0,0] / [0,0]`
  - `replay = [1,0] / [-1,0]`
  - knockout ������ replay ����������

- `f4_32345`��black `Ph`, Left, RB, west��
  - `current = [0,0] / [0,0]`
  - `replay = [3,0] / [-3,0]`
  - knockout ͬ����������

- `f5_2794`��orange `Ph`, Left, RB, west��
  - `current = [2,-1] / [0,1]`
  - `replay = [2,-1] / [0,1]`
  - knockout ����Ϊ `0`
  - ���� **�� replay ����**

- `f5_2784`��blue `CN`, Left, LT, south��
  - knockout ֻ�� `+1 px` ����߶�����
  - ���廹�� replay ����

���Ե�ǰ attached-group �����Ѿ��ֳܷɣ�

- **A. knockout-amplified outlier**
  - `black Ph, Left, RB, north`��`f4_32339`��
- **B. replay-dominant families**
  - `orange Ph, Left, RB, west`
  - `blue CN, Left, LT, south`
  - ��ɺ�ɫ `Ph/CN/NC`
- **C. replay �� knockout ����/ѹƽ�ļ���**
  - `f4_32345`, `f4_32347` ���� black `Ph`

### 3. `structext / nodezero` ��·�������ֻ�Ǵμ�����

�� `current` �� `structext / nodezero` �ȽϺ�

- `f4_32339`
  - `current = [7,1] / [-6,0]`
  - `structext = [6,1] / [-6,0]`
  - ֻ���� `1 px` ���
  - ˵�� **�����ⲻ�ڽṹ��ǩ replay path �л�**

- `f4_32337`��black `Ph`, Left, RB, west��
  - `current [2,0] -> structext [1,0]`
- `f5_2794`��orange `Ph`, Left, RB, west��
  - `current [2,-1] -> structext [1,-1]`

��Щ˵�� `structext/nodezero` ��ĳЩ replay-dominant ������ **1 px ��** ���ƣ�
�����򲻵� `f4_32339` ���� knockout �����쳣�ĸ���

### 4. ��ǰ���������ȼ�

���� molecule �ڲ� attached-group �����ٻ��һ���ߣ����ǣ�

1. **�ȵ������� knockout-amplified outlier**
   - `black Ph, Left, RB, north`��`f4_32339`��
2. �ٴ��� replay-dominant lateral families
   - `orange Ph`
   - `blue CN`
   - ��� `black Ph/CN/NC`
3. `structext/nodezero` ֻ��Ϊ replay ΢�����ߣ���Ӧ������Ϊ���޸�·��


## 2026-05-17 ???`f4_32339` ?? knockout ??? outlier??? replay-dominant outlier

??? `f4_32339` ?? `knockout-amplified outlier` ????????? `current/structext/nodezero` ????? `no-knockout` ?????????????? `pad`??????????

??? **same-shell + pad=0 ? label-box ??** ???
`tmp/frame-word-ab/frame-global3-attached-knockout-geometry-pad0.json`
???

- `f4_32339`
  - `currentDeltaDims = [7, 1]`
  - `currentDeltaTopLeft = [-6, 0]`
  - `replayDeltaDims = [7, 1]`
  - `replayDeltaTopLeft = [-6, 0]`
  - `knockoutDeltaDims = [0, 0]`
  - `knockoutDeltaTopLeft = [0, 0]`

????
- ??????`f4_32339` ?????? **replay ??????**?
- ?? knockout ???????

??????????????????? `knockoutDelta = 0`?
- `f5_2794`??? `Ph`?
- `f5_2784`??? `CN`?
- `f4_32343`
- `f4_32345`
- `f4_32347`

?? molecule ?? attached-group ??????????????
1. **replay-dominant families**
2. `structext/nodezero` ?? `1 px` ? replay ??
3. `DocumentKnockout` ?????? bug???? same-shell label residual ???

## 2026-05-17 replay ?????????`north + right-half + ? right-gap`

?????
- `scripts/summarize-attached-replay-geometry.py`
- ???
  - `tmp/frame-word-ab/frame-global3-attached-replay-geometry.json`

??? `attached-group` ? **replay ????** ???????????????
- `cdxmlLabelJustification`
- `componentHalfX`
- `primaryNeighborBucket`
- `gapRightBucket`

???
- `gapRight = -overhangToComponent.right`
- ?? label ??? component ???????????

### 1. ?????

??? `attached-group` ???
- `replayWidth` ? `gapRight` ??????? `-0.315`
- `replayWidth` ? `labelOffsetFromNode.x` ??????? `+0.428`
- `replayWidth` ? `componentRelCenter.x` ??????? `+0.317`

?????
- ?? **????**
- ?? **??????**
- ?? **component ???????**

replay ??????????

### 2. ????????? replay ??

??? replay ??????? `Ph`?????
- `cdxmlLabelJustification = Left`
- `componentHalfX = right`
- `primaryNeighborBucket = north`
- `gapRightBucket = lt40`

?????????
- `f4_32339`

?? replay ????
- `avgReplayWidth = 7`
- `avgReplayLeftShift = 6`
- `avgReplayL1 = 14`

????
- `north + right-half + gapRight >= 50`
  - `f5_2788`
  - `avgReplayWidth = 1`
  - `avgReplayLeftShift = 1`
  - `avgReplayL1 = 3`
- `west + right-half + gapRight < 40`
  - `f5_2794`, `f4_32323`, `f4_32337`
  - `avgReplayWidth = 2`
  - `avgReplayLeftShift = 0`
  - `avgReplayL1 = 2.67`
- `north + left-half`
  - `f4_32343`, `f2_34461`, `f1_28331`, `f2_37`, `f2_34464`
  - `avgReplayWidth = 1`
  - `avgReplayLeftShift = 0.2`
  - `avgReplayL1 = 1.6`

????????????
- `north` ????
- `right-half` ?????
- `gapRight` ??????
- ????????????
  - **?????north?**
  - **???????**
  - **???????**

### 3. ???????????

- `f4_32339`
  - `Left + RB + north`
  - `gapRight = 33.23`
  - replay ????
- `f5_2788`
  - ?? `Left + RB + north`
  - ? `gapRight = 90.21`
  - replay ??????
- `f4_32345`
  - `Left + RB + west`
  - `gapRight = 66.48`
  - replay ??? `0`
- `f5_2794`
  - `Left + RB + west`
  - `gapRight = 8.53`
  - ???????? `f4_32339` ?????

???? replay ?????????
1. **?? `north + right-half + small right-gap` ?????**
   - ?????`f4_32339`
2. ?? `south + left-half` ??
   - ?????`f5_2784`, `f2_43`, `f2_34463`
3. ???????? `west + right-half` ??
   - ?????`f5_2794`, `f4_32323`, `f4_32337`
## 2026-05-17 ???attached-group ?? primitive ????

- ?????`crates/chemcore-engine/examples/dump_text_primitives.rs`
- ????????? molecule ????? `render_document()` ???? Office ???? `RenderPrimitive::Text` ????? replay ????????? primitive ???????

### ????
- `f4_32337`
- `f4_32339`
- `f4_32343`
- `f4_32345`
- `f5_2784`
- `f5_2788`
- `f5_2794`

### ???
?? attached-group ??? `RenderPrimitive::Text` ???????????????? attached ??????

- `role = document-text`
- `text = ""`
- ?????????? `runs[0].text`
- `textAnchor = "start"`
- `boxWidth = null`
- `lineHeight = null`
- `baselineOffset = 21.8694`
- `rotate = 0`
- `fontSize = 26.67`
- `fontFamily = Arial`

?? black `Ph`?`f4_32337 / 32339 / 32343 / 32345`?? Office ????????
- `x/y`
- `nodeId`
- `runs[0].text`
- `fill/fontWeight`
????

### ????
??? attached-group ???primitive ? `x/y` ?? label box ??????? glyph bbox????

- `primitive.x = label.position.x + merged_molecule.translate.x`
- `primitive.y = label.position.y + merged_molecule.translate.y`

???
- `f4_32339`
  - `label.position = [1132.83, 496.64]`
  - merged molecule translate = `[136.88, 283.63]`
  - `primitive = [1269.71, 780.27]`

### ??
???????????? attached replay ????? `Left + right-half + north + small right-gap`???? Office ???????? richer primitive metadata ????????Office renderer ? attached label ??????

- ?? point-anchored start text
- ?? normal runs
- ?? boxWidth
- ?? attached layout / gap / quadrant / neighbor bucket

???? residual ???????
1. **renderer/downstream replay** ??? start-anchor point text ?????????/??????
2. **?? anchor choice (`label.position`)** ??? attached ?????????? ChemDraw ???

### ??????
????????????????
- ?????? Office ??????? renderer ?????? `x/y + run text`?????? `f4_32339` ? `Left + RB + north + small right-gap`?
- ???????????????????
  - ? render primitive ????? attached-label ??????
  - ?? engine ????? `label.position` ???????

### renderer ????????
? `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs` ??`node_id` ???? `preview_env_node_id_filter / preview_primitive_node_id` ????????????????????? DrawString/TextOut ????????Office ???**????**?????? `Left + RB + north + small right-gap` ?? replay ?????????? primitive ? `x/y + run text + ??` ????

## 2026-05-17 ???attached ??? primitive ???????

- ?????`scripts/summarize-attached-primitive-collapse.py`
- ???
  - `tmp/current-thiocyanation.payload.json`
  - `tmp/all-text-primitives.json`?? `dump_text_primitives` ???

### ????
???? `27` ? `attached-group*` ????? `RenderPrimitive::Text` ??????????

- `textAnchor = start`?`27 / 27`
- `boxWidth = None`?`27 / 27`
- `text = ""`?`27 / 27`
- ?????????? `runs[*].text`

???????? `cdxmlLabelAlignment` ???? primitive ??????
- `None -> start`: `8`
- `Left -> start`: `11`
- `Right -> start`: `5`
- `Above -> start`: `3`

??????? source label ? CDXML ?? `Left / Right / Above / None` ????? Office ????????????
- point-anchored
- start text
- no box width
- empty top-level text string

### ??????
???? outlier ???
- ???? `f4_32339` ??????? attached ?????
- ?? **attached ?????????? primitive ?????????**

?????????? Office ?? attached-family replay ?????????????
- renderer ????? `labelAlignment / boxField / attached layout` ?????
- ??? `payload + nodeId` ????? engine ?? richer metadata ?? primitive??? Office ???? `x/y + run text + ??` ??

### Office-only ??????
?? attached ????? primitive ????????????Office ???????????

- `draw_payload_vector_preview_internal(...)` ???????????? `OleObjectPayload`
- ??????? payload ????`preview_bond_context(payload)`
- `draw_preview_primitive / draw_gdiplus_primitive` ?????????? context

???`node_id` ??????????
- `preview_env_node_id_filter`
- `preview_primitive_node_id`

???????????????????????? Office-only ??????????????????????? `bond_context` ???? `label_context(node_id -> source label metadata)`?? `labelAlignment / boxField / label.position / glyphPolygons` ???????? replay ????

## 2026-05-17 ???attached replay ???????

- ?????`scripts/fit-attached-replay-family.py`
- ???`tmp/frame-word-ab/frame-global3-attached-knockout-geometry-pad0.json`
- ????????? bucket??? source metadata ? `replayL1` ?????????

### ??
??????? `DecisionTreeRegressor(max_depth=3)`?????
- `text`
- `fill`
- `cdxmlLabelAlignment`
- `cdxmlLabelJustification`
- `componentHalfX / componentHalfY / componentQuadrant`
- `primaryNeighborBucket`
- `nodeType`
- `sideX / sideY`
- `gapRight`

? `replayL1 = sum(abs(replayDeltaDims + replayDeltaTopLeft))` ?????????
- `R? = 0.9051`

### ??????
???????
- `gapRight <= 40.44`

???????????????
- `primaryNeighborBucket == north`

???????? replay ???
- `gapRight <= 40.44`
- `primaryNeighborBucket = north`
- ?? `replayL1 = 14`

??????
- `f4_32339`

????????? `north` ? attached ???
- ?????? `replayL1 ? 2`
- ?????? `replayL1 ? 4`

? `gapRight > 40.44` ???????
- `componentQuadrant == LT`
- `cdxmlLabelAlignment == Left`
- `fill == #0000ff`

?????? replay ???

### ??
??????????????????????????????????? source-metadata ???
- **?????right gap ????**
- **???????????? north**
- ???????/??/labelAlignment ?????

????????
- ?????? Office-only ??????? metadata ???? label ???????
- ??????? renderer ??? `gapRight` ? `primaryNeighborBucket` ?????????


## 2026-05-17 attached-label replay anchor sensitivity matrix

### Goal
Verify whether the dominant replay outlier `f4_32339` is mainly an x-anchor placement issue, by applying an Office-only replay nudge to the narrow family:
- `layout = attached-group`
- `labelJustification = Left`
- `componentHalfX = right`
- `primaryNeighborBucket = north`
- `gapRight <= 40.44`
- `fill = #000000`

### Implementation
- Added env-gated experiment hook in `renderer.rs`:
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT=<float px>`
- Same-shell comparison used the fixed `frame-global3` header frame `(1441,2994,14431,8656)`.

### Matrix
Reference summary:
- [tmp/frame-word-ab/attached-nudge-matrix.json](d:/Projects/chemcore/tmp/frame-word-ab/attached-nudge-matrix.json)

Key rows:
- `-12 px`: global `IoU = 0.856209`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `-6 px`: global `IoU = 0.856209`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `-3 px`: global `IoU = 0.857074`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `0 px` baseline: global `IoU = 0.861882`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `+3 px`: global `IoU = 0.858294`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `+6 px`: global `IoU = 0.856437`, `f4_32339 deltaDims = [8,1]`, `deltaTopLeft = [-6,0]`
- `+12 px`: global `IoU = 0.847765`, `f4_32339 deltaDims = [9,1]`, `deltaTopLeft = [-6,0]`

### Findings
- Across `[-12, +3] px`, the dominant outlier `f4_32339` is pixel-stable.
- `deltaTopLeft` stays exactly `[-6, 0]`.
- `deltaDims` stays exactly `[7, 1]`.
- Positive nudges do not move the left edge back toward ChemDraw.
- Once the positive nudge is large enough, the local ink box only gets wider:
  - `+6 px` -> width residual `7 -> 8`
  - `+12 px` -> width residual `7 -> 9`
- Global same-shell IoU is best at the baseline `0 px` case.

### Conclusion
This family is not anchor-placement dominated.
A straightforward replay x-anchor correction does not repair the characteristic `[-6, 0]` left shift; it only fattens the local ink box once the positive nudge is large enough. The dominant cause must lie deeper in replay width / ink generation rather than ordinary anchor placement.

### Next step
Treat `f4_32339` and its narrow family as a replay-width / ink-generation problem, not as a text-anchor problem. Avoid spending more time on simple x-anchor nudges.


## 2026-05-17 attached-label run font-scale sensitivity matrix

### Goal
Check whether the dominant replay outlier family is driven by local font size rather than anchor placement. The target family remained:
- `text = Ph`
- `fill = #000000`
- `layout = attached-group`
- `labelJustification = Left`
- `componentHalfX = right`
- `primaryNeighborBucket = north`
- `gapRight <= 40.44`

### Implementation
- Added env-gated experiment hook in `renderer.rs`:
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_EXPERIMENT=<scale>`
- Unlike the earlier anchor experiment, this path scales explicit `PreviewTextRun.font_size` values, so both layout and glyph-size calculations use the same adjusted run font sizes.
- Same-shell comparison again used the fixed `frame-global3` header frame `(1441,2994,14431,8656)`.

### Matrix
Reference summary:
- [tmp/frame-word-ab/attached-runscale-matrix.json](d:/Projects/chemcore/tmp/frame-word-ab/attached-runscale-matrix.json)

Key rows:
- `0.90x`: global `IoU = 0.859128`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `0.95x`: global `IoU = 0.860190`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `1.00x` baseline: global `IoU = 0.861882`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `1.05x`: global `IoU = 0.859765`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `1.10x`: global `IoU = 0.859274`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`

### Findings
- Across the full `0.90 .. 1.10` run-scale range, the dominant outlier `f4_32339` does not move at all.
- `deltaTopLeft` remains exactly `[-6, 0]`.
- `deltaDims` remains exactly `[7, 1]`.
- The local residual count stays at `30` throughout the entire matrix.
- Global same-shell IoU is still best at the baseline `1.00x` case.

### Conclusion
This family is not font-scale dominated either.
Even when explicit per-run font sizes are scaled before packaged replay, the local bbox of `f4_32339` is pixel-stable. Together with the earlier anchor matrix, this strongly suggests that the dominant cause is deeper replay width / ink-generation behavior rather than ordinary anchor placement or font size.

### Next step
Keep treating this narrow family as a replay-width / ink-generation problem. The next experiments should target how the replay path generates visible ink for attached-group labels, not how it places or sizes the text anchor.


## 2026-05-17 attached-label TextRenderingHint matrix

### Goal
Probe whether the narrow replay-dominant attached-label family is primarily controlled by packaged GDI+ text hinting rather than anchor placement or font size.

Target family:
- `text = Ph`
- `fill = #000000`
- `layout = attached-group`
- `labelJustification = Left`
- `componentHalfX = right`
- `primaryNeighborBucket = north`
- `gapRight <= 40.44`

### Implementation
- Added env-gated experiment hook in `renderer.rs`:
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TEXT_HINT_EXPERIMENT=<int>`
- The hook only overrides `GdipSetTextRenderingHint` for the narrow family during packaged replay.
- Same-shell comparison again used the fixed `frame-global3` header frame `(1441,2994,14431,8656)`.

### Matrix
Artifacts:
- `tmp/frame-word-ab/hint-matrix/hint-*.metrics.json`
- `tmp/frame-word-ab/hint-matrix/hint-*.labels.json`

Key rows:
- `baseline`: global `IoU = 0.861882`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `0`: global `IoU = 0.858124`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `1`: global `IoU = 0.858124`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `2`: global `IoU = 0.859541`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `3`: global `IoU = 0.858670`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `4`: global `IoU = 0.861882`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`
- `5`: global `IoU = 0.859114`, `f4_32339 deltaDims = [7,1]`, `deltaTopLeft = [-6,0]`

### Findings
- The narrow family is completely insensitive to packaged `TextRenderingHint` changes in the tested range.
- `f4_32339` stays pixel-identical for all tested hint values.
- Baseline / explicit hint `4` are identical, and every other hint only worsens global IoU slightly.

### Conclusion
This family is not text-hint dominated either. Together with the earlier x-anchor and run-scale experiments, the replay problem is now clearly deeper than ordinary text placement / size / hint controls.

## 2026-05-17 label local shape-IoU versus bbox metrics

### Motivation
The earlier attached-label work relied heavily on local bbox deltas (`deltaDims`, `deltaTopLeft`). That turned out to be too coarse for tightly packed catalyst labels: some labels keep the same bbox while still showing obvious red/blue shape residual inside the box.

### Tooling
Added:
- [scripts/compare-full-label-iou.py](d:/Projects/chemcore/scripts/compare-full-label-iou.py)

This computes local ink-shape IoU inside each label box under the same-shell best-shift alignment, instead of only comparing the outer bbox.

### Key findings
For black catalyst `Ph` labels under `frame-global3 + same-shell`:
- `f4_32333`: `IoU = 0.4926`, `oursOnly = 52`, `refOnly = 17`, `residual = 69`
- `f4_32335`: `IoU = 0.5954`, `oursOnly = 45`, `refOnly = 8`, `residual = 53`
- `f4_32347`: `IoU = 0.6257`, `oursOnly = 46`, `refOnly = 18`, `residual = 64`
- `f4_32345`: `IoU = 0.6970`, `oursOnly = 37`, `refOnly = 3`, `residual = 40`
- `f4_32337`: `IoU = 0.7097`, `oursOnly = 35`, `refOnly = 1`, `residual = 36`
- `f4_32339`: `IoU = 0.7521`, `oursOnly = 26`, `refOnly = 4`, `residual = 30`

This is the opposite of the earlier bbox-centric focus: `f4_32339` is still abnormal, but it is not the worst black `Ph` once local shape overlap is measured directly.

Grouped by catalyst half:
- top-half black `Ph`: `avgIoU = 0.5440`, `sumResidual = 122`
- bottom-half black `Ph`: `avgIoU = 0.6997`, `sumResidual = 243`

### Interpretation
- Local shape overlap is a better discriminator than bbox deltas for this family.
- The dominant catalyst replay problem is broader than the single `f4_32339` outlier.
- In shape-IoU terms, the top-half catalyst `Ph` labels are currently the worst subgroup.
- `f4_32339` remains useful because its bbox anomaly is large and stable, but it should no longer be treated as the sole representative of the family.

### Next step
Continue on the replay path, but pivot the main family from ?single-node `f4_32339`? to ?top-half catalyst black `Ph` labels under same-shell shape-IoU?. That is the sharper target for the next round.


## 2026-05-17 same-shell attached-label phase sensitivity (frame-global3 origin shifts)

This round tested a very narrow hypothesis: some attached-label replay families may be driven more by local pixel phase than by font size / hint / anchor.

New tool:
- `scripts/summarize-attached-phase-sensitivity.py`

Experiment setup:
- same-shell template: `tmp/frame-word-ab/frame-global3-shellchem.docx`
- base frame: `(1441, 2994, 14431, 8656)`
- only apply uniform frame-origin shifts, keeping width/height fixed:
  - `dy = 0`
  - `dy = +1`
  - `dy = +3`
- compare local label-shape IoU with:
  - `scripts/compare-full-label-iou.py`

Artifacts:
- `tmp/frame-word-ab/top-black-ph-phase-search.json`
- `tmp/frame-word-ab/phase-label-iou/frame_dy0.json`
- `tmp/frame-word-ab/phase-label-iou/frame_dy1.json`
- `tmp/frame-word-ab/phase-label-iou/frame_dy3.json`
- `tmp/frame-word-ab/phase-label-iou/attached-phase-sensitivity.json`

Key findings:

1. The top-half catalyst black `Ph` family is not uniform.
   Under the current best same-shell frame, the worst local IoUs are:
   - `f4_32333`: `0.4926`
   - `f4_32335`: `0.5954`
   - `f4_32347`: `0.6257`
   - `f4_32345`: `0.6970`
   - `f4_32337`: `0.7097`
   - `f4_32339`: `0.7521`

2. A pure vertical frame-origin nudge changes these labels differently.
   Example deltas:
   - `f4_32333`: `0.4926 -> 0.5188 -> 0.5191`
   - `f4_32335`: `0.5954 -> 0.6250 -> 0.6349`
   - `f4_32339`: `0.7521 -> 0.7398 -> 0.7054`
   - `f4_32345`: `0.6970 -> 0.6970 -> 0.6791`

3. The sensitivity aligns better with phase buckets than with text identity alone.
   The most useful buckets so far are:
   - `centerYPhase = 0.735`, `boxTopPhase = 0.2`
     - count `4`
     - avg `dy+1` gain `+0.0149`
     - avg `dy+3` gain `+0.0120`
   - `centerYPhase = 0.535`, `boxTopPhase = 0.0`
     - count `3`
     - avg `dy+1` gain `-0.0058`
     - avg `dy+3` gain `-0.0071`
   - `centerYPhase = 0.935`, `boxTopPhase = 0.4`
     - count `3`
     - avg `dy+1` gain `+0.0159`
     - avg `dy+3` gain `+0.0099`

4. This supports a stronger replay-phase hypothesis.
   We have already ruled out:
   - attached-label x-anchor nudges
   - run font-scale tweaks
   - packaged `TextRenderingHint` overrides

   The remaining signal is consistent with local replay depending on page-space phase / rasterization alignment.

5. Global tradeoff remains real.
   The same `dy` shifts that help the worst top black `Ph` labels reduce global same-shell IoU:
   - base `dy=0`: global `0.861882`
   - `dy=+1`: global `0.845352`
   - `dy=+3`: global `0.828192`

Current interpretation:
- `frame-global3` is still a global compromise.
- The next useful target is not a single label like `f4_32339`, but phase-sensitive attached-label replay buckets, especially within the catalyst top-half label family.


## 2026-05-17 attached-label phase buckets: vertical coherence beats horizontal coherence

Follow-up to the phase-sensitivity work: compare small uniform frame-origin shifts in `y` versus `x`.

Artifacts:
- `tmp/frame-word-ab/top-black-ph-phase-search.json`
- `tmp/frame-word-ab/phase-label-iou/`
- `tmp/frame-word-ab/phase-label-iou-x/`
- `tmp/frame-word-ab/phase-label-iou/attached-phase-sensitivity.json`
- `tmp/frame-word-ab/phase-label-iou-x/attached-phase-sensitivity-x.json`

Vertical (`dy`) findings:
- `f4_32333`: `0.4926 -> 0.5188 -> 0.5191`
- `f4_32335`: `0.5954 -> 0.6250 -> 0.6349`
- `f4_32339`: `0.7521 -> 0.7398 -> 0.7054`
- `f4_32345`: `0.6970 -> 0.6970 -> 0.6791`

This is not random. The response is much easier to explain by phase buckets:
- `centerYPhase = 0.735`, `boxTopPhase = 0.2`
  - avg `dy+1` gain `+0.0149`
  - avg `dy+3` gain `+0.0120`
- `centerYPhase = 0.535`, `boxTopPhase = 0.0`
  - avg `dy+1` gain `-0.0058`
  - avg `dy+3` gain `-0.0071`
- `centerYPhase = 0.935`, `boxTopPhase = 0.4`
  - avg `dy+1` gain `+0.0159`
  - avg `dy+3` gain `+0.0099`

Horizontal (`dx`) findings:
- `f4_32333`: best with strong negative `dx`
- `f4_32335`: mild improvement at `dx = +1`
- `f4_32339`: worsens for positive `dx`
- several other labels stay nearly flat around `dx in {-1,0,+1}`

Phase-bucket summaries also show a weaker and less coherent pattern for `x`:
- `centerYPhase = 0.735`, `boxTopPhase = 0.2`
  - avg `dx-1` delta `-0.0037`
  - avg `dx+1` delta `-0.0283`
  - avg `dx+3` delta `-0.0554`
- most buckets either stay near zero for `dx-1` or degrade for positive `dx`

Interpretation:
- `y` shifts reveal a family-level replay phase signal.
- `x` shifts mostly do not. They behave more like per-label tradeoffs than a coherent family rule.
- For the current catalyst black `Ph` problem, vertical phase is therefore a much more promising axis than horizontal frame translation.


## 2026-05-17 attached label page-space top-phase is a stronger predictor than world-space phase

Artifacts:
- `scripts/analyze-attached-page-phase.py`
- `tmp/frame-word-ab/attached-page-phase.full.json`

Method:
- Stop guessing from world-space `labelCenterWorld` / `worldBox.top` alone.
- Reconstruct the packaged GDI+ placement used by `draw_gdiplus_text()` and `draw_gdiplus_text_run()`:
  - `origin = transform.gdip_point((x, y))`
  - `baseline_top = baseline_offset * gdiplus_text_scale(transform)` for normal attached labels
  - `top = baseline_y - baseline_top`
  - `rectHeight = font_px * 1.45`
- Join those page-space phases with the same-shell `dy+1 / dy+3` local label-IoU sensitivity table.

Key findings:
1. The previously useful world-space buckets were still mixing labels that respond very differently.
   Example: several catalyst black `Ph` labels all lived in the old `centerYPhase = 0.735 / boxTopPhase = 0.2` bucket, but their page-space top phases split them into two families:
   - `topPagePhase ~= 0.628`: positive `dy` response
   - `topPagePhase ~= 0.308`: flat or negative `dy` response

2. `topPagePhase` is now the cleanest predictor seen so far.
   Strong buckets from `attached-page-phase.full.json`:
   - `topPagePhase = 0.6280433690`, `baselineYPagePhase = 0.6018614106`
     - count `2`
     - avg `dy+1 = +0.0279`
     - avg `dy+3 = +0.0330`
     - samples: `f4_32333`, `f4_32335`
   - `topPagePhase = 0.3401780222`, `baselineYPagePhase = 0.3139960638`
     - count `3`
     - avg `dy+1 = -0.0058`
     - avg `dy+3 = -0.0071`
     - samples: `f4_32331`, `f4_32339`, `f4_32343`
   - `topPagePhase = 0.0709496363`, `baselineYPagePhase = 0.0447676779`
     - count `2`
     - avg `dy+1 = +0.0154`
     - avg `dy+3 = +0.0223`
     - samples: `f5_2788`, `f5_2794`

3. `rectBottomPagePhase` tracks the same families, but mostly because `fontPx` and `baselineTopPx` are constant for these attached labels.
   For the current black/orange/blue attached-label families, `topPagePhase` is the most interpretable signal.

Interpretation:
- The dominant replay-family split is no longer best described as `north vs west` or `RB vs LT` alone.
- A more faithful description is:
  - packaged attached-label replay depends on page-space vertical phase generated by the GDI+ placement formula
  - local geometry / neighbor bucket still matters, but page-space `topPagePhase` explains the `dy` family response more directly than the old world-space buckets did.

Next step:
- treat `topPagePhase` as the primary axis for attached-label replay families
- compare it against the worst catalyst-top black `Ph` and the product-side orange/blue labels before making any new replay experiments


## 2026-05-17 attached-label same-shell `topPagePhase` band search

Artifacts:
- `scripts/search-attached-phase-policy.py`
- `tmp/frame-word-ab/attached-phase-policy-search.json`

Method:
- Take the packaged attached-label same-shell sensitivity table from `attached-page-phase.full.json`.
- Search simple non-wrapping `topPagePhase` interval policies.
- Allowed actions per interval:
  - `0`: no `y` nudge
  - `1`: reuse the measured `dy+1` replay variant
  - `3`: reuse the measured `dy+3` replay variant
- Search both:
  - unconstrained one-band / two-band policies
  - `safe` policies that reject any interval containing a label with negative delta for the chosen action

Key findings:
1. The highest-scoring unconstrained policy is broad but dirty:
   - one band `[0.0, 0.6313786402477035)` with `dy+1`
   - `totalDeltaIou = 0.2416888474`
   - but it includes `5` negatively affected labels

2. The highest-scoring unconstrained two-band policy is:
   - `[0.0, 0.6313786402477035) -> dy+1`
   - `[0.9027839824973398, 1.0) -> dy+3`
   - `totalDeltaIou = 0.2802613060`
   - but it still mixes in `5` negatively affected labels

3. The strongest safe policy is much cleaner and loses almost no score:
   - `[0.0, 0.3241855029598355) -> dy+1`
   - `[0.5640732919537186, 0.6313786402477035) -> dy+1`
   - `totalDeltaIou = 0.2791453908`
   - `safe_ratio = 0.9960` relative to the best unconstrained two-band policy
   - `selectedNegativeCount = 0`

4. The safe policy covers exactly the families that already looked healthy under positive `dy`:
   - low-phase band:
     - `f5_2788` orange `S`
     - `f5_2794` orange `Ph`
     - `f5_2784` blue `CN`
     - `f2_34461` black `N`
     - `f2_37` black `O`
     - `f4_32345` / `f4_32347` black `Ph` near zero or slightly positive
   - mid-high band:
     - `f4_32325` black `N`
     - `f2_41` black `S`
     - `f4_32333` / `f4_32335` top-half catalyst black `Ph`

5. This makes the current replay family picture sharper:
   - positive-`dy` attached-label replay is not a single continuous phase family
   - the most useful packaged same-shell rule so far is a two-band `dy+1` policy
   - labels in the middle `topPagePhase ~= 0.34 ~ 0.53` region remain mixed or fragile and should not be forced into the same family

Interpretation:
- `topPagePhase` is now strong enough to produce a real, reusable rule candidate rather than just post-hoc explanation.
- The safe two-band policy is especially valuable because it preserves nearly all measured uplift while avoiding the labels that we already know regress under positive `dy`.

Next step:
- keep using `topPagePhase` as the primary axis
- compare this safe two-band replay family against the worst catalyst-top black `Ph` and nearby control labels before introducing any packaged replay experiment hook


## 2026-05-17 attached-label packaged phase-band replay experiment

Status: negative result, reverted.

Goal:
- apply the safe same-shell `topPagePhase` policy directly inside packaged replay.
- candidate policy:
  - `[0.0, 0.3241855029598355) -> dy+1`
  - `[0.5640732919537186, 0.6313786402477035) -> dy+1`

What happened:
1. First attempt showed zero visible change.
2. Trace revealed the helper never matched because attached labels use `PreviewTextRun.script = "normal"`, not `None`.
3. After fixing that gate, the helper did hit real nodes during packaged EMF generation:
   - `f2_34461`, `f2_37`, `f2_41`
   - `f4_32325`, `f4_32333`, `f4_32335`, `f4_32345`, `f4_32347`
   - `f5_2784`, `f5_2788`, `f5_2794`
4. Same-shell Word replay still got worse:
   - baseline `frame-global3`: `best_iou = 0.8618815442410679`
   - phase-band replay: `best_iou = 0.8604985618408437`

Local label outcome:
- changed labels: `5 / 27`
- all changed labels got worse:
  - `f4_32345` `Ph` `0.69697 -> 0.64925`
  - `f4_32347` `Ph` `0.62573 -> 0.58721`
  - `f2_37` `O` `0.70588 -> 0.67816`
  - `f4_32343` `Ph` `0.70588 -> 0.68382`
  - `f2_34461` `N` `0.69072 -> 0.68041`
- the high-value target labels stayed unchanged:
  - `f4_32333`, `f4_32335`
  - `f5_2784`, `f5_2788`, `f5_2794`

Interpretation:
- the same-shell `frame-dy` sensitivity buckets do not transfer directly into a packaged replay `origin.Y += 1px` rule.
- a record-time y nudge is not equivalent to replaying the same EMF under a shifted `EMR_HEADER.frame`.
- this path should stay documented as a failed direct-productization attempt.

Artifacts:
- trace log: `tmp/frame-word-ab/attached-phasebands-trace.log`
- EMF: `tmp/frame-word-ab/attached-phasebands-trace.emf`
- same-shell docx: `tmp/frame-word-ab/attached-phasebands-hit-fg3.docx`
- replay PNG: `tmp/frame-word-ab/attached-phasebands-hit-fg3.wordcopy.png`
- compare JSON: `tmp/frame-word-ab/attached-phasebands-hit-fg3.bestshift.json`
- label IoU: `tmp/frame-word-ab/attached-phasebands-hit-fg3-label-iou.json`


### 2026-05-17 same-shell catalyst top-half black Ph frame search

Summary:
- Ran same-shell Word frame search around `frame-global3 = (1441,2994,14431,8656)` for the two worst catalyst top-half black `Ph` boxes.
- Regions:
  - `ph_top_left = (458,132,488,152)`
  - `ph_top_right = (496,132,525,152)`
  - `catalyst_top_half = (450,120,525,160)`
  - `catalyst_all = (432,120,555,242)`
  - `global = (0,0,555,242)`
- Artifacts:
  - `tmp/frame-word-ab/catalyst-toplabels-ysearch.json`
  - `tmp/frame-word-ab/catalyst-toplabels-xsearch.json`

Y-only search (`left=right=0`, `top,bottom in [-3,+3]`):
- Best local family candidate was `delta = [0,0,0,+1]` (bottom +1 only).
- Metrics:
  - `global = 0.860980`
  - `ph_top_left = 0.539568`
  - `ph_top_right = 0.616541`
  - `catalyst_top_half = 0.609971`
  - `catalyst_all = 0.737078`
- Several nearby `top/bottom` combinations collapse to the same top-label IoUs, which looks like a raster/phase equivalence class rather than a unique geometric optimum.

X-only search (`top=bottom=0`, `left,right in [-3,+3]`):
- Best candidate was just the baseline `delta = [0,0,0,0]`.
- Metrics:
  - `global = 0.861882`
  - `ph_top_left = 0.514085`
  - `ph_top_right = 0.588235`
  - `catalyst_top_half = 0.576705`
  - `catalyst_all = 0.733531`
- No x-only perturbation beat the baseline for the top-half family.

Conclusion:
- For the worst catalyst top-half black `Ph` family, whole-frame vertical phase is clearly the dominant axis.
- Horizontal frame tweaks do not explain this family; they underperform the baseline while y-only tweaks give stable local gains.
- Next step should stay on the y/phase line (finer y sweep or a narrow product experiment), not x-family searching.


### 2026-05-17 node-filtered attached-label packaged y-nudge

Goal:
- Test whether a local packaged replay `y` nudge can reproduce the same-shell whole-frame y-phase improvement for the worst top-half catalyst black `Ph` labels.
- Target nodes only: `f4_32333`, `f4_32335`.

Implementation:
- Added packaged preview env hook:
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_Y_NUDGE_EXPERIMENT`
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NODE_FILTER_EXPERIMENT`
- For this experiment, `Y_NUDGE` honors the explicit node filter directly and does not require the older `Left + right-half + north + small-gap` family matcher.

Same-shell Word replay results (base frame = frame-global3):
- baseline global IoU: `0.8618815442`
- `y = -1 px`:
  - global IoU `0.8634435284`
  - `f4_32333: 0.492647 -> 0.543307`
  - `f4_32335: 0.595420 -> 0.655738`
- `y = -2 px`:
  - global IoU `0.8643932117`
  - `f4_32333: 0.492647 -> 0.582677`
  - `f4_32335: 0.595420 -> 0.710744`
- `y = -3 px`:
  - global IoU `0.8621959027`
  - `f4_32333: 0.492647 -> 0.472868`
  - `f4_32335: 0.595420 -> 0.609756`
- `y = -4 px`:
  - global IoU `0.8602348055`
  - both labels degrade
- Positive nudges:
  - `+1 / +2 px` both degrade these labels and do not help global IoU.

Conclusion:
- This is the first successful local packaged replay intervention for an attached-label family.
- The effect direction matches the same-shell frame search: negative y helps, positive y hurts.
- The family is not purely a whole-frame phenomenon; for `f4_32333/f4_32335`, record-time local vertical placement is sufficient to move Word replay in the expected direction.
- Best tested local value is currently `-2 px`.
- This does NOT generalize automatically to the other catalyst black `Ph` labels (`f4_32339`, `f4_32343`, `f4_32345`, `f4_32347`), which remained unchanged under this node-filtered run.


### 2026-05-17 safe positive-dy attached-label family

After the narrow two-node experiment succeeded, I tested a broader node-filtered family built from labels with clearly positive same-shell `frame_dy1DeltaIou`.

Candidate node set:
- `f2_41`
- `f4_32325`
- `f5_2784`
- `f2_34461`
- `f4_32335`
- `f4_32333`
- `f5_2788`
- `f5_2794`

Record-time packaged replay experiment:
- env: `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_Y_NUDGE_EXPERIMENT`
- env: `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NODE_FILTER_EXPERIMENT`
- same-shell template: `frame-global3-shellchem.docx`
- patched frame remains `frame-global3 = (1441,2994,14431,8656)`

Global same-shell Word replay results:
- baseline `frame-global3`: `0.8618815442`
- family `y = -1 px`: `0.8659826868`
- family `y = -2 px`: `0.8705740666`
- family `y = -3 px`: `0.8668643796`

Current best is therefore the family rule:
- `safe-positive-dy family -> local packaged y nudge = -2 px`

Changed labels under the best `-2 px` run:
- `f4_32333` `Ph`: `0.492647 -> 0.582677`
- `f4_32335` `Ph`: `0.595420 -> 0.710744`
- `f2_41` `S`: `0.616279 -> 0.719512`
- `f4_32325` `N`: `0.693182 -> 0.802469`
- `f5_2784` `CN`: `0.528571 -> 0.646154`
- `f5_2788` `S`: `0.521739 -> 0.674419`
- `f5_2794` `Ph`: `0.573427 -> 0.729323`

Notes:
- `f2_34461` was part of the node filter candidate list but did not move in the packaged replay output.
- No label got worse in the measured same-shell label IoU diff for the `-2 px` family run.
- This is the first broad attached-label packaged replay rule that improves both local labels and the full same-shell global IoU.

Interpretation:
- Local packaged replay y placement is now a proven lever, not just a theoretical same-shell frame proxy.
- The winning family is broader than the original two catalyst top-half black `Ph` labels, but it is still selective; it does not cover all attached labels.
- The next step is to understand why `f2_34461` stayed inert and whether the same positive-dy family can be characterized more directly than an explicit node-id list.


### 2026-05-17 attached-label local y subfamilies

Follow-up question:
- Are the positive attached-label replay candidates all explained by one local packaged `y` nudge, or do they split into subfamilies?

First boundary-bucket test (`topPagePhase ~= 0.247`):
- `f2_34461` (`N`, black) and `f2_37` (`O`, black) were tested individually.
- Results:
  - `f2_34461`
    - `y=-1`: `0.690722 -> 0.697917`, global `0.86195044`
    - `y=-2`: no change
    - `y<=-3`: degrades strongly
  - `f2_37`
    - `y=-1`: `0.705882 -> 0.738095`, global `0.86211031`
    - `y=-2`: very small gain
    - `y=-3`: degrades strongly
- Conclusion: this phase bucket behaves like a `-1 px` family, not the earlier `-2 px` family.

Two-lane packaged replay experiment:
- Lane A (`-2 px`):
  - `f2_41,f4_32325,f5_2784,f4_32335,f4_32333,f5_2788,f5_2794`
- Lane B (`-1 px`):
  - `f2_34461,f2_37`

Same-shell global result:
- previous single-lane best: `0.8705740666`
- two-lane combined best: `0.8708744881`

Changed labels under the combined run:
- Lane A:
  - `f2_41` `S`: `0.616279 -> 0.719512`
  - `f4_32325` `N`: `0.693182 -> 0.802469`
  - `f5_2784` `CN`: `0.528571 -> 0.646154`
  - `f4_32335` `Ph`: `0.595420 -> 0.710744`
  - `f4_32333` `Ph`: `0.492647 -> 0.582677`
  - `f5_2788` `S`: `0.521739 -> 0.674419`
  - `f5_2794` `Ph`: `0.573427 -> 0.729323`
- Lane B:
  - `f2_34461` `N`: `0.690722 -> 0.697917`
  - `f2_37` `O`: `0.705882 -> 0.738095`

Interpretation:
- The earlier positive attached-label family is real, but it is not monolithic.
- At least two local packaged replay y-subfamilies now exist:
  - main family: `y = -2 px`
  - boundary phase bucket around `topPagePhase ~= 0.247`: `y = -1 px`
- This is the first evidence that attached-label replay can be improved with a small multi-lane local y policy, not just one broad family constant.


### 2026-05-17 expanded attached-label y lanes

Question:
- After proving the first `-2 px` and `-1 px` attached-label y subfamilies, do the remaining high-residual catalyst black `Ph` labels form another local y lane?

Individual local y scans (`-2/-1/+1/+2/+3`) for remaining black `Ph` labels:
- `f4_32339`: best `-2/-1` tie, strongest local gain at `-2` (`+0.0530`)
- `f4_32343`: best `-1` (`+0.0553`)
- `f4_32345`: best `-1` (`+0.0152`)
- `f4_32347`: best `-1` (`+0.0022`)
- `f4_32337`: best `-1` (`+0.0139`)
- `f4_32341`: best `-1` (`+0.0153`)

Important correction:
- Even labels with negative same-shell `frame_dy1DeltaIou` can still improve under local packaged negative y.
- So whole-frame dy sensitivity and local packaged y sensitivity are related but not equivalent.

Combined two-lane packaged replay test:
- Lane A (`-2 px`):
  - `f2_41,f4_32325,f5_2784,f4_32335,f4_32333,f5_2788,f5_2794,f4_32339`
- Lane B (`-1 px`):
  - `f2_34461,f2_37,f4_32343,f4_32345,f4_32347,f4_32337,f4_32341`

Same-shell global result:
- previous two-lane best: `0.8708744881`
- expanded two-lane result: `0.8724993974`

New/updated improvements in the expanded run:
- `f4_32339` `Ph`: `0.752066 -> 0.805085`
- `f4_32343` `Ph`: `0.705882 -> 0.755556`
- `f4_32345` `Ph`: `0.696970 -> 0.712121`
- `f4_32347` `Ph`: `0.625731 -> 0.643275`
- `f4_32337` `Ph`: `0.709677 -> 0.723577`
- `f4_32341` `Ph`: `0.707965 -> 0.723214`
- plus all previously improved labels remain positive.

Current interpretation:
- Attached-label local packaged replay is now provably at least a two-lane y policy.
- Lane A (`-2 px`) covers the earlier positive family plus `f4_32339`.
- Lane B (`-1 px`) covers the phase-boundary pair (`f2_34461`,`f2_37`) and most of the remaining catalyst black `Ph` cluster.
- This is no longer a one-off per-node fix; it behaves like a real, if still narrow, family decomposition.

Open question:
- Can the current lane split be reduced from explicit node filters to a smaller geometry/phase rule using features such as `topPagePhase`, `componentQuadrant`, `primaryNeighborBucket`, `fill`, and `text`?

### 2026-05-17 attached-label x-fontscale correction

Question:
- Was the earlier attached-label x/font-scale no-op result real, or a bad experiment setup?

Key correction:
- The earlier no-op conclusion was invalid.
- `preview_attached_label_replay_nudge_px()` and `preview_attached_label_replay_font_scale()` were still gated by the narrow `preview_attached_label_replay_matches()` predicate.
- On the current full-doc dataset, that predicate naturally matches only `f4_32339`.
- So earlier scans that tried to use node filters against labels like `f4_32333`, `f4_32335`, `f5_2784`, `f5_2794` were mostly measuring a path that never fired.

Analysis hook improvement:
- Added node-filter-aware matching for:
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT`
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_EXPERIMENT`
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TEXT_HINT_EXPERIMENT`
- If `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NODE_FILTER_EXPERIMENT` is present, these axes now honor that node filter directly instead of the old narrow family predicate.

Isolated proof on a previously non-matching label (`f4_32333`):
- `x = +3 px` with node-filter targeting `f4_32333`:
  - `EmfPlusDrawString.x`: `4324.372 -> 4335.622` (`+11.25` page-space units)
  - fallback `EMR_EXTTEXTOUTW` reference `x`: `1153 -> 1156`
  - fallback bounds shifted right by `+3 px`
- `font-scale = 0.94` with node-filter targeting `f4_32333`:
  - `EmfPlusDrawString` size: `220.096 x 144.954 -> 206.890 x 136.256`
  - fallback font height: `-27 -> -25`
  - fallback bounds shrink: `right 1185 -> 1183`, `top/bottom 584..615 -> 585..612`

Conclusion:
- Attached-label local x and local font-scale are real live knobs in packaged EMF replay.
- The earlier no-op result was caused by testing labels that did not satisfy the old match predicate.
- These axes should now be treated as valid for future attached-label family experiments.

### 2026-05-17 narrow x/font-scale attached-label subfamily on top of phase3band

Question:
- After proving the `phase3band` y-policy, can the newly validated attached-label `x` and `font-scale` axes add a further same-shell gain without hurting the whole family?

Setup:
- Base replay policy: `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_PHASE_POLICY_EXPERIMENT=phase3band`
- Same-shell template: `tmp/frame-word-ab/frame-global3-shellchem.docx`
- Fixed frame: `(1441,2994,14431,8656)`
- Reference: `tmp/v28-wrapper-ablate10/v28-rerun.shape2.png`
- New experiments were run in:
  - `tmp/frame-word-ab/phase3-xfs-narrow-20260517`
  - `tmp/frame-word-ab/phase3-xfs-finetune-20260517`
  - `tmp/frame-word-ab/phase3-xfs-combos-20260517`
  - `tmp/frame-word-ab/phase3-xfs-finetune2-20260517`

Baseline:
- `phase3band`: `IoU = 0.8751508326`

Broad attached-label family matrix (all current positive-y family labels):
- `x = +1`: `0.8630814649`
- `x = +2`: `0.8462634579`
- `x = +3`: `0.8298641112`
- `x = -1`: `0.8584657141`
- `font-scale = 0.97`: `0.8731319299`
- `font-scale = 0.94`: `0.8499721138`
- `font-scale = 1.03`: `0.8544288768`

Conclusion:
- Once the node-filter-aware hooks were wired correctly, `x` and `font-scale` are real live axes.
- But when applied to the whole positive attached-label family, both axes hurt the global same-shell replay.

Narrow subfamily search:
- The first profitable narrow rule was:
  - `x = +1 px` on `f4_32333,f4_32347`
  - same-shell result: `0.8765799855`
- Adding a light font shrink to the same pair improved further:
  - `x = +1 px` on `f4_32333,f4_32347`
  - `font-scale = 0.96 ~ 0.98` on the same pair
  - same-shell result: `0.8777482484`

Local label-box effects for the best narrow variant (`xpair_fs0p96`):
- `f4_32333` `Ph`: `0.582677 -> 0.694215` (`+0.111538`)
- `f4_32347` `Ph`: `0.643275 -> 0.736196` (`+0.092921`)
- incidental neighbor gain:
  - `f4_32343` `Ph`: `0.755556 -> 0.761194` (`+0.005638`)

Interpretation:
- The profitable `x/font-scale` rule is not a broad attached-label family property.
- It is currently a very narrow top-half catalyst black-`Ph` replay subfamily.
- This rule is real but small: it adds about `+0.00260` global IoU on top of the already-strong `phase3band` baseline.
- The rest of the positive-y family should not inherit this x/font-scale tweak.

Current best layered policy stack:
1. packaged attached-label `phase3band` y-policy
2. plus narrow local `x/font-scale` rule for `f4_32333,f4_32347`

Open question:
- Can this narrow `x/font-scale` rule be promoted from an explicit node list to a stable geometric/phase predicate, or is it genuinely a tiny replay micro-family?

### 2026-05-17 narrow x/font-scale micro-family on top of phase3band

Question:
- After proving that attached-label `x` and `font-scale` are real live knobs, can they add a safe global same-shell gain on top of the `phase3band` y-policy?

Baseline:
- `phase3band` + `frame-global3`: `IoU = 0.8751508326`

Broad family test (same positive-y attached-label family used by `phase3band`):
- `x = +1`: `0.8630814649`
- `x = +2`: `0.8462634579`
- `x = +3`: `0.8298641112`
- `x = -1`: `0.8584657141`
- `font-scale = 0.97`: `0.8731319299`
- `font-scale = 0.94`: `0.8499721138`
- `font-scale = 1.03`: `0.8544288768`

Conclusion:
- `x` and `font-scale` are real, but broad application across the whole positive-y family is counterproductive.

Narrow candidate search:
- `x = +1` on `f4_32333,f4_32347`
  - `IoU = 0.8765799855`
- `x = +1` on `f4_32333,f4_32347` + `font-scale = 0.96~0.98` on the same pair
  - best stable result: `IoU = 0.8777482484`
- Local box gains for `xpair_fs0p96`:
  - `f4_32333` `Ph`: `0.582677 -> 0.694215` (`+0.111538`)
  - `f4_32347` `Ph`: `0.643275 -> 0.736196` (`+0.092921`)
  - incidental side gain:
    - `f4_32343` `Ph`: `0.755556 -> 0.761194` (`+0.005638`)

Comparison against a broader-looking geometric family:
- Candidate family `Right justification + east primary neighbor` = `f4_32333,f4_32341,f4_32347`
  - `x = +1`: `0.8753821400`
  - `x = +1` + `font-scale = 0.97`: `0.8765491711`
- Both are worse than the narrower pair-only rule.
- Single-node probes on `f4_32341` also fail to justify including it:
  - `x = +1` only on `f4_32341`: `0.8739549839`
  - `font-scale = 0.97` only on `f4_32341`: `0.8737739186`

Interpretation:
- The profitable `x/font-scale` rule is not a general attached-label family and not even the whole `Right + east` black-`Ph` group.
- It is a much narrower micro-family currently represented by:
  - `f4_32333`
  - `f4_32347`
- Shared traits of that micro-family:
  - `text = Ph`
  - `fill = #000000`
  - `cdxmlLabelJustification = Right`
  - `primaryNeighborBucket = east`
  - `gapRight �� 149.63`
  - `topPagePhase �� 0.628 / 0.308`
- Nearby control `f4_32341` shares `Right + east`, but has:
  - `gapRight �� 216.14`
  - `topPagePhase �� 0.436`
  - and does **not** benefit.

Current best layered policy stack:
1. packaged attached-label `phase3band` y-policy
2. plus narrow `x/font-scale` micro-family:
   - `x = +1 px`
   - `font-scale = 0.96~0.98`
   - only for `f4_32333,f4_32347`

Open question:
- Can this pair-only rule be promoted to a stable geometric predicate (`Right + east + moderate gapRight + specific topPagePhase bands`), or is it genuinely just a two-node replay micro-family?

### 2026-05-17 xpair + gapfamily font-scale micro-family formalization

Question:
- After the attached-label y policy (`phase3band`) and the narrow `xpair` rule were validated, can the remaining profitable `font-scale` tweak be promoted from an explicit node list to a reproducible geometric predicate?

New tooling:
- `scripts/run-attached-fs-atlas.py`
  - Runs a same-shell atlas on top of a fixed baseline (`phase3band + xpair`) and emits per-node global/local IoU deltas.
- `scripts/search-attached-microfamily.py`
  - Merges `attached-page-phase.full.json` with atlas summaries and searches simple categorical / threshold predicates.

Baseline stack before this round:
- `phase3band + frame-global3`: `IoU = 0.8751508326`
- `x = +1 px` on `f4_32333,f4_32347`: `IoU = 0.8765799855`

`font-scale = 0.97` atlas on top of `xpair`:
- Output dir:
  - `tmp/frame-word-ab/xpair-fs-atlas-20260517`
- Positive nodes:
  - `f4_32333`: global `+0.0006242983`, local `+0.0600685`
  - `f4_32347`: global `+0.0005437765`, local `+0.0392266`
  - `f4_32343`: global `+0.0004632547`, local `+0.0418363`
- All other nodes are `0` or negative.

Best stacked result so far:
- `phase3band`
- plus `x = +1` on `f4_32333,f4_32347`
- plus `font-scale = 0.97` on `f4_32333,f4_32343,f4_32347`
- same-shell full-doc result:
  - `IoU = 0.8782118405`

Predicate search results for the `x = +1` branch:
- `best_safe_min_count` exact rule:
  - `cdxmlLabelJustification == Right`
  - `gapRight <= 149.63`
- This matches exactly:
  - `f4_32333`
  - `f4_32347`

Predicate search results for the `font-scale = 0.97` branch:
- `best_safe_min_count` exact rule:
  - `137.245 <= gapRight <= 166.240`
- This matches exactly:
  - `f4_32333`
  - `f4_32343`
  - `f4_32347`

Interpretation:
- The attached-label `x` and `font-scale` gains are no longer just hand-picked node lists.
- They can now be described as two narrow replay micro-families:
  - `x-family`: `Right-justified` labels with `gapRight <= 149.63`
  - `font-scale family`: labels with `137.245 <= gapRight <= 166.240`
- In the current full-doc same-shell oracle, these two rules are the best verified local replay refinements on top of `phase3band`.

Open questions:
- Can the two `gapRight` interval rules be made more interpretable by combining them with `topPagePhase` or local attachment geometry?
- Do these rules survive on other fixtures beyond the current `thiocyanation` full-doc oracle?
### 2026-05-17 x-family safe third-node extension on top of gapfamily baseline

Question:
- Once the best stacked replay policy became
  - `phase3band`
  - `x = +1` on `f4_32333,f4_32347`
  - `font-scale = 0.97` on `f4_32333,f4_32343,f4_32347`
  can the `x` family be extended by one more safe node without hurting the existing gains?

Experiment:
- Added `f4_32327` into the `x = +1` filter while keeping the same `font-scale` gapfamily unchanged.
- Same-shell template:
  - `tmp/frame-word-ab/frame-global3-shellchem.docx`
- Fixed frame:
  - `(1441,2994,14431,8656)`

Result:
- Previous best stacked result:
  - `IoU = 0.8782118405`
- New `xtriplet + gapfamily-fs` result:
  - `IoU = 0.8783631384`
- Absolute gain:
  - `+0.0001512979`

Local effect:
- `f4_32327` improved from:
  - `0.662921348`
  to:
  - `0.681818182`
  (`+0.018896834`)
- `f4_32333`, `f4_32343`, `f4_32347` stayed unchanged at their already-improved values.

Interpretation:
- `f4_32327` is a real positive carry-on node on top of the previous best stacked baseline.
- The earlier x-only predicate search already hinted at a stronger 3-node rule than the exact pair-only rule.
- This same-shell validation confirms that the `x` replay family is not strictly a 2-node pair anymore.

Current best verified stacked policy:
- `phase3band`
- `x = +1` on:
  - `f4_32327`
  - `f4_32333`
  - `f4_32347`
- `font-scale = 0.97` on:
  - `f4_32333`
  - `f4_32343`
  - `f4_32347`
- same-shell full-doc result:
  - `IoU = 0.8783631384`

Open question:
- Can the 3-node `x` family be promoted to a stable predicate such as a `gapRight + xPagePhase` band, rather than remaining an explicit node set?
### 2026-05-17 x-family extension atlas on top of gapfamily baseline

Question:
- After promoting the `font-scale` branch to the 3-node `gapRight` family, can the `x = +1` branch safely grow beyond the original pair `f4_32333,f4_32347`?

New tooling:
- `scripts/run-attached-x-atlas.py`
  - Runs a same-shell atlas over single-node `x = +1` additions on top of a fixed stacked baseline.

Baseline stack for this atlas:
- `phase3band`
- `x = +1` on `f4_32333,f4_32347`
- `font-scale = 0.97` on `f4_32333,f4_32343,f4_32347`
- same-shell full-doc baseline:
  - `IoU = 0.8782118405`

Atlas result:
- Output dir:
  - `tmp/frame-word-ab/xtriplet-fs-gapfamily-xatlas-20260517`
- The only additional positive node is:
  - `f4_32327`
    - global `+0.0001512979`
    - local label `+0.018896834`
- `f4_32333` and `f4_32347` remain neutral because they are already in the baseline x-family.
- All remaining nodes are negative.

New best stacked result:
- `phase3band`
- `x = +1` on:
  - `f4_32327`
  - `f4_32333`
  - `f4_32347`
- `font-scale = 0.97` on:
  - `f4_32333`
  - `f4_32343`
  - `f4_32347`
- same-shell full-doc result:
  - `IoU = 0.8783631384`

Interpretation:
- The `x` replay family can be extended by one more node beyond the original pair, but only narrowly.
- Once `f4_32327` is included, the single-node x atlas shows no other profitable additions.
- So the current evidence says the x-family has effectively saturated at a 3-node set.

Updated practical rule set:
- `x-family` current best node set:
  - `f4_32327`
  - `f4_32333`
  - `f4_32347`
- `font-scale family` current best rule:
  - `137.245 <= gapRight <= 166.240`
  - which matches `f4_32333,f4_32343,f4_32347`

Open question:
- Can the 3-node x-family itself be promoted to a clean geometric predicate (for example a `gapRight + xPagePhase` band), or is `f4_32327` still an isolated carry-on case?### 2026-05-17 y-atlas on top of the current best stacked baseline

Question:
- After fixing the current best stacked baseline (`phase3band + xtriplet + gapfamily-fs`), do any additional local `y` nudge families still exist outside the built-in `phase3band` policy?

New tooling:
- `scripts/run-attached-y-atlas.py`
  - Runs a same-shell atlas of single-node packaged `y` nudges on top of a stacked replay baseline.
  - Supports the same baseline stack as the x/font-scale atlases:
    - `phase-policy`
    - baseline `x-family`
    - baseline `font-scale family`

Experiments:
- `y = -1 px`
  - output dir:
    - `tmp/frame-word-ab/xtriplet-fs-gapfamily-yneg1-atlas-20260517`
- `y = -2 px`
  - output dir:
    - `tmp/frame-word-ab/xtriplet-fs-gapfamily-yneg2-atlas-20260517`

Baseline stack:
- `phase3band`
- `x = +1` on:
  - `f4_32327`
  - `f4_32333`
  - `f4_32347`
- `font-scale = 0.97` on:
  - `f4_32333`
  - `f4_32343`
  - `f4_32347`
- same-shell full-doc baseline:
  - `IoU = 0.8783631384`

Results:
- For both `y = -1` and `y = -2` atlases:
  - `positive_count = 0`
  - `max_delta = 0.0`
- No node produced a positive global delta beyond the current baseline.

Interpretation:
- Under the current best stacked policy, the existing `phase3band` has already saturated the remaining profitable local `y` replay space.
- The attached-label replay problem no longer looks like an unmodeled `y`-family problem.
- Further gains are more likely to come from:
  - secondary `x` families
  - additional font-scale microfamilies
  - or a different replay knob altogether

### 2026-05-17 dual-x atlas on top of the current best stacked baseline

Question:
- Does a second `x` replay family still exist if we keep the current best `x = +1` family as the baseline and search for an additional candidate family independently?

Tooling refinement:
- `renderer.rs`
  - added a second experimental `x` replay channel, symmetric to the existing dual `y` channels:
    - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT_2`
    - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT_2`
- `scripts/run-attached-x-atlas.py`
  - now supports:
    - `--baseline-x-nudge`
    - baseline `x-family` + candidate `x-family` as two independent channels

Experiment:
- baseline stack:
  - `phase3band`
  - baseline `x = +1` on:
    - `f4_32327`
    - `f4_32333`
    - `f4_32347`
  - `font-scale = 0.97` on:
    - `f4_32333`
    - `f4_32343`
    - `f4_32347`
- candidate atlas:
  - `x = -1 px`
- output dir:
  - `tmp/frame-word-ab/xtriplet-fs-gapfamily-xdualneg1-atlas-20260517`

Result:
- One positive singleton remains:
  - `f1_28331`
    - `globalIou = 0.8785754573`
    - `globalDelta = +0.0002123189`
    - `labelDelta = +0.0214324968`
- All other nodes are non-positive.

Current best stacked policy therefore becomes:
- `phase3band`
- `x = +1` on:
  - `f4_32327`
  - `f4_32333`
  - `f4_32347`
- `x = -1` on:
  - `f1_28331`
- `font-scale = 0.97` on:
  - `f4_32333`
  - `f4_32343`
  - `f4_32347`
- same-shell full-doc result:
  - `IoU = 0.8785754573`

Microfamily search on the dual-x atlas:
- output:
  - `tmp/frame-word-ab/xtriplet-fs-gapfamily-xdualneg1-atlas-20260517/microfamily-search.json`
- best safe minimum rule is still effectively singleton:
  - `text == N`
  - `componentQuadrant == LT`
  - matches exactly:
    - `f1_28331`

Interpretation:
- The old belief that the `x-family` had fully saturated at three `x = +1` nodes was incomplete.
- There is a second, much narrower `x = -1` microfamily, but it is currently only supported by one profitable node.
- So the replay stack is now better described as:
  - primary positive `x = +1` family
  - one narrow negative `x = -1` singleton carry-on
  - saturated `phase3band` on `y`

### 2026-05-17 true-best baseline saturation of x / y / font-scale microfamilies

Context correction:
- The earlier `x/y/font-scale` atlases were not all evaluated on the *true* current best stacked baseline.
- Once we fixed the baseline to include both:
  - primary `x = +1` family on:
    - `f4_32327`
    - `f4_32333`
    - `f4_32347`
  - secondary `x = -1` singleton on:
    - `f1_28331`
  - plus `font-scale = 0.97` on:
    - `f4_32333`
    - `f4_32343`
    - `f4_32347`
- the same-shell full-doc baseline became:
  - `IoU = 0.8785754573`

Tooling refinement:
- `renderer.rs`
  - added a third experimental `x` replay channel:
    - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT_3`
    - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT_3`
- `scripts/run-attached-fs-atlas.py`
  - now supports:
    - `--baseline-x2-filter`
    - `--baseline-x2-nudge`
- `scripts/run-attached-y-atlas.py`
  - now supports:
    - `--baseline-x2-filter`
    - `--baseline-x2-nudge`
- `scripts/run-attached-x-atlas.py`
  - now supports:
    - baseline `x-family #1`
    - baseline `x-family #2`
    - candidate `x-family #3`

#### font-scale atlas on the true best baseline

Experiment:
- output:
  - `tmp/frame-word-ab/xtriplet-xneg1-fsplus097-atlas-20260517`

Result:
- `positive_count = 0`
- No node produced a positive global delta beyond the current best stacked baseline.

Interpretation:
- The existing `font-scale = 0.97` family has saturated the profitable `font-scale` microfamily space under the true current best baseline.

#### y-atlas on the true best baseline

Experiment:
- output:
  - `tmp/frame-word-ab/xtriplet-xneg1-fs-gapfamily-yneg1-atlas-20260517`

Result:
- `positive_count = 0`
- No node produced a positive global delta beyond the current best stacked baseline.

Interpretation:
- The existing `phase3band` has saturated the profitable local `y` replay space under the true current best baseline.

#### third x-family atlas on the true best baseline

Experiments:
- candidate `x = +1`
  - `tmp/frame-word-ab/xtriplet-xneg1-fs-gapfamily-xplus1-third-atlas-20260517`
- candidate `x = -1`
  - `tmp/frame-word-ab/xtriplet-xneg1-fs-gapfamily-xneg1-third-atlas-20260517`
  - tail rerun:
    - `tmp/frame-word-ab/xtriplet-xneg1-fs-gapfamily-xneg1-third-tail-20260517`
- candidate `x = +2`
  - `tmp/frame-word-ab/xtriplet-xneg1-fs-gapfamily-xplus2-third-atlas-20260517`
- candidate `x = -2`
  - `tmp/frame-word-ab/xtriplet-xneg1-fs-gapfamily-xneg2-third-atlas-20260517`

Result:
- All four third-family atlases produced:
  - `positive_count = 0`
- No additional profitable third `x` family exists at `+1`, `-1`, `+2`, or `-2` under the current best stacked baseline.

#### magnitude check for the current x families

One-off variants tested:
- `best_current`
  - `xtriplet = +1`
  - `xsingleton = -1`
  - `font-scale = 0.97`
- `xtriplet_plus2`
- `xsingleton_neg2`
- `xsingleton_neg3`
- `xtriplet0`

Result:
- `best_current`: `0.8785754572556603`
- `xtriplet_plus2`: `0.8752413127413128`
- `xsingleton_neg2`: `0.8772749234981478`
- `xsingleton_neg3`: `0.8761567554518387`
- `xtriplet0`: `0.8758248833091904`

Interpretation:
- The current magnitudes are locally better than these simple alternatives.
- At this point the attached-label replay stack is effectively saturated along the three microfamily axes already explored:
  - `x`
  - `y`
  - `font-scale`

Current best stacked policy remains:
- `phase3band`
- `x = +1` on:
  - `f4_32327`
  - `f4_32333`
  - `f4_32347`
- `x = -1` on:
  - `f1_28331`
- `font-scale = 0.97` on:
  - `f4_32333`
  - `f4_32343`
  - `f4_32347`
- same-shell full-doc result:
  - `IoU = 0.8785754573`

Takeaway:
- Further gains are unlikely to come from more local `x/y/font-scale` microfamily mining.
- The next profitable direction should switch to a different replay knob or a different family decomposition.

## 2026-05-17 attached-label text-hint atlas on the true best baseline

Question:
- After `phase3band + x(+1/-1) + font-scale=0.97` became the current best same-shell replay stack, is there still any profitable attached-label `TextRenderingHint` family?

Baseline stack before this round:
- `phase3band`
- `x = +1` on:
  - `f4_32327`
  - `f4_32333`
  - `f4_32347`
- `x = -1` on:
  - `f1_28331`
- `font-scale = 0.97` on:
  - `f4_32333`
  - `f4_32343`
  - `f4_32347`
- baseline same-shell full-doc result:
  - `IoU = 0.8785754573`

Experiment:
- Added dedicated node-filter support for:
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TEXT_HINT_NODE_FILTER_EXPERIMENT`
- Added atlas runner:
  - `scripts/run-attached-hint-atlas.py`
- Output:
  - `tmp/frame-word-ab/hint-atlas-combined.json`

Result:
- All packaged attached-label hint candidates `0..5` had:
  - `positive_count = 0`
- For every node, the best safe hint remained:
  - `hint = 4`
  - `globalDelta = 0`

Interpretation:
- Attached-label replay residuals on the current best baseline are **not** driven by packaged `TextRenderingHint`.
- This axis is saturated / non-productive for the current oracle.

Takeaway:
- `hint` should be removed from the active hypothesis set.
- The next promising axis should be a different replay knob, not more hint tuning.

## 2026-05-17 attached-label top-nudge families on the true best baseline

Question:
- After `x / y / font-scale / hint` microfamilies saturated, can a more direct packaged vertical-placement knob improve same-shell replay?

New packaged replay hook:
- Added packaged attached-label `top` placement override:
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_EXPERIMENT`
  - `CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_NODE_FILTER_EXPERIMENT`
- Later extended to:
  - `_2`
  - `_3`
  so multiple top-nudge families can be stacked in the same replay.

New tooling:
- `scripts/run-attached-top-atlas.py`
- `scripts/search-attached-top-policy.py`
- Summary output:
  - `tmp/frame-word-ab/top-family-summary-20260517.json`
- Combined atlas:
  - `tmp/frame-word-ab/top-atlas-combined.json`
- Phase-band search:
  - `tmp/frame-word-ab/top-policy-search-20260517.json`

Baseline stack before top-nudge:
- same-shell full-doc:
  - `IoU = 0.8785754573`

### Single-family same-shell validation

Direct same-shell validation on the full doc (not just atlas deltas):

- `top = -2 px` on:
  - `f5_2784`
  - `f4_32321`
  - `f5_2788`
  - `f4_32323`
  - `f5_2794`
  - `f2_34464`
  - `f4_32345`
  - `f4_32329`
  - `f4_32331`
  - `f1_28322`
  - `f4_32337`
  - `f4_32347`
  - result:
    - `IoU = 0.8813422602`

- `top = +2 px` on:
  - `f2_41`
  - `f4_32335`
  - `f4_32333`
  - result:
    - `IoU = 0.8792130302`

- `top = +1 px` on:
  - `f2_43`
  - `f2_37`
  - result:
    - `IoU = 0.8787878788`

Interpretation:
- `top = -2 px` is a strong new family, materially better than baseline.
- `top = +2 px` and `top = +1 px` are both weaker alone, but still positive.

### Stacked top-nudge validation

Two-family stack:
- `top = -2 px` family
- plus `top = +2 px` family
- result:
  - `IoU = 0.8819825638`

Three-family stack:
- `top = -2 px` family
- plus `top = +2 px` family
- plus `top = +1 px` family
- result:
  - `IoU = 0.8821962051`

This is the best same-shell result reached so far on this branch.

Compared to the previous best stacked baseline:
- `0.8785754573 -> 0.8821962051`
- absolute gain:
  - `+0.0036207478`

Label-box attribution for the three-family stack:
- Positive local gains include:
  - `f5_2784` `CN`: `+0.096034`
  - `f2_41` `S`: `+0.056804`
  - `f5_2788` `S`: `+0.036425`
  - `f4_32321` `NC`: `+0.032678`
  - `f4_32323` `CN`: `+0.021139`
  - `f2_34464` `O`: `+0.019277`
  - `f2_43` `O`: `+0.016714`
  - `f5_2794` `Ph`: `+0.015038`
  - `f4_32335` `Ph`: `+0.011945`
  - `f4_32329` `N`: `+0.011905`
  - `f4_32331` `N`: `+0.010989`
  - `f4_32345` `Ph`: `+0.010956`
  - `f1_28322` `N`: `+0.009709`
  - `f2_37` `O`: `+0.008893`
- Observed label losses:
  - none

So this is not a fragile single-label tweak; it is a real multi-family replay improvement.

### Phase-band search versus explicit node families

Using `top-atlas-combined.json` plus `attached-page-phase.full.json`, I searched compact `topPagePhase` band policies.

Best safe one-band policy:
- `[0.0, 0.3241855) -> top = -1`
- summed atlas delta:
  - `0.001694617`

Best safe three-band policy from atlas:
- `[0.0, 0.3241855) -> -1`
- `[0.4201406, 0.5640733) -> -1` or `-2`
- `[0.6056385, 0.6313786) -> +2`
- summed atlas delta:
  - `0.003018076`

But same-shell validation shows that these compact phase-band families still underperform the explicit node-family stack:

- predicate-based `m2 + p2`:
  - `IoU = 0.8811329890`
- predicate-based `m2 + p2 + p1(f2_37)`:
  - `IoU = 0.8812040997`
- explicit node-family stack:
  - `IoU = 0.8821962051`

Interpretation:
- `topPagePhase` is useful, but not yet sufficient by itself.
- The current strongest attached-label replay policy is still the explicit 3-family node stack.
- The next step is not to abandon predicates, but to search for richer `top` families that combine:
  - `topPagePhase`
  - `gapRight`
  - `fill`
  - `text`
  - `quadrant / neighbor`

Current best verified stacked policy on the branch:
- `phase3band`
- `x = +1` on:
  - `f4_32327`
  - `f4_32333`
  - `f4_32347`
- `x = -1` on:
  - `f1_28331`
- `font-scale = 0.97` on:
  - `f4_32333`
  - `f4_32343`
  - `f4_32347`
- `top = -2 px` on:
  - `f5_2784`
  - `f4_32321`
  - `f5_2788`
  - `f4_32323`
  - `f5_2794`
  - `f2_34464`
  - `f4_32345`
  - `f4_32329`
  - `f4_32331`
  - `f1_28322`
  - `f4_32337`
  - `f4_32347`
- `top = +2 px` on:
  - `f2_41`
  - `f4_32335`
  - `f4_32333`
- `top = +1 px` on:
  - `f2_43`
  - `f2_37`
- same-shell full-doc result:
  - `IoU = 0.8821962051`

## 2026-05-17 richer top-family predicates can now reproduce the explicit node stack

Question:
- Can the current best attached-label `top` families be expressed as a small set of reusable predicates, instead of an explicit node list?

New tooling:
- `scripts/search-attached-top-microfamilies.py`
  - action-aware microfamily search over:
    - categorical:
      - `text`
      - `fill`
      - `componentQuadrant`
      - `cdxmlLabelJustification`
      - `primaryNeighborBucket`
    - numeric:
      - `gapRight`
      - `topPagePhase`
      - `xPagePhase`
      - `baselineTopPxPhase`

Outputs:
- `tmp/frame-word-ab/top-microfamily-m2.json`
- `tmp/frame-word-ab/top-microfamily-p2.json`
- `tmp/frame-word-ab/top-microfamily-p1.json`

### Best safe microfamilies per action (atlas space)

`top = -2`
- strongest safe predicate:
  - `gapRight >= 40.445`
  - `gapRight <= 149.630`
  - `topPagePhase <= 0.404148`
- atlas total delta:
  - `0.0017751908`
- matched nodes:
  - `f1_28322`
  - `f4_32329`
  - `f4_32331`
  - `f4_32345`
  - `f4_32347`
  - `f5_2784`
  - `f5_2788`

This is not yet the full explicit `-2` family, but it captures the dominant core.

`top = +2`
- best safe predicate:
  - `gapRight <= 180.990`
  - `topPagePhase >= 0.605638`
- atlas total delta:
  - `0.0007789475`
- matched nodes:
  - `f2_41`
  - `f2_43`
  - `f4_32333`
  - `f4_32335`

Because this pulls in `f2_43`, it is useful but not yet identical to the explicit `+2` stack.

`top = +1`
- best safe compact predicate:
  - `text == O`
  - `gapRight <= 180.990`
- atlas total delta:
  - `0.0002123987`
- matched nodes:
  - `f2_37`
  - `f2_43`

This exactly matches the previously validated explicit `+1` family.

### Two-predicate recovery for `top = -2`

The next step was to search whether the missing `-2` nodes can be recovered by a second safe predicate.

Best two-predicate `-2` union found in atlas space:

- predicate A:
  - `gapRight <= 149.630`
  - `topPagePhase <= 0.532088`
  - `xPagePhase <= 0.636529`
- predicate B:
  - `gapRight <= 180.750`
  - `topPagePhase <= 0.532088`
  - `xPagePhase >= 0.716492`

Union nodes:
- `f1_28322`
- `f4_32321`
- `f4_32323`
- `f4_32329`
- `f4_32331`
- `f4_32337`
- `f4_32345`
- `f4_32347`
- `f5_2784`
- `f5_2788`
- `f5_2794`

This still misses only:
- `f2_34464`

### Singleton补丁：把 `f2_34464` 规则化

继续对剩余 `f2_34464` 做最小 safe predicate 搜索，得到：

- `componentQuadrant == LB`
- `gapRight >= 225.110`

This picks exactly:
- `f2_34464`

and no extra safe positives.

### Same-shell validation

With these predicate-derived node sets, same-shell validation on the full document gives:

1. Broad predicate stack:
- `top=-2`: 11-node union above
- `top=+2`: `f2_41,f2_43,f4_32333,f4_32335`
- `top=+1`: `f2_37,f2_43`
- result:
  - `IoU = 0.8812040997`

2. Predicate stack with singleton `f2_34464`补齐 + explicit-safe `+2/+1`
- `top=-2`:
  - `f1_28322`
  - `f2_34464`
  - `f4_32321`
  - `f4_32323`
  - `f4_32329`
  - `f4_32331`
  - `f4_32337`
  - `f4_32345`
  - `f4_32347`
  - `f5_2784`
  - `f5_2788`
  - `f5_2794`
- `top=+2`:
  - `f2_41`
  - `f4_32333`
  - `f4_32335`
- `top=+1`:
  - `f2_37`
  - `f2_43`
- result:
  - `IoU = 0.8821962051`

This is exactly the same as the previous explicit-node optimum.

Interpretation:
- The current best attached-label `top` policy is no longer just a hand-picked node list.
- It can now be reproduced by:
  - one compact `+1` predicate
  - one compact `+2` predicate
  - one 2-piece `-2` family plus one singleton patch
- In other words, `top` has crossed from “node tuning” into “rule family”.

Takeaway:
- The `top` axis is now largely understood.
- The next profitable direction is not more brute-force top-nudge search, but either:
  - compressing the remaining singleton patch further, or
  - moving to the next unsolved replay axis (`x` family / non-text residual).

## 2026-05-18 x-family under the current best top stack

Question:
- After the `top` families were promoted into rule form, can the attached-label `x` axis also be compressed from node lists into reusable predicates on top of the current best top stack?

Baseline for this round:
- `top = -2 px` on:
  - `f1_28322`
  - `f2_34464`
  - `f4_32321`
  - `f4_32323`
  - `f4_32329`
  - `f4_32331`
  - `f4_32337`
  - `f4_32345`
  - `f4_32347`
  - `f5_2784`
  - `f5_2788`
  - `f5_2794`
- `top = +2 px` on:
  - `f2_41`
  - `f4_32333`
  - `f4_32335`
- `top = +1 px` on:
  - `f2_37`
  - `f2_43`
- `font-scale = 0.97` on:
  - `f4_32333`
  - `f4_32343`
  - `f4_32347`

Atlas runs:
- `tmp/frame-word-ab/x-atlas-on-topstack-plus1-20260517`
- `tmp/frame-word-ab/x-atlas-on-topstack-neg1-20260518`

### `x = +1` branch

Top positive nodes:
- `f4_32347`: `globalDelta = +0.0013838331`
- `f4_32333`: `+0.0013031619`
- `f4_32327`: `+0.0001515193`

Everything else is already non-positive on this baseline.

Best safe predicate search result:
- `gapRight >= 121.165`
- `gapRight <= 180.670`
- `xPagePhase >= 0.209741`

This matches exactly:
- `f4_32327`
- `f4_32333`
- `f4_32347`

and no negative members.

Score:
- `totalDeltaIou = 0.0028385143`
- `neg = 0`

Interpretation:
- Under the true best `top` stack, the positive `x = +1` family is no longer just a hand-picked 3-node set.
- It now has a clean safe predicate.

### `x = -1` branch

Atlas result:
- only `f1_28331` remains positive
  - `globalDelta = +0.0002126957`
- every other candidate is negative

Best safe compact predicate search result:
- `text == N`
- `componentQuadrant == LT`

This matches exactly:
- `f1_28331`

Interpretation:
- The negative `x` branch still behaves like a singleton carry-on rather than a broader family.
- There is no evidence on the current top-stacked baseline for a second reusable `x = -1` microfamily.

### Updated conclusion

The attached-label `x` axis is now understood as:
- one real positive family:
  - `x = +1` when
    - `121.165 <= gapRight <= 180.670`
    - `xPagePhase >= 0.209741`
- one remaining negative singleton:
  - `x = -1` on `f1_28331`

This means the `x` axis has largely crossed into rule form, but not completely:
- the positive branch is now reproducible by predicate
- the negative branch is still effectively a singleton patch

Takeaway:
- If we keep pushing replay rule formalization, the next best target is **not** more `x = +1` atlas mining.
- The profitable next step is either:
  - compressing the remaining `x = -1` singleton further,
  - or switching to the next unsolved replay family outside the current `x/y/font-scale/top` stack.

## 2026-05-18 replay stack is now almost fully expressible as predicates

Question:
- After re-running the `x` atlases on top of the current best `top` stack, can the attached-label replay stack be rewritten mostly as rules rather than hand-picked node sets?

New atlas runs:
- `tmp/frame-word-ab/x-atlas-on-topstack-plus1-20260517`
- `tmp/frame-word-ab/x-atlas-on-topstack-neg1-20260518`

### `x = +1` is now an exact safe predicate

Under the current best `top` stack + `font-scale` baseline, the positive `x = +1` atlas gives only three strictly positive nodes:
- `f4_32347`
- `f4_32333`
- `f4_32327`

Best safe predicate search result:
- `gapRight >= 121.165`
- `gapRight <= 180.670`
- `xPagePhase >= 0.209741`

This matches exactly:
- `f4_32327`
- `f4_32333`
- `f4_32347`

with:
- `totalDeltaIou = 0.0028385143`
- `neg = 0`

Interpretation:
- The positive `x` family is no longer just a 3-node tuning set.
- It has crossed into a compact reusable rule.

### `x = -1` is still a singleton carry-on

The negative `x = -1` atlas remains sharply peaked:
- only `f1_28331` is positive
  - `globalDelta = +0.0002126957`
- every other candidate is negative

Best safe compact predicate search result:
- `text == N`
- `componentQuadrant == LT`

This matches exactly:
- `f1_28331`

Interpretation:
- The negative `x` branch is still a singleton microfamily.
- It is compact enough to describe by rule, but not yet a broader reusable family.

### Revisiting `top = +2`: there is also an exact compact rule

Earlier notes emphasized the broader safe predicate:
- `gapRight <= 180.990`
- `topPagePhase >= 0.605638`

which also pulled in `f2_43`.

Rechecking the same atlas shows an exact safe 3-node variant already exists:
- `gapRight <= 149.630`
- `topPagePhase >= 0.605638`

This matches exactly:
- `f2_41`
- `f4_32333`
- `f4_32335`

and excludes:
- `f2_43`

So the `top = +2` family is cleaner than previously stated.

### Current best stack in rule form

The current best attached-label replay stack can now be written almost entirely as predicates:

- `font-scale = 0.97`
  - `137.245 <= gapRight <= 166.240`

- `x = +1`
  - `121.165 <= gapRight <= 180.670`
  - `xPagePhase >= 0.209741`

- `x = -1`
  - `text == N`
  - `componentQuadrant == LT`

- `top = +1`
  - `text == O`
  - `gapRight <= 180.990`

- `top = +2`
  - `gapRight <= 149.630`
  - `topPagePhase >= 0.605638`

- `top = -2`
  - predicate A:
    - `gapRight <= 149.630`
    - `topPagePhase <= 0.532088`
    - `xPagePhase <= 0.636529`
  - predicate B:
    - `gapRight <= 180.750`
    - `topPagePhase <= 0.532088`
    - `xPagePhase >= 0.716492`
  - singleton patch:
    - `componentQuadrant == LB`
    - `gapRight >= 225.110`

Interpretation:
- The replay stack is no longer dominated by ad-hoc node lists.
- What still remains node-like is mainly the semantic role of the `x = -1` singleton, not its ability to be described compactly.

Takeaway:
- A large part of the current best same-shell replay policy has now crossed from “node tuning” into “rule stack”.
- The next profitable direction is not more atlas brute force on these same axes, but either:
  - compressing the remaining semantic singleton(s) further,
  - or switching to the next unsolved family outside the current attached-label microfamily stack.

## 2026-05-18 current best rule-stack residual attribution

Question:
- After rewriting most of the attached-label replay stack into predicates, what still dominates the same-shell residual against the `v28-rerun.shape2.png` oracle?

Replay setup:
- Start from the current best attached-label replay stack:
  - `font-scale = 0.97` when `137.245 <= gapRight <= 166.240`
  - `x = +1` when `121.165 <= gapRight <= 180.670` and `xPagePhase >= 0.209741`
  - `x = -1` when `text == N && componentQuadrant == LT`
  - `top = +1` when `text == O && gapRight <= 180.990`
  - `top = +2` when `gapRight <= 149.630 && topPagePhase >= 0.605638`
  - `top = -2` using the two compact predicates plus the `LB + gapRight >= 225.110` singleton patch
- Export a fresh raw docx/EMF.
- Patch that EMF into the `frame-global3-shellchem.docx` shell.
- Patch `EMR_HEADER.frame` to the current best same-shell frame:
  - `(1441, 2994, 14431, 8656)`
- Use Word `CopyAsPicture` and re-run same-shell best-shift / residual attribution.

Outputs:
- `tmp/frame-word-ab/current-best-rule-stack-20260518/fg3.docx`
- `tmp/frame-word-ab/current-best-rule-stack-20260518/wordcopy.fg3.png`
- `tmp/frame-word-ab/current-best-rule-stack-20260518/bestshift.fg3.json`
- `tmp/frame-word-ab/current-best-rule-stack-20260518/label-attribution.fg3.json`
- `tmp/frame-word-ab/current-best-rule-stack-20260518/molecule-partition.fg3.json`

Headline result:
- `best_iou = 0.8821962051`
- `dx = -1`
- `dy = 0`

Interpretation:
- The predicate stack is not just a local label fix. Under the current best shell/frame, it becomes the best known same-shell Word replay result so far.
- The replay is now good enough that residual attribution can be trusted again without the previous “wrong shell / wrong frame” ambiguity.

### Dominant residual is still molecule-internal labels

Same-shell residual totals:
- `residualPixelCount = 1459`
- `labelUnionResidualCount = 709`
- `componentUnionResidualCount = 933`
- `componentNonLabelResidualCount = 224`

Interpretation:
- Roughly half of the remaining residual still lands directly inside label boxes.
- Most of the component-side residual is still label-driven rather than skeleton-driven.

### Top residual labels under the current best stack

Largest `label-attribution.fg3.json` entries:
- `f4_32321` `NC` (black, attached-group): `41`
- `f4_32333` `Ph` (black, attached-group): `36`
- `f4_32345` `Ph` (black, attached-group): `36`
- `f4_32347` `Ph` (black, attached-group): `36`
- `f5_2794` `Ph` (orange, attached-group): `34`
- `f4_32335` `Ph` (black, attached-group): `33`
- `f4_32337` `Ph` (black, attached-group): `33`
- `f5_2784` `CN` (blue, attached-group): `33`
- `f4_32341` `Ph` (black, attached-group): `31`
- `f1_28328` `O` (black, attached-group): `30`
- `f4_32323` `CN` (black, attached-group): `30`
- `f2_34461` `N` (black, attached-group): `29`

Interpretation:
- Even after the predicate stack, the heaviest unresolved family is still catalyst / product attached-group text.
- The next productive analysis step should not return to external centered text; it should continue down these molecule-internal label families.

### Component breakdown after the current best stack

From `molecule-partition.fg3.json`:
- `bottom_right_catalyst_component`
  - `residual = 395`
  - `label = 384`
  - `non-label = 11`
- `bottom_left_ligand_component`
  - `residual = 162`
  - `label = 110`
  - `non-label = 52`
- `top_right_product_component`
  - `residual = 160`
  - `label = 91`
  - `non-label = 69`
- `bottom_center_reagent_component`
  - `residual = 153`
  - `label = 124`
  - `non-label = 29`
- `top_left_substrate_component`
  - `residual = 63`
  - `label = 0`
  - `non-label = 63`

Interpretation:
- `bottom_right_catalyst_component` remains the dominant unresolved component, and it is almost purely a label problem now.
- `top_right_product_component` and `bottom_center_reagent_component` are still mixed but label-heavy.
- `top_left_substrate_component` is now clearly a non-label replay problem and should be tracked separately from the attached-label line.

Takeaway:
- The current best predicate stack has pushed the same-shell Word replay to `0.882196`, but it has not flattened the label problem.
- The dominant residual has now cleanly narrowed to:
  1. catalyst attached-group labels,
  2. product/reagent attached-group labels,
  3. a smaller non-label substrate replay residual.

## 2026-05-18 current best stack saturation probe

Question:
- Once the attached-label replay stack reaches `IoU = 0.882196`, are the remaining top residual labels still improvable by the already-known local action axes (`x`, `top`, `font-scale`) one node at a time?

New tooling:
- `scripts/probe-attached-actions-on-stack.py`
  - Takes a fixed same-shell replay baseline plus a list of target nodes and actions.
  - Rebuilds a probe variant for each `(node, action)` pair and reports:
    - global best-shift IoU delta
    - local label-box IoU delta

Baseline stack under test:
- `font-scale = 0.97` when `137.245 <= gapRight <= 166.240`
- `x = +1` when `121.165 <= gapRight <= 180.670` and `xPagePhase >= 0.209741`
- `x = -1` when `text == N && componentQuadrant == LT`
- `top = +1` when `text == O && gapRight <= 180.990`
- `top = +2` when `gapRight <= 149.630 && topPagePhase >= 0.605638`
- `top = -2` using the two compact predicates plus the `LB + gapRight >= 225.110` singleton patch
- same-shell frame:
  - `(1441, 2994, 14431, 8656)`

Probe targets:
- Top residual labels from `label-attribution.fg3.json`:
  - `f4_32321`
  - `f4_32345`
  - `f5_2794`
  - `f4_32335`
  - `f4_32337`
  - `f5_2784`
  - `f4_32341`
  - `f1_28328`
  - `f4_32323`
  - `f2_34461`

Probe actions:
- `x:+1`
- `x:-1`
- `top:+2`
- `top:-2`
- `font:0.97`

Outputs:
- `tmp/frame-word-ab/probe-current-best-20260518/summary.json`

Headline result:
- Almost the entire matrix is already saturated.
- The only positive global delta found in this first pass is:
  - `f1_28328` with `top:-2`
  - `globalDelta = +0.0000997504`
  - `labelDelta = +0.0161224490`

Representative outcomes:
- `f4_32321`
  - `x:+1`: global `-0.0016606`, local `-0.1229`
  - `x:-1`: global `-0.0011823`, local `-0.1001`
  - `top:+2 / top:-2 / font:0.97`: exact no-op
- `f4_32345`
  - `x:+1`: global `-0.0017316`
  - `x:-1`: global `-0.0012957`
  - `top:+2 / top:-2`: no-op
  - `font:0.97`: slight negative
- `f5_2794`
  - `x:+1 / x:-1`: both negative
  - `top:+2 / top:-2 / font:0.97`: no-op
- `f2_34461`
  - `x:-1`: negative
  - `top:+2 / font:0.97`: no-op

Interpretation:
- Under the current best stack, the known local microfamily axes are close to exhausted for the heaviest residual labels.
- This is not just a global-IoU effect: most local label-box IoU values are also flat or negative under these additional one-node nudges.
- The current attached-label replay stack should therefore be treated as near-saturated along:
  - `x`
  - `top`
  - `font-scale`
  when applied as more of the same single-node extensions.

Takeaway:
- Future gains are unlikely to come from simply adding more one-off `x/top/font` nodes on top of the current stack.
- The next profitable direction should switch to:
  - a different replay knob, or
  - a different family decomposition for the remaining catalyst/product/reagent labels.

## 2026-05-18 y-probe tooling correction and additive-y rerun

Question:
- Were the earlier `x` versus `y` probe conclusions real, or were they partially polluted by probe-tooling behavior?

What was wrong:
- The first `probe-attached-actions-on-stack.py` implementation had two issues:
  1. the `y` branch was accidentally wired through the `x` probe slot because of a control-flow bug in the script;
  2. even after fixing that bug, `phase3band` still short-circuited `preview_attached_label_replay_y_nudge_px(...)`, so explicit `y` probes were being masked by the phase policy.

Fixes:
- Script fix:
  - `scripts/probe-attached-actions-on-stack.py`
  - corrected the `x`/`y` branch structure so `y` actions no longer overwrite `x` env slots.
- Renderer analysis hook:
  - `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
  - added additive `Y_DELTA` experiment env slots that are applied **after** `phase3band`, instead of being shadowed by it.

Sanity check:
- After the fixes, the generated `docx` / embedded `image1.emf` are no longer byte-identical between `x:+2` and `y:+1 / y:-1` probes.
- Example:
  - `f4_32321.x.p2` embedded `image1.emf` hash:
    - `d4a47731e658baf2bffa4c1ef7b5b9509f82e4b17b9f43c376f26623bf2e5c40`
  - `f4_32321.y.p1` embedded `image1.emf` hash:
    - `b9d4ec5d32e25f0493b41382211de00c8eb92bbbac6dece2744420d117f2630f`

Corrected probe:
- Output:
  - `tmp/frame-word-ab/probe-current-best-yx2-fixed/summary.json`
- Same baseline stack:
  - `frame = (1441, 2994, 14431, 8656)`
  - `phase-policy = phase3band`
  - existing `x/top/font-scale` stack unchanged
- Probe set:
  - nodes:
    - `f4_32321`
    - `f4_32345`
    - `f5_2794`
    - `f4_32335`
    - `f4_32337`
    - `f5_2784`
    - `f4_32341`
    - `f1_28328`
    - `f4_32323`
    - `f2_34461`
  - actions:
    - `x:+2`
    - `x:-2`
    - `y:+1`
    - `y:+2`
    - `y:-1`
    - `y:-2`

Corrected headline result:
- The earlier “broad positive `y` family” no longer holds.
- After fixing the tooling and letting `y` deltas actually stack on top of `phase3band`, only a very narrow positive set remains:
  - `f1_28328 y:-2`
    - `globalDelta = +0.0003841807`
    - `labelDelta = +0.0457057823`
  - `f1_28328 y:-1`
    - `globalDelta = +0.0001709472`
    - `labelDelta = +0.0232941662`
  - `f2_34461 y:+1`
    - `globalDelta = +0.0000710583`
    - `labelDelta = +0.0063740403`
- Everything else in this `x±2 / y±{1,2}` matrix is flat or negative.

Interpretation:
- The previous “`x` and `y` produce the same replay” result was a tooling artifact.
- The previous “large positive `y` family” interpretation was also too optimistic because explicit `y` probes were being swallowed by `phase3band`.
- Once probed correctly, the remaining profitable `y`-delta family is much narrower:
  - one `attached-group-above` black `O` case (`f1_28328`) prefers more negative `y`;
  - one black `N` case (`f2_34461`) still prefers a small positive `y`.

Takeaway:
- Future attached-label replay work should treat `y-delta` as a **small residual family**, not a broad primary family.
- The safer working model is now:
  - `phase3band` remains the main vertical policy;
  - additional `y` deltas are narrow exceptions layered on top.

## 2026-05-18 residual `y-delta` subfamilies after metadata backfill

Question:
- Once the corrected additive `y-delta` probe is trusted, can the remaining profitable `y` exceptions be promoted from node lists into reusable subfamily rules?

Metadata backfill:
- The two strongest negative-`y` nodes from the corrected probe were missing geometry context in the earlier attached-page-phase table:
  - `f1_28325`
  - `f1_28328`
- I backfilled them from:
  - `tmp/thiocyanation-source.cdxml`
  - `tmp/all-text-primitives.json`
- Both are:
  - black `O`
  - `layout = attached-group-above`
  - in the bottom-left ligand fragment

Re-checking the corrected positive set:
- `y:+1`
  - `f2_34461`
  - `f4_32339`
- `y:-1`
  - `f1_28322`
  - `f1_28325`
  - `f1_28328`
  - `f4_32331`
- `y:-2`
  - `f1_28325`
  - `f1_28328`

Candidate microfamilies:

1. Negative-`y` `attached-group-above` oxygen family
- Rule candidate:
  - `text == O`
  - `fill == #000000`
  - `layout == attached-group-above`
- Node realization in this document:
  - `f1_28325`
  - `f1_28328`
- Action:
  - `y = -2`

2. Weak negative-`y` east-facing black `N` family
- Observed positives:
  - `f1_28322`
  - `f4_32331`
- Shared features:
  - `text == N`
  - `fill == #000000`
  - `primaryNeighborBucket == east`
  - `gapRight ≈ 114..117`
  - `topPagePhase ≈ 0.34..0.39`
- Action:
  - `y = -1`

3. Previous positive-`y` north-mixed family
- Nodes:
  - `f2_34461`
  - `f4_32339`
- Shared shape:
  - black attached labels
  - `primaryNeighborBucket == north`
  - `topPagePhase < 0.35`
- Action:
  - `y = +1`
- But this family is now suspect and must be judged by whole-document same-shell replay, not single-node deltas alone.

Same-shell bundled policy check:
- Harness:
  - `tmp/frame-word-ab/y-policy-eval/summary.json`
- Baseline (current top/x/font/phase stack under this harness):
  - `IoU = 0.8751508326`
- `negO_only`
  - `y = -2` on `f1_28325,f1_28328`
  - `IoU = 0.8758348757`
  - `delta = +0.0006840431`
- `eastN_only`
  - `y = -1` on `f1_28322,f4_32331`
  - `IoU = 0.8753117207`
  - `delta = +0.0001608881`
- `northMixed_only`
  - `y = +1` on `f2_34461,f4_32339`
  - `IoU = 0.8745476478`
  - `delta = -0.0006031848`
- `negO_plus_eastN`
  - `IoU = 0.8759958156`
  - `delta = +0.0008449830`
- `negO_plus_northMixed`
  - `IoU = 0.8752312766`
  - `delta = +0.0000804441`
- `eastN_plus_northMixed`
  - `IoU = 0.8747084841`
  - `delta = -0.0004423485`
- `all_three`
  - `IoU = 0.8759253299`
  - `delta = +0.0007744973`

Interpretation:
- The old “small positive north family” does **not** survive whole-document same-shell validation.
- The strongest residual `y` gain now comes from:
  1. black `O` with `layout = attached-group-above` using `y = -2`
  2. plus a much weaker east-facing black `N` `y = -1` carry-on
- The corrected residual `y` family should therefore be rewritten as:
  - primary:
    - `text == O && fill == #000000 && layout == attached-group-above -> y = -2`
  - secondary:
    - black `N`, `primaryNeighborBucket == east`, `gapRight ≈ 114..117`, `topPagePhase ≈ 0.34..0.39 -> y = -1`
  - discard:
    - the previous `northMixed -> y:+1` family for the packaged same-shell full-doc objective

Takeaway:
- The corrected additive-`y` story is now much narrower and cleaner than the earlier broad-family interpretation.
- Residual `y` is still real, but it is no longer a dominant axis:
  - one compact negative-`y` oxygen family
  - one tiny negative-`y` `N` carry-on
  - no durable positive-`y` north family at the full-document same-shell level

## 2026-05-18 residual `y-delta` predicates verified against the full attached-label set

Question:
- Can the two surviving residual `y` microfamilies be written as exact predicates on the current full attached-label set, instead of remaining as node lists?

Backfilled attached-label set:
- Source tables:
  - `tmp/frame-word-ab/frame-global3-attached-replay-geometry.json`
  - `tmp/frame-word-ab/attached-page-phase.full.json`
  - `tmp/all-text-primitives.json`
- Purpose:
  - restore missing metadata for the ligand `O` labels
  - verify whether the residual `y` families uniquely map to a reusable predicate

Exact predicate verification:

1. Negative-`y` black oxygen family
- Predicate:
  - `text == O`
  - `fill == #000000`
  - `layout == attached-group-above`
- Matches in the current full attached-label set:
  - `f1_28325`
  - `f1_28328`
- This is now an **exact family**, not a loose heuristic.

2. Weak negative-`y` east-facing black `N` family
- Predicate:
  - `text == N`
  - `fill == #000000`
  - `primaryNeighborBucket == east`
  - `110 <= gapRight <= 120`
  - `0.34 <= topPagePhase <= 0.40`
- Matches in the current full attached-label set:
  - `f1_28322`
  - `f4_32331`
- This is also an **exact family** on the current document, not just a remembered pair.

Same-shell bundled validation:
- Harness:
  - `tmp/frame-word-ab/y-policy-eval/summary.json`
- Baseline:
  - `IoU = 0.8751508326`

Validated bundles:
- `negO_only`
  - `y = -2` on the exact black-`O` `attached-group-above` family
  - `IoU = 0.8758348757`
  - `delta = +0.0006840431`
- `eastN_only`
  - `y = -1` on the exact east-facing black-`N` family
  - `IoU = 0.8753117207`
  - `delta = +0.0001608881`
- `negO_plus_eastN`
  - combine both exact predicates
  - `IoU = 0.8759958156`
  - `delta = +0.0008449830`

Counterexample bundle:
- `northMixed_only`
  - `y = +1` on `f2_34461,f4_32339`
  - `IoU = 0.8745476478`
  - `delta = -0.0006031848`
- So the old north-oriented positive-`y` interpretation is no longer viable at the full-doc same-shell level.

Interpretation:
- The residual `y` axis has now stabilized into two exact subfamilies:
  1. **primary**
     - black `O`
     - `layout = attached-group-above`
     - `y = -2`
  2. **secondary**
     - black `N`
     - east-facing
     - `gapRight ≈ 114..117`
     - `topPagePhase ≈ 0.34..0.40`
     - `y = -1`
- The positive-`y` branch does not survive whole-document validation and should be treated as a misleading local artifact.

Takeaway:
- Residual `y` replay is no longer an open-ended search space.
- On the current document, it can already be expressed as:
  - one exact `attached-group-above` black-`O` family
  - plus one exact east-facing black-`N` carry-on family
- This means future replay work should stop treating `y` as a broad generic knob and instead keep it as a tiny residual rule layer.

## 2026-05-18 泛化抽样验证：PPT 历史报告里的 ChemDraw OLE 对象

目标：
- 不再只盯 `thiocyanation-source`
- 改为从 `tmp/Group past reports` 里的历史 `pptx` 抽出真实 ChemDraw OLE 对象
- 用这些历史对象验证当前 `same-shell Word replay` 规则是否能泛化

新增工具：
- `scripts/extract-ppt-chemdraw.py`
  - Windows 原生提取 `ppt/pptx -> embedded .bin -> .cdx -> .cdxml`
  - 同时抽出 PowerPoint 中 fallback 预览图（优先 `EMF`）
  - 现在已修掉同一个 `slide + rid + target` 的重复记录；会优先保留带 preview 的条目
- `scripts/compare-ppt-extract-sameshell.py`
  - 对抽出的每个 `cdxml`
  - 生成 chemcore payload / docx
  - 再把同一个 `docx` 壳里的 `word/media/image1.emf` 替换成 PPT 内嵌的 ChemDraw 预览 `EMF`
  - 最后对两边都用 Word `CopyAsPicture`，得到真正 same-shell 的回放图做逐像素比较

流程口径：
1. `pptx` 内嵌 OLE `bin` -> `cdx`
2. `cdx -> cdxml`
3. `cdxml -> chemcore payload -> chemcore docx`
4. `patch-docx-image1.py` 把同一壳里的 `image1.emf` 换成 PPT 里的 ChemDraw preview emf
5. 对两份 `docx` 都走 `word-copy-inline-shape.ps1`
6. 用 `png-best-shift.py` 算 best-shift IoU

### 样本 A：`CCH-20181022-working report.pptx`

提取输出：
- `tmp/ppt-sample-cch`
- `entryCount = 4`

same-shell 比较输出：
- `tmp/ppt-sample-cch/same-shell-compare/summary.json`

结果：
- `slide2.oleObject1`: `IoU = 0.376683`, `dx = 4`, `dy = 6`
- `slide3.oleObject2`: `0.334622`, `dx = 7`, `dy = 2`
- `slide4.oleObject3`: `0.382603`, `dx = 4`, `dy = 3`
- `slide5.oleObject4`: `0.374624`, `dx = 4`, `dy = 1`

汇总：
- `avg IoU = 0.367133`
- `avg dx = 4.75`
- `avg dy = 3.00`

### 样本 B：`工作汇报-李家圆-2017-1-19.pptx`

提取输出：
- `tmp/ppt-sample-ljy8`
- `entryCount = 8`

same-shell 比较输出：
- `tmp/ppt-sample-ljy8/same-shell-compare/summary.json`

结果范围：
- `IoU = 0.305593 ~ 0.339142`
- `dx = 3 ~ 6`
- `dy = 2 ~ 12`

汇总：
- `avg IoU = 0.328405`
- `avg dx = 4.625`
- `avg dy = 7.00`

### 样本 C：`WJG-lit.-2021-04-15.pptx`

提取输出：
- `tmp/ppt-sample-wjg4`
- `entryCount = 4`

same-shell 比较输出：
- `tmp/ppt-sample-wjg4/same-shell-compare/summary.json`

结果：
- `slide11.oleObject1`: `IoU = 0.363680`, `dx = 3`, `dy = 9`
- `slide12.oleObject2`: `0.418695`, `dx = 6`, `dy = 8`
- `slide13.oleObject3`: `0.369069`, `dx = 9`, `dy = 8`
- `slide15.oleObject4`: `0.523464`, `dx = 11`, `dy = 13`

汇总：
- `avg IoU = 0.418727`
- `avg dx = 7.25`
- `avg dy = 9.50`

### 跨样本结论

1. 当前围绕 `thiocyanation-source` 打出来的规则，**没有泛化**到旧报告对象
- 历史样本的 same-shell `best_iou` 普遍只有 `0.30 ~ 0.52`
- 与当前主线文档里 `0.88+` 的水平差距极大

2. 失配不是偶然单点，而是跨样本稳定存在
- 三组样本的平均值分别是：
  - `0.367`
  - `0.328`
  - `0.419`
- 三组都明显偏低

3. 平移偏差也表现出稳定模式
- `dx` 多在正方向 `+3 ~ +11`
- `dy` 多在正方向 `+1 ~ +13`
- 说明当前问题不只是局部文字家族，而是更高层的 replay / frame / placement 语义没有泛化

4. 所以从这一步开始，主线判断要调整：
- 不能再把 `thiocyanation-source` 的高 IoU 当成“系统已经基本对了”
- 更合理的说法是：
  - **我们在一个特定文档上已经过拟合得很好**
  - 但在真实历史 PPT ChemDraw 对象上，还没有形成可泛化规律

后续优先级：
1. 继续扩大真实 PPT 样本池，但保持每份 `4~12` 个对象，避免一次性把问题淹没
2. 优先分析这些历史样本里稳定出现的：
   - `dx > 0`
   - `dy > 0`
   偏差模式
3. 不再只围绕 `thiocyanation-source` 的局部标签 microfamily 做推理，而要回到更高层的 replay / frame / shell 语义

## 2026-05-18 第二轮泛化抽样：扩到 19 个历史 OLE 对象，并验证 frame 主因是否泛化

新增样本：
- 	mp/ppt-sample-zww4
  - 来源：	mp/Group past reports/2018-2019/work report/20190506/ZWW-2019-5-6.pptx
  - entryCount = 4
  - same-shell 可跑结果 3 个，1 个失败（preview-not-emf，是 WMF 预览）

ZWW 三个对象结果：
- slide2.oleObject1: IoU = 0.454442, dx = 4, dy = 6
- slide4.oleObject3: IoU = 0.528250, dx = 4, dy = 5
- slide5.oleObject4: IoU = 0.423868, dx = 4, dy = 4

为了把跨样本现象固定下来，新增工具：
- scripts/summarize-ppt-sameshell-generalization.py

聚合了当前 4 组样本（cch / ljy8 / wjg4 / zww4）后，得到：
- 总对象数：19
- vgBestIou = 0.377750
- vgDx = +5.105
- vgDy = +6.368
- 平均 rameDelta = [ +894, -320, +1587, -98 ]
- 平均 sizeDelta = [ +692, +222 ]
- 平均尺寸比：
  - vgWidthRatio = 1.051026
  - vgHeightRatio = 1.018358

解释：
- 跨样本看，ChemDraw 预览 EMF 相对我们的 image1.emf，确实稳定更“宽”一些，平均宽度约多 5.1%。
- 同时 best-shift 也稳定落在 dx > 0, dy > 0 一侧。
- 所以**泛化主问题仍然存在**，而且不是单个历史文件偶然偏掉。

但更关键的新结论是：
- **单文档线上发现的 rame 主因，不能直接泛化到这批历史样本。**

我做了 3 组跨样本 frame-policy A/B：

1. 固定 vg_delta（直接把平均 rameDelta 加到我们的 image1.emf 上）
- 结果：vgIou = 0.106409
- 相比 baseline  .377750，大幅变差

2. 固定 median_delta
- 结果：vgIou = 0.095815
- 同样大幅变差

3. 直接把每个对象的 EMR_HEADER.frame 抄成对应 ChemDraw preview 的 frame（rame-chemref）
- 结果：vgIou = 0.104478
- 也显著变差

另外还试了按比例的宽/高规则：
- x_ratio_only
- xy_ratio

结果同样失败：
- x_ratio_only: vgIou = 0.142049
- xy_ratio: vgIou = 0.122534

这一步非常重要，因为它说明：
- 历史样本里的 same-shell 差异，**不能简单地靠“把 frame 调成 ChemDraw 那张”来解释**。
- 也不能靠一个全局固定的 rameDelta 或尺寸比例去修。
- 也就是说，	hiocyanation-source 那条围绕 EMR_HEADER.frame 的主因链，在真实历史 PPT OLE 对象上**不具备直接可移植性**。

当前更稳妥的判断是：
1. 跨历史样本，确实存在稳定的正向 dx/dy 偏移与尺寸差
2. 但这些差异不是一个“固定 frame 规则”能解释的
3. 所以下一步不该继续套单文档 rame 经验，而应该去查：
   - 历史 OLE 对象里 ChemDraw preview EMF 与我们 packaged EMF 的**内容回放语义**到底差在哪里
   - 尤其是：为什么 same-shell 下，抄 ChemDraw 的 frame 反而会把回放做坏

Takeaway：
- 到这一步可以正式收回一个旧假设：
  - “只要找到正确 EMR_HEADER.frame，就能泛化解决 Word replay 差异”
- 这个假设在 	hiocyanation-source 上成立，但在历史 PPT 样本上**被证伪**。
- 后续泛化主线应该转去研究：
  - **real PPT ChemDraw object 的 packaged replay 语义差**
  - 而不是继续把单文档 frame 调优当成系统解。

## 2026-05-18 第三轮泛化抽样：跨 19 对象的 EMF 记录结构差异

为了判断历史样本上的 same-shell 偏差到底更像 rame 还是更像 replay 结构差，我又补了一层工具：
- scripts/summarize-ppt-emf-record-counts.py

它直接对每个 same-shell-compare 目录里的：
- *.chemcore.docx -> word/media/image1.emf
- *.chemdraw-shell.docx -> word/media/image1.emf

跑 scripts/emf-inspect.mjs，然后聚合 ours/ref 的记录计数差异。

产物：
- 	mp/ppt-generalization-record-counts.json

当前 19 个历史对象的聚合结果：

- EMR_EXTTEXTOUTW
  - ours 77.79
  - ref 78.00
  - vgDelta = -0.21
- EmfPlusDrawString
  - ours 78.95
  - ref 79.53
  - vgDelta = -0.58
- EmfPlusSetPageTransform
  - ours 2.0
  - ref 2.0
  - 完全一致

也就是说：
- **文本对象数量本身并没有大偏差**
- 历史样本上的泛化问题，不像是“我们少画/多画了一堆文字”

但结构性差异非常大：

- EMR_EXTCREATEFONTINDIRECTW
  - ours 26.32
  - ref 59.21
- EMR_POLYGON16
  - ours 60.00
  - ref 3.63
- EMR_MODIFYWORLDTRANSFORM
  - ours 361.79
  - ref 258.95
- EmfPlusObject
  - ours 287.63
  - ref 208.21
- EmfPlusFillPolygon
  - ours 51.37
  - ref 1.21
- EmfPlusSave / Restore
  - ours 119.26 / 119.26
  - ref   / 0

Interpretation：

1. 历史样本里，ChemDraw preview 的 replay 结构和我们不是“小差一点”，而是**系统性不同**。
2. 这批对象里，ChemDraw 不是靠大量 Save/Restore + FillPolygon + Polygon16 在回放；我们却明显更依赖这组结构。
3. 结合前面的 same-shell IoU 结果，可以比较有把握地说：
   - **历史 OLE 对象上的泛化主问题，已经不像单文档那样主要由 EMR_HEADER.frame 驱动**
   - 更像是 packaged preview 的**记录结构 / replay 语义家族根本不同**

这也能解释前面那个看起来反直觉的结果：
- 即使把 EMR_HEADER.frame 直接抄成 ChemDraw preview 的 frame，same-shell 平均 IoU 仍会从  .377750 掉到  .104478
- 因为两边的内容 replay 结构本来就不是同一家族，单独改 frame 只会把错位放大

Takeaway：
- 到这一步，泛化主线要正式从“找通用 frame 规则”转向“找通用 replay 家族差”。
- 更值得继续打的方向变成：
  1. 为什么历史 ChemDraw preview 普遍几乎不用 Save/Restore，而我们大量使用
  2. 为什么历史 ChemDraw preview 基本不用 FillPolygon/Polygon16，而我们大量使用
  3. 这些差异与 dx > 0 / dy > 0 / 低 IoU 的相关性到底有多强

## 2026-05-18 Office 预览可见 halo：默认隐藏 DocumentKnockout

用户重新指出了一个比 same-shell 指标更基础的显示错误：
- Office / Word 里每个标签都像被一层框或外壳圈出来
- 这不是普通文字对齐问题，而是内部退让几何泄漏成了可见结果

已有证据：
- engine 的 `document_svg` 本来就默认隐藏 `DocumentKnockout`
- Office 预览此前却把 `RenderRole::DocumentKnockout` 当成正常可见 primitive 回放
- `knockout-only` 与 `text-only` 的叠加里，`knockout` 基本是文字的“胖超集”

本轮采取的最小产品修复：
- 在 `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`
- `office_preview_primitive_visible()` 默认 **不再**让 `DocumentKnockout` 进入可见回放
- 仅保留一个显式调试开关：
  - `CHEMCORE_EMF_SHOW_DOCUMENT_KNOCKOUT=1`
- 旧的 `CHEMCORE_EMF_HIDE_DOCUMENT_KNOCKOUT` 仍保留，调试时优先级更高

行为语义：
- 默认用户可见回放：`DocumentBond | DocumentGraphic | DocumentText`
- `DocumentKnockout` 只用于内部分析/调试，不再直接画到 Word 预览里

手工验证：
- 重新导出 `tmp/thiocyanation-source.chemcore.v78.docx`
- 用 Word `CopyAsPicture` 生成：
  - `tmp/thiocyanation-source.chemcore.v78.wordcopy.png`
  - `tmp/thiocyanation-source.chemcore.v78.product-crop.png`
  - `tmp/thiocyanation-source.chemcore.v78.catalyst-crop.png`
- 肉眼结果：右上产物与右下催化剂周围那层明显的白色外壳/框基本消失

备注：
- 这一步主要修的是“可见 halo”这个独立 bug
- 它不等同于 same-shell Word replay 主线已经完全对齐
- 后续 frame / replay 泛化分析仍按原主线继续进行


## 2026-05-18 ? 19 ??????????????

????????????????????????????? same-shell PPT ??? `role-report`?????

- `scripts/summarize-ppt-text-boxes.py`

????

- `tmp/ppt-sample-*/same-shell-compare/*.payload.json`
- ??? `*.chemcore.wordcopy.png`
- ??? `*.chemdraw.wordcopy.png`
- ????????? `bestshift.json`

??? `compare-full-text-boxes.py`?????????????????

- `tmp/ppt-generalization-text-boxes.json`

??????? `compare-full-text-boxes.py` ????????

- payload ?????? UTF-8????? `load_json_any_encoding`
- `text_preview()` ??? ASCII ????? Windows `gbk` ??????????????

### ??????

?????

- `fontFamilySummary`
  - `Arial`: `301 / 302`
  - ?? `1` ? `Symbol`

?????????????????????? `Arial` ???**???????????????**?

?? text-like family??????? `+ / L / B` ?????? token ???

1. `left|normal|lines=1`
   - `count = 113`
   - `avgDw = +0.27`
   - `avgDh = -0.18`
   - `avgDx = -0.54`
   - `avgDy = -0.16`
   - **??????????????????????????**

2. `left|normal|lines=2`
   - `count = 14`
   - `avgDw = +0.14`
   - `avgDh = -0.64`
   - `avgDx = -0.07`
   - `avgDy = -0.36`
   - **???????????????**

3. `center|normal|lines=2`
   - `count = 13`
   - `avgDw = +0.23`
   - `avgDh = -1.08`
   - `avgDx = +0.15`
   - `avgDy = -1.00`
   - **???????????????????????**

4. `center|normal,subscript,superscript|lines=3`
   - `count = 10`
   - `avgDw = -5.30`
   - `avgDh = -1.20`
   - `avgDx = -0.20`
   - `avgDy = +0.20`
   - **?????????????????????**
   - ???????????????? **??????**?
   - ?????
     - `Pd(CH3CN)4(BF4)2 (10 mol%) / L (15 mol %) / SelectFluor-BF4 (1.5 equiv)`
     - `CsOCF3 (5.0 equiv) / THF/DCM (4:6, v/v) / -20 oC, 36 hrs`

5. `left|normal,subscript,superscript|lines=1`
   - `count = 6`
   - `avgDw = +2.17`
   - `avgDh = +2.67`
   - `avgDx = -1.50`
   - `avgDy = -2.50`
   - **???????????????**
   - ???? **??????????????**?
   - ?????
     - `Ar = 3,5-2tBuC6H3`

6. `left|normal,subscript|lines=1`
   - `count = 5`
   - `avgDw = -1.60`
   - `avgDh = -0.80`
   - `avgDx = +0.40`
   - `avgDy = +1.00`
   - **??????????????????????**

### ????????????

???????????????????????

- ???????? `thiocyanation-source` ???
- ???????????????
- ????????? **??????** ????????????????
  1. `center|normal,subscript,superscript|lines=3`
  2. `left|normal,subscript,superscript|lines=1`

????????????????
- `left|normal|lines=1`
- `left|normal|lines=2`
- `center|normal|lines=2`

????????????????????????????????? replay ??????????? token ?????? patch?


## 2026-05-18 ?? cch ????????

?????????????????????????????????? `ppt-sample-cch` ???
`tmp/ppt-generalization-text-boxes.json` ?????????? 3 ????`ljy8 / wjg4 / zww4`?? text-like family?

??????

- ?????????
  - `center|normal,subscript,superscript|lines=3`
  - `left|normal,subscript,superscript|lines=1`
- ??????????????????

????????????????

- `left|normal|lines=1`
  - `count = 93`
  - `avgDw = +0.32`
  - `avgDh = -0.17`
  - `avgDx = -0.46`
  - `avgDy = -0.23`
  - ????????

- `left|normal|lines=2`
  - `count = 12`
  - `avgDw = +0.00`
  - `avgDh = -0.75`
  - `avgDx = +0.08`
  - `avgDy = -0.33`
  - ?????

- `left|normal,subscript|lines=1`
  - `count = 5`
  - `avgDw = -1.60`
  - `avgDh = -0.80`
  - `avgDx = +0.40`
  - `avgDy = +1.00`
  - ??????????????

????

1. `center|normal,subscript,superscript|lines=3` ????????????? `ppt-sample-cch` ????????????
2. `left|normal,subscript,superscript|lines=1`??? `Ar = 3,5-2tBuC6H3`????????????????????????????????
3. ???? 3 ??????????????????????????????????????????
   - packaged replay ?????
   - ???? attached labels
   - ?????? mixed-script ???

Takeaway?
- ???????????????
  - **??? `cch` ???????? mixed-script ??????????? ChemDraw OLE ????????**
- ?????????????????
  1. ?????????????????????
  2. `cch` ? mixed-script ????????????
  3. ???? attached labels ???????


## 2026-05-18 ? 19 ? PPT ?????????

??? same-shell label-box ????????????????????????????

?????
- `scripts/summarize-ppt-label-boxes.py`
- `scripts/summarize-ppt-derived-label-families.py`
- ???`tmp/ppt-generalization-label-boxes.json`
- ???`tmp/ppt-generalization-derived-label-families.json`

### ???

1. ??????????????????? `attached-group-above`?
- `attached-group-above` ???
  - `count = 18`
  - `avgIoU = 0.3059`
  - `avgResidual = 78.17`
- ???????? `attached-group` ???

2. `attached-group-above` ??????? multiline ????
- `attached-group-above + lineCount=2 + black`?
  - `count = 7`
  - `avgIoU = 0.2866`
  - `avgResidual = 131.29`
  - `avgDy = +1.57`
- ?????`O2\nS`, `H\nN`
- ???????? + ??????????????????

3. ?? lateral attached-group ???? multiline ??????
- `attached-group + lineCount=2 + black`?
  - `count = 8`
  - `avgIoU = 0.3347`
  - `avgResidual = 158.13`
  - `avgDh = +3.0`
- ?????? `N\nBoc` ???????

4. ?????????????????????????????
- `attached-group + lineCount=1 + charCount>=4 + black`?
  - `count = 24`
  - `avgIoU = 0.3797`
  - `avgResidual = 174.75`
  - `avgDx = -0.58`
- ?????`C6H3Cl(p)`, `C6H3F(p)`, `CO2Bn`, `DIPP`, `NHBoc`, `Tol(m)`

5. ?? 2 ?? attached-group ????????????
- `attached-group + lineCount=1 + charCount=2 + black`?
  - `count = 113`
  - `avgIoU = 0.3891`
  - `avgResidual = 60.03`
- ?????`CN`, `NC`, `NH`, `Ph`, `Ar`, `Br`, `Cl`
- ??????????????? multiline / long black ???

6. ????? attached-group ?????
- `attached-group + lineCount=1 + charCount=1 + black`?
  - `count = 52`
  - `avgIoU = 0.4748`
  - `avgResidual = 31.10`
- ?????`N`, `O`, `F`, `R`
- ???????????? attached labels ????????

### ???????????

- ???????
  - `attached-group-above`??? multiline black?`O2\nS`, `H\nN`?
- ???????
  - lateral `attached-group` ? multiline black?`N\nBoc`?
- ???????
  - ??? lateral labels?`C6H3Cl(p)`, `C6H3F(p)`, `DIPP`, `NHBoc` ??
- ??????
  - ??????? 2-char ?????`CN/NC/Ph/Ar/...`?

### Takeaway

????????????????????????????????????????????
- ?????????
- ?????????
- ?????????

????????????????????????????????????????????


## 2026-05-18 PPT ???attached-group-above / multiline family policy ??

?????? `19` ??? PPT ChemDraw OLE ????? family-level policy ????????? node ? same-shell ???

### ????

- ???`tmp/ppt-policy-matrix/none/summary.json`
  - `avgBestIou = 0.376166`
- ???`Intel Core Ultra 9 275HX`
  - `24 cores / 24 logical processors`
- ????????????
  - `above-single-black-x+1`
  - `above-multi-black-y-{1,2,3}`
  - `lateral-multi-black-y-{1,2,3}`

### ???????

- `none`
  - `0.376166`
- `above-single-black-x+1`
  - `0.376143`
- `above-multi-black-y-1`
  - `0.376258`
- `above-multi-black-y-2`
  - `0.376380`
- `above-multi-black-y-3`
  - `0.376465`
- `lateral-multi-black-y-1`
  - `0.376205`
- `lateral-multi-black-y-2`
  - `0.376297`
- `lateral-multi-black-y-3`
  - `0.376430`
- `above-single-black-x+1,above-multi-black-y-2`
  - `0.376357`
- `above-single-black-x+1,above-multi-black-y-3`
  - ?? 18 ???`0.379136`
  - ????? `ppt-sample-ljy8/slide5.oleObject7` ??19 ???????`0.376444`

### ????

1. `above-multi-black-y-3` ?????? family-level ??
- ?? `19` ?????????? baseline?`0.376465 > 0.376166`
- ????
  - `attached-group-above + multiline + black`
  - ????? baseline / line-step ????????????

2. `above-single-black-x+1` ??????????
- ?? `above-multi-black-y-3` ??????????????
- ??? label-box ?????????????
  - ???? `O` ??
  - ??? `O / N / S` ?????????
- ???????????????????????????????

3. `lateral-multi-black-y-*` ??????
- `y-3` ???????? baseline ? `0.000264`
- ????????????????

### ?????

? `above-single-black-x+1,above-multi-black-y-3` ??? label-box ?????

- ?????
  - `ppt-sample-ljy8/slide4.oleObject6` ?? `S / N`
  - `ppt-sample-cch` ?? `O`
- ??????????
  - `ppt-sample-zww4/slide2.oleObject1` ??? `O`

???????
- `above-single-black-x+1` ?????????
- `above-multi-black-y-3` ??????????

### Takeaway

????????????????? attached-label ????

- `attached-group-above + multiline + black -> y:-3`

??

- `attached-group-above + single-line + black -> x:+1`

?????????????????????????????


### ???? policy runner

??????? family-level policy??????

- `scripts/run-ppt-general-label-policy-matrix.py`

???
- ? policy ??????? `evaluate-ppt-general-label-policy.py`
- ??????? CPU ??? `min(16, cpu_count)` worker
- ?????? PPT same-shell policy ??????????

?? smoke test?
- `jobs = 2`
- policies = `none`, `above-multi-black-y-3`
- ????
  - `tmp/ppt-policy-matrix-par-smoke/`

?????????????? rerun ?????????????
- `above-multi-black-y-3 > none`

???????????? runner ?????????????????????????
