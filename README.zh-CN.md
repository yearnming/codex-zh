# Codex CLI 中文化

本项目基于上游 `openai/codex`，目标是在不改变功能逻辑的前提下提供完整中文体验，并保持与上游同步。

## 核心原则
- 最小改动、可回放、可审计
- `main` 紧跟上游，`l10n/zh` 只包含中文化补丁

## 本地化资源
- 资源文件：`locales/zh-CN.json`
- 翻译规范：`docs/zh/翻译规范.md`
- 中文帮助：`docs/zh/CLI-帮助.md`
- 贡献指南：`CONTRIBUTING.zh-CN.md`

## 同步说明
- 上游更新后，`l10n/zh` 通过 rebase 重放中文化补丁。
- 详细流程见项目方案文档。

## 语言切换
默认启用中文。需要英文时，请使用：
- `codex --lang en`
- `codex --locale en`
执行 `codex --help` 会显示中文帮助摘要。
已提供子命令中文摘要：`login`、`logout`、`exec`、`review`、`mcp`、`mcp-server`、`app-server`、`sandbox`、`resume`、`fork`、`completion`、`apply`、`cloud`、`debug`、`features`。
