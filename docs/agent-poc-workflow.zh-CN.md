# ChemCore Agent POC 工作流

这是面向 agent 集成时推荐展示的概念验证流程：

```text
自然语言需求
  -> agent 选择 ChemCore 命令
  -> chemcore-cli 执行确定性的 JSON 命令
  -> ChemCore 返回 selector、局部截图、输出文件和审计报告
  -> 人类检查可编辑结果
```

这个 POC 应聚焦“反应式图编辑”，而不是一开始就宣称全自动化学研究。
一个合适的演示是：读取公开 CDXML 反应图，查询周边对象，执行一小组
JSON 命令，导出 CDXML/SVG/PNG 或 Office payload，并保留 `results.json`
审计报告。

## 演示步骤

1. 运行 `chemcore-cli version --pretty` 和 `chemcore-cli capabilities --pretty`。
2. 用 `chemcore-cli targets figure1.cdxml --pretty` 发现对象 id。
3. 用 `chemcore-cli context ... --capture-out ...` 检查局部区域。
4. 用 `chemcore-cli detail ...` 获取精确对象 JSON。
5. 用 `chemcore-cli new` 或 `chemcore-cli run` 生成或修改文档。
6. 用 `capture`、`export` 或 `copy` 导出视觉或可编辑结果。
7. 保存命令脚本、输出文档、局部截图和审计报告。

`examples/agent` 目录提供了这些步骤的 one-shot 和 JSONL session 示例。
