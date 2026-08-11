# 字形内核

ChemSema 在 Rust 内核中根据真实字体轮廓统一计算标签排版和键退让。轮廓数据来自 [shared/glyph_outlines.json](../shared/glyph_outlines.json)，覆盖多种字体，以及字体实际提供的常规、粗体、斜体和粗斜体 face。每个 face 还保存真实的横向 kerning pair，使可见文本、字符锚点和退让几何使用同一组字形位置。

设标签 margin 为 `m`、当前字形字号为 `s`：

```text
q = min(m, 0.25 * s)
natural = 真实字形轮廓 ⊕ 半径为 m 的欧氏圆盘
feature = 凸包顶点向字形中心移动 0.5q 后，叠加半径 1.5q 的圆
axial = 四个轴向 ±10° 内的接触扇区
clip = natural ∪ feature ∪ axial
```

裁剪时使用有实际宽度的键体与 `clip` 求交。这是一条统一函数规则，不是逐字符查表，也不做 360 度拟合。

## 数据职责

- `glyphPolygons`：逐字符真实轮廓的凸包，用于编辑、命中和字符锚点。
- `glyph_clip_polygons`：只在运行时存在的派生退让几何，不是 CCJS 权威字段，也不序列化。
- 原来的 `shared/glyph_clip_polygons.json` 字符表及其生成器已经删除，不存在旧渲染 fallback。
- 缺字时走明确的字体替换链，最终使用真实的 `□` 字形轮廓；替换轮廓同时提供 metrics 和退让几何，不存在即时合成的矩形退让 fallback。
- kerning 来自所选字体 face 的横向 OpenType/TrueType `kern` 数据，只在解析到同一 face、字号和 script 基线的相邻字形之间生效；不得跨字体替换、字号、style 或 script 边界。
- kerning 不注入用于补齐文档 viewBox 的粗略、来源中立文本宽度估算。该估算必须保持作者页面框架；精确 kerning 只属于字形排版，不能让内容未变的整张图发生平移或缩放。

## 重建时机

文档加载、确认文字编辑、修改字体/style 或 MarginWidth 时，两层几何原子重建。用户在打开的文字编辑框中输入时，不改动文档几何；用户拖拽标签时，每次 pointer move 都同步平移两层几何，所以松开鼠标前键退让已经实时更新。

## 生成与验证

运行 `python scripts/generate-glyph-outlines.py`。`.mjs` 入口只转调同一个生成器，避免出现两套 manifest schema。manifest version 3 把轮廓、advance 和 face kerning 放在同一个权威输入中。构建脚本会先用 gzip 压缩 manifest 再嵌入，内核首次使用时只解压一次。随后运行 Rust 内核测试和 viewer WASM 构建。
