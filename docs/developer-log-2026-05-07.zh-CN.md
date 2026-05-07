# Chemcore 开发者日志 - 2026-05-07

作者：张家骏

时间范围：2026-05-07 00:00 至 2026-05-08 凌晨，Asia/Shanghai

说明：按今天实际工作节奏，2026-05-08 凌晨的提交仍并入 2026-05-07 开发日。

基线提交：`9496e9f Improve Office OLE preview fidelity`

工作目录：`D:\chemcore`

### 总结

今天的主线分成三层。

第一层是继续把编辑语义收到 Rust engine。分组、取消分组、层级顺序、右键菜单、对象设置、旋转/缩放数值面板、两键交汇 margin whiteout，以及选中对象属性修改，都从 viewer 临时逻辑继续迁到内核和 desktop service API。Object Settings 最终从“所有对象打开都一样”的静态窗口，改成按当前选择动态显示字段：键显示键长、线宽、双键间距、margin width、hash spacing；图形/箭头复用线宽；多选时取字段并集，某一项数值不一致时返回 mixed/空白，用户输入后只修改拥有该属性的对象。

第二层是桌面性能和长期架构收敛。昨天讨论过 `TauriEngineHost` 每次 pointer/focus 都走 IPC + JSON snapshot 会让桌面端在大文件下变卡，今天正式把长期规则写进文档：桌面默认编辑热路径使用 `DesktopHybridEngineHost`，也就是 WebView 内同步 WASM core 负责 hover、focus、hit testing、selection、drag 等高频交互；Tauri native service 负责文件、剪贴板、导出、Office/OLE、窗口和后台预览。`TauriEngineHost` 保留为诊断和未来增量 native path，但不再作为当前桌面默认热交互路径。

第三层是 Office/OLE 集成。今天从空的 `apps/chemcore-office` 开始，建立了独立 COM local server、当前用户注册命令、`IClassFactory`、`IOleObject`、`IDataObject`、`IPersistStorage`、`IViewObject2`、`IRunnableObject` 基础接口、OLE clipboard 写入、可双击激活桌面端的 verb、compound storage payload、OLE presentation streams、`CF_ENHMETAFILE`、Word OOXML package writer，以及 WMF/EMF 预览绘制路径。这个路径还没有达到 ChemDraw 的外部显示质量，但已经不再是“只能复制一张普通图”：Office 现在能识别 Chemcore OLE class，Word 能粘贴为可双击打开的 Chemcore object，并且对象内部保存 Chemcore document payload。

### 工作范围和代码面

今天检查的提交范围是：

```text
719aa73^..9496e9f
```

这段提交总差异约为 70 个文件、约 19443 行新增和 706 行删除。和 5 月 6 日相比，今天不是单纯新增几个编辑功能，而是同时推进了三条长期主线：Rust engine 对编辑语义的继续接管、桌面 hybrid runtime 的正式定型、Office/OLE 可编辑嵌入对象的第一轮贯通。

从代码分布看，工作量集中在四组文件：

```text
crates/chemcore-engine
  分组、层级、右键菜单、Object Settings、两键交汇 margin、选择语义、剪贴板文档裁剪。

crates/chemcore-desktop-service
  desktop engine session API、snapshot 分级、对象设置/菜单/选择/剪贴板/分组/order 命令桥接。

apps/chemcore-desktop/src-tauri
  Tauri command 边界、桌面剪贴板、Office OLE clipboard 调用、native service 能力暴露。

apps/chemcore-office
  独立 COM local server、注册、OLE interfaces、clipboard data object、compound storage、
  presentation streams、IViewObject2 绘制、Word OOXML package writer、WMF/EMF preview。

viewer/
  只保留 UI/host 职责：动态 Object Settings 表单、右键菜单展示、desktop hybrid host cache、
  选择交互、cache bust 和桌面/Web 行为一致性。
```

今天最重要的结构性判断是：不能让 Office 层、桌面层或 viewer 层绕过 engine 重新实现化学规则。Object Settings、右键菜单、旋转/缩放、选择、分组、bond crossing whiteout 这些看似 UI 的东西，实际都会影响文档语义、导出、Office payload 和两端一致性，因此必须落在 engine API 里。相反，Office OLE、Windows 剪贴板、注册表、compound storage、Word OOXML 这些系统集成能力不应该塞进 engine，而应该由 desktop/office adapter 调用 engine 的 render/document API。

### 分组、层级和 Scene Object 顺序

今天先补齐 native grouping 和 ordering。Rust engine 新增 `engine/groups.rs`，把 group/ungroup 和 bring/send order 操作放进 engine command，而不是让 viewer 直接重排 scene object。`group_selection()` 会在同一 siblings 层内把选中对象包进 `object_type = "group"` 的 `SceneObject`，保留 children、z-index 和插入位置；`ungroup_selection()` 则把 group 的 children 放回原 sibling 层。排序命令通过 z-index rank 做 bring forward、send backward、bring to front、send to back。

这次还同步改了选择、删除、拖拽、渲染和 CDXML import/export。`engine/select.rs` 开始把 `line`、`bracket`、`symbol`、`shape`、`group` 等 scene object 纳入同一套命中和区域选择；`document.rs`、`render.rs`、`render/bounds.rs` 和 CDXML 相关代码也开始支持 nested scene objects。桌面 service 和 Tauri command 同时暴露 group/order API，viewer 通过 `EngineHost` 调用，不再在前端自己解释层级。

这一步的意义是让后续 Object Settings、右键菜单、复制粘贴和 Office payload 都能处理“对象树”，而不是只能处理扁平对象列表。

这里还有一个隐含边界：Chemcore 的 molecule fragment 目前仍是主要可编辑化学对象，但页面上的 reaction arrow、graphic shape、bracket、symbol、text、group 都是 scene object。以前很多操作默认只有 molecule fragment，这会让对象树一旦出现就变得脆弱：删除、选择、拖拽、层级、复制、导出各走各的。今天的分组/order 不是为了做一个按钮，而是为了把 scene object 的生命周期正式纳入 engine，后续 Office 粘贴整页反应式时才不会把分子和箭头/文字/试剂条件拆散。

测试也围绕这个边界展开：分组后 selection 指向新 group，取消分组后 children 回到同一层；z-index 排序保持稳定 rank；嵌套对象在 render bounds、hit testing 和 region selection 中仍能被找到；CDXML import/export 不因为 children 结构丢掉图形对象。

### 两键交汇和 Margin Width

两键交汇 margin 是今天第一个明确的绘图规则新增。这里的 margin width 只用于键与键无共享端点的 crossing：上层键绘制前先生成白色 knockout，让下层键在交叉处断开并露出白边。默认值为 `2.0`，ACS Document 1996 为 `1.6`，当前内部字段仍以世界厘米单位数值保存和显示。

相关代码集中在：

- `document.rs`：`Bond` 增加 `margin_width`。
- `editing.rs` 和 `render_constants.rs`：`EditorOptions` 增加 margin width 默认值和 ACS 值。
- `cdxml.rs` / `cdxml/export.rs`：导入导出 `MarginWidth`，并写入 document defaults。
- `render_objects.rs`：新增 `render_bond_crossing_knockouts()`，按 crossing angle、上层键视觉宽度、下层键视觉宽度和 margin width 计算 whiteout polygon。
- `render/bond_metrics.rs`：统一 `margin_width_for_bond()` 和 legacy template fallback。
- `docs/bond-rendering-rules.zh-CN.md`、`docs/format-v0.1.md`、`docs/format-v0.1.zh-CN.md`：把 margin width 写入规则和格式文档。

测试覆盖了 later crossing bond 生成 white margin knockout、共享端点不走 crossing margin、Default/ACS margin 默认值、CDXML import/export 后 margin width 保留，以及 Object Settings 中 margin width 字段的显示和应用。

这条规则特意没有复用 endpoint contact kernel。两根键如果共享端点，那是普通键连接，应由 `render_contact` 处理 join、contact patch、wedge/hash retreat 和 label clipping；两根键如果只是几何交叉，才用 margin whiteout。这样可以避免把普通化学键连接误画成“上键盖下键”，也避免在环、支链和多键节点附近多打一块白洞。

render 层的计算也不是简单按固定矩形擦掉。`render_bond_crossing_knockouts()` 会先判断两条线段是否在内部相交，过滤近似平行情况，再根据 crossing angle 把下层键视觉宽度投影到上层键方向。对于 double bond、bold/wedge/hash 等视觉宽度不同的键，whiteout 的长度和宽度都需要随实际 bond geometry 变化。这个选择让 margin width 成为真实绘图参数，而不是 SVG 里临时加一个 stroke-white trick。

### 右键菜单和 Object Settings 收到内核

右键菜单先做了一版菜单矩阵和 viewer UI，随后按用户反馈把 Object Settings 和 context menu 从前端收到 engine。新增 `engine/context_menu.rs`，把右键菜单的菜单项、状态、可用性和命令 payload 由 Rust engine 生成。桌面 service、Tauri command 和 WASM 都暴露 `context_menu_json`，viewer 只负责显示菜单和转发用户选择。

Object Settings 也在同一次提交中被改成 engine-owned dialog payload。新增 `viewer/object_settings_host.js` 和 `viewer/numeric_dialog_host.js` 只是 UI host；字段定义、数值、单位、mixed 状态和应用逻辑都由 `engine/presets.rs` 返回和执行。前端不再硬编码“键长、线宽、粗键宽、双键间距、margin width、hash spacing”这些字段应该何时出现。

随后修正了一个关键语义：Object Settings 修改的是选中对象自己的属性，不应顺手改全局默认键长。`apply_object_settings_to_selection()` 会遍历当前 selection 中的 bonds 和 graphics，只对拥有对应属性的对象写入字段；键长会移动对应键的端点，line width 会写入 bond/graphic stroke，bold width 只作用于 wedge/bold 类键，bond spacing 只作用于多键，hash spacing 只作用于 hash/hashed wedge 类键。

多选 mixed 行为也一起补齐。`object_settings_fields()` 会先取当前选择对象支持字段的并集，再用 `object_setting_field_value()` 判断某一字段是否一致。一致时返回数值，不一致时返回 `mixed: true`，UI 显示为空白。用户填入新值时，engine 只对对应字段非空的对象应用。这个设计比 ChemDraw 式“所有打开都一样”更贴合 Chemcore 的动态对象模型。

这轮 Object Settings 的最终字段是六个：

```text
Bond Length
Line Width
Bold Width
Double Spacing
Margin Width
Hash Spacing
```

其中 `Double Spacing` 继续保持百分比语义，因为它对应 ChemDraw/ACS 中双键间距相对键长的比例；其他几项默认用 `cm` 显示，`pt` 只是可切换单位。UI 不再使用浏览器 number input 的内置步进校验，因为它会在合法输入附近弹出奇怪的“最接近有效值”提示。engine 侧只校验正数：不是 0，不是负数，不是无法解析的文本。

这部分后来还影响到右键弹窗、旋转面板和缩放面板的归属判断。结论是：凡是修改文档几何或对象属性的 dialog，都应该像颜色一样由 engine 暴露可渲染 payload 和 apply payload。viewer 可以负责排版、输入和焦点，但不能自己决定哪些对象可改、如何应用、是否入 undo、是否刷新 label geometry。

### 桌面性能和 Hybrid Runtime

用户在大文件中测试桌面端时发现聚焦/点击延迟明显，而浏览器端正常。今天先针对已有 native path 做了一轮性能优化：`crates/chemcore-desktop-service` 增加 `DesktopEngineSnapshotMode` 和 `snapshot_json()`，把 document、selection、interaction、state 四种刷新范围区分开；`viewer/engine_host.js` 给 `TauriEngineSession` 增加本地 cache、snapshot apply、export dirty 标记和串行 mutation queue；viewer 侧避免每一次选择/hover 都刷新完整 document JSON、render list、bounds、CDXML 和 SVG。

这些优化让 native IPC path 不再每次都传完整文档，但分析后仍确认：真正高频的 pointer move、hover、focus、selection drag 如果全部走 Tauri IPC，架构上仍会比 WebView 内同步 WASM 慢。于是今天把长期策略写进 `docs/windows-desktop-office-architecture.zh-CN.md`、`docs/architecture.zh-CN.md`、`docs/rust-engine-architecture.zh-CN.md` 和 `docs/project-rules.zh-CN.md`。

最终规则是：

```text
DesktopHybridEngineHost 是桌面默认编辑运行时。
WASM core 负责热交互路径。
native desktop service 负责文件、剪贴板、导出、Office/OLE、窗口和后台任务。
TauriEngineHost 只保留为 ?engine=tauri-native 诊断路径和未来实验路径。
```

`a05844e` 和 `d5f4227` 只是 cache bust，确保桌面 WebView 拿到新的 `viewer/app.js` 和 `viewer/engine_host.js`，避免测试时继续运行旧 bundle。

这里的取舍比较关键。WASM 并不等于“前端另写一套内核”，它是同一个 Rust `chemcore-engine` 编译出来的 editor runtime。桌面端 hybrid 的长期含义是：编辑热循环在 WebView 里同步、低延迟地调用同一个 core；文件、系统剪贴板、Office/OLE、注册、窗口和导出走 native service。这样仍是一核两壳，而不是 Web 端一套、桌面端一套。

`TauriEngineHost` 没有被删除，因为它仍然有诊断价值，也可能在未来用于后台任务或经过重新设计的增量 native path。但今天已经明确：如果 native path 未来要承接热交互，必须先满足合并/取消 pointer 事件、增量 diff、避免完整 JSON snapshot、真实大文件下延迟不高于 hybrid path。没有这些前提，强行把 hover/focus 都放进 IPC 只会让专业软件显得迟钝。

### 选择语义修正

今天还修复了几个选择语义问题。第一个是化学编辑上很重要的小问题：画一根键，在末端写 `Ph`，切回选择工具时，应该把端点 label 纳入同一分子组件，而不是只选中那根键。engine 选择渲染和 text edit 逻辑开始把 endpoint label bounds 包进 molecule selection，`text_tool.rs` 增加对应测试。

后续 Office 调试过程中又发现两类选择回归。`aace7e1` 把 selection box 改得更贴近内容 bounds，并避免单击选中 OLE 对象/画布元素时触发不必要 rerender。`a4a7b71` 修复 click selection refresh 和 component focus：框选仍可选组件，单击选中时不应把其他对象全部清成不可聚焦状态，也不应出现页面跳一下的刷新感。

最后恢复了单击 primitive selection 的产品语义：单击仍选中单个键、点、label、shape 等 primitive；双击或组件命令才选中整个分子。这一点和今天较早的 endpoint label component 修复并不冲突：endpoint label 应参与分子组件判断，但单击命中不应直接升级成整分子选择。

这组修复反映了一个 UI 规则：Chemcore 需要同时支持“精细编辑”和“组件级编辑”。单击是精细编辑，适合改一根键、一段文字、一个点；框选和双击可以进入组件级，适合移动整分子或整个反应块。之前在修 component selection 时把单击也抬成整分子选择，会让用户失去局部编辑的入口。今天把这两个层次重新分开。

### Office/OLE Server 骨架

今天后半段进入 Office/OLE。项目新增 `apps/chemcore-office`，并把它加入 Cargo workspace。这个 crate 生成 `chemcore-office.exe`，作为独立 COM local server，而不是桌面应用进程里的一个临时功能。`package.json` 增加：

```text
npm run office:register-dev
npm run office:unregister-dev
npm run office:print-registration
npm run office:self-test
```

注册信息采用固定对象身份：

```text
Display name:       Chemcore Document
ProgID:             Chemcore.Document
Versioned ProgID:   Chemcore.Document.1
CLSID:              {CB69F54F-F21E-44DE-84FB-89D98FECE056}
Local server:       chemcore-office.exe
```

开发期注册写入 `HKCU\Software\Classes`，通常不需要管理员权限。machine scope 命令也已预留，但应交给正式 installer 或管理员 PowerShell。

随后在骨架上补了最小 OLE object interfaces。`windows_office.rs` 新增 `IClassFactory`、COM 引用计数、`ChemcoreOleObject`、interface part 指针、`IOleObject`、`IDataObject`、`IPersistStorage`、`IViewObject2` 和 `IRunnableObject` vtable。`office:self-test` 会无 Office 环境下验证 class factory、接口查询、CLSID、IDataObject format 和 storage 基础路径。

这一步最大的风险是 Rust 直接实现 COM vtable 的复杂度。代码里为每个接口维护一个 `InterfacePart<T>`，它保存 vtable 指针和 owner 指针；`QueryInterface` 根据 IID 返回对应 interface part，并统一走 `chemcore_object_add_ref()` / `chemcore_object_release()` 管理对象生命周期。这个结构比简单写几个 extern 函数麻烦，但它让同一个 `ChemcoreOleObject` 可以同时表现为 `IDataObject`、`IOleObject`、`IPersistStorage`、`IViewObject2` 和 `IRunnableObject`，这是 Office OLE embed 必须具备的基础形态。

今天没有选择 Office Add-in 作为主线。Add-in 可以以后做 Ribbon、模板库、批量插入等增强，但不能替代 OLE object，因为 Word/PPT 里的双击编辑、嵌入对象存储和静态 presentation 都是 OLE compound document 机制的一部分。Ketcher 这类 Web sketcher 可以参考化学编辑和格式处理，但不能直接给我们 ChemDraw/ChemSketch 式 Windows Office 双击体验。

### OLE Storage、Clipboard 和双击激活

Chemcore OLE object 今天开始持久化 compound storage payload。`IPersistStorage::Save` 调 `OleSave` 和 `WriteClassStg`，写入：

```text
ChemcoreManifest
ChemcoreDocument
ChemcorePreviewSvg
\x02OlePres001
\x03EPRINT
```

其中 `ChemcoreDocument` 保存 Chemcore document JSON，manifest 记录 ProgID、CLSID、payload stream、preview stream 和 presentation stream 名称。后续 Office 文档中的对象恢复和编辑回写都应从这些 stream 继续补齐。

桌面端复制和 OLE clipboard 也被接起来。`apps/chemcore-desktop/src-tauri/src/lib.rs` 中的 native clipboard write 除了写 Chemcore fragment、document JSON、CDXML、SVG 和 Unicode text，还会调用同目录的 `chemcore-office.exe --copy-clipboard-payload <payload.json>`。OLE clipboard object 支持 `Embedded Object`、`Embed Source`、`Object Descriptor`、Chemcore JSON、CDXML、SVG、Unicode text 和 `CF_ENHMETAFILE` 等格式。

双击激活路径也在今天打通。`IOleObject::DoVerb` 会把对象 payload 写入临时文件，再启动 `chemcore-desktop.exe` 打开。这样 Word 中双击 Chemcore object 时，即使桌面端当前没有运行，也能由 OLE local server 拉起桌面端。

clipboard format 枚举也做过调整，目标是让 Office 粘贴优先使用 Chemcore OLE object，而不是拿普通 SVG/WMF fallback 当作纯图片。随后继续修 Word OLE paste 和 activation，同时新增 `crates/chemcore-engine/examples/cdxml_to_clipboard_payload.rs`，用于把 `tmp/*.cdxml` 生成可直接喂给 OLE clipboard 的 JSON payload，方便复现实测。

这里的 format 顺序很敏感。Office 会根据 `IDataObject::EnumFormatEtc`、`QueryGetData`、`GetData` 和可用 `STGMEDIUM` 决定粘贴结果。如果它优先接受普通 SVG/WMF，用户得到的就是一张不可编辑图片；如果它接受 `Embedded Object`/`Embed Source`，用户得到的是可以双击的 Chemcore 对象。今天多次调整的核心就是让“可编辑对象”成为 Office 的首选，同时保留 SVG/CDXML/text 作为跨软件 fallback。

### Word 预览、对象尺寸和外部显示图

今天后半夜大部分工作都在追 Word 中对象显示质量。第一步是增加 OLE metafile preview，向 Office 暴露 `CF_ENHMETAFILE`；随后修 Word OLE preview rendering，补足 `IViewObject2::Draw`、extent、object descriptor 和 metafile medium 路径，让 Word 不再只看到空白对象。

随后修对象尺寸。之前 Word 中对象框远大于分子，是因为 extent 使用页面/画布尺寸，而不是内容 bounds。现在 `visible_payload_bounds()` 通过 `parse_document_json()`、`render_document()` 和 `render_primitives_bounds()` 计算可见 primitive 的最小 bounds，再换算到 HIMETRIC extent。小于 Word 默认 A4 内容宽度时按真实 cm 尺寸显示，超过时再缩放。

之后修剪贴板 payload 只保留选择片段导致内容丢失的问题。Office OLE paste 需要完整 document payload，不能只给被选 fragment 的简化片段，否则 Word 预览会缺少同页其他对象或资源。后续进一步把 clipboard selection document 和 Office object preview 收紧：engine 新增 `clipboard_selection_json()` 和 `document_from_selection()`，能生成只包含选中对象但资源完整、bounds 正确的 Chemcore document。

中间曾尝试用 SVG-rendered bitmap 做 OLE display fallback，借助 `resvg` 把 SVG 渲染为 bitmap再画到 DC。这个路径能解决一部分空白问题，但 Word 对 OLE preview 的长期结构仍偏向 metafile 和 storage presentation。因此今天又切回 vector preview：直接把 engine render primitives 映射到 GDI line/polygon/text，而不是让 Office 从普通 SVG fallback 猜。

OLE storage 里的 presentation streams 也被补齐：`\x02OlePres001` 写入 OLE presentation stream，`\x03EPRINT` 写入 enhanced print EMF bits。`office:self-test` 会读回这些 stream，并检查 EPRINT 是否确实包含 EMF payload。

今天还开始做直写 Word OOXML 结构：`chemcore-office.exe --write-word-docx-payload <payload.json> <output.docx>` 会生成含 `word/embeddings/oleObject1.bin` 和 `word/media/image1.emf` 的 docx package。这是对 ChemDraw docx 结构的长期追赶路径，因为 ChemDraw 的外部预览图是 first-class media，而不是让 Word 粘贴时临时反推。

后续继续提升 WMF/EMF 预览保真。`windows_office.rs` 现在有 `PreviewTransform`、`draw_payload_vector_preview()`、`draw_preview_primitive()`、`draw_preview_line()`、`draw_preview_polygon()`、`draw_preview_text()`、`preview_text_lines()`、`create_preview_font()`、上下标 baseline/scale 和 ANSI text fallback。最后一版把文本、粗线、多边形中心线、颜色、preview canvas 最大尺寸和 transform 都做了收紧，避免 Word 中出现“框很大、图很小、文本缺失或线宽夸张”的明显问题。

需要注意的是：今天的最终 Word 预览质量仍未达到 ChemDraw 水平。当前 native vector renderer 只覆盖了 engine render primitives 的基础集合，复杂 path、字体度量、高级填充、透明/clip、部分文本布局仍需要继续补。今天重要的是把可编辑 OLE object、storage payload、presentation stream 和 OOXML EMF package writer 的长期骨架打通。

这里也明确了为什么不能只靠 SVG。SVG 是很好的 Web 和现代文档 fallback，但 Windows Office 的 OLE 对象预览、打印缓存和老式兼容路径仍然大量围绕 metafile/presentation stream 工作。ChemDraw/ChemSketch 这类软件之所以在 Word/PPT 里双击编辑且未安装时仍有静态图，靠的是 OLE object + presentation，而不是一个浏览器式 SVG embed。因此 Chemcore 的目标不是放弃 SVG，而是同时提供 native object、CDXML/SVG fallback、EMF/WMF presentation 和 OOXML media。

今天 Word 手工测试暴露的问题也都围绕这层：一开始粘贴进去是空白，随后能显示但对象框远大于内容，再后来出现只粘了部分对象、图很小、线宽过粗、文本缺失、单击对象会闪一下或选中逻辑不对。这些都不是单一 bug，而是 OLE 的多个面同时不完整：payload 裁剪、extent、presentation stream、IDataObject format 优先级、IViewObject2 绘制、Word 自己生成预览的缓存路径、viewer cache bust 和 selection 行为。今天的代码把这些面逐个接上，但后续还要继续用真实 Word/PPT 文档做黑盒验证。

### 文档更新

今天同步更新了几份规则文档：

- `docs/project-rules.zh-CN.md`：把 Office/OLE 和 hybrid desktop runtime 写入项目规则。
- `docs/windows-desktop-office-architecture.zh-CN.md`：明确 `DesktopHybridEngineHost` 是桌面默认编辑运行时，`TauriEngineHost` 只作为诊断/未来实验路径；记录 Chemcore OLE ProgID、CLSID、注册命令、storage stream 和 Word OOXML EMF package writer。
- `docs/architecture.zh-CN.md`、`docs/rust-engine-architecture.zh-CN.md`：同步补充 desktop hybrid runtime 的长期解释。
- `docs/right-click-context-menu-matrix.zh-CN.md`：记录右键菜单矩阵和菜单行为。
- `docs/bond-rendering-rules.zh-CN.md`、`docs/format-v0.1.md`、`docs/format-v0.1.zh-CN.md`：记录 margin width 和两键交汇 whiteout。

### 测试和验证

今天的提交中持续补了 Rust tests，重点覆盖：

- group/ungroup/order 的 scene object 行为。
- margin width 默认值、ACS 值、CDXML import/export、render knockout。
- Object Settings 动态字段、选中对象应用、mixed selection。
- endpoint label 参与 molecule selection。
- click selection、component selection、selection bounds。
- OLE self-test、storage streams、Word OOXML package writer。

Office 路径还做过实际 Word 手工验证：从 `tmp/氰化.cdxml` 和后续反应条件样例导入、全选、复制、粘贴到 Word，反复检查对象能否双击打开、是否粘贴为空白、是否缺对象、对象框是否远大于内容、文字是否丢失、线宽是否失真。最后的代码已经支持粘贴为 Chemcore OLE object，并能双击拉起桌面端；显示质量仍需要继续追 ChemDraw。

本日志写入前工作树为干净状态，HEAD 为 `9496e9f`。写入日志后新增：

```text
docs/developer-log-2026-05-07.zh-CN.md
docs/developer-log-2026-05-07.en.md
```

### 后续注意事项

Office/OLE 是今天最大的新增面，也是风险最高的面。后续不能满足于“Word 能粘贴一个对象”，而要继续沿着 ChemDraw 式结构追：compound storage payload、外部 EMF/OOXML preview、双击激活、编辑后回写、PowerPoint/Excel 验证、无 Chemcore 安装时的静态显示 fallback，以及未来读取 ChemDraw OLE/CDX payload 的兼容路径。

预览渲染不能长期堆在 `windows_office.rs` 一个大文件里。今天为了快速验证 OLE 结构把 GDI renderer 写在 Office crate 内，后续应抽到可测试的 render/export crate 或 native preview module，让 SVG、EMF、WMF、PDF、Office preview 共享更多 render primitive 映射和测试。

Object Settings 现在已经回到 engine，后续新增属性时应继续遵守动态字段原则：对象有什么属性才显示什么，多选取并集，mixed 显空白，应用时只改拥有该字段的对象。不要再把 ChemDraw 的静态 dialog 逻辑硬搬进前端。

桌面性能方向已经明确：默认不把热编辑路径切到 Tauri IPC。要改 native path，必须先证明大文件下 hover/focus/drag 延迟不输 hybrid path，并且不能靠完整 JSON snapshot 刷新整个世界。
