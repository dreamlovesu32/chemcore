# 物理 Windows GUI 工作节点进度

更新日期：2026-08-11

本文档是当前专用物理 Windows 测试机的长期状态源。聊天记录、临时终端输出和 Codex 进程都不是权威状态源。

## 1. 机器与仓库绑定

| 项目 | 当前值 | 状态 |
| --- | --- | --- |
| 物理机 | `JIAJUN`，Windows 11 Pro `10.0.26200` | 已核验 |
| 交互账号 | `JIAJUN\dream`，Session `11` | 已核验；不配置自动登录 |
| CPU / 内存 | 24 逻辑处理器 / 63.435 GiB | 自适应使用，不设固定 10 核/30 GiB 上限 |
| 正式仓库 | `D:\Projects\chemsema` | 从公开 GitHub 全新克隆 |
| 可信基线 | `dc9d8a78b1f7ebfcc42b7077ec49f842650fef20` | 当前分支起点 |
| 工作分支 | `agent/physical-gui-worker-windows-20260811` | 进行中 |
| 旧仓库归档 | `D:\Projects\chemsema-pre-physical-gui-20260811` | 完整保留，未清除未跟踪测试/化学文件 |
| 其他仓库 | `D:\Projects\chemsema-nomenclature` 等 | 不在本任务清理范围；未修改 |

资源策略为 `adaptive`：允许使用全部可用 CPU 和内存，但新场景开始前必须保留至少 4 GiB 可用物理内存；超过系统提交阈值或桌面/会话证明失败时停止准入。物理桌面始终只允许一个输入流。

## 2. 旧软件和用户文档边界

- 已完成 `D:\Projects` 下相关化学软件仓库、ChemSema 安装程序、快捷方式、文件关联、进程、服务和计划任务的窄范围清点。
- 未删除任何 `.ccjs`、`.ccjz`、`.cdx`、`.cdxml`、`.mol`、`.sdf` 或图片等用户化学文档。
- 旧目标仓库未直接删除，而是整体移动到日期归档路径。
- 当前仍安装的 ChemSema `1.0.0-beta.1` 来自较早构建；待当前分支完成最终打包后替换。
- `.ccjs`、`.ccjz` 已指向 ChemSema；`.cdx`、`.cdxml` 当前仍残留到不存在的旧 ChemDraw 路径，必须由当前 ChemSema 安装/注册流程修复并复验。
- Office、WebView2、Git、Node、Rust 和其他用户软件未被清理。WebView2 实际版本为 `151.0.4129.72`。

## 3. 可信基线结果

| 门槛 | 结果 | 原生日志 SHA-256 |
| --- | --- | --- |
| `npm ci` | 通过，0 漏洞 | `693E6311544FD49E7A50A3E0C53B3BD76C236547B24CF77749F3493ACFFF19E3` |
| `npm run gui-platform:test` | 72/72（基线） | `C589030C3FD0C5AC8009C4EC92E649981269874610857C4113F58ADA7BBC1F87` |
| `npm run gui-platform -- audit` | 25 场景、0 缺口、0 警告 | `AE82FC391A8726B245D07B142B685FC8D353C5B4BF44AAD3D597E7BE418E8CB2` |
| `CI=true npm run verify` | 通过 | stdout `D63E0799641D00A8343A5B6D1737BA6DD37CB2ACAE0A0F119F985642BB1D0FA3` |

原始日志位于仓库外：`D:\Projects\.codex-chemsema-physical-gui\baseline-dc9d8a78-20260811\raw`。

## 4. 已实现的物理工作节点能力

- 明确的 `physical-windows` worker profile 和 factory；现有 Hyper-V profile、10/30 聚合预算和 SYSTEM CDP 证明保持独立。
- 当前账号、交互 Session、Explorer、Default input desktop、前台窗口、候选 PID、候选可执行文件和 SHA-256 的 fail-closed 证明。
- 内容寻址的输入代理与桌面候选部署。
- Windows `SendInput` 的真实鼠标、拖动、键盘和文本输入；每个请求绑定完整账号、会话、候选 PID、候选路径和受限运行目录。
- 高 DPI 感知，UIA 与输入代理统一使用物理屏幕坐标。
- 持久输入通道、持久 loopback CDP 通道、UIA 查询、原生文件对话框、文档输出和 SHA-256 验证的证据传输。
- 单实例后台守护进程、原子锁、心跳、子进程身份、检查点、停止请求、资源余量暂停、产品失败暂停和无自动重试规则。
- 防睡眠辅助进程及心跳；不修改 Winlogon，不配置无密码自动登录。
- 停止/重置只允许终止状态文件中记录且可执行路径属于测试根的进程。

当前自动测试：GUI 平台 `78/78`；输入代理 Rust `8/8`。

## 5. 物理机运行证据

| 场景 | 结果 | 动作 | 候选 SHA-256 | 证据 |
| --- | --- | ---: | --- | --- |
| `core.bond.draw-single.production` | 通过 | 2/2 | `348e53ea300daa62463eaaeaba1c07248a919e089f158afdc9a264c03eed1d35` | `D:\Projects\.codex-chemsema-physical-gui\first-e2e-attempt2-dc9d8a78-20260811` |
| `core.document.save-open-roundtrip.production` | 通过 | 17/17 | `61ad1a32bb1f36d229e985a360febc9b716439051167a21df8bfe52773be922d` | `D:\Projects\.codex-chemsema-physical-gui\roundtrip-attempt2-dc9d8a78-20260811` |
| `core.arrow.property-matrix-persistence.production` | 通过 | 40/40 | `16d0fd50a9ba28a282acaacc12c30d52a790ebc6d3bf15bebdef423337b1d26f` | `D:\Projects\.codex-chemsema-physical-gui\arrow-roundtrip-fixed-wasm-dc9d8a78-20260811`；evidence key `f5b2faf49fcc05c49f5152ac75336e3ca83b6c773a005f983e1613e988aabfff` |

第二个场景真实完成：创建键、保存到受限 CCJS 路径、再编辑、关闭、选择不保存、原生打开对话框重开、继续编辑，并用独立文件 oracle 验证保存文件为 2 原子、1 键、1 分子、1 对象。

以下失败证据永久保留，不计为产品通过：

- 第一轮绘图：PowerShell 物理协调器输入分派、保留变量和停止函数缺陷。
- 第一轮保存/重开：200% DPI 下 UIA 物理坐标与非 DPI-aware 输入代理坐标不一致。
- 第一轮后台守护进程：候选源码闭包在 `package.json` 变化后过期；队列正确暂停且未自动重试。
- 第一轮箭头属性关闭/重开：保存文件仍为 `curved-mirror`，但导入规范化错误降级为 `solid`；失败证据位于 `D:\Projects\.codex-chemsema-physical-gui\arrow-roundtrip-dc9d8a78-20260811`。已修正通用规范化规则并增加 Rust 回归。
- 第二轮箭头属性关闭/重开：`desktop:build-fast` 复用了修复前 WASM，却生成了新源码闭包清单；失败证据位于 `D:\Projects\.codex-chemsema-physical-gui\arrow-roundtrip-fixed-dc9d8a78-20260811`。快速构建现强制重建 WASM，并由 GUI 平台测试锁定。

## 6. 覆盖与未完成项

| 项目 | 当前值 |
| --- | ---: |
| 登记功能/覆盖条目 | 36 |
| 登记场景 | 25 |
| 本物理机当前通过 | 3 |
| 本物理机产品失败 | 0（发现 1 项并已修复、回归通过） |
| 设施失败（已修并留证） | 4 类 |
| 尚待本物理机执行 | 22 |
| 24 小时稳定性 | 未开始 |
| 1000 次循环/等价压力 | 未开始 |
| DPI/缩放/分辨率矩阵 | 仅当前 200% 主机暴露并修复坐标缺陷；矩阵未完成 |
| Office/剪贴板/文件关联 | 未完成 |
| 大/超大文档 | 未完成 |

下一批顺序：最终干净提交绑定的保存/重开回归；其余 22 个生产黑盒场景；Office/关联；大文档、压力和 24 小时稳定性。

## 7. 第一阶段剩余门槛

- 用最终当前构建替换旧安装并复验四类文件关联、开始菜单、Office 激活和用户文档。
- 在干净提交上重建候选，启动后台守护进程并验证 Codex 进程退出独立性。
- 创建每小时 Codex 后台检查。
- 提交并推送独立分支，等待 GitHub CI 通过。
- 已完成同场景覆盖“创建、属性修改、保存、关闭、重开、独立语义检查”的组合门槛；需在最终干净提交候选上再次绑定证据。

第一阶段尚未完成；不得把当前三项通过解释为完整 GUI 矩阵通过。
