# ChemCore CLI 命令指南

这份文档给调用方直接使用 `chemcore-cli`。调用方不需要阅读 ChemCore 源代码，只需要按这里的命令行和 JSON 字段组织输入。

## 1. 启动 CLI

在仓库根目录运行：

```powershell
npm run cli -- <command> [args...]
```

等价于：

```powershell
cargo run -p chemcore-cli -- <command> [args...]
```

编译后也可以直接运行：

```powershell
target\debug\chemcore-cli.exe <command> [args...]
```

查看帮助：

```powershell
npm run cli -- --help
```

如果 Windows PowerShell 因执行策略拦截 `npm.ps1`，把示例里的 `npm` 换成 `npm.cmd`：

```powershell
npm.cmd run cli -- --help
```

桌面端安装版会把 `chemcore-cli.exe` 和 GUI 一起安装，并随安装包携带英文详细指南
`chemcore-cli-guide.md`。安装器会把 CLI 目录加入 PATH。安装后打开新的终端，
从这些命令开始：

```powershell
chemcore-cli guide --pretty
chemcore-cli guide --kind detailed --pretty
chemcore-cli doctor --pretty
chemcore-cli capabilities --pretty
```

`--pretty` 只改变 JSON 的空白字符布局：把紧凑单行 JSON 输出成带换行和
缩进的 JSON。它不改变字段、值、输出文件、退出码、schema、排序或命令行为。
不加 `--pretty` 时，JSON 是紧凑单行 JSON。

## 2. 文件命令

打开文件就是把文件路径作为 `inspect`、`run`、`convert` 或 `export` 的输入参数。

```text
chemcore-cli guide [--kind agent|detailed|all] [--include-content] [--pretty] [--out <path>]
chemcore-cli about [--pretty] [--out <path>]
chemcore-cli capabilities [--pretty] [--out <path>]
chemcore-cli doctor [--pretty] [--out <path>]
chemcore-cli examples [basic|capture-copy|all] [--pretty] [--out <path>]
chemcore-cli schema [commands|targets|capture|context|detail|guide|copy|json-output|command-script|all] [--pretty] [--out <path>]
chemcore-cli inspect <input> [--include summary,objects,molecules,resources,styles] [--out <path>] [--pretty]
chemcore-cli targets <input> [--out <path>] [--pretty]
chemcore-cli context <input> --target <selector> [--radius <pt>] [--out <context.json>] [--capture-out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--pretty]
chemcore-cli detail <input> --target <object:id|molecule:index|node:id|bond:id> [--summary-only] [--include-resource] [--out <detail.json>] [--pretty]
chemcore-cli capture <input> --target <selector> [--out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--expand <pt>] [--expand-rel <fraction>] [--pretty]
chemcore-cli copy <input> [--target <selector>] [--payload <payload.json>] [--no-copy] [--pretty]
chemcore-cli new [commands.json|-] --out <path> [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]
chemcore-cli run <input> <commands.json|-> [--out <path>] [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]
chemcore-cli convert <input> <output> [--format <format>]
chemcore-cli export <input> <output> [--format <format>]
```

常用调用：

```powershell
npm run cli -- inspect input.cdxml --include summary,objects,molecules --out inspect.json --pretty
npm run cli -- targets input.cdxml --out targets.json --pretty
npm run cli -- capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --expand-rel 0.15 --pretty
npm run cli -- new commands.json --out output.cdxml --results results.json --pretty
npm run cli -- run input.cdxml commands.json --out output.cdxml --results results.json --document-json after.ccjs --pretty
npm run cli -- convert input.cdxml output.svg
npm run cli -- convert input.cdxml output.ccjs
```

文件输出规则：

- `capture` 可以省略 `--out`；此时会把 PNG 写到系统临时目录下的 `chemcore-cli` 子目录，并在 `output.path` 返回真实路径。
- `copy` 可以省略 `--payload`；此时会把剪贴板 payload JSON 写到系统临时目录下的 `chemcore-cli` 子目录，并在 `payload.path` 返回真实路径。
- `new`、`convert`、`export` 会创建主要文档文件，必须显式给输出路径。
- 所有写文件命令都会在写完后检查目标是否存在、是否为普通文件、以及字节数是否符合预期或最低要求。校验失败就是命令失败。

`new` 从空白 ChemCore 内部文档开始，不需要指定输入格式。保存格式由 `--out` 后缀决定：

```powershell
npm run cli -- new --out blank.ccjs --quiet
npm run cli -- new commands.json --out figure.cdxml
npm run cli -- new commands.json --out figure.svg
```

输出路径没有后缀，或者输出到 stdout 时，用 `--save-format` 指定保存格式：

```powershell
npm run cli -- new commands.json --out output --save-format cdxml
npm run cli -- run input.cdxml commands.json --out - --save-format svg --quiet
```

`convert` 和 `export` 用 `--format` 覆盖输出格式：

```powershell
npm run cli -- convert input.cdxml output --format svg
```

支持格式：

| 格式 | 读入 | 写出 | 用途 |
| --- | --- | --- | --- |
| `json` | yes | yes | ChemCore 内部 JSON，`.json` 后缀按内部 JSON 处理 |
| `ccjs` | yes | yes | ChemCore 内部 JSON，推荐作为未压缩内部格式 |
| `ccjz` | yes | yes | gzip 压缩 ChemCore JSON |
| `cdxml` | yes | yes | ChemDraw XML |
| `cdx` | yes | yes | ChemDraw binary |
| `sdf` | yes | yes | MDL SD file |
| `svg` | no | yes | 矢量导出 |

## 3. 命令脚本格式

`commands.json` 可以是一条 JSON object，也可以是 JSON array。

单条命令：

```json
{
  "type": "insert-template",
  "template": "benzene",
  "x": 300.0,
  "y": 260.0
}
```

多条命令：

```json
[
  {
    "type": "insert-template",
    "template": "benzene",
    "x": 300.0,
    "y": 260.0
  },
  {
    "type": "add-arrow",
    "begin": { "x": 370.0, "y": 260.0 },
    "end": { "x": 520.0, "y": 260.0 },
    "variant": "solid",
    "headSize": "small",
    "curve": "arc270",
    "headStyle": "full",
    "tailStyle": "none",
    "head": true,
    "tail": false,
    "bold": false,
    "noGo": "none"
  }
]
```

字段约定：

| 名称 | JSON 形态 | 说明 |
| --- | --- | --- |
| 点坐标 | `{ "x": 100.0, "y": 120.0 }` | 页面坐标 |
| 锚点 | `{ "x": 100.0, "y": 120.0, "nodeId": "n1" }` | `nodeId` 或 `objectId` 可选；没有 id 时按坐标创建或放置 |
| 目标集合 | `{ "nodes": [], "bonds": [], "objects": [], "labelNodes": [] }` | 用于移动、旋转、删除 |
| 文本 runs | `{ "text": "H", "script": "normal" }` | `script` 可为 `normal`、`subscript`、`superscript`、`chemical` |

坐标单位使用 ChemCore 文档坐标。`x` 向右增大，`y` 向下增大。

## 4. 执行报告、id 和内部 JSON

机器人调用 `new` 或 `run` 时应始终写 `--results`。`results.json` 是机器判断命令是否执行、是否修改文档、新建/更新/删除了哪些 id、失败原因、以及本次读写了哪些文件的主要依据。默认它是轻量审计报告，不是完整历史栈。

```powershell
npm run cli -- run input.cdxml commands.json --out output.cdxml --results results.json --document-json after.ccjs --pretty
```

### 4.1 顶层报告字段

`results.json` 是一个 object：

```json
{
  "ok": true,
  "commandCount": 1,
  "executedCount": 1,
  "failedIndex": null,
  "commands": [],
  "document": {
    "hashAlgorithm": "sha256",
    "hashInput": "chemcore-document-json-v1",
    "beforeHash": "64 hex chars",
    "afterHash": "64 hex chars",
    "hashChanged": true,
    "beforeRevision": 0,
    "afterRevision": 1
  },
  "io": {
    "operation": "run",
    "input": { "path": "input.cdxml" },
    "script": "commands.json",
    "output": { "path": "output.cdxml", "format": "cdxml" }
  },
  "documentJson": {
    "ok": true,
    "path": "after.ccjs",
    "format": "json"
  },
  "save": {
    "ok": true,
    "path": "output.cdxml",
    "format": "cdxml"
  }
}
```

字段含义：

| 字段 | 含义 |
| --- | --- |
| `ok` | 整个脚本是否全部成功。保存失败也会使它变成 `false` |
| `commandCount` | 脚本中命令总数 |
| `executedCount` | 成功进入 engine 并返回结果的命令数 |
| `failedIndex` | 失败命令的 0-based index；全部成功时为 `null` |
| `commands` | 每条命令的执行报告 |
| `document` | 脚本执行前后的文档 hash 和 revision。用它判断文档是否变化，不需要保存完整快照 |
| `io` | 本次调用的操作名、输入文件、命令脚本、输出文件 |
| `final` | 脚本结束后的可选 inspect 快照。只有显式使用 `--inspect-after` 时才有 |
| `documentJson` | `--document-json` 写出结果 |
| `save` | `--out` 写出结果 |
| `error` | 顶层失败原因；成功时没有该字段 |

CLI 失败时进程退出码为非 0，并在 stderr 打印错误；如果传了 `--results`，仍会尽量写出结构化报告。

### 4.2 单条命令报告字段

`commands[i]` 的形态：

```json
{
  "index": 0,
  "ok": true,
  "executed": true,
  "changed": true,
  "commandType": "add-bond",
  "command": {},
  "revision": 1,
  "beforeRevision": 0,
  "document": {
    "hashAlgorithm": "sha256",
    "hashInput": "chemcore-document-json-v1",
    "beforeHash": "64 hex chars",
    "afterHash": "64 hex chars",
    "hashChanged": true,
    "beforeRevision": 0,
    "afterRevision": 1
  },
  "changeSummary": {
    "createdCount": 3,
    "updatedCount": 1,
    "deletedCount": 0,
    "createdSelectors": {
      "objects": [],
      "nodes": ["node:n_1", "node:n_2"],
      "bonds": ["bond:b_3"],
      "styles": []
    },
    "updatedSelectors": { "objects": ["object:obj_editor_molecule"], "nodes": [], "bonds": [], "styles": [] },
    "deletedSelectors": { "objects": [], "nodes": [], "bonds": [], "styles": [] },
    "touchedSelectors": ["node:n_1", "node:n_2", "bond:b_3", "object:obj_editor_molecule"]
  },
  "targets": {},
  "created": {
    "nodes": ["n_1", "n_2"],
    "bonds": ["b_3"]
  },
  "updated": {
    "objects": ["obj_editor_molecule"]
  },
  "deleted": {},
  "diagnostics": {},
  "engineResult": {}
}
```

字段含义：

| 字段 | 含义 |
| --- | --- |
| `ok` | 这条命令是否成功 |
| `executed` | 是否成功进入 engine 并拿到 `engineResult` |
| `changed` | 命令是否改变文档。合法但没有造成变化时为 `false` |
| `commandType` | 原始命令的 `type` |
| `document` | 这条命令执行前后的文档 hash 和 revision |
| `changeSummary` | 以 selector 形式汇总的新建、更新、删除 id，主要给 agent 维护历史使用 |
| `created` | 新建的节点、键、scene object、style id |
| `updated` | 被修改的节点、键、scene object、style id |
| `deleted` | 被删除的节点、键、scene object、style id |
| `engineResult` | ChemCore engine 原始结果 |
| `after` | 这条命令执行后的可选 inspect 快照。只有显式使用 `--inspect-after` 时才有 |

判断规则：

| 情况 | 机器应如何判断 |
| --- | --- |
| `ok=true, executed=true, changed=true` | 命令执行成功，并修改了文档 |
| `ok=true, executed=true, changed=false` | 命令合法执行，但没有产生修改 |
| `ok=false, executed=false` | 命令没有成功执行；看 `error.message` |
| 顶层 `ok=false` 且 `save.skipped=true` | 脚本失败，目标 `--out` 没有保存 |

### 4.3 命令失败报告

失败命令示例：

```json
{
  "index": 1,
  "ok": false,
  "executed": false,
  "changed": false,
  "commandType": "add-bond",
  "command": {
    "type": "add-bond",
    "variant": "wrong-style"
  },
  "error": {
    "stage": "execute-command",
    "message": "unknown variant `wrong-style`, expected one of `single`, `double`, `triple`, `dashed`, `dashed-double`, `bold`, `bold-dashed`, `wavy`, `wedge`, `hashed-wedge`, `hollow-wedge`"
  }
}
```

常见 `error.stage`：

| stage | 含义 |
| --- | --- |
| `read-script` | 命令 JSON 文件无法读取或不是 object/array |
| `execute-command` | 单条命令字段错误、枚举值错误、缺字段，或命令需要当前没有的交互上下文 |
| `inspect-after` | 可选的命令后 inspect 失败 |
| `inspect-final` | 可选的脚本结束后 inspect 失败 |
| `write-document-json` | `--document-json` 写出失败 |
| `save-output` | `--out` 保存失败 |

脚本失败时，已经成功的前序命令会保留在内存文档中，并体现在 `document`、命令条目、以及按需写出的 `--document-json` 中；目标 `--out` 不会保存。

### 4.4 可选 after 快照

默认命令报告不包含 `after` 快照，顶层报告也不包含 `final`。这样大文件和长脚本的 results 不会迅速膨胀。CLI 只报告本次发生了什么；历史、回退和分支实验应由调用方或 agent 用 git、临时文件或自己的日志维护。

需要逐条命令的结构快照时，显式使用 `--inspect-after`：

```text
summary,objects,molecules
```

使用 `--inspect-after summary,objects,molecules` 后，分子修改结果可从这里读：

```text
commands[i].after.molecules
```

其中包含当前分子片段的节点、键、元素、坐标、标签：

```json
{
  "molecules": [
    {
      "objectId": "obj_editor_molecule",
      "resourceRef": "mol_editor",
      "nodeCount": 2,
      "bondCount": 1,
      "nodes": [
        {
          "id": "n_1",
          "element": "C",
          "atomicNumber": 6,
          "position": [100.0, 120.0],
          "label": null
        }
      ],
      "bonds": [
        {
          "id": "b_3",
          "begin": "n_1",
          "end": "n_2",
          "order": 1,
          "stereo": null
        }
      ]
    }
  ]
}
```

显式控制 after 内容：

```powershell
npm run cli -- run input.cdxml commands.json --results results.json --inspect-after summary,molecules
npm run cli -- run input.cdxml commands.json --results results.json --inspect-after summary,objects,molecules,styles
npm run cli -- run input.cdxml commands.json --results results.json --inspect-after none
```

`--no-inspect-after` 等价于 `--inspect-after none`。

### 4.5 获取对象 id

编辑已有对象时需要 id。id 从 `inspect`、`targets`、`results.commands[i].created` 或 `results.commands[i].changeSummary` 获取。只有显式使用 `--inspect-after` 时，才从 `results.commands[i].after` 获取。

创建时写 `--results`：

```powershell
npm run cli -- new commands.json --out output.cdxml --results results.json --pretty
```

创建节点、键或对象的命令会把新 id 记录在：

```text
commands[i].created.nodes
commands[i].created.bonds
commands[i].created.objects
```

读取已有文件时用 `inspect`：

```powershell
npm run cli -- inspect input.cdxml --include summary,objects,molecules --out inspect.json --pretty
```

`inspect.json` 中：

| section | 内容 |
| --- | --- |
| `summary` | 对象、分子、节点、键数量和页面范围 |
| `objects` | 文本、箭头、图形、括号、轨道等 scene object 的 id、类型、bbox、styleRef |
| `molecules` | 分子片段、节点 id、键 id、元素、坐标、标签 |
| `resources` | fragment/text/json 等资源摘要 |
| `styles` | 样式表摘要 |

### 4.6 读取内部 JSON

完整内部 JSON 有三种读法。

把已有文件转成内部 JSON：

```powershell
npm run cli -- convert input.cdxml input.ccjs
```

执行编辑时同时写内部 JSON：

```powershell
npm run cli -- run input.cdxml commands.json --out output.cdxml --results results.json --document-json after.ccjs --pretty
```

直接把编辑结果保存为内部 JSON：

```powershell
npm run cli -- run input.cdxml commands.json --out after.ccjs --results results.json --pretty
```

`--document-json` 适合调试，因为它可以和 `--out output.cdxml` 同时使用。脚本中途失败时，它会写出失败发生时内存里的 ChemCore 内部 JSON。

### 4.7 agent 的 target、context、detail、capture、copy 工作流

agent 需要精确 id、精确截图或周边对象信息时，按这个顺序走，不要猜坐标：

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
chemcore-cli context input.cdxml --target object:obj_shape_001 --radius 80 --out context.json --capture-out context.png --scale 5 --pretty
chemcore-cli detail input.cdxml --target object:obj_shape_001 --out detail.json --pretty
chemcore-cli capture input.cdxml --target object:obj_shape_001 --out object.png --scale 6 --expand-rel 0.15 --pretty
chemcore-cli copy input.cdxml --target object:obj_shape_001 --pretty
```

支持 target 的命令接受这些 selector：

```text
all
object:<scene-object-id>
molecule:<zero-based-molecule-index>
node:<node-id>
bond:<bond-id>
bounds:<minX>,<minY>,<maxX>,<maxY>
```

`bounds:` 用于截图类裁剪。`detail` 只接受单个 object、molecule、node 或 bond
selector；不接受 `all` 或 `bounds`。

`targets` 返回稳定 selector 和 bounds，按 `objects`、`molecules`、`nodes`、
`bonds` 分组。调用方不知道精确 selector 时，先跑 `targets`，再跑 `context`、
`detail`、`capture` 或 `copy`。

`context` 返回目标周边的对象摘要、分子摘要、节点摘要、键摘要、bounds、方向、
距离、重叠标记、group 祖先、子对象 id 和 link 元数据。`context` 只返回摘要。
需要原始 JSON 时，把返回的 selector 交给 `detail`。

`detail` 返回一个被选实体。默认包含该实体的 raw JSON。只需要 id、bounds 和关系
元数据时，加 `--summary-only`。查看对象并且需要嵌入引用资源时，加
`--include-resource`。

`capture` 把渲染后的裁剪图写入 `--out`，stdout 只输出 JSON manifest。省略
`--out` 时，会把 PNG 写到系统临时目录下的 `chemcore-cli` 子目录，并返回
`output.defaulted=true` 和真实的 `output.path`。SVG 是矢量。PNG 默认
`--scale 4`；需要更清晰或固定像素预算时，用 `--scale`、`--width` 或
`--height`。文件落盘校验通过后，manifest 会包含 `output.verified=true` 和
`output.bytes`。用绝对扩展（`--expand`、`--expand-left`、`--expand-right`、
`--expand-top`、`--expand-bottom`）和相对扩展（`--expand-rel`、
`--expand-rel-left`、`--expand-rel-right`、`--expand-rel-top`、
`--expand-rel-bottom`）把周边内容纳入截图。

`copy` 把可编辑 ChemCore Office/OLE payload 放到 Windows 剪贴板。调试剪贴板
payload 时，用 `--payload payload.json --no-copy`，这样只写 payload，不碰剪贴板。
省略 `--payload` 时，payload JSON 会写到系统临时目录下的 `chemcore-cli` 子目录。

## 5. 分子对象

### 5.1 创建单个原子

```json
{
  "type": "add-element",
  "symbol": "O",
  "atomic_number": 8,
  "center": { "x": 120.0, "y": 120.0 }
}
```

`add-element` 字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `symbol` | string | 元素符号，例如 `C`、`N`、`O`、`Cl` |
| `atomic_number` | number | 原子序数 |
| `center` | anchor | 放置坐标 |

### 5.2 创建键并自动创建端点碳原子

```json
{
  "type": "add-bond",
  "begin": { "x": 100.0, "y": 120.0 },
  "end": { "x": 140.0, "y": 120.0 },
  "order": 1,
  "variant": "single"
}
```

`add-bond` 字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `begin` | anchor | 起点；没有 `nodeId` 时按坐标创建碳原子 |
| `end` | anchor | 终点；没有 `nodeId` 时按坐标创建碳原子 |
| `order` | number | `1`、`2`、`3` |
| `variant` | string | 键样式 |

`variant` 可选值：

```text
single
double
triple
dashed
dashed-double
bold
bold-dashed
wavy
wedge
hashed-wedge
hollow-wedge
```

### 5.3 在已有原子之间加键

先从 `inspect` 或 `results` 得到节点 id，然后：

```json
{
  "type": "add-bond",
  "begin": { "nodeId": "node_a", "x": 100.0, "y": 120.0 },
  "end": { "nodeId": "node_b", "x": 140.0, "y": 120.0 },
  "order": 2,
  "variant": "double"
}
```

有 `nodeId` 时以节点为目标，`x/y` 仍要提供。

### 5.4 创建环模板

```json
{
  "type": "insert-template",
  "template": "benzene",
  "x": 300.0,
  "y": 260.0
}
```

`template` 可选值：

```text
ring-3
ring-4
ring-5
ring-6
ring-7
ring-8
benzene
chair-6-right
chair-6-left
```

直链结构用多条 `add-bond` 命令创建。

### 5.5 修改键样式

按键 id 修改：

```json
{
  "type": "apply-bond-style",
  "bondIds": ["bond_1"],
  "style": "double-center"
}
```

`style` 可选值：

```text
single-plain
single-dashed
single-hashed
single-hashed-wedged
single-bold
single-bold-wedged
single-hollow-wedged
single-wavy
double-left
double-right
double-center
double-bold
double-dashed
double-double-dashed
triple-plain
```

也可以使用较短别名：

```text
single
dashed
hashed
hashed-wedge
bold
wedge
hollow-wedge
wavy
double
triple
```

### 5.6 替换原子标签

```json
{
  "type": "replace-node-label",
  "node_id": "node_1",
  "label": "OH"
}
```

### 5.7 设置原子标签 runs

```json
{
  "type": "set-node-label-runs",
  "nodeId": "node_1",
  "runs": [
    { "text": "SO", "fontSize": 10.0, "script": "normal" },
    { "text": "3", "fontSize": 10.0, "script": "subscript" },
    { "text": "H", "fontSize": 10.0, "script": "normal" }
  ],
  "fontFamily": "Arial",
  "fontSize": 10.0,
  "fill": "#000000",
  "defaultChemical": true
}
```

### 5.8 修改原子标签样式

```json
{
  "type": "apply-text-style",
  "textObjectIds": [],
  "labelNodeIds": ["node_1"],
  "nodeIds": [],
  "command": "font-size",
  "value": "12"
}
```

`command` 可选值：

```text
font-family
font-size
align
line-height
bold
italic
underline
superscript
subscript
formula
```

`align` 的 `value` 可为 `left`、`center`、`right`、`justify`。开关型命令的 `value` 可用 `true`、`false`、`on`、`off`、`1`、`0`。

## 6. 箭头对象

### 6.1 创建箭头

```json
{
  "type": "add-arrow",
  "begin": { "x": 370.0, "y": 260.0 },
  "end": { "x": 520.0, "y": 260.0 },
  "variant": "solid",
  "headSize": "small",
  "curve": "arc270",
  "headStyle": "full",
  "tailStyle": "none",
  "head": true,
  "tail": false,
  "bold": false,
  "noGo": "none"
}
```

`add-arrow` 字段：

| 字段 | 可选值 |
| --- | --- |
| `variant` | `solid`、`curved`、`curved-mirror`、`hollow`、`open`、`equilibrium`、`unequal-equilibrium` |
| `headSize` | `large`、`medium`、`small` |
| `curve` | `arc270`、`arc180`、`arc120`、`arc90` |
| `headStyle` | `none`、`full`、`left`、`right` |
| `tailStyle` | `none`、`full`、`left`、`right` |
| `head` | boolean |
| `tail` | boolean |
| `bold` | boolean |
| `noGo` | `none`、`cross`、`hash` |

### 6.2 修改箭头几何

```json
{
  "type": "set-arrow-geometry",
  "objectId": "arrow_1",
  "begin": { "x": 360.0, "y": 260.0 },
  "end": { "x": 540.0, "y": 260.0 },
  "curve": 0.0,
  "headStyle": "full",
  "tailStyle": "none"
}
```

`curve` 是角度数值。直线箭头用 `0.0`。

### 6.3 修改箭头样式

```json
{
  "type": "apply-arrow-style",
  "objectIds": ["arrow_1"],
  "variant": "equilibrium",
  "headSize": "small",
  "curve": "arc270",
  "headStyle": "full",
  "tailStyle": "full",
  "head": true,
  "tail": true,
  "bold": false,
  "noGo": "none"
}
```

### 6.4 修改箭头线型

```json
{
  "type": "apply-line-style",
  "objectIds": ["arrow_1"],
  "style": "dashed"
}
```

`style` 可选值：

```text
plain
dashed
bold
```

## 7. 文本对象

### 7.1 创建普通文本

```json
{
  "type": "add-text",
  "position": { "x": 120.0, "y": 80.0 },
  "text": "Yield 85%",
  "fontFamily": "Arial",
  "fontSize": 10.0,
  "fill": "#000000",
  "align": "left",
  "lineHeight": 12.0,
  "box": [0.0, 0.0, 160.0, 14.0]
}
```

### 7.2 创建带上下标的文本

```json
{
  "type": "add-text",
  "position": { "x": 120.0, "y": 110.0 },
  "runs": [
    { "text": "H", "fontSize": 10.0, "script": "normal" },
    { "text": "2", "fontSize": 10.0, "script": "subscript" },
    { "text": "O", "fontSize": 10.0, "script": "normal" }
  ],
  "fontFamily": "Arial",
  "fontSize": 10.0,
  "fill": "#000000"
}
```

run 字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `text` | string | 文本片段 |
| `fontFamily` | string | 可选 |
| `fontSize` | number | 可选 |
| `fill` | string | 可选，例如 `#000000` |
| `fontWeight` | number | 可选，例如 `400`、`700` |
| `fontStyle` | string | 可选，例如 `normal`、`italic` |
| `underline` | boolean | 可选 |
| `script` | string | `normal`、`subscript`、`superscript`、`chemical` |

### 7.3 替换文本对象内容

```json
{
  "type": "set-text-runs",
  "objectId": "text_1",
  "runs": [
    { "text": "Fe", "script": "normal", "fontSize": 10.0 },
    { "text": "3+", "script": "superscript", "fontSize": 10.0 }
  ],
  "fontFamily": "Arial",
  "fontSize": 10.0,
  "fill": "#000000"
}
```

也可以直接用 `text`：

```json
{
  "type": "set-text-runs",
  "objectId": "text_1",
  "text": "Updated note",
  "fontSize": 11.0
}
```

### 7.4 修改文本样式

```json
{
  "type": "apply-text-style",
  "textObjectIds": ["text_1"],
  "labelNodeIds": [],
  "nodeIds": [],
  "command": "bold",
  "value": "true"
}
```

## 8. 图形对象

### 8.1 创建图形

```json
{
  "type": "add-shape",
  "kind": "rect",
  "style": "solid",
  "color": "#000000",
  "begin": { "x": 80.0, "y": 80.0 },
  "end": { "x": 180.0, "y": 140.0 }
}
```

`kind` 可选值：

```text
circle
ellipse
round-rect
rect
cross-table
tlc-plate
```

`style` 可选值：

```text
solid
dashed
shaded
filled
shadowed
```

### 8.2 修改图形几何

适用于 `circle`、`ellipse`、`rect`、`round-rect`。

```json
{
  "type": "set-shape-geometry",
  "objectId": "shape_1",
  "begin": { "x": 90.0, "y": 90.0 },
  "end": { "x": 210.0, "y": 150.0 }
}
```

对 `circle`、`ellipse`，`begin` 是中心，`end` 是主轴端点。对 `rect`、`round-rect`，`begin` 和 `end` 是包围框对角点。

### 8.3 修改图形样式

```json
{
  "type": "apply-shape-style",
  "objectIds": ["shape_1"],
  "style": "shadowed"
}
```

`style` 可选值：

```text
plain
dashed
filled
shaded
faded
shadowed
```

## 9. 括号和符号对象

### 9.1 创建括号

```json
{
  "type": "add-bracket",
  "kind": "square",
  "begin": { "x": 100.0, "y": 100.0 },
  "end": { "x": 180.0, "y": 160.0 }
}
```

### 9.2 创建符号

```json
{
  "type": "add-symbol",
  "kind": "circle-plus",
  "center": { "x": 220.0, "y": 120.0 }
}
```

`kind` 可选值：

```text
round
square
curly
double-dagger
dagger
circle-plus
plus
radical-cation
lone-pair
circle-minus
minus
radical-anion
electron
```

### 9.3 修改括号类型

```json
{
  "type": "apply-bracket-kind",
  "objectIds": ["bracket_1"],
  "kind": "curly"
}
```

`apply-bracket-kind` 可用 `round`、`square`、`curly`。

## 10. 轨道对象

### 10.1 创建轨道

```json
{
  "type": "add-orbital",
  "template": "p",
  "style": "hollow",
  "phase": "plus",
  "color": "#000000",
  "center": { "x": 300.0, "y": 120.0 },
  "end": { "x": 340.0, "y": 120.0 }
}
```

字段可选值：

| 字段 | 可选值 |
| --- | --- |
| `template` | `s`、`p`、`dxy`、`oval`、`hybrid`、`dz2`、`lobe` |
| `style` | `hollow`、`shaded`、`filled` |
| `phase` | `plus`、`minus` |

### 10.2 修改轨道模板

```json
{
  "type": "apply-orbital-template",
  "objectIds": ["orbital_1"],
  "template": "dxy"
}
```

### 10.3 修改轨道样式

```json
{
  "type": "apply-orbital-style",
  "objectIds": ["orbital_1"],
  "style": "filled"
}
```

### 10.4 修改轨道相位

```json
{
  "type": "apply-orbital-phase",
  "objectIds": ["orbital_1"],
  "phase": "minus"
}
```

## 11. 通用目标编辑

### 11.1 移动对象、节点、键

```json
{
  "type": "move-targets",
  "targets": {
    "nodes": ["node_1"],
    "bonds": [],
    "objects": ["text_1", "arrow_1"],
    "labelNodes": []
  },
  "delta": { "dx": 10.0, "dy": -5.0 }
}
```

### 11.2 旋转对象、节点、键

```json
{
  "type": "rotate-targets",
  "targets": {
    "nodes": ["node_1", "node_2"],
    "bonds": ["bond_1"],
    "objects": ["arrow_1"],
    "labelNodes": []
  },
  "center": { "x": 200.0, "y": 200.0 },
  "degrees": 30.0
}
```

### 11.3 删除对象、节点、键

```json
{
  "type": "delete-targets",
  "targets": {
    "nodes": ["node_1"],
    "bonds": ["bond_1"],
    "objects": ["text_1"],
    "labelNodes": []
  }
}
```

目标集合字段：

| 字段 | 目标 |
| --- | --- |
| `nodes` | 分子节点 |
| `bonds` | 分子键 |
| `objects` | scene object，例如文本、箭头、图形、括号、符号、轨道 |
| `labelNodes` | 原子标签节点 |

## 12. 排列、分组和层级

### 12.1 调整层级

```json
{
  "type": "apply-selection-order",
  "objectIds": ["arrow_1", "text_1"],
  "command": "bring-front"
}
```

`command` 可选值：

```text
bring-front
send-back
bring-forward
send-backward
front
back
forward
backward
```

### 12.2 分组

```json
{
  "type": "group-selection",
  "object_ids": ["arrow_1", "text_1"]
}
```

### 12.3 取消分组

```json
{
  "type": "ungroup-selection",
  "object_ids": ["group_1"]
}
```

### 12.4 链接和取消链接

```json
{
  "type": "link-selection",
  "object_ids": ["bracket_1", "text_1"]
}
```

```json
{
  "type": "unlink-selection",
  "object_ids": ["bracket_1", "text_1"]
}
```

## 13. 文档样式和对象设置

### 13.1 应用文档样式预设

```json
{
  "type": "apply-document-style",
  "preset": "acs-document-1996"
}
```

`preset` 可选值：

```text
default
acs-document-1996
```

### 13.2 设置默认对象参数

```json
{
  "type": "apply-object-settings",
  "settings": {
    "bondLength": 14.4,
    "lineWidth": 0.6,
    "boldWidth": 2.0,
    "bondSpacing": 18.0,
    "marginWidth": 1.6,
    "hashSpacing": 2.5
  }
}
```

### 13.3 对指定对象应用对象参数

```json
{
  "type": "apply-object-settings-to-selection",
  "bond_ids": ["bond_1"],
  "object_ids": ["arrow_1", "shape_1"],
  "settings": {
    "bondLength": 14.4,
    "lineWidth": 0.6,
    "boldWidth": 2.0,
    "bondSpacing": 18.0,
    "marginWidth": 1.6,
    "hashSpacing": 2.5
  }
}
```

`settings` 字段都可以只传需要修改的项。

## 14. 文档读写命令脚本

CLI 子命令已经覆盖大多数文件读写。需要在 JSON 命令脚本中读取导出结果时，可以使用只读命令。

检查当前文档：

```json
{
  "type": "inspect-document",
  "include": ["summary", "objects", "molecules"]
}
```

导出当前文档内容：

```json
{
  "type": "export-document",
  "format": "svg"
}
```

脚本内格式转换：

```json
{
  "type": "convert-document",
  "from": "cdxml",
  "to": "json",
  "content": "<CDXML>...</CDXML>"
}
```

`format`、`from`、`to` 可选值：

```text
json
ccjs
cdxml
cdx
sdf
svg
```

## 15. 从空白文档生成苯环和箭头

`commands.json`：

```json
[
  {
    "type": "insert-template",
    "template": "benzene",
    "x": 300.0,
    "y": 260.0
  },
  {
    "type": "add-arrow",
    "begin": { "x": 370.0, "y": 260.0 },
    "end": { "x": 520.0, "y": 260.0 },
    "variant": "solid",
    "headSize": "small",
    "curve": "arc270",
    "headStyle": "full",
    "tailStyle": "none",
    "head": true,
    "tail": false,
    "bold": false,
    "noGo": "none"
  }
]
```

保存到桌面 CDXML：

```powershell
npm run cli -- new commands.json --out "$env:USERPROFILE\Desktop\benzene-arrow.cdxml" --results results.json --pretty
```

检查：

```powershell
npm run cli -- inspect "$env:USERPROFILE\Desktop\benzene-arrow.cdxml" --include summary,objects,molecules --pretty
```

## 16. 编辑已有文件的标准流程

第一步，读取摘要、稳定 selector 和 bounds：

```powershell
npm run cli -- inspect input.cdxml --include summary,objects,molecules --out before.json --pretty
npm run cli -- targets input.cdxml --out targets.json --pretty
```

如果编辑依赖周边对象，先看周边，再展开一个 selector：

```powershell
npm run cli -- context input.cdxml --target object:arrow_1 --radius 80 --out context.json --capture-out context.png --scale 5 --pretty
npm run cli -- detail input.cdxml --target object:arrow_1 --out detail.json --pretty
```

第二步，写编辑脚本：

```json
[
  {
    "type": "apply-document-style",
    "preset": "acs-document-1996"
  },
  {
    "type": "apply-bond-style",
    "bondIds": ["bond_1"],
    "style": "double-center"
  },
  {
    "type": "set-arrow-geometry",
    "objectId": "arrow_1",
    "begin": { "x": 360.0, "y": 260.0 },
    "end": { "x": 540.0, "y": 260.0 },
    "curve": 0.0,
    "headStyle": "full",
    "tailStyle": "none"
  },
  {
    "type": "set-text-runs",
    "objectId": "text_1",
    "text": "Updated condition",
    "fontSize": 10.0
  }
]
```

第三步，执行并保存：

```powershell
npm run cli -- run input.cdxml edit.json --out output.cdxml --results edit-results.json --pretty
```

第四步，再检查：

```powershell
npm run cli -- inspect output.cdxml --include summary,objects,molecules --out after.json --pretty
```
