# ChemSema 专用物理 GUI 测试机交接

## 目标

把一台空闲 Windows 电脑建设为 ChemSema 的专用、无人值守 GUI 测试工作机。代码以
GitHub `dreamlovesu32/chemsema` 的 `main` 分支为唯一来源；测试机不得依赖开发机目录
拷贝，也不得要求 Codex 在测试运行期间持续在线。

这台电脑是专用物理测试机，不再要求 Hyper-V。现有 Hyper-V worker、场景、oracle、
证据协议和资源预算是可复用基线，但不得把 VM 名称、检查点或 PowerShell Direct 当作
物理机执行的前提。

## 开始前必须读取

按顺序完整读取：

1. `AGENTS.md`；
2. `docs/gui-test-platform-and-demo-reliability.zh-CN.md`；
3. `docs/gui-test-progress.zh-CN.md`；
4. `packages/gui-test/` 下的协议、runner、driver、oracle、qualification 和现有 worker；
5. 本文件。

不得根据聊天摘要代替仓库文件。进度以当前提交中的注册表、场景和进度文档为准。

## 不可降低的标准

- 必须通过真实鼠标、键盘和 Windows 输入链路点击并使用每一个公开功能，实际创建或
  绘制每一类公开对象，并修改其全部公开属性；覆盖 `0/1/2/many` 同类、异类和混合
  多对象、组合与层级、复制粘贴、撤销重做、保存重开、复杂文档、大文档和超大文档。
- 生产黑盒资格不得调用 Test ABI、私有调试状态或内部文档注入；用户动作必须是 OS
  输入，判定必须来自公开 UI/UIA/DOM、文件、剪贴板或 Office payload、截图、日志、
  崩溃制品和浏览器级性能 trace。
- 增量选择只能跳过依赖闭包已被证明确实未变化的场景；影响关系不确定时必须扩大范围。
  每晚、里程碑和发布前仍保留相应的完整门禁。
- 所有证据内容寻址并校验 SHA-256；缺失、截断、候选混合、诊断非空或环境不合格都必须
  fail closed。
- 资源预算由机器本地 profile 声明并持续观测，不再固定为 10 CPU/30 GiB；允许充分使用
  专用机资源，但必须保留安全余量，检测到低内存、失去响应或资源失控时暂停队列。
- 24 小时 soak 的实际持续时间不得缩短；1000 次展示流程必须是真实完整执行，不得用
  推算代替。

## 仓库接管与基线验证

在新电脑的 PowerShell 中，优先使用 `D:\Projects\chemsema`。如果磁盘布局不同，可以
选择另一固定绝对路径，但必须把路径写入测试机本地配置，不能提交个人绝对路径。

```powershell
New-Item -ItemType Directory -Force -Path 'D:\Projects' | Out-Null
Set-Location 'D:\Projects'
git clone --filter=blob:none https://github.com/dreamlovesu32/chemsema.git
Set-Location 'D:\Projects\chemsema'
git switch main
git pull --ff-only origin main
git status --short
git rev-parse HEAD
git remote -v
```

`git status --short` 必须没有输出。接管任务给出的基线提交必须是当前 `HEAD` 的祖先；
若不是，停止并报告，不得自行回退或强制重置。

检测并安装缺失的 Git、Node.js/npm、Rustup、仓库锁定的 Rust toolchain、
`wasm32-unknown-unknown` target、MSVC C++ Build Tools、Windows SDK、WebView2 runtime 和
Tauri 构建依赖。优先使用仓库声明的版本，不要盲目升级生成物工具链。然后执行：

```powershell
Set-Location 'D:\Projects\chemsema'
npm ci
rustup show
npm run gui-platform:test
npm run gui-platform -- audit
$env:CI='true'
npm run verify
Remove-Item Env:CI -ErrorAction SilentlyContinue
```

若完整验证失败，先保存原始退出码和精简日志，区分代码失败、工具缺失、网络失败、超时、
生成物不同步和工作区污染；不得把超时直接写成测试失败，也不得修改基线来迎合环境。

## 物理工作机实现边界

先审计现有 worker 协议，再补充一个明确命名的物理 Windows worker/profile。不要偷偷让
现有 `windows-gui-worker-current.json` 同时代表 Hyper-V 和物理机。两类环境必须有明确、
可验证的能力声明。

物理 worker 至少需要：

1. 本机按当前专用机政策使用已登录的真实 Windows 账户，不创建额外测试账户，也不配置
   无密码自动登录；profile 必须精确绑定 `DOMAIN\\User` 和交互会话，凭据不得进入仓库或证据；
2. 固定且解锁的交互式控制台会话，固定 DPI、分辨率、主题、语言区域、字体、WebView2
   版本和电源策略；屏幕锁定、会话切换、RDP 改变桌面尺寸或检测到非测试输入时立即
   fail closed；
3. 候选安装、启动、进程身份、前台窗口、UIA/CDP观察和真实输入前后状态证明；不得向
   用户正在操作的其他电脑注入输入；
4. 每个场景结束后原子写入 checkpoint、运行报告、证据 manifest 和 SHA-256；机器重启
   后只能从最后一个完整 checkpoint 续跑；
5. 后台队列、场景分片、影响图选择、资源预算、超时、重试上限、失败聚类和最终资格汇总；
6. 独立的 `runs/` 或外部结果根目录。大型截图、视频、trace、日志和候选安装包不提交 Git；
   Git 只保存协议、场景、oracle、小型固定 fixture 和必要的回归摘要；
7. 测试启动后完全脱离 Codex。Codex只负责建设、读取压缩结果、分析新型失败和修复；不得
   轮询或陪跑数小时。

优先把物理机差异封装在 worker adapter，复用现有 scenario、driver、oracle、evidence、
qualification 和 impact graph。不要复制另一套测试平台。

## 无人值守证明

在进入大规模场景前，必须完成一次独立性验收：

1. Codex生成一个有唯一 ID、固定候选哈希和明确场景分片的运行 manifest；
2. 启动独立后台执行器，记录 PID、启动命令、日志路径和结果根目录；
3. Codex任务结束，不等待测试；
4. 后台进程在没有Codex参与的情况下完成至少一个真实 production-black-box 场景；
5. 新的短任务只读取最终摘要和必要失败证据，验证候选哈希、动作、oracle、诊断和全部制品
   哈希；
6. 重启测试机后验证一次 checkpoint 续跑；
7. 只有以上全部通过，才扩展到长队列、复杂/大文档、24小时 soak 和1000次展示流程。

## 每次短 Codex 任务的结束条件

每个任务必须在仓库内更新 `docs/gui-test-progress.zh-CN.md`，并留下：

- 当前提交和候选 SHA-256；
- 已完成、失败、阻塞和未开始清单；
- 精确验证命令、退出码和证据路径；
- 是否修改了影响闭包；
- 下一项最小可执行工作；
- 干净工作区或明确列出的保留修改。

达到边界后结束 Codex 任务。不得为了“继续观察”而让任务保持运行。

## 禁止事项

- 不复制开发机整个工作目录、`target/`、`node_modules/`、个人凭据或临时证据到测试机；
- 不使用 `git reset --hard`、强制推送或覆盖未知修改；
- 不把测试机上的实验修改直接提交到 `main`；使用独立分支、验证和PR；
- 不在测试运行过程中自动拉取新提交；一个运行 manifest 始终绑定一个候选哈希；
- 不因为物理机没有Hyper-V而删除现有Hyper-V能力；两种worker可以长期并存。

## 2026-08-11 本机已落地状态

- `physical-windows` 已作为独立 worker kind 落地，与 `hyper-v` 通过 factory 明确分流；物理
  profile 保存在 `%LOCALAPPDATA%\\ChemSema\\gui-test\\profiles\\physical-current.json`，不提交
  机器名、账户和机器标识。
- 输入由持久 Rust agent 通过 Windows `SendInput` 完成；每次输入精确验证当前账户、会话、
  候选 PID、内容寻址可执行文件、前台窗口和有界 run root。UIA 与 CDP 只负责定位和观察。
- 输入 agent 使用 Per-Monitor-V2 DPI awareness，使 Windows 原生对话框的 UIA 物理像素、
  `GetWindowRect` 和 `SetCursorPos` 使用同一坐标空间。
- 候选 `11bb7b20b2988b9cb9db856bc3398000b0bdbedeeaa39a6a04babfef2199133c`
  已在本机通过单键真实 click/drag，以及 17 动作的保存/关闭/重开/继续编辑场景；后者
  独立 CCJS oracle 为 3/3 通过，evidence key 为
  `0b32fddce406dd9edd4f2604450fc9288eff707b104a2fc8ba76f59a05e89030`。
- `npm run gui-physical-worker -- start|status|stop` 提供脱离 Codex 的连续后台队列。执行器
  具有单实例租约、15 秒心跳、PID 清单、提交/候选/profile/queue 哈希绑定、逐场景检查点、
  低内存暂停、明确停止请求和 evidence manifest SHA-256。
- Computer Use 不属于正常执行循环；只允许在自动定位无法诊断的罕见校准问题中临时使用。

机器本地启动示例：

```powershell
npm run gui-physical-worker -- start `
  --profile "$env:LOCALAPPDATA\\ChemSema\\gui-test\\profiles\\physical-current.json" `
  --queue "$env:LOCALAPPDATA\\ChemSema\\gui-test\\queues\\current.json" `
  --state-root "$env:LOCALAPPDATA\\ChemSema\\gui-test\\daemon"
```

状态和停止命令使用相同的 `--state-root`。后台队列只接受干净工作区；启动后不得自动拉取
代码或跨候选续跑。当前尚未完成重启后的 checkpoint 恢复验收、完整 25 场景资格、Office、
最终安装包、复杂/large/xlarge、24 小时 soak 和 1,000 次展示门禁。
