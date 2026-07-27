# 浏览器内核启动门禁

浏览器页面不得在化学内核不可用时显示可交互编辑器。这里的“后台”是实际
执行文档、化学语义和渲染规则的 ChemSema 内核；Web 使用 WASM，桌面混合
模式同时使用 WASM 布局内核和 Tauri 原生服务。

## 状态

- `loading`：只显示启动页，编辑器和标题栏保留 `hidden`；
- `ready`：WASM 加载成功，真实 `WasmEngine` 会话能够返回合法 document
  JSON 和 render list，并且初始文档标签页加载成功后，才显示编辑器；
- `failed`：保留阻断页，继续隐藏编辑器，显示明确错误与重试按钮。

`hidden` 属性是结构门禁，不依赖 CSS 动画或透明度，因此样式加载失败也不会
短暂暴露假工具栏和空画布。失败分支不创建前端替代文档、不切换到演示 renderer，
也不把初始化异常吞掉。

## 回归

```powershell
npm run regression:runtime-gate
npm run regression:runtime-gate:browser
```

浏览器回归包含两条真实网络路径：

1. 正常加载 WASM，等待 `body[data-runtime-state="ready"]`，确认编辑器可见；
2. 网络层主动阻断 WASM，等待 `body[data-runtime-state="failed"]`，确认编辑器、
   标题栏均不可见且阻断页可见。
