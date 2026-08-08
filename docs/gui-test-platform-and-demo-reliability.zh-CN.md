# ChemSema GUI 测试平台与展示可靠性长期架构

状态：长期架构与实施合同。本文定义 ChemSema GUI 自动化、真实桌面验证、模型测试、故障注入、展示准入和测试制品的最终边界。任何临时脚本、人工冒烟或 AI 逐步操作都不能替代本文要求的发布证据。

本文与以下规范共同组成质量体系：

- [发布质量矩阵](./release-quality.zh-CN.md)定义公开能力的可信度和发布边界；
- [Windows 桌面端与 Office 集成长期架构](./windows-desktop-office-architecture.zh-CN.md)定义产品运行时边界；
- [核心契约审计](./core-contract-audit-2026-07-23.zh-CN.md)定义 engine、viewer、desktop 和 Office 之间不得漂移的责任；
- [CCJS 0.2 稳定化架构](./ccjs-v0.2-stability-architecture.zh-CN.md)定义文档、容器、补丁、恢复和格式门禁。

## 1. 决策摘要

ChemSema 必须建设一个独立的 GUI 测试平台，但平台代码、产品专属场景、Test ABI、fixture 和发布门禁必须留在 ChemSema 主仓库中，以便产品修改和对应测试原子提交。大型 trace、视频、长时间 soak 日志、VM 镜像和安装包进入外部制品存储；仓库保存可复现 manifest、SHA-256、最小失败文件和准入结果。

最终测试体系采用同一场景协议驱动四类执行面：

1. **WebdriverIO Tauri driver**：真实 Tauri 桌面程序、WebView、IPC、多窗口和前后端日志的主端到端执行器；
2. **Playwright driver**：高速浏览器回归、真实 WebView2 CDP 交叉验证、视觉比较和失败 trace；
3. **Windows UI Automation / input driver**：原生文件对话框、窗口焦点、系统剪贴板、文件关联、外部拖放、触摸、笔和输入法；
4. **production black-box driver**：不含测试后门的最终安装包和已安装程序，只通过公开 UI、文件和系统边界验证。

AI 不进入每一步鼠标键盘执行循环。AI 负责生成候选场景、发现覆盖缺口、分析失败包和归纳历史缺陷；确定性 runner 负责大规模执行、复现、收缩和判定。

## 2. 为什么这是产品架构而不是测试脚本

“单元测试通过，但展示时出现意外 bug”通常不是缺少某一条断言，而是同一用户操作跨越了多层状态：

```text
Windows 输入/窗口
  -> WebView DOM/SVG
  -> viewer interaction state
  -> WASM engine
  -> Tauri/native service
  -> 文件、剪贴板、Office、恢复日志
  -> viewer state/render 同步
```

任何一层的时序、revision、焦点、缓存或错误恢复不一致，都可能让局部测试通过而完整工作流失败。GUI 质量因此必须成为和文件格式、Rust engine、桌面服务同等级的版本化工程能力。

本平台的目标不是证明“测试跑过”，而是建立以下证据：

- 用户动作确实到达了预期目标；
- engine、viewer、native service 和持久化文件处于同一 revision；
- 用户看到的画面与语义状态一致；
- 保存重开、撤销重做和崩溃恢复保持合同；
- 异常、延迟、资源失败和窗口切换不会留下半提交状态；
- 最终安装包在干净机器上能够重复完成真实展示流程；
- 测试系统能够主动杀死已知类别的错误，而不是只在正确实现上变绿。

## 3. 仓库与发布边界

### 3.1 留在主仓库

以下内容必须与 ChemSema 代码原子提交：

- 场景协议、报告协议和覆盖协议的 schema；
- GUI test runner、driver adapter、oracle、generator 和 shrinker；
- Test ABI 和测试构建 feature；
- 产品专属场景与展示脚本；
- 小型 canonical fixture、最小历史回归文件和必要视觉基线；
- driver capability matrix、平台矩阵和错误分类；
- CI workflow、准入配置和基线升级记录；
- 每个 release candidate 对应的 qualification manifest。

这保证修改 GUI、IPC 或文档协议的提交不能在不更新测试的情况下合入，也避免主仓库与测试仓库之间的版本错配。

### 3.2 放入外部制品存储

以下内容默认不长期提交 Git：

- 每次成功运行的完整视频和 trace；
- 大型公开/私有 CDXML、CCJZ corpus；
- 数万张随机测试截图；
- 24 小时以上 soak 的原始日志；
- Windows VM 镜像、WebView2 runtime 镜像和安装包；
- crash dump、性能 profile 和大体积内存快照。

仓库中的 manifest 必须记录 artifact URI、媒体类型、字节数、SHA-256、生成提交、测试平台、驱动版本、运行 seed 和保留策略。准入证据不能只指向“latest”。

### 3.3 未来拆仓条件

只有当通用 runner 已服务至少两个真正独立、拥有独立发布周期的产品，并形成稳定公共 API 后，才允许把不含 ChemSema 语义的执行核心抽取为独立仓库或发布包。ChemSema 场景、fixture、Test ABI、覆盖登记和发布门禁始终留在本仓库。

### 3.4 用户桌面与执行环境边界

自动化不得抢占开发者正在使用的 Windows 桌面。无需真实 OS 输入的 engine、format、headless browser、UIA pattern 和静态 oracle 在后台 worker 运行；需要真实 click/drag/draw、前景焦点、触摸、笔、IME、原生对话框或系统快捷键的场景只能进入独立且解锁的 Hyper-V guest 桌面或专用测试机。host coordinator 只负责启动、监控、复制 manifest/制品和回收 VM，不向 host 用户会话注入输入，也不读取 host 剪贴板或用户文件。

真实输入前必须验证 VM/session id、目标进程、前景窗口和允许的 test run root；任何目标落到 host 用户会话、身份不明确或桌面被锁定的情况都立即 fail closed。普通 RDP 最小化、断开或锁屏状态不能作为正式真实输入环境。GPU、触摸/笔、多显示器和其他不能由 Hyper-V 等价表示的验证进入带相应硬件的独立 worker。

## 4. 建议目录

```text
packages/gui-test/
  package.json
  src/
    cli/
    protocol/
    runner/
    scheduler/
    drivers/
      wdio-tauri/
      playwright-browser/
      playwright-webview2/
      windows-uia/
      production-black-box/
    actions/
    oracles/
    coverage/
    generators/
    shrinker/
    fault-injection/
    mutation/
    reporters/

crates/chemsema-test-support/
  测试构建专用 Rust observability、fault injection 和 Test ABI。

tests/gui/
  schemas/
  scenarios/
    core/
    tools/
    dialogs/
    documents/
    clipboard-office/
    windows/
    accessibility/
    performance/
    recovery/
    demo/
  fixtures/
  baselines/
  coverage/
  qualification/

.github/workflows/
  gui-pr.yml
  gui-nightly.yml
  demo-qualification.yml
  release-qualification.yml
```

平台拥有独立 CLI，例如：

```text
chemsema-gui-test list
chemsema-gui-test run --suite core --driver wdio-tauri
chemsema-gui-test run --scenario demo.main --repeat 1000
chemsema-gui-test explore --model editor --seed 42 --steps 10000
chemsema-gui-test reproduce failure.json
chemsema-gui-test shrink failure.json
chemsema-gui-test qualify --candidate manifest.json
```

## 5. 版本化场景协议

所有固定、生成和收缩后的测试使用 `chemsema.gui.scenario.v1`。协议必须是可 Schema 验证的数据，不把场景逻辑藏在任意 JavaScript 闭包里。

最小场景包含：

- `id`、`title`、`schema`；
- 所需 capability 和适用 driver；
- fixture、初始窗口、DPI、locale、theme 和 runtime profile；
- 带稳定 action id 的步骤；
- 每一步的完成条件和时间预算；
- 最终与中间 oracle；
- 覆盖标签、风险级别、所有者和来源缺陷；
- 可重放 seed；
- 允许的已知诊断，默认为空。

示意：

```json
{
  "schema": "chemsema.gui.scenario.v1",
  "id": "editor.bond.undo-redo-save-reopen",
  "requires": ["document", "pointer", "filesystem"],
  "fixture": "blank-document",
  "steps": [
    { "id": "tool", "action": "control.invoke", "target": { "testId": "tool-bond" } },
    { "id": "draw", "action": "pointer.drag-world", "from": [100, 100], "to": [140, 100] },
    { "id": "undo", "action": "keyboard.chord", "keys": ["Control", "Z"] },
    { "id": "redo", "action": "keyboard.chord", "keys": ["Control", "Y"] },
    { "id": "save", "action": "document.save", "path": "result.ccjz" },
    { "id": "reopen", "action": "document.reopen" }
  ],
  "expect": [
    { "oracle": "document.fingerprint", "atoms": 2, "bonds": 1 },
    { "oracle": "runtime.no-unexpected-diagnostics" },
    { "oracle": "render.local-snapshot", "baseline": "bond-one" },
    { "oracle": "persistence.roundtrip" }
  ]
}
```

场景协议只描述意图。driver 负责把意图翻译为具体 locator、WebDriver action、CDP pointer、UIA pattern 或 OS input。

## 6. 稳定目标与动作词汇

### 6.1 目标优先级

控件定位按以下顺序：

1. `role + accessible name`：用户与辅助技术可感知的公共合同；
2. 稳定 `data-testid` / AutomationId：不适合使用自然名称的内部稳定合同；
3. document entity id、node id、bond id：画布语义对象；
4. world point 或目标几何：必须测试空白位置、命中边缘、交叉或框选时使用；
5. 原始屏幕坐标：只允许 OS 边界场景，并必须记录窗口矩形、DPI 和坐标空间。

禁止把 CSS 层级、`nth-child`、易变文本或偶然 RuntimeId 当作长期场景合同。

### 6.2 动作族

平台至少支持：

- control：invoke、click、double-click、context-menu、focus、blur、set-value、select-option；
- pointer：move、hover、down、up、drag、lasso、wheel、press-and-hold；
- keyboard：press、chord、sequence、text、IME composition；
- touch/pen：tap、double-tap、long-press、swipe、pinch、stretch、pressure/tilt path；
- document：new、open、save、save-as、close、reopen、external-open、file-drop；
- clipboard/Office：copy、cut、paste、paste-special、OLE open/update/roundtrip；
- window：create、activate、move、resize、minimize、maximize、restore、close、multi-window drag；
- runtime：cold-start、restart、crash、recover、offline、backend-delay、resource-failure；
- observation：wait-ready、wait-quiescent、checkpoint、screenshot、snapshot、trace-marker。

每个动作必须声明坐标空间、输入设备、modifier、button、重复次数和完成语义。`sleep(1000)` 不能作为正常完成条件；等待必须绑定可观察状态、revision、事件或预算。

### 6.3 用户可见功能穷举覆盖合同

测试的基本单位不是“页面能够打开”，而是用户能够真实完成一项功能。每个用户可见的工具、按钮、菜单项、右键命令、快捷键、对话框、属性编辑器、文件命令和系统集成都必须至少有一个场景，通过与用户相同的公开输入路径真实点击、按键、拖拽、绘制、输入或选择，并用独立 oracle 验证结果。调用 engine 命令、Test ABI、JavaScript 函数或直接注入文档只能用于前置布置和诊断，不能计作该功能的真实交互覆盖。

每一种可创建对象都必须通过 GUI 实际创建或绘制，而不是只打开预制 fixture。对象的完整生命周期至少包括：选择、取消选择、hover/focus、移动，以及适用时的缩放、旋转、控制点编辑、文本或化学内容编辑；逐项修改所有公开可写属性；复制、剪切、粘贴、重复、删除；undo/redo；保存、关闭、重开；以及适用的导入、导出、剪贴板和 Office 往返。每项属性必须覆盖默认值、代表性普通值、边界值、混合值、无效值/取消路径和保存重开后的值，不允许只验证属性面板接受输入而不验证画面、文档语义和持久化结果。

对象数量是强制覆盖维度，至少区分 `0`、`1`、`2` 和 `many`。多对象测试必须同时包含同类型与不同类型对象，并覆盖：逐个选择、追加/移除选择、框选/套索、全选、重叠与相交对象、远距离对象、部分在可视区外、锁定/隐藏/不可应用对象、group 内外对象、嵌套 group，以及大文档中的多选。所有支持批量操作的功能都必须真实应用于多对象，并验证：公共属性修改、混合值显示、部分适用时的行为、层级与相对位置保持、操作原子性、一次 undo/redo 的事务边界，以及无残留 selection/preview/handle。

还必须覆盖对象之间的组合行为，而不是把每种对象孤立测试：连接、吸附、对齐、分布、层级、group/ungroup、前后次序、跨 group 移动、复制后的引用关系、删除依赖、跨文档粘贴，以及分子、键、原子标签、文本、符号和其他图形对象之间所有公开允许或明确禁止的交互。被禁止的组合也必须验证明确反馈、文档不变和无脏历史记录。

覆盖登记的最小可审计单元为：

```text
feature × object-type × cardinality × selection/state × input-mode
        × property/value-class × persistence-boundary × platform-profile
```

高风险和核心编辑功能必须覆盖定义的完整矩阵；其他组合可以采用有约束的 pairwise/模型生成减少重复，但不得省略任何公开功能、对象类型、对象数量类别、可写属性或持久化边界。registry 必须区分 `real-user-path`、`setup-only` 和 `oracle-only`；只有第一类能够满足真实功能覆盖。新增功能如果没有同步登记对象、数量、状态、属性和真实输入场景，CI 必须拒绝合并。

## 7. Driver 架构

所有 driver 实现同一接口：

```text
prepare(profile)
launch(candidate)
capabilities()
resolve(target)
perform(action)
observe(query)
checkpoint(label)
collect_artifacts(policy)
shutdown()
```

driver 必须返回标准 action receipt：动作开始/结束时间、解析目标、输入类型、前后 revision、前后窗口、完成条件、诊断和证据引用。

### 7.1 WebdriverIO Tauri

这是实际桌面端到端主驱动。它负责启动测试构建或候选二进制、控制 WebView 元素、验证多窗口、调用测试许可范围内的 Tauri API、捕获前后端日志并支持并行 worker。测试插件只在显式测试 feature 中注册，生产构建不得携带执行任意脚本或 mock IPC 的能力。

### 7.2 Playwright browser 与 WebView2

浏览器 driver 承担高并发交互、视觉、ARIA 和大量模型序列。WebView2 driver 使用独立 CDP 端口和独立 user-data folder 连接真实桌面 WebView，交叉验证 WebDriverIO 结果，并产生 Playwright trace。不同 worker 不得共享 WebView2 profile。

### 7.3 Windows UIA 与真实输入

UIA pattern 优先用于可访问控件、窗口和原生对话框；只有 hover、自由画布拖拽、快捷键、触摸、笔、输入法或必须验证低层输入时才使用真实注入。真实输入 runner 只能在独占、解锁、无通知干扰的测试桌面运行，并在注入前重新确认前景窗口、目标矩形、DPI 和进程身份。

### 7.4 Production black-box

黑盒 driver 只接受最终 installer、安装目录或发布压缩包。它不得加载测试插件、调用 Test ABI、设置内部文档或读取 `window.__chemsemaDebug`。允许的 oracle 只有公开 UI/UIA、进程退出、日志/崩溃制品、输入输出文件、剪贴板/Office payload 和最终截图。

### 7.5 双执行池与并行边界

平台维护 `background-worker` 和 `interactive-isolated-worker` 两类执行池。前者可以按 CPU/内存预算高并发分片；后者每个 Windows interactive desktop 同一时刻只允许一个真实输入流，避免焦点竞争。真实 GUI 并行通过多个隔离 VM/专用会话实现，不能在同一桌面启动多个会抢前景的 driver。调度器按场景 capability 自动路由，禁止用较弱 driver 冒充真实输入覆盖。

## 8. Test ABI 与可观测性平面

现有 `window.__chemsemaDebug` 不是稳定测试协议。长期必须替换为显式版本化、测试构建专用的 `chemsema.test.abi.v1`。

Test ABI 可以提供：

- 加载 canonical fixture 和重置隔离 profile；
- 读取规范化 document fingerprint、revision、selection 和 undo/redo cursor；
- 坐标空间转换以及目标的权威 bounds；
- 等待 UI action、render patch、native mirror、journal 和 autosave 全部静止；
- 读取 render counters、long task、pending task 和缓存版本；
- 注册确定性 clock、random seed 和 fault profile；
- 导出日志、事件、状态转换和最小诊断；
- 触发受控 crash、I/O 错误、IPC 延迟或资源故障。

Test ABI 不得：

- 绕过 UI 直接完成被测试的用户动作；
- 在生产 feature 中存在；
- 暴露任意脚本执行；
- 改变化学、命中测试或渲染规则；
- 让测试构建和生产构建使用不同业务实现。

高频事件必须进入结构化 event journal，至少记录 action id、command、revision、source runtime、commit/cancel、render patch、native acknowledgement、错误和耗时。失败报告不能依赖临时 console 文本拼接。

## 9. 多重 Oracle

单一截图或内部 JSON 都不能单独判定 GUI 正确。关键场景至少组合以下 oracle：

### 9.1 交互 Oracle

- 目标可见、稳定、可接收事件且未被遮挡；
- 焦点、hover、cursor、selection/focus handle 与当前工具一致；
- gesture 只有一个 start、零或多个 update，以及唯一 commit/cancel；
- 完成或取消后所有 preview、mask、overlay 和 pending action 清空。

### 9.2 文档与化学 Oracle

- canonical CCJS fingerprint；
- 原子、键、立体化学、反应、关系、层级和资源不变量；
- revision 每个内容命令只推进一次；
- undo 恢复 before fingerprint，redo 恢复 after fingerprint；
- local WASM、native service 和已保存 snapshot 达成一致。

### 9.3 渲染与视觉 Oracle

- DOM/SVG 语义对象、render primitive 和局部/全窗口 screenshot；
- per-object/per-file 基线，不以总通过率掩盖单文件回归；
- 基线绑定 OS、driver、WebView2/浏览器、字体、DPI、主题和 GPU profile；
- 动态光标、时间、动画或 caret 只能通过声明式 mask 处理；
- 基线升级必须附带差异、原因、影响范围和审核记录。

### 9.4 无障碍 Oracle

- ARIA/UIA 树、accessible name、role、state 和 control pattern；
- 完整 Tab 顺序和纯键盘工作流；
- 对话框焦点圈、焦点陷阱、Escape/Enter 行为和状态事件；
- 画布对象必须有可导航的语义测试/无障碍表示，不能只依赖像素坐标。

### 9.5 持久化与外部 Oracle

- 保存后文件存在、非零、哈希和格式正确；
- 由新进程重开，而不是继续使用内存文档；
- CLI/engine 独立验证和目标格式往返；
- 系统剪贴板、Office/OLE、EMF preview 和文件关联结果；
- crash 后 recovery journal 只恢复已确认提交。

### 9.6 运行质量 Oracle

- 零未处理 exception/rejection、Rust panic、WASM trap、driver error；
- 非白名单 console warning/error 为失败；
- UI action recovery/fallback 发生即记录并按场景策略阻断；
- 启动、动作、渲染、保存和恢复有明确预算；
- 内存、handle、WebView/process 数和临时文件在循环后回到允许范围。

## 10. 模型测试、性质测试与 AI

### 10.1 权威状态模型

测试平台维护显式编辑器状态模型：

```text
document lifecycle
× active tool
× selection/focus
× pointer gesture
× dialog/menu
× tab/window
× clipboard
× persistence/recovery
× backend health
```

模型定义合法动作、预期转换、禁止转换和不变量。覆盖以状态和边为单位，而不是只统计代码行或测试数量。

### 10.2 生成与收缩

generator 按固定 seed 产生：

- 正常用户旅程；
- 边界点击、交叉目标和快速工具切换；
- 重入、取消、窗口失焦和异步竞态；
- 大文件、延迟、I/O/剪贴板失败；
- 跨格式、跨标签、跨窗口和 Office 链路。

失败必须保存 seed、模型状态、动作 receipt、候选文件和全部证据。shrinker 在保留失败签名的前提下删除动作、缩小文档、简化坐标和减少对象，产生可提交的最小固定回归。

### 10.3 AI 边界

AI 可以：

- 从新功能、缺陷、覆盖矩阵和代码差异生成候选场景；
- 对失败 trace、截图差异、event journal 和 crash dump 进行分类；
- 建议可能缺失的状态边和 fault profile；
- 把探索结果转化为待审核的固定场景。

AI 不可以：

- 作为每一步执行是否成功的唯一 oracle；
- 用自然语言结论替代机器报告；
- 自动接受视觉基线更新；
- 在没有可复现 seed/场景时把一次探索宣称为回归覆盖；
- 因为“看起来正常”忽略结构化错误或语义不变量。

## 11. 故障注入与变异验证

### 11.1 Fault profiles

测试构建必须能够确定性模拟：

- native IPC 延迟、乱序、超时和明确失败；
- 文件不存在、权限拒绝、磁盘满、部分写入和原子替换失败；
- 剪贴板被占用、格式缺失和畸形 payload；
- WebView reload、WASM 初始化失败和后台任务取消；
- 字体/资源缺失、图片解码失败和超大资源；
- autosave、journal checkpoint、恢复和退出之间的竞态；
- Office server 不可用、对象回写失败和 preview 生成失败。

所有 fault 都必须有稳定 id、触发点、次数、延迟和预期用户结果。测试不能依靠真实破坏机器状态来模拟错误。

### 11.2 Mutation qualification

测试平台正式准入前，必须运行受控变异，例如：

- 删除事件监听器或 command dispatch；
- 偏移 hit-test 或坐标转换；
- 丢弃 Document Patch 或 native acknowledgement；
- 让 save 返回成功但写出损坏文件；
- 保留 gesture preview 或 stale selection；
- 跳过 undo revision；
- 让 UI 显示旧 snapshot；
- 吞掉异常或把错误降级为 warning。

核心门禁必须杀死所有指定变异。存活变异意味着 oracle 或覆盖存在短板，不能以正确版本通过为由宣布平台完成。

## 12. 覆盖登记

`chemsema.gui.coverage.v1` 至少跟踪：

- 每个用户可见控件、命令和属性是否拥有 `real-user-path` 场景，而不是只有 Test ABI/fixture 路径；
- 每个工具的 create/select/hover/focus/move/resize/rotate/style/copy/cut/paste/delete/undo/redo；
- 每个对象类型的导入、显示、命中、编辑、导出和保存重开；
- `0/1/2/many`、同类/异类、group/嵌套、重叠、锁定、隐藏、部分适用和大文档多对象状态；
- 每个公开可写属性的默认/普通/边界/混合/无效值、批量修改、undo/redo 和 round-trip；
- 对象间连接、吸附、对齐、分布、层级、组合、前后次序、依赖删除和跨文档关系；
- 每个对话框的打开、确认、取消、无效输入、键盘和焦点；
- 每个快捷键和 modifier；
- 每个文件格式的 open/edit/save/reopen/export；
- Web、Tauri test build、production build、Office 和 OS 边界；
- success、cancel、error、timeout、recovery 和 crash 分支；
- DPI、窗口、locale、theme、WebView2 和 GPU profile；
- 每个历史缺陷对应的永久场景；
- 每个展示动作对应的固定场景和 black-box 证据。

新增工具、对象类型、命令、属性、对话框、文件格式或系统能力时，registry 必须穷举要求的能力面；未登记、没有真实点击/绘制/修改场景或只有内部注入测试的项目直接阻断 CI，不能依靠人工记忆补测试。

### 12.1 复杂文档与大文档必须真实构建

fixture 打开测试不能替代构建测试。平台必须从空白文档开始，通过 GUI 真实绘制至少四个规模层级，并由 coverage registry 记录对象、原子/键、关系、group 深度、属性变化和动作数量：

- `small`：单对象和最小组合，用于精确语义与失败收缩；
- `complex`：包含多分子、多种图形/文本/符号、连接、反应或其他适用关系、同类/异类多选、group/嵌套、层级、复制粘贴和属性混合的完整工作流；
- `large`：至少数百对象或约 1,000 原子量级，通过连续真实绘制、复制、模板插入、批量属性和组合操作建立；
- `xlarge`：达到当前公开性能合同上限，初始目标为 5,000 原子或等价渲染/交互复杂度，并随产品上限提高。

`large/xlarge` 必须分别覆盖“从空白逐步构建”和“打开既有大文件后继续编辑”，验证增量 patch 而非全量刷新、pointer/toolbar 延迟、选择与拖拽反馈、内存/handle、autosave/journal、undo/redo、保存重开、崩溃恢复和最终 canonical fingerprint。为提高速度允许使用真实 UI 的复制、批量命令和模板，但不得直接注入最终文档冒充绘制。复杂文档还必须运行跨对象长序列和至少 24 小时混合 soak。

### 12.2 代码—测试影响图与证据复用

`chemsema.gui.impact.v1` 将源文件、crate/package、生成的 WASM、viewer surface、Tauri/Office command、文档 schema、对象/命令/属性 capability、driver、oracle 和场景连接成有向依赖图。每次变更从实际 diff 出发计算传递影响闭包；测试选择不能只按目录名或人工标签猜测。

每个通过结果以内容寻址 evidence key 保存：

```text
scenario + scenario-data + product-component closure + generated artifacts
         + fixture/baseline + driver/oracle/runner + build flags
         + OS/runtime/font/GPU profile + capability contract version
```

当且仅当上述闭包哈希一致、环境仍在声明兼容范围、证据未过期且历史上没有相关逃逸缺陷时，既有结果才可复用；报告必须列出 `executed`、`reused`、`invalidated` 及原因。代码没有变化但依赖锁、编译器、WASM、WebView2、字体、schema、driver、oracle、测试数据或环境变化时，相关证据自动失效。核心协议、共享 interaction/render/document path 和无法证明影响边界的改动按广泛影响处理。

完整质量门禁检查的是“全部必需 coverage cell 是否具有当前有效证据”，不是“本次是否重新执行全部场景”。确定性且闭包未变的成功测试不重复消耗资源；新增代码、传递影响、过期证据、历史缺陷关联、随机/模型新 seed、soak、泄漏和环境漂移测试必须执行。impact selector 自身接受 mutation 和逃逸缺陷回放；一次漏选即扩大对应依赖边并永久加入回归。

## 13. 确定性与隔离

每个 worker 使用独立：

- 临时目录和文档目录；
- WebView2/user-data profile；
- CDP/WebDriver/Test ABI 端口；
- clipboard namespace 或独占系统剪贴板锁；
- journal、autosave、日志和制品目录；
- clock/random seed 和 locale/timezone 配置。

runner 必须控制动画、系统通知、网络、更新检查和后台任务。测试不能读写用户真实配置、最近文件、剪贴板历史或项目文件。所有产生的文件必须位于已解析并验证的 test run root 内。

重试只用于收集更多证据，不能把第一次失败改写为通过。任何“第一次失败、第二次成功”都记为 flaky failure，并阻断对应门禁，直到根因修复或场景被证明错误。

### 13.1 当前工作站资源合同

本机测试的硬上限为所有 ChemSema 测试 worker 合计 **10 个 CPU execution unit（按 Windows 逻辑处理器/vCPU 计）和 30 GiB host committed-memory 增量**，不是每个 worker 各自获得该额度。guest vCPU、host worker CPU slot 和 coordinator 都从同一个 10-unit 配额扣除；guest 分配内存、`vmwp`/Hyper-V 开销、host runner、缓存和报告进程都计入同一个 30 GiB 增量。调度器通过 Hyper-V vCPU/动态内存、Windows Job Object/affinity、进程采样和 admission control 共同约束；超过任一预算时排队，不能依靠 OOM 或系统调度碰运气。磁盘空间、写入速率、GPU、温度和电源状态单独监控，资源不足时安全暂停并可从 checkpoint 恢复。

默认吞吐 profile 为两个隔离 interactive worker，各最多 4 vCPU/10 GiB；host coordinator/headless shard 保留 2 CPU unit，host runner 与 Hyper-V 开销合计保留 10 GiB。需要 `xlarge`、soak、Office 或高内存场景时切换为单一 heavy worker，最多 8 vCPU/20 GiB，host 侧仍保留 2 CPU unit/10 GiB 并遵守总上限。资源控制看实际 host 增量而非只相信 VM 配置值；实际 worker 数由基准校准，不能以并行数量换取超时、内存交换、前景竞争或 flaky。

### 13.2 VM 生命周期与安全

每个正式 VM profile 使用版本化基线、干净 checkpoint、专用测试账户和独立密钥；PowerShell Direct 凭据只能通过系统安全提示取得，以 host 当前账户的 DPAPI 加密保存在仓库外，并将 ACL 限制为该账户、SYSTEM 和 Administrators，禁止明文进入命令、日志、场景或 CI 制品。启动后验证 OS、WebView2、字体、Office、DPI、GPU 和更新状态，结束后导出证据并回滚。测试网络默认隔离，只按 manifest 放行所需端点；需要联网时，coordinator 从实际 Hyper-V switch 读取 gateway/prefix，配置 guest 地址并分别验证 DNS、HTTPS 和文件往返，不能依赖一次性的旧 DHCP 租约。禁止挂载用户项目目录、复用个人浏览器 profile、同步个人 OneDrive 或共享 host 剪贴板。coordinator 必须支持断电/重启后的幂等恢复、孤儿进程清理和制品完整性检查。

## 14. 运行报告与失败包

每次运行产生 `chemsema.gui.run.v1`：

- commit、dirty state、candidate SHA-256、installer SHA-256；
- OS/build、WebView2、driver、字体、DPI、GPU、locale；
- suite、scenario、seed、worker、起止时间；
- 每一步 action receipt 和 oracle 结果；
- coverage delta 和未覆盖要求；
- impact graph 输入、evidence key、executed/reused/invalidated 决策及原因；
- host/guest/session id、前台隔离证明和 CPU/内存/磁盘/GPU 资源曲线；
- 所有 fault/mutation；
- 失败签名与制品 manifest；
- runner 自身错误与环境不确定性。

失败包至少包含：

- 原始与收缩后的场景；
- 初始 fixture、最终内存 snapshot 和写出文件；
- Playwright/WebDriver trace 或等价 action journal；
- 前后端结构化日志、Windows 事件和 crash dump 引用；
- 关键步骤截图、视觉差异和 ARIA/UIA snapshot；
- 可复制的单条 reproduce 命令。

成功运行默认只保留摘要和抽样证据；展示/发布 qualification 的全部证据按发布保留策略归档。

## 15. CI 与运行层级

### 15.1 `verify`

保留 Rust、格式、WASM、容器和静态合同门禁。它不能再被描述为完整 GUI 验证。

### 15.2 `gui-pr`

每个影响 viewer、engine interaction、Tauri command、文件、剪贴板、Office 或测试协议的 PR 必须执行：

- headless browser 核心场景；
- Windows Tauri test build 核心用户旅程；
- 由 `chemsema.gui.impact.v1` 选择传递受影响场景，并复用其余仍有效的内容寻址证据；
- 对 impact selector 运行固定 sentinel/mutation，无法证明边界时扩大而不是缩小范围；
- 语义、视觉、ARIA、日志和性能预算；
- 对改动区域的 mutation smoke。

### 15.3 `gui-nightly`

- 验证全用户功能、全对象/工具/公开属性和 `0/1/2/many` 矩阵均有当前有效证据，只执行新增、受影响、过期、轮换环境及不可缓存场景；
- 持续执行复杂/大文档真实构建、模型新 seed、长序列、soak、泄漏和随机 fault，不能因代码未变永久复用；
- 真实 WebView2 与 WebdriverIO 交叉执行；
- 模型生成、长序列和自动收缩；
- fault profile、内存/handle 泄漏、恢复；
- 原生 Windows 对话框、剪贴板、文件关联和 Office；
- 多 DPI、窗口、WebView2/GPU profile。

### 15.4 `release-qualification`

只接受已冻结、带 SHA-256 的最终安装候选。qualification manifest 必须让每个用户功能、对象、公开属性、单/多对象核心矩阵和复杂/大文档 cell 都指向对当前组件闭包有效的真实交互证据；闭包完全相同的证据可以复用，不因换了无关代码而机械重跑。最终安装包仍必须从干净 VM 执行不可复用的安装/冷启动、production black-box 集成 sentinel、受影响功能、保存重开、升级/卸载/重装和发布 soak。任何影响 component closure、打包组合或运行环境的变化只使对应证据失效，而不是无条件抹掉全部历史证明。

## 16. Demo Qualification Gate

展示是独立发布面。每个正式展示必须有版本化 `chemsema.gui.demo.v1` 脚本，准确描述现场要执行的打开、绘制、编辑、保存、Office 和窗口动作。不得在展示当天临时更换未验证文件或操作路径。

候选展示包至少满足：

1. 从干净 Windows VM 安装并冷启动；
2. 完全离线执行，不依赖开发服务器、缓存或在线资源；
3. 使用最终生产构建，不加载 Test ABI；
4. 所有展示脚本连续执行至少 1,000 次，零失败、零 flaky、零意外诊断；
5. 在至少三种独立机器/VM profile 上通过；
6. 覆盖 100%、125%、150%、200% DPI 和展示实际分辨率；
7. 完成至少 24 小时混合用户旅程 soak，零 crash、hang、未处理错误和不可恢复状态；
8. 内存、handle、进程、临时文件和 autosave/journal 增长在批准预算内；
9. 保存的所有文件由新进程重开并独立验证；
10. Office/OLE、剪贴板或外部集成若属于展示内容，必须在目标 Office 版本上通过；
11. 归档 candidate、installer、场景、环境、报告和 SHA-256；
12. 演示者只使用通过 qualification 的不可变候选。

任一核心场景失败后，即使重跑成功，原 qualification 仍失败。修复必须产生新的 candidate 哈希并重新执行受影响资格集；不得手工修改报告或只重跑失败步骤。

## 17. 运行时可靠性配套

测试不能替代可靠性设计。为降低展示事故，产品必须同时具备：

- 单一结构化 UI action/error bus；
- 每个动作明确 commit/cancel/failed receipt；
- local/native revision barrier 和一致性诊断；
- 启动 health gate、资源预加载和离线资源保证；
- 自动保存、hash-chain journal、崩溃恢复和可验证 checkpoint；
- 超时后显式用户状态，不静默吞错或无限等待；
- 可导出的最小诊断包和隐私边界；
- 重要功能的安全降级，而不是假成功。

展示门禁必须把任何 recovery fallback、后台错误、未预期 warning 或超预算动作视为信号。界面最终成功不代表路径健康。

## 18. 现有测试迁移

现有 Playwright、GUI regression、viewer interaction、stability user paths、toolbar、text editor、runtime gate、large-document 和 Office 测试是有价值的证据来源，但不继续扩展为更多独立大脚本。

迁移顺序：

1. 建立 inventory：每个脚本、场景、fixture、断言、driver、风险和当前 CI 入口；
2. 抽取重复 server、browser、mock、日志和等待逻辑；
3. 把每条稳定行为转成 `chemsema.gui.scenario.v1`；
4. 把内部状态读取转为 Test ABI oracle，把用户动作保留为真实 GUI action；
5. 让相同场景先在 Playwright browser 和 WebdriverIO Tauri 通过；
6. 为原生边界增加 UIA/production black-box 版本；
7. 对迁移场景运行 mutation qualification；
8. 只有新旧结果和覆盖映射一致后，才删除旧脚本入口；
9. 最终把 `gui-pr`、`gui-nightly` 和 qualification 纳入正式 CI。

迁移期间不得用新平台存在为理由降低当前门禁；旧测试只有在有等价或更强替代证据后才能退休。

## 19. 实施阶段与验收

### Phase 0：事故账本与覆盖基线

- 汇总历史展示 bug、当前 GUI 脚本和已知 flaky；
- 建立 capability/coverage registry；
- 建立 source/component/capability/scenario 影响图和初始 evidence key；
- 固定当前 demo journey 和发布候选信息；
- 记录当前门禁真实执行面，禁止把 browser mock 称为 desktop E2E。

完成条件：每个历史展示 bug 有根因类别、最小复现或明确缺口，以及未来场景 id。

### Phase 1：协议与 runner 内核

- 场景、报告、coverage、artifact manifest schema；
- 10 CPU unit/30 GiB admission control、增量 scheduler、隔离、action receipt、oracle 和 shrink 基础；
- 标准 CLI 和单场景 reproduce；
- runner 自测与 mutation harness。

完成条件：同一数据场景可由 fake driver 和 Playwright driver 执行，失败可稳定复现并生成结构化包。

### Phase 2：Test ABI 与双桌面驱动

- 测试 feature、结构化 event journal、quiescence、fault injection；
- WebdriverIO Tauri 和 Playwright WebView2；
- Hyper-V background/interactive 双池、host 前台 fail-closed 和 PowerShell Direct 生命周期；
- test build 与 production build 权限隔离验证。

完成条件：核心编辑旅程在 browser、真实 Tauri test build 和 production black-box 上得到一致语义结果。

### Phase 3：现有回归迁移

- 迁移全部当前 GUI/interaction/stability 脚本；
- 建立视觉、ARIA、持久化和性能 oracle；
- 将快速核心集纳入 PR。

完成条件：旧脚本覆盖无损映射，新平台杀死规定变异，旧入口可审计退休。

### Phase 4：模型、故障和平台矩阵

- 状态模型、生成器、收缩器；
- fault profiles、Office/系统边界；
- DPI/WebView2/GPU/locale 矩阵；
- nightly dashboard 和 flaky 零容忍。
- complex/large/xlarge 从空白真实构建、长序列和 24 小时 soak。

完成条件：长期随机运行产生的失败都可重放；所有覆盖缺口机器可读；关键 fault/mutation 均被门禁捕获。

### Phase 5：展示与发布资格

- demo recorder、versioned demo journey；
- 干净 VM、最终 installer、千次重复和 24 小时 soak；
- qualification manifest、签名/哈希和归档。

完成条件：展示者只能选择通过资格的不可变候选；资格证据可由第三方在相同环境重放。

## 20. 完成定义

不能因为“建立了新目录”“可以自动点按钮”或“某次全绿”宣布平台完成。稳定 GUI 测试平台至少满足：

- 所有产品 GUI 场景使用版本化协议；
- browser、真实 Tauri、OS boundary 和 production black-box 均有正式 driver；
- 测试构建与生产构建的安全边界经过自动验证；
- 历史展示 bug 全部有永久场景；
- 每个用户可见功能都经过真实点击/输入/拖拽，而不是仅由内部 API 或文档注入覆盖；
- 每类对象均由 GUI 实际创建或绘制，并覆盖全部公开可写属性、完整生命周期和保存重开；
- `0/1/2/many`、同类/异类多对象、组合/嵌套/部分适用和对象间交互拥有明确场景；
- 功能、对象、数量、状态、输入、属性、持久化、错误分支和环境矩阵机器可读；
- complex、large 和 5,000 原子或等价 `xlarge` 文档均有从空白真实构建及继续编辑证据；
- source/component/capability/scenario 影响图可审计，完整门禁复用未变闭包证据并只执行受影响、过期和不可缓存测试；
- 所有 worker 合计遵守 10 CPU execution unit/30 GiB，上层桌面从不接收测试输入；
- 固定 seed 失败可以自动重放和收缩；
- 核心 mutation set 全部被杀死；
- PR、nightly、release 和 demo 门禁均在 CI/测试机上持续运行；
- 不允许 flaky rerun-to-green；
- 最终安装包而不是浏览器替身拥有发布和展示证据；
- 大型制品有哈希、环境和保留 manifest；
- 文档、协议、CLI help 和实际实现同步。

## 21. 当前工作站验证状态（2026-08-08）

已验证 Hyper-V PowerShell 模块存在，`vmms` 与 `vmcompute` 服务运行；host 报告 24 个逻辑处理器和约 63.4 GiB 物理内存。`jiajun\dream` 已加入并在当前 token 中启用 `Hyper-V Administrators`。现有 Windows 11 测试 VM（本文别名 `windows-gui-worker-current`）为 Generation 2、8 vCPU、动态内存 4–20 GiB；其配置、自动检查点和 VHDX/AVHDX 链可访问，2026-08-08 已实际启动、验证 guest heartbeat/time/KVP/shutdown integration，并经 Hyper-V 正常停止和完成自动检查点合并。用户确认 guest Office 已激活，但本次未在 Office UI 内独立验证。

专用 `chemsema-test` guest 账户和 `vmicvmsession` 已启用；凭据通过安全窗口取得，以 DPAPI 加密保存在仓库外，ACL 仅允许 `jiajun\dream`、SYSTEM 和 Administrators。PowerShell Direct 已实际连接到 Windows 11 guest。由于 Default Switch DHCP 仅产生 `169.254.*`，coordinator 读取 host 的 `172.31.0.1/20` 后为 guest 配置 `172.31.15.250/20`；DNS 解析和 HTTPS 443 成功，真实 `https://www.microsoft.com/` 请求返回 HTTP 200/201,253 bytes。host→guest 文件 SHA-256 与 guest→host 返回内容均完全一致。当前已打通 VM 生命周期、PowerShell Direct、guest 联网和双向文件路径。

### 首个可执行平台纵向切片

仓库现已在 `packages/gui-test` 与 `tests/gui` 中包含首个可执行纵向切片：场景、运行报告、覆盖清单、影响图、制品清单和 worker profile JSON Schema；严格 Schema 校验；规范化内容寻址 evidence key；传递影响选择；总计 10 CPU unit/30 GiB 的 fail-closed 资源准入；动作硬预算和前后状态收据；fake driver、Playwright browser driver 和 Hyper-V coordinator。版本化场景 `core.bond.draw-single` 使用公开可访问性目标和真实指针拖拽输入。同一场景已通过 fake driver 的 runner 自测，并在无头 Edge 中实际执行成功，生成经过 Schema 验证的 `chemsema.gui.run.v1` 报告。旧回归脚本在逐项登记和迁移期间继续保持有效，不因新平台存在而退役。

coordinator 已在 `windows-gui-worker-current` 上跨过隔离桌面边界。专用账户使用 LSA 保存的自动登录密钥实现无人值守登录，Winlogon 注册表不存在明文密码。Rust agent 以 host/guest SHA-256 一致方式传入，运行时不创建或抢占控制台窗口，并明确区分 service session 0 与已解锁的交互 `Default` 桌面。桌面候选由当前源码构建，复制到按 SHA-256 寻址的 guest 目录，启动前重新校验哈希，并以普通用户完整性级别运行。每个激活/点击/拖拽回执都绑定专用账户、非零 session、精确候选 PID/可执行文件、前台窗口、窗口内坐标和受限 run directory。

2026-08-08 已在不占用 host 前台的情况下，通过正式 scenario runner 执行第一条 production desktop sentinel。`production-black-box` driver 会启动隔离 VM，安装内容寻址的 agent 与候选程序，应用并逐项验证专用用户桌面 baseline，启动 production desktop，通过 guest loopback CDP 解析带 scope 的 `Single bond` 控件和 `#viewer-container`，再由受守卫的 guest agent 发送一次真实 Windows 点击和一次八步真实拖拽。经过 Schema 验证的 `chemsema.gui.run.v1` 报告记录了两项动作完成、候选 SHA-256 `72f99bcd35b8dc24a837001e0fa6d707bc26e50f18a6428bec9fb42c6a27103f`、渲染键从 0 变为 1、窗口标题进入脏状态、DOM 与诊断 oracle 均通过，以及 evidence key `cbbcbca14237b0281e683b35f0907d473c33b4c1a45cd255bc14006370214176`。每次 CLI 运行现会把经过验证的报告保存为不可变 SHA-256 对象，并在 evidence key 与 run id 下写入经过 Schema 验证的 manifest。Windows UI Automation 继续负责原生/窗口表面；CDP 提供语义边界和独立观测，OS 输入仍由外部守卫代理真实发送。版本化专用用户 baseline 会减少反复出现的 `CloudExperienceHost` 账户提示；若仍出现，agent 只有在系统路径、窗口类、标题和应用模型 ID 四项精确匹配时才允许关闭。完整截图/trace/log bundle 和其余 capability 矩阵尚未完成。

此后确定性重置已经投入运行：每个 production 场景都会按不可变 ID 恢复 profile checkpoint，拒绝自动 checkpoint，并在启动前验证 worker 保持关闭。输入也已改为一个常驻交互 guest agent，通过有界文件通道只接受固定 click/drag/key 协议，不再为每个动作创建计划任务。键盘输入采用白名单和物理扫描码，拒绝安全注意序列与 Windows 键组合，并在注入前后重新验证精确的前台候选程序。

正式 `core.history.undo-redo-bond.production` 场景现已在不可变 production 候选上通过：真实鼠标点击与拖拽创建一根键，再发送真实 `Control+Z` 和 `Control+Y`，内核文档与独立 DOM 观测共同证明键数严格按 `0 -> 1 -> 0 -> 1` 变化。候选 SHA-256 为 `739faffa72717bff3eeca5b2817ff1c5f8459a49ab7bc06ab2e0a9ed3bc10773`，evidence key 为 `5d1dbe4bce601b3e950232b5191756859445bf44581e3526dc3dc91277e757fa`；四项动作均保持在原 12 秒预算内，两项最终 oracle 全部通过。该场景没有通过放宽门禁变绿，而是发现并修复了三个真实 production 缺陷：undo/redo 绕过版本化命令结果管线、对象变为空渲染时残留旧图元，以及图元索引缺失或失步时产生幽灵 DOM。production receipt 现在还保留实际入口资源 URL、引擎类型、历史状态、内核键数、命令结果和增量同步模式，使后续失败能区分输入、命令、模型与渲染层。持久 CDP 传输和完整截图/trace/log bundle 仍未完成。

当前入口为：

```powershell
npm run gui-platform -- list
npm run gui-platform -- validate tests/gui/scenarios/core/draw-single-bond.json
npm run gui-platform -- audit
npm run gui-platform -- impact viewer/app.js
npm run gui-platform -- worker host-attest
npm run gui-platform -- worker start
npm run gui-platform -- worker guest-attest
npm run gui-platform -- worker prepare-guest
npm run gui-platform -- worker install-agent
npm run gui-platform -- worker configure-desktop-baseline
npm run gui-platform -- worker agent-attest-service
npm run gui-platform -- worker stop
npm run gui-platform -- run tests/gui/scenarios/core/draw-single-bond.json --driver fake
npm run gui-platform -- run tests/gui/scenarios/core/draw-single-bond.json --driver playwright-browser
npm run gui-platform -- run tests/gui/scenarios/core/draw-single-bond-production.json --driver production-black-box
npm run gui-platform:test
```

## 22. 上游技术依据

- Tauri WebDriver 与 WebdriverIO：<https://v2.tauri.app/develop/tests/webdriver/>
- Tauri WebDriver CI：<https://v2.tauri.app/develop/tests/webdriver/ci/>
- Playwright WebView2/CDP：<https://playwright.dev/docs/webview2>
- Playwright actions 与 auto-waiting：<https://playwright.dev/docs/input>、<https://playwright.dev/docs/actionability>
- Playwright trace：<https://playwright.dev/docs/trace-viewer>
- Playwright 视觉比较：<https://playwright.dev/docs/test-snapshots>
- Playwright ARIA snapshot：<https://playwright.dev/docs/aria-snapshots>
- Microsoft UI Automation 自动测试：<https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-usefortesting>
- Windows UI Automation CLI 与输入注入：<https://learn.microsoft.com/en-nz/windows/apps/dev-tools/winapp-cli/ui-automation>
- Hyper-V PowerShell Direct：<https://learn.microsoft.com/en-us/windows-server/virtualization/hyper-v/powershell-direct>

这些工具是 driver 和证据基础，不是 ChemSema 场景模型、化学 oracle、展示准入或发布责任的替代品。
