# codex app-server（中文）

运行 App Server（实验性），或生成协议相关产物。

## 用法
```
codex app-server [--listen URL] [--analytics-default-enabled]
codex app-server generate-ts -o DIR [--prettier PRETTIER_BIN] [--experimental]
codex app-server generate-json-schema -o DIR [--experimental]
```

## 说明
- `--listen` 支持 `stdio://`（默认）与 `ws://IP:PORT`。
- `generate-ts` 生成 TypeScript 绑定。
- `generate-json-schema` 生成 JSON Schema。
