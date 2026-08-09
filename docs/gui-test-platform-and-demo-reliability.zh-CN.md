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

黑盒 driver 只接受最终 installer、安装目录或发布压缩包。它不得加载测试插件、调用 Test ABI、设置内部文档或读取 `window.__chemsemaDebug`。允许的 oracle 只有公开 UI/UIA 与公开 DOM 状态、进程退出、浏览器级性能 trace、日志/崩溃制品、输入输出文件、剪贴板/Office payload 和最终截图。CDP 可以传输公开观察与浏览器 trace，但不得注入被测用户动作或读取应用私有状态。

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

2026-08-08 已在不占用 host 前台的情况下，通过正式 scenario runner 执行第一条 production desktop sentinel。`production-black-box` driver 会启动隔离 VM，安装内容寻址的 agent 与候选程序，应用并逐项验证专用用户桌面 baseline，启动 production desktop，通过 guest loopback CDP 解析带 scope 的 `Single bond` 控件和 `#viewer-container`，再由受守卫的 guest agent 发送一次真实 Windows 点击和一次八步真实拖拽。经过 Schema 验证的 `chemsema.gui.run.v1` 报告记录了两项动作完成、候选 SHA-256 `72f99bcd35b8dc24a837001e0fa6d707bc26e50f18a6428bec9fb42c6a27103f`、渲染键从 0 变为 1、窗口标题进入脏状态、DOM 与诊断 oracle 均通过，以及 evidence key `cbbcbca14237b0281e683b35f0907d473c33b4c1a45cd255bc14006370214176`。每次 CLI 运行都会把经过验证的报告保存为不可变 SHA-256 对象，并在 evidence key 与 run id 下写入经过 Schema 验证的 manifest。Windows UI Automation 继续负责原生/窗口表面；CDP 提供语义边界和独立观测，OS 输入仍由外部守卫代理真实发送。版本化专用用户 baseline 会减少反复出现的 `CloudExperienceHost` 账户提示；若仍出现，agent 只有在系统路径、窗口类、标题和应用模型 ID 四项精确匹配时才允许关闭。在第一条 sentinel 完成时，完整截图/trace/log bundle 和其余 capability 矩阵仍未完成。

此后确定性重置已经投入运行：每个 production 场景都会按不可变 ID 恢复 profile checkpoint，拒绝自动 checkpoint，并在启动前验证 worker 保持关闭。输入也已改为一个常驻交互 guest agent，通过有界文件通道只接受固定 click/drag/key 协议，不再为每个动作创建计划任务。键盘输入采用白名单和物理扫描码，拒绝安全注意序列与 Windows 键组合，并在注入前后重新验证精确的前台候选程序。

正式 `core.history.undo-redo-bond.production` 场景现已在不可变 production 候选上通过：真实鼠标点击与拖拽创建一根键，再发送真实 `Control+Z` 和 `Control+Y`，内核文档与独立 DOM 观测共同证明键数严格按 `0 -> 1 -> 0 -> 1` 变化。候选 SHA-256 为 `739faffa72717bff3eeca5b2817ff1c5f8459a49ab7bc06ab2e0a9ed3bc10773`，evidence key 为 `5d1dbe4bce601b3e950232b5191756859445bf44581e3526dc3dc91277e757fa`；四项动作均保持在原 12 秒预算内，两项最终 oracle 全部通过。该场景没有通过放宽门禁变绿，而是发现并修复了三个真实 production 缺陷：undo/redo 绕过版本化命令结果管线、对象变为空渲染时残留旧图元，以及图元索引缺失或失步时产生幽灵 DOM。production receipt 现在还保留实际入口资源 URL、引擎类型、历史状态、内核键数、命令结果和增量同步模式，使后续失败能区分输入、命令、模型与渲染层。

持久 CDP 观测现已通过版本化有界请求/响应通道投入运行。每次 run 只安装一个隐藏 observer；Schema 与运行时共同要求它以 SYSTEM 身份运行在 session 0，只接受固定 `locate`、`state`、`count`、`count-state`、`distinct-count` 和 `distinct-count-state` 模式，去重计数还必须使用三种白名单身份属性之一，因此不能抢占交互前台，也不能执行任意表达式。同一 production history 场景再次通过，evidence key 为 `703df348f29d23c4845063aa9a34c72fcd3e1ecf5e41fa23d7e90c4cfb5e7ed3`。四项动作从原来每项约 7.2 秒降至 5.1–5.3 秒，同时保持原预算与语义 oracle。

版本化 guest 动作事务也已投入运行。production runner 先解析公开 UI 目标，随后只调用一次 PowerShell Direct；有界 guest 脚本依次取得独立 CDP `before`、提交且仅提交一次受守卫输入、判断场景声明的固定 completion 条件并返回最终 CDP `after`。输入代理和 CDP observer 仍是不同进程与不同协议，因此事务合并不能让输入实现伪造自己的 oracle。请求与 receipt Schema 保持严格，completion 超时必须在端到端动作预算内为目标解析与传输预留 4 秒。production history 场景以 evidence key `5f30cc7fb7d6dbf83600f7ba26135768a183314278be08bce2852b9e1a5ee159` 通过；四项动作分别在 3.1–3.4 秒完成，相比仅持久 CDP 又降低约 36%，内核/DOM 的 `0 -> 1 -> 0 -> 1` 证据不变。在该阶段，直接认证的 guest→host 制品传输和完整 production bundle 仍未完成。

第一条 production 多对象工作流 `core.selection.clipboard-delete-multi-bond.production` 也已投入运行：从空白文档开始，以受守卫真实 OS 输入绘制两根键、切换框选工具、全选、经真实 Windows 剪贴板复制、粘贴为四根键、再次全选、原子删除，并验证 undo/redo。候选 SHA-256 `dea620b455daeb253c4141e2e999eae376c5b53ecd0f7a7034795db401ea58f6` 以 evidence key `ff6fc4512e70cee602ce87118087408de8634076731bc6c9b82c9ca98519695c` 通过；独立 DOM 证据记录 `0 -> 1 -> 2 -> 4 -> 0 -> 4 -> 0`，第二次全选将 overlay 从 21 个图元扩展到 39 个，最终既无陈旧 selection overlay，也无意外诊断。该场景发现并修复了一个真实的 revision 稳定交互缓存缺陷：粘贴后全选已经正确更新引擎 selection 与 selection bounds，但前端仍渲染缓存的空 overlay。动作协议现在还要求每项端到端预算内部必须为目标解析与传输保留 4 秒，使 completion 失败时能在外层预算终止前返回精确诊断。

混合分子/图形对象覆盖也已通过 `core.selection.clipboard-delete-mixed-bond-arrow.production` 投入运行。从空白文档开始，受守卫真实鼠标输入创建一根单键和一个实心箭头；随后以真实键盘输入跨两类对象全选，经 Windows 剪贴板复制粘贴，删除得到的四个对象，再执行撤销和重做。候选 SHA-256 `dea620b455daeb253c4141e2e999eae376c5b53ecd0f7a7034795db401ea58f6` 以 evidence key `285e571b80b2442751b0cd74933e07b805bbe457405618c05ac689485ef02acf` 通过。独立回执记录键图元 `0 -> 1 -> 2 -> 0 -> 2 -> 0`、不同箭头身份 `0 -> 1 -> 2 -> 0 -> 2 -> 0`、两次全选 overlay 分别为 21 和 39 个图元，最终无 overlay 且无意外诊断。平台新增严格的 `dom-distinct-count` oracle，只能按白名单中的 `data-object-id`、`data-node-id` 或 `data-bond-id` 去重计数，避免把同一对象的多个 SVG 图元误判为多个对象。首次运行以 evidence key `2aa13393f23d7fe85b0513aaf276b9b35593a45ae3159fb62ab6c5b2daccd893` 失败关闭，根因是场景使用了静态标记中的 `Arrow` 名称，而运行时会正确暴露当前默认属性名 `Small arrow head`；已纠正定位名称，没有放宽唯一性或可见性要求。这只关闭了“键+箭头全选及剪贴板”这一个单元；分组/嵌套与区域/追加选择由各自场景跟踪，其他对象类型、部分可应用删除和跨边界剪贴板仍保留为显式缺口。

二层混合组合现已通过 `core.group.nested-mixed-clipboard.production` 投入运行。受守卫真实鼠标从空白文档创建一个分子和两个箭头；真实 `Control+G` 先把分子与一个箭头组合，再把这个混合 group 与第二个箭头组合为二层结构。选中的嵌套根经 Windows 剪贴板复制粘贴，随后全选两个外层根并以 `Control+Shift+G` 批量取消组合，再对这一个事务执行撤销和重做。候选 SHA-256 `50b3b36ffbdc95eebf1588ec80a7fe258ab7681ec094925ce6db49b400b3a308` 以 evidence key `0bca63951877cdaf30d3452ef11bde6d43a29f5aa355f8cd950da837ee5b638e` 通过。独立结构证据记录 group 身份 `0 -> 1 -> 2 -> 4 -> 2 -> 4 -> 2`，复制、取消组合和历史恢复前后的嵌套 group 身份为 `0 -> 1 -> 2 -> 0 -> 2 -> 0`，键为 `0 -> 1 -> 2`，箭头为 `0 -> 1 -> 2 -> 4`；批量取消组合后立即得到 39 个 selection overlay 图元，历史恢复后瞬态 overlay 为 0。此次修复了两类通用产品缺陷：group selection 遗漏所选分子对象，父 group 与其后代同时在 selection 中时也无法组合，而取消组合又把分子子对象误归为普通图形；增量渲染则追加 group 的聚合图元，却不重挂原有 DOM，造成键重复并丢失可观察层级。引擎现在把选择规范化为最外层已选对象并恢复完整分子选择；viewer 只递归重建受影响的 hierarchy 子树及对象 wrapper，并刷新图元索引，不退回整文档刷新。首次 production 运行以 evidence key `11ca9037e59119b4ff727fd887701e951abca6f1e7b338894f7a92f022e05773` 保留重复 DOM 缺陷证据。第二次运行保留 evidence key `c1cd08ca88422434273126c74959514071e1ac6ed3dacd1f0aae8156b0ab2161`：所有结构 oracle 已通过，但暴露出场景 oracle 时点错误；现已改为在取消组合后立即验证子对象 selection，并在 redo 后验证既有历史合同会清除瞬态 selection。这只关闭了“快捷键、同文档剪贴板、分子+箭头、深度二”单元；右键菜单组合、更深层级、其他对象类型、变换、跨 group 移动、锁定/隐藏成员、保存重开和格式边界仍是显式工作。

production 制品导出已经投入运行并且 fail-closed。runner 把每个 driver payload 的 SHA-256 纳入 evidence key，将报告与 payload 保存为不可变内容寻址对象，并由经过 Schema 验证的 manifest 引用。production CDP 只做 observer，不是输入通道或特权产品 API：它只读取公开 DOM/window 状态并采集浏览器级证据；回归测试明确禁止 production 脚本出现 `window.__chemsemaDebug` 或从内存导出 CCJS。化学文档正确性必须由后续真实 GUI 保存、外部解析/不变量检查、GUI 重开这一闭环证明。当前 guest bundle 包含最终 PNG、完整公开 DOM、公开 runtime/window/render 状态、WebView 日志和覆盖完整操作过程的 JSON 性能 trace。

性能 trace 在第一个用户动作之前通过 CDP `Tracing.start` 的 `ReturnAsStream` 模式启动，在最终采集时等待 `Tracing.tracingComplete`，拒绝底层报告的数据丢失，通过 `IO.read` 有界读取，并在超过 64 MiB 时失败。普通 CDP receipt 保持 20 秒上限；只有固定的 `artifact-export` 模式获得 90 秒 guest 上限和 110 秒 host 上限。guest 在传输前计算每个有界 payload 的哈希，coordinator 通过认证的 PowerShell Direct session 逐文件复制，并在 host 独立复核字节数和 SHA-256。任何截断、缺失字节、哈希漂移、制品重名、trace 数据丢失/超限、采集失败或关闭失败都会使整次 run 失败；较晚失败时，已采集制品按 `failure` 而不是 `sample` 保留。

此前非资格失败证据继续保留：`0d6d2cb791f607294a9d66102e243e5ab3e61b72c39b3b3b467fab06ac261165` 暴露 DOM 截断，`c5cc3386f0fa512d8ae77dec8e3f0edf5dcab4e144e297e83b88286a2456bb55` 超出控制台传输预算，`6db5152b88ff38708d62a73dc569b72894d5ebf220bd53dca93e4ef1fe607a49` 与 `08c40cf795f3b9e01986e914f2c6a0c0fb6e0d2a495a543a5a1aad22d18454aa` 暴露 trace 最终 EOF 空块的 PowerShell 空数组语义，`b01117987dcfa6496e2e09cfd74e13c9f0476c40872a4e90e3c57458fa6bb960` 则完成全部 19 项复杂操作、但超过旧的最终采集上限；后续通过没有覆盖这些失败。最终六场景影响闭包分别以 `21dd6e3d70e92825a25c02774744a98a11d0bb01e065f77c4d09999df5b28b72`、`01b1344d09fb2129e04b92e072a77a2c8e3c7097e6947d6ddfa9a6507d4ef71f`、`54a1b9801adcfb7c546818ec066c33f3305f967226eaddbcbfb078c1a5a432e1`、`262350489416c5048f0e6de1d7119797595c50181317781b67b92a61729446d4`、`998f2ed8e4130b03bad700ce7f819ff36e75ffe6727db8bea7de138931c939d1` 和 `c41db8db42e66fcc29f67b13d5a5c196af061fc4cca9745fd93c4142ff5a01de` 通过。所有 payload 实算哈希一致且未截断；五条 production trace 均可解析为 JSON，分别包含 30,328、46,504、120,892、150,253 和 226,318 个事件；每次 production run 最终都把 VM 恢复为 `Off`。视频、崩溃 dump、逐动作截图和真实保存—解析—重开化学文档证据仍是明确工作。

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

### 原生保存、独立文件 oracle、放弃修改、重开与继续编辑

生产场景 `core.document.save-open-roundtrip.production` 已完整通过真实用户路径，且不注入产品内部状态：绘制一根键；打开 Windows“另存为”；用真实鼠标和键盘聚焦、全选并输入受限输出路径；保存；独立取回并验证 CCJS；再画一根未保存的键；关闭脏标签；点击 `Don't Save`；经 Windows“打开”重新载入磁盘文件；证明只恢复已保存的一根键；最后继续绘制第二根键。原生文件名控件使用精确 UI Automation id，并同时约束 control type 与 class。原生模态窗口存在期间，driver 只使用 UI Automation 和受守卫的 OS 输入；精确顶层对话框消失后，只读专用交互会话以刷新前台坐标，不再发送强制激活动作，随后恢复 WebView 观察。

资格运行使用生产候选 SHA-256 `4a7dcc47e2f4469f5aed4f7963c6a7506fa413f7c20879984a9179632ebb6b07`，evidence key 为 `e71f891c964b3cecc4ac0d1f456e4b870c72ed27ebfa001068aed7aaf4d019d6`。保存的 CCJS 为 2,787 bytes，SHA-256 `fa5d1660cc988f21c357884f1da0e8e7eee2b9b18930e46b6a1c1268b988372b`；发布版 CLI 的 `inspect` 与 chemical validation 独立证明其为 CCJS 0.2，包含 2 个节点、1 根键、1 个分子、1 个对象和 0 个验证问题。最终公开 DOM 证明重开并继续编辑后有 2 根键。证据包含保存的 CCJS、独立 inspect 报告、最终截图/状态/完整 DOM、WebView 日志和可解压、非空的 3,138,150-byte gzip 性能轨迹。host/guest 文件大小与 SHA-256 完全一致；VM 最终回到 `Off`，分配内存为 0。

该路径发现并修复了真实产品缺陷：对已保存但再次修改的标签选择 `Don't Save` 时，标签虽关闭，但对应 recovery journal 记录未删除，随后打开未变化的磁盘文件会复活已放弃的编辑。生命周期现在会在 discard 关闭前精确压缩该文档的恢复记录，并有专门的 journal 回归测试。探索阶段的失败运行继续作为 failure evidence 保留；没有放宽门禁，也没有 rerun-to-green。

针对提交 `7a529cd` 的影响选择资格闭包已通过全部 7 个登记场景。Playwright 浏览器场景的 evidence key 为 `9dc9e46476f82cb3f0d626af73f987eec42a9a4a2d862f9626cfba2c34f5589f`。生产单键、撤销/重做、多键剪贴板/删除、键/箭头异类对象、嵌套混合组合和保存/重开场景的 evidence key 依次为 `91f33166dd237b2cd9d9532a76f72f29f80076bb9013b8a2f2cf7ebcd93e3cc7`、`b41dbaa41388d5d935f0bd1216178ed952238f518d6fe5b834b8b9bcce067302`、`0104c9132065c15056602beddea1a2a7beb880a6158c22e1a758f2b293cfd830`、`a3b36dcf1e77516c0c602ce5be6be8aa59a996cf14679a9a86e0513fb60ce6a4`、`c7073a695b43a246e30af1873a225b8dfa880822f1396287e4192533fc580fe6` 和 `e71f891c964b3cecc4ac0d1f456e4b870c72ed27ebfa001068aed7aaf4d019d6`。共 67 个动作全部完成；每个 manifest 对象重新计算 SHA-256 均精确一致；没有制品截断，没有运行诊断；每次生产运行后 VM 均回到 `Off` 且分配内存为 0。影响图现已明确把 GUI 输入 agent crate 与 `Cargo.lock` 映射到 GUI 平台，并把 recovery-journal 回归测试映射到文档 I/O，消除了这些已知路径因未知性而触发的全量扩展。

区域选择与追加选择的混合对象基数覆盖现已通过 `core.selection.region-additive-mixed-cardinalities.production` 投入运行。受守卫真实鼠标先创建一个分子和两个箭头，再依次执行空区域框选、单箭头框选、分子加箭头的异类双对象框选，以及按住 Shift 框选第三个对象。场景不用“看起来有选框”代替语义证明，而是以删除结果和撤销/重做作为 oracle：单对象路径删除后保留一个箭头和一根键；异类双对象路径删除后保留一个箭头且键为 0；Shift 追加后的多对象路径删除后箭头和键均为 0，随后 undo 恢复两个箭头，redo 再次清空。生产候选 SHA-256 `4a7dcc47e2f4469f5aed4f7963c6a7506fa413f7c20879984a9179632ebb6b07` 的 21 个动作与 4 个最终 oracle 全部通过，evidence key 为 `ef69a3036502697a5960df64886aee9f7ac6c73e6e01f7d65e4683aaeb658b36`；6 个 manifest 对象重新计算 SHA-256 均精确一致，诊断为空，VM 回到 `Off` 且分配内存为 0。首次生产尝试在第一个无修饰键点击处 fail-closed，并保留 evidence key `d5450e515986e4d69b4349f35b2258087d620f61178f8dc610493d6f708e9758`：PowerShell 曾把缺失的 `modifiers` 字段规范化为包含一个 null 的数组。现在会先过滤空 modifier，再执行白名单校验；非空修饰键在 Schema、driver、coordinator 与原生输入代理四层仍保持唯一、最多三个，且只允许 Shift、Control 和 Alt。

针对提交 `1f65db5` 的影响选择资格闭包已在没有未知性扩展的情况下通过全部 8 个登记场景。Playwright 浏览器场景的 evidence key 为 `4fb19ef38e44d8a0441bb70bcc09f12bfa95fdf10f2124553c39acbb5870ceca`。生产单键、撤销/重做、多键剪贴板/删除、键/箭头异类对象、嵌套混合组合、保存/重开和区域/追加基数场景的 evidence key 依次为 `388e4efea4cfde977dbc46a4262852b6a128f0370379b53295e93479854b89c2`、`7ffae8c76778a0fa9bc42b8bf83341f706ebf2da0d9e2f67792356cbdbe70092`、`beb3627670e4c9c9c4101a12dd342b1da58effbe11602ca66e40a84325059368`、`683e2398d4d7abd74e9a173ba617fd723b04fb8e63c7439271a911ef1e4cf719`、`d72eb7c63641ce93a985306e18722f5049f7dd9bd1788f4afe3a28516cd670c6`、`43c53632e254d14b94dd003d4c7c01755eb1189c43fda5d93ef6a4b85c0afbb1` 和 `ef69a3036502697a5960df64886aee9f7ac6c73e6e01f7d65e4683aaeb658b36`。共 88 个动作与 26 个最终 oracle 全部通过；51 个 manifest 对象的文件大小和 SHA-256 均重新校验一致；无制品截断、无运行诊断。生产 VM 最终为 `Off`，配置 8 个处理器，分配内存为 0。

跨文档剪贴板覆盖现已通过 `core.clipboard.cross-document-mixed.production` 投入运行。受守卫真实输入在源文档创建并选中“分子+箭头”，经 Windows 剪贴板复制，通过公开 `New file` 标签按钮创建第二个文档，先证明当前目标文档为空，再粘贴到这个独立文档。动作回执记录文档标签 `1 -> 2`、目标文档键 `0 -> 1`、目标文档不同箭头身份 `0 -> 1`；5 个最终 oracle 证明共有两个标签、恰有一个活动标签，以及目标文档的精确混合对象数量。候选 `4a7dcc47e2f4469f5aed4f7963c6a7506fa413f7c20879984a9179632ebb6b07` 的 11 个动作全部通过，evidence key 为 `f20a768332299ecc0d642ac3e4605607f9271749fdb119d14f3be32fd5b7d835`。6 个 manifest 对象重新计算 SHA-256 均精确一致，诊断为空，VM 回到 `Off` 且分配内存为 0。浏览器/桌面互传、Office、选择性粘贴和独立打开文档等边界仍是显式工作。

生产动作传输现改为每个场景复用一个有界 host broker 和一个经过认证的 PowerShell Direct `PSSession`。针对 `01c3525` 的九场景资格运行首次在混合对象 redo 处 fail-closed，evidence key 为 `fe6b0ad2f3f4bee8d4643d5e59706fa7e286a29a9775adc0e397e48f67ed76cf`：产品已正确把键数从 2 改为 0，但每动作新建 host 进程和临时会话的返回耗时达到 12.003 秒，违反未改变的 12 秒端到端预算。没有提高预算。broker 只接受有界 JSONL 和白名单中的 `action-transaction`，把严格参数白名单转换为命名 splatting，并在 VM 关闭前释放；guest 输入与 CDP 观测仍相互独立。首次 broker 集成失败保留在 `7ae540b9b1a3dd0bf8c4955dba3f17ae03504611779b1442bf0a91f2e705828e`，根因是位置数组 splatting，已改为验证后的命名参数。完全相同的混合场景随后以 `636d57f48d5c343f468cda12d5a719374194edbaf58e0871de08021ece342fa4` 通过：首个包含会话建立的动作耗时 4.292 秒，其余动作稳定在 2.077–2.301 秒，redo 为 2.301 秒。

broker 资格闭包已通过全部 9 个登记场景：浏览器 `576267d3c1695ad40e619b59e82a41d0bd9b59f21f4f0a1a2d1324c656a868ba`；生产单键 `8f3050f525f7d5fca641b49a02ebacc99ffa9d7fd4940d23f68bd06b7e1808d9`、历史 `9e2d1f03b021f6b4c9eca52ae0328228c09075568938cc622a32a426c02a971a`、多键 `50c9042b06a256d8add5c140fb088ae5f50fd2c659a406ea6feb9d21711b37fd`、混合对象 `636d57f48d5c343f468cda12d5a719374194edbaf58e0871de08021ece342fa4`、嵌套组合 `fd1b4d24dc89b84c07bf32afbcdedbeb014bea5202dc7339d654ea973447cacd`、保存/重开 `7111c295b24643fba15762b04b9af4cdbda244ea5da70408087f3200642d9212`、区域/追加 `86360d45c8cd3199286d8a8b513746b05636d67c28068877f00ce04880f04628`、跨文档剪贴板 `2eb71278cea5ec472ba0c2ce6947470487b0a308e9b57c59ab962dfe80c25552`。99 个动作与 31 个最终 oracle 全部通过；57 个 manifest 对象重新计算 SHA-256 均一致；诊断和截断均为 0。VM 最终为 `Off` 且分配内存为 0。影响图现在区分生产传输、生产 driver、浏览器 driver 与平台测试来源，使仅传输层变化只失效 8 个生产场景，不再无意义重跑浏览器场景。

### 锁定对象合同与“部分可应用”删除

CCJS 0.2 已把 `locked` 定义为“对象是否可编辑”，但桌面端此前没有公开的锁定/解锁操作，通用删除路径也没有执行这一合同；只有图片拖动、谱图等少数交互零散读取该属性。引擎现在通过公开右键菜单提供 `Lock`/`Unlock`，把一次锁定变化记录为一个可撤销命令，把选中的分子节点/键映射到所属分子，并在对象自身或任一祖先锁定时判定其“有效锁定”。通用删除会规范化选择，仅删除有效可编辑成员，保留锁定成员；全锁定选择删除为空操作，锁定与未锁定混合删除则仍是一个完整的 undo/redo 事务。

生产场景 `core.selection.locked-partial-delete.production` 使用受守卫真实输入完整点击公开路径：绘制两个箭头，框选第一个，精确右击该渲染实体，点击 `Lock`，再次打开菜单并观察 `Unlock`，共同选中已锁定与未锁定箭头，执行删除、撤销、重做，最后证明幸存箭头仍显示 `Unlock`。候选 SHA-256 `7dfee3e1fe541336f9809d46a299febc6cfa1d965314beea02b2e69269d66124` 的 15 个动作和 3 个最终 oracle 全部通过，evidence key 为 `32e99f68f555666037266c87e983b9b204a52ea46db2f008973781c7e70dc24a`；6 个 manifest 对象实算哈希完全一致，诊断为空。本轮还为稳定的渲染对象 wrapper 增加了语义化 `entity-id` 生产目标、严格的公开右键菜单 completion，并收紧场景 Schema：不相关的 target/value 字段会在场景验证时直接拒绝，不再拖到动作协议才失败。

两次失败运行继续作为诊断证据保留。`8b43321a71b1cc6ccb77199340041a16ae41c08922ad6d7494776aecd85a3658` 证明首版语义定位器错误要求 `data-object-type`，而增量渲染对象 wrapper 的稳定合同实际是 `data-object-id` 加 `data-renderer`；定位器已按该通用合同修复。`0378f5c5eaa815b798f76fa5414851742974a7226e9471f0460d513104ded95a` 证明带 target 的 `actionable` completion 曾通过场景 Schema，却被更严格的事务 Schema 正确拒绝；场景现改用 DOM completion，两层 Schema 也都会拒绝这种无关字段组合。

受影响资格闭包随后通过 10 份报告：浏览器单键 `8eb5a5061cdfd43c8c2ff7bc76024fc56466f88e66912c55c739f32173216b77`；生产单键 `b2180ec1aec77a0937456da1fcf212cdc99e89ec64aca26f5d8c2282f2ce4725`、历史 `a9c90df5943ed038b095f408f704e02f7548e0c4a50f60ace0769b0637be281d`、多键剪贴板/删除 `febb2f752c619f0043a54ee242c39c10ad1bcb47fda351f348fbe83358bef6ec`、键/箭头混合 `03a5a3eef6a8123884e769e63ed68a40ea7a004e83fa9d8087a6966dcb3ba724`、嵌套组合 `9ae1dfae810db87d204f13694877d9c575820b5d831f205bc16076e9b9f9ccc6`、保存/重开 `e170d036ccad86b12716f1d65c74ef937109c6e909353fcf7ae679ad99d8111c`、区域/追加选择 `6ccd952adb99d57956f8b0bd3828c02c5018422a2e341d3a87648353fb55794f`、跨文档剪贴板 `d0885adeee208a740d9db9c0d9c4bfa255f1761dba16a6a7d345512bfb32bd00`，以及上述新锁定部分删除场景。所有动作完成、所有 oracle 通过、诊断为空；63 个 manifest 对象均被独立重读，文件大小和 SHA-256 与声明完全一致。VM 最终为 `Off`，分配内存为 0。

这只关闭了“两个箭头、一锁一解锁、混合选择后部分删除”这一单元，并不宣称锁定覆盖已经完整。组合/嵌套选择、隐藏或重叠对象、其他对象类型与属性、经 GUI 建立的锁定祖先、保存重开与格式边界、以及大文档行为仍是明确的 registry 工作。因此 coverage registry 把该能力记为 partially migrated，而不是 complete。

### 锁定对象的部分变换与世界坐标证据

选择变换现在会先把“界面上已选择的对象”投影为“有效可编辑成员”，再建立移动、旋转、缩放、排列或命令目标。锁定分子不再贡献可移动节点；对象自身或任一祖先锁定时，也不再贡献对象变换。文档即时预览渲染器执行同一套“有效锁定”规则，因此低延迟 DOM 预览不会再把引擎已正确排除的锁定成员在视觉上一起拖走。活动按压期间的 pointer down、move、up 现按序执行；专门回归测试证明，即使 move/up 紧接着到达，也会等待异步手势初始化完成，不会越过 pointer down。

生产协议新增通用 completion oracle `entity-rect-deltas`：在同一个受守卫 OS 输入事务前后，观测 1–16 个稳定渲染实体 id。它把本地 `getBBox()` 的四个角通过 `documentContent.getCTM().inverse() * entity.getCTM()` 转换为文档世界坐标矩形，从而排除 viewBox/根相机变化，同时保留对象自身及嵌套变换。每个期望明确声明 `stationary` 或 `moved` 及有界世界单位容差；动作回执保留屏幕矩形、世界矩形、最大位移和逐实体判定。

18 步生产场景 `core.selection.locked-transform.production` 真实绘制两个箭头，经公开右键菜单锁定第一个，共同选择并拖动，执行撤销与重做；随后清空选择，经公开菜单解锁第一个，再在两者均可编辑时重复拖动、撤销和重做。候选 SHA-256 `22294e1dfccd1460b8c97a408c7b2f13ebabb713e703afb3427a922c04f61e5d` 以 evidence key `0d4567affec5d8661ce1940d93a8b53c2f19665c7f58d6ed426ce914bd37dbb1` 通过。锁定拖动及两次历史往返中，`obj_line_1` 的世界位移严格为 0，`obj_line_2` 约为 38.25；解锁后，两对象在拖动、撤销和重做中的位移均约为 33.00。6 个 manifest 对象的文件大小与 SHA-256 独立复算全部一致，诊断为空，VM 最终回到 `Off` 且分配内存为 0。

四次失败运行继续作为诊断证据保留，而不是被最终通过覆盖。`8f9e1c432aacc35e6ae76a6068e2334d6c0d1e3cdff9e955c68baa4c944886ac` 证明屏幕矩形会把 viewBox 变化误判为对象移动；`4b9a39118f40ac244f636b48a5a19b098ea554a8fe0c0f869467adbe10c1294b` 与 `9e114a0dc3e4361d69b09fa2e6ec1a6d967599bdeca40d23f10ec9512f895cba` 证明普通 `getBBox()` 不包含元素 transform，同时快速输入可能越过异步 pointer-down 初始化；`f31bef1e689f724bdc473c29a63995e985eb4baddd5ca901709837ee08b3e2ff` 最终从 DOM 证明两个对象被固化了相同的预览 transform，从而在后端过滤已正确后，定位出前端有效锁定规则仍不一致。

本单元只关闭了“一锁一解锁的两个箭头混合选择”中的指针移动、Unlock 与历史往返。旋转、缩放和排列的引擎路径已经执行同一投影规则，但尚无等价生产 GUI 场景；分子、文本、形状、组合与锁定祖先，属性编辑器和其他命令，保存重开及格式边界，复杂/large/xlarge 文档，长序列与 endurance 也仍是明确未完成单元。因此 registry 新增的 `capability.selection.transform-partial` 状态是 partially migrated，而不是 complete。

候选 `22294e1dfccd1460b8c97a408c7b2f13ebabb713e703afb3427a922c04f61e5d` 的最终影响选择闭包在没有未知性扩展的情况下通过全部 11 个登记场景：浏览器单键 `a037d110e20ae9f63392292f56efd4109827ead8478ad8f128b54ddecde2ec6c`；生产单键 `cfaac5ef5a8db226154ea2fcd9f87e3531b269022f669a561afabee3308d2ec1`、历史 `44549b59e7745bd5aba2ed002d1895f27b03589462545c200662c868cde0140e`、多对象剪贴板/删除 `6f667b79643666b414e059549b614f01222cb854665d43d25afc6452c780bc99`、键/箭头混合 `8673c04af5ec75c01c9ef2b26cdf2f4751ba512340e41f3cda22189392164f7a`、区域/追加 `140916d4a2aec4521bbebb6fe3118fedfdce0f1c3978dcd33d31e7f65dd8cd4b`、锁定部分删除 `e169a4df9a441803700b74358d3e9b6a122e95a1acb67b83b05e208bb300b7fa`、锁定部分变换 `d9e6adfcd62e8546d57cd9e39cb512546f97da84ece62473f68937582638f8d3`、跨文档剪贴板 `b85d94199ac85d4bcab68f5b95f46d1d29bcfcb2a350d67ab6d2ff0d67e99deb`、嵌套组合 `b4a7423528da7e97e95d3dd0493de55b10decaddc303cea9043962d0cf3c04a7` 和保存/重开 `9a22648023ba54cfb382c34c520840241f8d4608fa7037276684b9290ed8d36f`。132 个动作和 37 个最终 oracle 全部通过，诊断为 0。69 个 manifest 对象均被独立重读（含按 UTF-8 读取中文原生对话框报告），文件大小和 SHA-256 与声明完全一致；生产 VM 最终为 `Off`，分配内存为 0。

异类生产场景 `core.selection.locked-molecule-arrow-transform.production` 把合同扩展到 GUI 真实创建的分子与箭头。它经公开右键菜单锁定分子，证明拖动、撤销和重做中分子聚合世界几何严格保持 0 位移，而箭头移动约 43.50 世界单位；随后公开 Unlock，并证明第二次拖动及历史往返中两个语义实体均移动约 43.50。候选 `22294e1dfccd1460b8c97a408c7b2f13ebabb713e703afb3427a922c04f61e5d` 的 19 个动作和 4 个最终 oracle 全部通过，evidence key 为 `3bd51ce9eafcb7318d5ed51a1089913dfb19d419a0c5daffe83c54f6153a7aa3`；6 个 manifest 对象重新计算哈希完全一致，诊断为空，VM 回到 `Off` 且分配内存为 0。

语义 `entity-id` 观测现在同时支持“可见场景对象 render root”和“由多个同 object id 图元组成的分子”。输入定位优先选择有可见几何的 render root，否则选择一个稳定可见图元；几何观测同样优先可见 root，否则合并全部可见图元的屏幕矩形与 relative-CTM 世界矩形。水平或竖直 SVG 线只要任一轴有长度即可操作；空的增量渲染哨兵组不再遮蔽真实图元。四份失败证据分别记录被推翻的假设：`8688518ca0d26b01ccfa3f7f78fb203ff5d6c0993e0d3e79e3818439eae86279` 错误强制要求 `data-renderer` wrapper；`1f8cf81a246759d2966a16612d856a626b0783ca5a276dbbaf467dd213f0804c` 把水平线的零高度矩形误判为不可见；`2045c9cac396906170ee97d0c31a9a500225bd47ac8c454c07194efc3ee2539d` 暴露分子分配 id 后场景使用了错误箭头 id；`29674d04c1c5b171e78e6b89491ab2f18c81f1cc19eb8cfd9e1f4a3fcf432fa4` 证明空 render 哨兵在后置观测中遮蔽了真实分子图元。

由此产生的 12 场景影响闭包全部通过：浏览器 `7c91004b393e8f2dcbf24860c33df09e7f4299a25077e7e3cf93c462b6195504`；生产单键 `735526d363cd02ac5edba8f917536459d82c50a667d6897a823cf96f1088350f`、历史 `e74b877c11f94d36dc39d2e68e8ed7b77a13fa5b534b02f4630c50b35063db74`、多对象 `447138916016ccad629b012190b0e33b299a16e92ac5560da63dc1419758397a`、键/箭头混合 `2243dfa3bd1e78bc5be1640ea27f00b6f38bef735849d31c8a0c66359a4bcf70`、区域/追加 `700efe10f693d8d39cf3f076b62b6882a63890f4b75b63f0d387dfc8bfb89fc7`、锁定分子/箭头变换 `12696f5863b77aff23b6218b8d7fca81130cba648b2ff92eee2d7dec68473f20`、锁定部分删除 `64a704e5ea8cbe4efd0cddf583f4e2272c0e7e28acf606d7069097a8eca47ce7`、锁定箭头变换 `4d5a08ca15d41666160f727ba9e2507867eadd8b3b6bb9c1cafa28a6dc6b149f`、跨文档剪贴板 `bd4d652f589d173bcdd1cf9c6e58e8f46a23a1737a3a1f2cd03d17082ded802b`、嵌套组合 `8f6e6d7d1e1e6ad6e81f0e2814bcf45c9ca1e7fa49e0e92356e7f100b399bb7c` 和保存/重开 `32b67e2ec93b63538830ec71b2b6b10170731414afda857176383301df76619f`。151 个动作与 41 个最终 oracle 全部通过，诊断为 0；75 个 manifest 对象独立复算大小和 SHA-256 全部一致；VM 最终为 `Off` 且分配内存为 0。

### 锁定祖先组合的原子交互与后代变换

生产场景 `core.group.locked-ancestor-transform.production` 通过真实鼠标、右键菜单和键盘创建两条箭头并组成 group，经公开 `Lock` 锁定父组，再创建一个可编辑根对象并全选。锁定阶段的拖动、撤销和重做中，两个后代 `obj_line_1`、`obj_line_2` 的最大世界坐标位移每次严格为 0，根对象 `obj_line_4` 每次移动约 33.000；清空选择后，从任一可见后代重新选中锁定父组并执行公开 `Unlock`，随后第二次拖动及撤销/重做中三个对象均移动约 33.000。候选 SHA-256 `01bba532076bffbef1770be96c5f5a17abb080b5e6654c3b7dadd7d7ecf4b6ec` 的 22 个动作和 4 个最终 oracle 全部通过，evidence key 为 `7fb2ace1b8e9df6cf76742f1c0de6dbcaaaec66d7b6264c457cde5d03acb9ddb`；5 个工件全部保留，诊断为 0，VM 最终为 `Off` 且分配内存为 0。

该场景发现并修复了两个产品缺陷和一个测试平台缺陷。已选 group 的外接框中心可能是空白，旧的右键命中也会把可见后代降级为普通子对象；引擎现在把已选祖先 group 的后代右键提升到该 group，并让任何锁定祖先 group 表现为原子对象：普通点击或右键其后代都会选中锁定父组，因此用户可以可靠地看到并执行 `Unlock`，嵌套时会沿祖先链查找。生产语义定位则把 group 的输入点放到一个真实可见的渲染后代上，同时继续用完整语义实体聚合世界几何。失败证据 `a80bcb249bce75a47fb6eeb0c8f262af5f0cc8454ee134a18c5d84041e5f2bb0` 与 `1a78ad44008926f9bd60a4357077ac61ad34914c622a781b02713759e8cf86b8` 记录空白外接框中心无法打开菜单，`70be641a752d6d38df0e0ea94907bb870758eceb59fcd9f50ab83de874a3f473` 则证明锁定后代的变换合同已经成立、但父组当时无法重新选中并解锁。覆盖登记现为 26 项、13 个场景、0 个未解释缺口、0 条警告；锁定嵌套祖先、后代部分选择、删除和属性命令、保存重开与格式边界仍是显式工作。

候选 `01bba532076bffbef1770be96c5f5a17abb080b5e6654c3b7dadd7d7ecf4b6ec` 的最终 13 场景影响闭包全部通过：浏览器单键 `6af74e1e86c62d1d265807f7eeef8272030b046f6e2331e5b078bf9dc773a329`；生产单键 `a31f33f17939db7faf9f76c36e96a25cdb63d53bcf3f694e7b2836eb3cd8b1c1`、历史 `6f7f5ed13262d4820b377aa0a24191dd1a17ca9ed024159c62ce0dd776ef8911`、多对象 `e4eac55c2e18683c1fc8f3ca75381508a67eedda9195980d9b3eda9e9afcd189`、键/箭头混合 `0b0837dff78e9ecdbe64eeef65526e5ba7f8c4b6e7a713fabffe0c428debb91c`、区域/追加 `fd2f6583ec3c387c4aa578ae1f834ac92107cab8b36b4b0ec693a6c227d3260f`、锁定部分删除 `a92eae8291430061e739e7045dfd23a5a2b5441393e6838fbcda38044704b392`、锁定箭头变换 `836cf34e89f7e351f2c0d69a841db0570931bf0e962c51d84120ec2fc4c34500`、锁定分子/箭头变换 `dc28bf3f51884bf3a3f5ef23d042f41ec9a47de72cda33547f40656b53798fa0`、跨文档剪贴板 `d5abbc23b4cc0cbeee1f901af62ba8a24d130f788133794db341195a21fa3ef7`、嵌套组合 `02a79f2a623543d0b6e52caeb9c42695e8b749b4002bedc21488724627f35a47`、保存/重开 `78cd47f8ed7ca87ae3c7994971bfbab11be6322bc2061630a7f071d785f2295d`，以及锁定祖先组合 `7fb2ace1b8e9df6cf76742f1c0de6dbcaaaec66d7b6264c457cde5d03acb9ddb`。173 个动作与 45 个最终 oracle 全部通过，失败动作、失败 oracle 和诊断均为 0；81 个 manifest 对象均被独立重读，文件大小与 SHA-256 全部一致；所有生产报告只包含上述一个候选哈希。VM 最终为 `Off`、分配内存为 0，且没有残留 `vmwp` 进程。

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
