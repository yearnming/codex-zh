# Codex CLI 帮助（中文）

本帮助文档是 `codex --help` 的中文化摘要版，覆盖常用入口与命令说明。

## 基本用法
```
codex [OPTIONS] [PROMPT]
codex [OPTIONS] <COMMAND> [ARGS]
```

## 常用命令
- `codex exec`：非交互执行（批处理/自动化场景）
- `codex review`：非交互代码审查
- `codex login`：登录（含 ChatGPT 与设备码）
- `codex logout`：退出登录
- `codex mcp`：管理 MCP 外部服务器
- `codex mcp-server`：以 MCP 服务器（stdio）模式运行
- `codex app-server`：运行 App Server 或生成协议产物（实验）
- `codex completion`：生成 shell 补全脚本
- `codex sandbox`：在 Codex 沙箱中执行命令
- `codex apply`：将 Codex 的最新 diff 应用到本地
- `codex resume`：恢复上一次会话
- `codex fork`：从历史会话分叉
- `codex cloud`：浏览 Codex Cloud 任务（实验）
- `codex debug`：调试工具（开发者/内部）
- `codex features`：查看或切换功能开关

说明：部分命令被标记为实验或内部用途，可能随上游变动。

## 语言切换
设置环境变量 `CODEX_LOCALE=zh-CN`，或系统语言包含 `zh`，即可启用中文帮助与错误提示。
