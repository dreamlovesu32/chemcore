# 文本符号表与 Glyph Profile 规则

本文档只覆盖文本编辑里的 Unicode 字符和特殊字符，不覆盖 `SceneObject { type: "symbol" }` 的电荷、自由基、孤对电子等化学符号。

## 目标

文本编辑器必须能稳定插入和渲染常见符号，不能因为某个字符没有手工写入 `shared/glyph_profiles.json` 就退回错误宽度或错误高度。`%` 这类问题的根因不是单个字符漏补，而是 glyph profile 覆盖和兜底策略不完整。

当前规则：

- 文本符号以 Unicode 字符为内部存储值。
- 符号表 UI 从共享 catalog 读取字符分组。
- Rust glyph kernel 是 caret advance、ink box、background box、glyph polygon 和 label clipping 的权威。
- Viewer 可以展示符号表和把字符插入当前文本编辑会话，但不能重新定义 glyph 几何。
- 未收录字符必须有 Unicode 类别兜底，不能一律落到窄标点 profile。

## 共享数据

文本符号表使用：

```text
shared/text_symbols.json
```

该文件只表达 UI 分组和 Unicode 字符，不携带化学语义。字符进入文档后仍是普通 text run 内容。

Glyph profile 使用：

```text
shared/glyph_profiles.json
```

该文件继续保存确定性的 normalized profile：

- `advanceEm`
- `inkLeftEm`
- `inkTopEm`
- `inkRightEm`
- `inkBottomEm`
- `padXEm`
- `padYEm`
- `shape`
- `visible`

新增或调整文本符号时，应优先运行生成脚本，而不是手工补单个字符：

```bash
python scripts/generate-glyph-profiles.py
```

生成脚本会读取 `shared/text_symbols.json`，从本机可用字体测量字符 advance 和 ink bbox，并只补充 manifest 中缺失的字符。已有人工校准 profile 会保留。

## Runtime 兜底

即使字符不在 `shared/glyph_profiles.json`，Rust 和 viewer 也必须按 Unicode 类别给出保守 profile：

- 空白字符：可见性为 false，只贡献 advance。
- CJK 与全角字符：按接近 1em 的方形 profile 处理。
- 希腊、拉丁扩展、西里尔等字母：按字母 profile 处理，不退成窄标点。
- 数学符号和箭头：按宽符号 profile 处理。
- 未知符号：按中等宽度保守矩形处理。

兜底 profile 的裁剪形状必须保守。自动生成脚本能提供真实 ink bbox；如果不能可靠判断角裁剪，就使用矩形而不是激进切角。

## 裁剪策略

第一阶段保证所有字符至少有可用 bbox 裁剪：

1. 已收录字符使用 `shared/glyph_profiles.json` 的 profile。
2. 未收录字符使用 Unicode 类别兜底 profile。
3. `glyphPolygons` 由 Rust glyph kernel 输出。
4. label clipping 和 knockout 消费 `glyphPolygons`，没有 polygon 时才退回 label box。

角裁剪只在 profile 明确声明时启用。中文和复杂符号默认使用保守矩形，避免键线切进笔画。

## UI 行为

右下角文本符号表是普通文本输入辅助：

- 点击右下角按钮展开符号表，面板向左打开。
- 点击字符时，如果当前有 active text editor，直接插入到 caret。
- 如果没有 active text editor，则切到 Text 工具，并把该字符作为下一次文本创建后的待插入字符。
- 未固定时，点击字符后自动收回。
- 右上角固定按钮开启后，点击字符不自动收回。

该 UI 不应混入电荷、自由基、孤对电子等化学 `symbol` 对象。
