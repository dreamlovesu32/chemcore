# ChemSema CLI 命令指南

这份文档说明 `chemsema-cli` 的直接用法：打开文件、创建对象、编辑对象、检查结果，以及从命令错误中恢复。

## 1. 启动 CLI

在仓库根目录运行：

```powershell
npm run cli -- <command> [args...]
```

等价于：

```powershell
cargo run -p chemsema-cli -- <command> [args...]
```

编译后也可以直接运行：

```powershell
target\debug\chemsema-cli.exe <command> [args...]
```

查看帮助：

```powershell
npm run cli -- --help
```

如果 Windows PowerShell 因执行策略拦截 `npm.ps1`，把示例里的 `npm` 换成 `npm.cmd`：

```powershell
npm.cmd run cli -- --help
```

桌面端安装版会把 `chemsema-cli.exe` 和 GUI 一起安装，并随安装包携带英文详细指南
`chemsema-cli-guide.md`。安装器会把 CLI 目录加入 PATH。安装后打开新的终端，
从这些命令开始：

```powershell
chemsema-cli guide --pretty
chemsema-cli guide --kind detailed --pretty
chemsema-cli version --pretty
chemsema-cli doctor --pretty
chemsema-cli capabilities --pretty
```

`--pretty` 会把紧凑单行 JSON 格式化为带换行和缩进的 JSON。字段、值、输出文件、退出码、schema、排序和命令行为保持一致。默认 JSON 是紧凑单行 JSON。

## 调用模式

ChemSema CLI 有两种调用模式。

当每个操作都可以启动一个进程、读取输入文件、写输出文件、打印一次 JSON 结果并退出时，用 PowerShell 单命令模式。这是独立检查、转换、导出、复制、精确截图，或单次 `new`/`run` 编辑批处理的最简单模式。单命令模式是无状态的：编辑通过显式输出路径写出，例如 `--out`、`--results` 或 `--document-json`。

```powershell
chemsema-cli targets input.cdxml --out targets.json --pretty
chemsema-cli capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --pretty
chemsema-cli run input.cdxml commands.json --out edited.cdxml --results results.json --pretty
```

当很多操作都针对同一个文档时，用 JSONL session。用 `chemsema-cli session [input]` 启动一个长驻进程，然后向 stdin 每行写一个 JSON 请求，从 stdout 每行读一个 JSON 响应。session 会把文档保持在内存里，反复执行 `targets`、`detail`、`context`、`capture`、`execute` 和 `save` 时复用同一份已载入文档。

```powershell
chemsema-cli session input.cdxml
```

```jsonl
{"id":1,"op":"targets"}
{"id":2,"op":"capture","target":"molecule:0","out":"molecule.png","width":1800}
{"id":3,"op":"save","out":"edited.cdxml"}
{"id":4,"op":"exit"}
```

自动 CDXML/CDX 导入缓存属于单命令模式。它把归一化导入文档存到磁盘，让重复的单命令调用复用导入结果。JSONL session 是同一个大文件长时间迭代时的调用模式。

## 2. 文件命令

打开文件就是把文件路径作为 `inspect`、`run`、`convert` 或 `export` 的输入参数。

```text
chemsema-cli --version
chemsema-cli version [--pretty] [--out <path>]
chemsema-cli guide [--kind agent|detailed|all] [--include-content] [--pretty] [--out <path>]
chemsema-cli about [--pretty] [--out <path>]
chemsema-cli capabilities [--pretty] [--out <path>]
chemsema-cli doctor [--pretty] [--out <path>]
chemsema-cli examples [basic|capture-copy|all] [--pretty] [--out <path>]
chemsema-cli schema [protocol|commands|targets|capture|context|bundle|detail|diff|guide|copy|json-output|command-script|command-transaction|all] [--pretty] [--out <path>]
chemsema-cli inspect <input> [--include summary,objects,molecules,resources,styles] [--out <path>] [--pretty]
chemsema-cli targets <input> [--out <path>] [--pretty]
chemsema-cli context <input> --target <selector> [--target <selector> ...] [--targets <selector;selector>] [--radius <pt>] [--out <context.json>] [--capture-out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--pretty]
chemsema-cli bundle <input> --target <selector> [--target <selector> ...] [--targets <selector;selector>] --out-dir <directory> [--context-radius <pt>] [--capture-format png|svg] [--capture-width <px>] [--capture-height <px>] [--capture-scale <n>] [--subset-format ccjs|ccjz|cdxml|cdx|sdf] [--pretty]
chemsema-cli detail <input> --target <object:id|molecule:index|node:id|bond:id> [--summary-only] [--include-resource] [--out <detail.json>] [--pretty]
chemsema-cli capture <input> --target <selector> [--target <selector> ...] [--targets <selector;selector>] [--selection-only] [--crop-bounds <minX,minY,maxX,maxY>] [--out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--expand <pt>] [--expand-rel <fraction>] [--pretty]
chemsema-cli copy <input> [--target <selector>] [--payload <payload.json>] [--no-copy] [--pretty]
chemsema-cli session [input]
chemsema-cli new [commands.json|-] --out <path> [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]
chemsema-cli run <input> <commands.json|-> [--out <path>] [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]
chemsema-cli convert <input> <output> [--format <format>] [--scale <n>|--width <px>|--height <px>]
chemsema-cli export <input> <output> [--format <format>] [--scale <n>|--width <px>|--height <px>]
chemsema-cli diff <before> <after> --out <diff.json> [--pretty]
chemsema-cli label-query --text <source-label> [--connection-angle <deg> ...] [--connection-count <n>] [--display-mode connection-auto|right-auto|left-auto|preserve-right|preserve-left|preserve-center] [--no-default-chemical] [--pretty]
chemsema-cli label-query --visible-text <visible-label> [--connection-angle <deg> ...] [--connection-count <n>] [--display-mode connection-auto|right-auto|left-auto|preserve-right|preserve-left|preserve-center] [--pretty]
```

常用调用：

```powershell
npm run cli -- inspect input.cdxml --include summary,objects,molecules --out inspect.json --pretty
npm run cli -- targets input.cdxml --out targets.json --pretty
npm run cli -- capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --expand-rel 0.15 --pretty
npm run cli -- capture input.cdxml --target object:obj_text_1 --selection-only --crop-bounds 0,0,800,600 --out text-layer.png --width 4800 --height 3600 --pretty
npm run cli -- new commands.json --out output.cdxml --results results.json --pretty
npm run cli -- run input.cdxml commands.json --out output.cdxml --results results.json --document-json after.ccjs --pretty
npm run cli -- convert input.cdxml output.svg
npm run cli -- convert input.cdxml output.png --scale 6
npm run cli -- convert input.cdxml output.emf
npm run cli -- convert input.cdxml output.ccjs
npm run cli -- bundle input.cdxml --target molecule:0 --out-dir molecule-0-bundle --context-radius 40 --capture-format png --subset-format ccjs --pretty
npm run cli -- diff before.ccjs after.ccjs --out diff.json --pretty
```

标签查询示例：

```powershell
npm run cli -- label-query --text CF3 --connection-angle 0 --pretty
npm run cli -- label-query --visible-text H2N --connection-angle 0 --pretty
npm run cli -- label-query --visible-text CF3 --connection-angle 0 --pretty
npm run cli -- label-query --visible-text "(II)Cu" --display-mode right-auto --connection-angle 93 --connection-angle 17 --connection-angle -30 --pretty
```

`label-query --text` 是正向 contract：ChemSema 接收 source text、连接几何
和 `defaultChemical`，返回该标签是否 accepted、最终 display text、source
runs、生成氢是否可作为键锚点，以及 functional-group recognition 元数据。
`label-query --visible-text` 是给导入图或外部测量图使用的反向 contract：ChemSema
先用同一套 label group 反写规则从可见文本生成 source 候选，再逐个调用正常
label engine，并推荐 display 能回到可见文本的候选。如果没有任何 chemical
source 候选既合法又能渲染回可见文本，反向报告会推荐
`defaultChemical:false` 的 plain-text 候选，让调用方尊重源文档排版，而不是强行化学化。

正向和反向报告都会给出 `commandFields`，这是可直接复制到
`set-node-label-runs` 的字段集合，包括 source/edit `text`、`sourceText`、
source `runs`、`defaultChemical`，以及必要时的 `displayMode`。OCR 调用方应优先
使用这个对象，不要再从 display/recognition 字段手工拼命令。

`--display-mode` 用来区分普通 endpoint 命令布局和 ChemDraw 导入对齐语义。默认
`connection-auto` 按传入连接角度走普通 endpoint label flow；`right-auto`
表示 CDXML/CDX 中没有 `LabelDisplay` 的 `LabelAlignment=Right`，ChemSema 可以
按化学 group 反写显示，同时把原始第一组保持为右端锚点。`preserve-right`、
`preserve-left`、`preserve-center` 表示强制保留可见显示。若自动反写 source 和
强制保留 source 产生相同可见文本和相同锚点策略，像素无法恢复隐藏 source 选择；
调用方应显式保留这个歧义，而不是编造不可观测的区别。

文件输出规则：

- `capture` 可以省略 `--out`；此时会把 PNG 写到系统临时目录下的 `chemsema-cli` 子目录，在 `output.path` 返回真实路径，并给出 `default_output_path` warning。
- `copy` 可以省略 `--payload`；此时会把剪贴板 payload JSON 写到系统临时目录下的 `chemsema-cli` 子目录，在 `payload.path` 返回真实路径，并给出 `default_payload_path` warning。
- `new`、`convert`、`export` 会创建主要文档文件，需要显式给输出路径。
- 所有写文件命令都会在写完后检查目标是否存在、是否为普通文件、以及字节数是否符合预期或最低要求。校验失败就是命令失败。

对象级 bundle 和 diff：

- `bundle` 会写出 `manifest.json`、`target.json`、`context.json`、`editable-subset.<format>`、`capture.png` 或 `capture.svg`、`identity-map.json`、以及 `provenance.json`。
- `manifest.json` 明确区分 `editableScope` 和 `visualScope`。视觉上下文可以包含附近的非目标对象；只有 editable scope 是允许修改的范围。
- `context.json` 保留 `selectionBoxRelation` 和 `isTarget`，调用方可以区分明确选中的对象和只是出现在截图里的邻居。
- `diff` 按 object/resource/style/node/bond 身份比较两个可编辑文档，报告创建、更新、删除和字段级变化。

事务式编辑：

- `new`、`run` 和 JSONL session `execute` 除了普通 command object/array，也接受 `chemsema.command-transaction.v1` envelope。
- Transaction 会检查可选的 document hash/revision 前置条件，在 clone 的 engine state 上执行命令，用结构化 diff 校验 `scope.editableTargets`，全部通过后才替换原状态。
- `dryRun: true` 会报告执行结果、diff、允许 selector 和 unexpected changes，但不会修改当前打开的文档，也不会写出文档输出文件。

导入缓存规则：

- CDXML/CDX 输入会自动使用归一化文档导入缓存，加速重复 CLI 调用。缓存键包含源文件内容、格式、CLI 版本和可执行文件标记；文件变化或重新构建后的二进制会自动 miss。
- 用 `CHEMSEMA_CLI_DISABLE_CACHE=1` 关闭导入缓存。用 `CHEMSEMA_CLI_CACHE_DIR=<path>` 指定缓存目录。`chemsema-cli doctor --pretty` 会报告当前生效的缓存设置。

错误输出规则：

- 错误 JSON 包含 `error.kind`、`message`、`hint`、`fix`、`usage`、`examples` 和 `suggestions`。
- 缺参数错误使用 `error.kind="missing_argument"`，并包含 `error.fix.action="provide_required_argument"`，以及机器可读的 `missing` 和 `expected` 字段。
- `error.fix` 是主要修复对象，`usage` 和 `examples` 提供命令上下文。

协议 contract：

- `chemsema-cli --version` 输出一行纯文本，适合 shell 检查。
- `chemsema-cli version --pretty` 以 JSON 返回产品版本和协议版本。
- `chemsema-cli schema protocol --pretty` 返回当前运行时的协议 id。
- 面向机器的稳定 contract 放在 [docs/protocol](./protocol/README.md)。

`new` 从空白 ChemSema 内部文档开始。命令接收命令脚本和输出路径。保存格式由 `--out` 后缀决定：

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
npm run cli -- convert input.cdxml output --format png --width 1800
npm run cli -- convert input.cdxml output --format emf
```

支持格式：

| 格式 | 读入 | 写出 | 用途 |
| --- | --- | --- | --- |
| `json` | yes | yes | ChemSema 内部 JSON，`.json` 后缀按内部 JSON 处理 |
| `ccjs` | yes | yes | ChemSema 内部 JSON，推荐作为未压缩内部格式 |
| `ccjz` | yes | yes | gzip 压缩 ChemSema JSON |
| `cdxml` | yes | yes | ChemDraw XML |
| `cdx` | yes | yes | ChemDraw binary |
| `sdf` | yes | yes | MDL SD file |
| `svg` | no | yes | 矢量导出 |
| `png` | no | yes | 位图导出，默认 `--scale 10`；可用 `--scale`、`--width` 或 `--height` 控制分辨率 |
| `emf` | no | 仅 Windows | 直接导出 Enhanced Metafile 矢量文件；与 Office/OLE 预览共用同一绘制器 |

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

坐标单位使用 ChemSema 文档坐标。`x` 向右增大，`y` 向下增大。

## 4. 执行报告、id 和内部 JSON

机器人调用 `new` 或 `run` 时传入 `--results`。`results.json` 是机器判断命令是否执行、是否修改文档、新建/更新/删除了哪些 id、失败原因、以及本次读写了哪些文件的主要依据。默认它是轻量审计报告。

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
    "hashInput": "chemsema-document-json-v1",
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
| `document` | 脚本执行前后的文档 hash 和 revision，用于在保持报告精简的同时判断文档是否变化 |
| `io` | 本次调用的操作名、输入文件、命令脚本、输出文件 |
| `final` | 脚本结束后的可选 inspect 快照，使用 `--inspect-after` 时出现 |
| `documentJson` | `--document-json` 写出结果 |
| `save` | `--out` 写出结果 |
| `error` | 顶层失败原因；成功时省略该字段 |

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
    "hashInput": "chemsema-document-json-v1",
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
| `engineResult` | ChemSema engine 原始结果 |
| `after` | 这条命令执行后的可选 inspect 快照，使用 `--inspect-after` 时出现 |

判断规则：

| 情况 | 机器应如何判断 |
| --- | --- |
| `ok=true, executed=true, changed=true` | 命令执行成功，并修改了文档 |
| `ok=true, executed=true, changed=false` | 命令合法执行，但没有产生修改 |
| `ok=false, executed=false` | 命令没有成功执行；看 `error.message` |
| 顶层 `ok=false` 且 `save.skipped=true` | 脚本失败，`--out` 保存被跳过 |

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
| `read-script` | 命令 JSON 读取或解析阶段拒绝脚本形状 |
| `execute-command` | 单条命令字段错误、枚举值错误、缺字段，或命令需要当前没有的交互上下文 |
| `inspect-after` | 可选的命令后 inspect 失败 |
| `inspect-final` | 可选的脚本结束后 inspect 失败 |
| `write-document-json` | `--document-json` 写出失败 |
| `save-output` | `--out` 保存失败 |

脚本失败时，已经成功的前序命令会保留在内存文档中，并体现在 `document`、命令条目、以及按需写出的 `--document-json` 中；目标 `--out` 保存会报告 `save.skipped=true`。

### 4.4 可选 after 快照

默认命令报告包含变化摘要。`--inspect-after` 会添加每条命令的 `after` 快照和顶层 `final` 快照。CLI 报告本次发生了什么；历史、回退和分支实验可由调用方或 agent 用 git、临时文件或自己的日志维护。

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

编辑已有对象时需要 id。id 从 `inspect`、`targets`、`results.commands[i].created` 或 `results.commands[i].changeSummary` 获取。使用 `--inspect-after` 时，`results.commands[i].after` 也包含命令后快照里的 id。

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

`--document-json` 适合调试，因为它可以和 `--out output.cdxml` 同时使用。脚本中途失败时，它会写出失败发生时内存里的 ChemSema 内部 JSON。

### 4.7 agent 的 target、context、detail、capture、copy 工作流

agent 需要精确 id、精确截图或周边对象信息时，使用这个 selector 工作流：

```powershell
chemsema-cli targets input.cdxml --out targets.json --pretty
chemsema-cli context input.cdxml --target object:obj_shape_001 --radius 80 --out context.json --capture-out context.png --scale 5 --pretty
chemsema-cli detail input.cdxml --target object:obj_shape_001 --out detail.json --pretty
chemsema-cli capture input.cdxml --target object:obj_shape_001 --out object.png --scale 6 --expand-rel 0.15 --pretty
chemsema-cli copy input.cdxml --target object:obj_shape_001 --pretty
```

支持 target 的命令接受这些 selector：

```text
all
object:<scene-object-id>
molecule:<zero-based-molecule-index>
node:<node-id>
bond:<bond-id>
bounds:<minX>,<minY>,<maxX>,<maxY>
selection:<selector;selector>
```

`bounds:` 用于截图类裁剪。`detail` 接受单个 `object:<id>`、`molecule:<index>`、`node:<id>` 或 `bond:<id>` selector。
`capture` 和 `context` 可以用重复 `--target`、`--targets <selector;selector>`、`selection:<selector;selector>`，或 JSONL session 的 `target`/`targets` 数组传入多个目标。目标框是这些目标 bounds 的最小并集，和 GUI 选择框一致。

`targets` 返回稳定 selector 和 bounds，按 `objects`、`molecules`、`nodes`、
`bonds` 分组。它是 `context`、`detail`、`capture` 和 `copy` 前面的发现步骤。

`context` 返回目标周边的对象摘要、分子摘要、节点摘要、键摘要、bounds、方向、
距离、重叠标记、selection-box relation、group 祖先、子对象 id 和 link 元数据。
`selectionBox.contents` 列出目标框内的项目；每项包含 `selectionBoxRelation="inside"`
或 `"partial"`，并且只有显式选中的目标才有 `isTarget=true`。需要原始 JSON 时，把返回的 selector 交给 `detail`。

`detail` 返回一个被选实体。默认包含该实体的 raw JSON。需要 id、bounds 和关系
元数据摘要时，加 `--summary-only`。查看对象并且需要嵌入引用资源时，加
`--include-resource`。

`capture` 把渲染后的裁剪图写入 `--out`，stdout 输出 JSON manifest。多个目标会按最小并集目标框裁剪，图中显示这个框里可见的内容以及指定扩展范围。省略
`--out` 时，会把 PNG 写到系统临时目录下的 `chemsema-cli` 子目录，并返回
`output.defaulted=true`、真实的 `output.path`，以及 `kind="default_output_path"` 的
`warnings[]` 条目。SVG 是矢量。PNG 默认
`--scale 10`；需要更清晰或固定像素预算时，用 `--scale`、`--width` 或
`--height`。文件落盘校验通过后，manifest 会包含 `output.verified=true` 和
`output.bytes`。用绝对扩展（`--expand`、`--expand-left`、`--expand-right`、
`--expand-top`、`--expand-bottom`）和相对扩展（`--expand-rel`、
`--expand-rel-left`、`--expand-rel-right`、`--expand-rel-top`、
`--expand-rel-bottom`）把周边内容纳入截图。

截图 manifest 还包含 `render.mode`、`render.primitiveCount` 和
`render.targets`。这些字段说明本次裁剪如何渲染，以及截图范围内包含了多少
node、bond 和 object 目标。`context` 使用 `--capture-out` 写截图时，会在
`capture.render` 下返回同样的字段。

`copy` 把可编辑 ChemSema Office/OLE payload 放到 Windows 剪贴板。调试剪贴板
payload 时，用 `--payload payload.json --no-copy` 写出 payload JSON 文件。
省略 `--payload` 时，payload JSON 会写到系统临时目录下的 `chemsema-cli` 子目录。
同时会给出 `kind="default_payload_path"` 的 `warnings[]` 条目。

`session` 启动一个给 agent 使用的长驻 JSON Lines 进程。第一行 stdout 是紧凑的
`ready` event；之后每输入一行紧凑 JSON request，就读取一行紧凑 JSON response。
session 会把一个文档保持在内存里，所以反复执行 `targets`、`detail`、`context`、
`capture`、`execute` 和 `save` 时复用同一份已载入文档。`execute` 会返回
before/after revision 和每条命令结果，调用方可以用 git、临时文件或自己的日志维护历史。

```json
{"id":1,"op":"open","input":"input.cdxml"}
{"id":2,"op":"targets"}
{"id":3,"op":"capture","target":"molecule:0","out":"molecule.png","width":1800}
{"id":4,"op":"execute","command":{"type":"add-text","position":{"x":40,"y":40},"text":"note"}}
{"id":5,"op":"save","out":"edited.ccjs"}
{"id":6,"op":"exit"}
```

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
  "preserveImplicitHydrogenLabel": false,
  "defaultChemical": true
}
```

`preserveImplicitHydrogenLabel` 可选。对 `NH2` 这类端点元素氢标签设为
`true` 时，ChemSema 会把源文本视为用户/导入文档明确保留的标签；即使
当前键级推导出的隐式氢数量通常会把它刷新成 `NH`，也会继续保留源文本。

### 5.8 设置原子标签是否按化学解释

```json
{
  "type": "select-targets",
  "targets": { "nodes": ["node_1"] }
}
```

```json
{
  "type": "set-interpret-chemically-for-selection",
  "enabled": false
}
```

这对应 ChemDraw 风格的 `Interpret Chemically`。关闭后，ChemSema 会用
`defaultChemical:false` 保留普通文本，把 label source runs 改成 normal，
并清除化学识别诊断。它不是隐式氢显示开关。

### 5.9 覆盖隐式氢数量

```json
{
  "type": "select-targets",
  "targets": { "nodes": ["node_1"] }
}
```

```json
{
  "type": "set-implicit-hydrogen-count-for-selection",
  "count": 0
}
```

使用 `count:0` 可以隐藏自动生成的氢，同时保留原子继续按化学解释。使用
`count:null` 可以让选中节点回到按价态自动推导隐式氢。

### 5.10 设置原子电荷

```json
{
  "type": "set-node-charge",
  "nodeId": "node_1",
  "charge": 1
}
```

当调用方已经拥有显式或推断出的形式电荷语义时使用，包括导入图或外部测量图：
可见标签例如 `NH2` 在恢复出的键级下让氮达到 4 价，因此应写入正电荷。
ChemSema 会在更新电荷后刷新隐式氢、标签识别和附着标签几何。

### 5.11 修改原子标签样式

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
outline
shadow
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
| `outline` | boolean | 可选 |
| `shadow` | boolean | 可选 |
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

### 8.4 修改原生光谱数据

一次性替换已有 `spectrum` 对象的全部语义字段，命令支持撤销。

```json
{
  "type": "set-spectrum-data",
  "objectId": "obj_spectrum_1",
  "spectrum": {
    "class": "nmr",
    "xLow": 0,
    "xSpacing": 0.5,
    "xType": "parts-per-million",
    "xAxisLabel": "ppm",
    "yLow": 0,
    "yScale": 1,
    "yType": "arbitrary-units",
    "yAxisLabel": "intensity",
    "dataPoints": [0, 0.2, 1, 0.3, 0]
  }
}
```

目前没有光谱预测命令，也没有预测右键菜单入口。

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

命令脚本在单次调用模式（`new`/`run`）和 JSONL session 模式（`session` +
`execute`）中使用同一套 JSON 命令。选择命令会修改当前内存中的选区，供同一
脚本或同一 session 后续命令使用；选择命令本身不会修改文档 revision。

### 11.1 选择状态

选择明确目标：

```json
{
  "type": "select-targets",
  "targets": {
    "nodes": ["node_1"],
    "bonds": ["bond_1"],
    "objects": ["text_1", "arrow_1"],
    "labelNodes": []
  }
}
```

传一个目标就是单选，传多个目标就是多选。全选当前可见/可编辑内容：

```json
{ "type": "select-all" }
```

清空当前选区：

```json
{ "type": "clear-selection" }
```

执行 `select-targets` 或 `select-all` 后，GUI 风格的选择命令可以省略 id 数组，
直接作用在当前选区上。这包括 `apply-selection-arrange`、`scale-selection`、
`center-selection-on-page`、`apply-selection-color`、`apply-selection-order`、
`group-selection`、`ungroup-selection`、`link-selection`、`set-link-policy`、
`unlink-selection`、`paste-analysis-caption`、`apply-chemical-property`、
`apply-text-style`、`apply-bond-style`、`apply-line-style`、图形/括号/轨道样式命令、
`apply-object-settings-to-selection`、`delete-selection` 和 `cut-selection`。

对当前选中的完整单分子创建标准 ChemicalProperty：

```json
{
  "type": "apply-chemical-property",
  "property_type": { "code": 1, "name": "ChemicalName" },
  "value": "ethane",
  "isActive": true
}
```

传入 `propertyId` 时编辑已有属性。删除使用
`{"type":"delete-chemical-property","propertyId":"chemical_property_30"}`。
完整类型、活动状态、重新计算和 CDX/CDXML 边界见
`chemical-property-model.zh-CN.md`。

### 11.2 移动对象、节点、键

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

### 11.3 旋转对象、节点、键

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

### 11.4 缩放或拉伸目标

```json
{
  "type": "scale-targets",
  "targets": {
    "nodes": ["node_1", "node_2"],
    "bonds": ["bond_1"],
    "objects": ["text_1"]
  },
  "scaleX": 1.25,
  "scaleY": 0.8,
  "pivot": { "x": 200.0, "y": 200.0 }
}
```

省略 `pivot` 时，会以目标边界框中心为缩放中心。`scaleX` 和 `scaleY` 相同
时是等比缩放，不同则是脚本化拉伸。

### 11.5 删除对象、节点、键

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

### 12.1 排列当前选区

```json
[
  {
    "type": "select-targets",
    "targets": { "objects": ["text_1", "text_2", "text_3"] }
  },
  {
    "type": "apply-selection-arrange",
    "command": "align-left"
  }
]
```

`command` 可选值：

```text
align-left
align-right
align-top
align-bottom
align-h-center
align-v-center
distribute-h
distribute-v
flip-h
flip-v
```

### 12.2 调整层级

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

省略 `objectIds` 时，会使用当前选区。

### 12.3 分组

```json
{
  "type": "group-selection",
  "object_ids": ["arrow_1", "text_1"]
}
```

省略 `object_ids` 时，会使用当前选区。

### 12.4 取消分组

```json
{
  "type": "ungroup-selection",
  "object_ids": ["group_1"]
}
```

省略 `object_ids` 时，会使用当前选区。

### 12.5 链接和取消链接

```json
{
  "type": "link-selection",
  "object_ids": ["bracket_1", "text_1"]
}
```

```json
{
  "type": "set-link-policy",
  "objectIds": ["bracket_1", "text_1"],
  "policy": "auto"
}
```

```json
{
  "type": "unlink-selection",
  "object_ids": ["bracket_1", "text_1"]
}
```

省略对象 id 时，会使用当前选区。`policy` 只能是 `auto`、`linked` 或
`unlinked`。只有选区恰好匹配一种已注册的有类型 Link 签名时才能显式 Link；
例如两个分子不能 Link。完整数据和交互契约见
[`link-model.zh-CN.md`](link-model.zh-CN.md)。

选中完整且唯一的单分子后，可以把当前分析结果粘贴成有关联的分析文本：

```json
{
  "type": "paste-analysis-caption",
  "digits": 2
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

ChemSema JSON 会把当前文件默认参数持久化为 `style.preset` 和
`style.defaults`。`chemsema-cli run` 会从输入文件恢复这些默认值，因此后续编辑命令
没有显式传入样式参数时，会使用文件自己的键长、线宽、锲形键宽、hash 间距、
边距和字体默认值。

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

`settings` 字段都可以传需要修改的项。

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

## 17. 导入 SMILES 与复制化学标识符

使用共享的 Rust 化学内核从 SMILES 创建文档：

```powershell
npm run cli -- insert-smiles "CC(=O)O" --out acetic-acid.ccjs --x 120 --y 100 --pretty
```

使用 `--input existing.ccjs` 可插入已有文档。最终渲染后的选择框左上角会对齐
`--x`、`--y`；以点分隔的多个组分仍属于同一条导入记录。

以下命令为第一个完整分子生成标识符；也可以重复提供 `--node <id>` 来限定分子：

```powershell
npm run cli -- chemistry acetic-acid.ccjs --format smiles --pretty
npm run cli -- chemistry acetic-acid.ccjs --format inchi --pretty
npm run cli -- chemistry acetic-acid.ccjs --format inchi-key --pretty
```

当前 SMILES 输出为异构 SMILES，并使用与输入编号无关的 canonical 原子排序。四面体
奇偶性与归一化 E/Z 斜线方向都会按 canonical 遍历重写，因此这些情况也返回
`"canonical": true`。当前比较器能够区分全部四个配体时，普通四面体中心还会带 `cip`
R/S 描述符。InChI 和 InChIKey 使用官方 IUPAC InChI
1.07.5 在本机生成：原生构建静态链接随仓库提供的 C 库，GUI 则加载官方
WebAssembly 构建；整个过程不依赖 Python，也不访问远程服务。

当前 InChI 桥接层通过确定的立体坐标投影保留归一化 E/Z，并通过与构型一致的 V2000
楔键保留四面体手性。它仍会拒绝
配位键，而不会悄悄将其改成单键；金属断键与重连层策略将作为独立的化学内核任务
继续开发。

命令脚本也可使用相同操作：

```json
[
  { "type": "insert-smiles", "smiles": "CCO", "x": 120.0, "y": 100.0 },
  { "type": "chemical-analysis", "format": "smiles", "targets": {} }
]
```
