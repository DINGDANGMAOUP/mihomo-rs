# mihomo-rs 渐进式 Examples

本目录采用分段渐进方式组织，从本地可离线执行的基础步骤开始，逐步过渡到需要运行中 mihomo 服务的在线能力。

## 运行方式

```bash
cargo run --example 01_bootstrap
```

## 分段说明

1. `01_bootstrap.rs`
- 初始化 `ConfigManager`、`VersionManager`，验证自定义 home 目录。

2. `02_config_profiles.rs`
- 演示配置保存、切换、列举 profile 的完整流程。

3. `03_version_inventory.rs`
- 演示版本列表与默认版本查询（不触发下载）。

4. `04_service_lifecycle_dry_run.rs`
- 演示 `ServiceManager` 的构造与状态检查（不启动真实进程）。

5. `05_proxy_queries.rs`
- 演示代理节点、代理组、当前选择读取（需要 mihomo API 可用）。

6. `06_connection_queries.rs`
- 演示连接列表、过滤与统计（需要 mihomo API 可用）。

7. `07_streaming.rs`
- 演示日志/流量流式读取入口（需要 WebSocket API 可用）。

8. `08_complete_workflow.rs`
- 把前面步骤串成完整工作流模板。

## 覆盖策略

- 基础段（01-04）用于离线验证 SDK 结构与本地状态管理。
- 在线段（05-08）用于覆盖 HTTP/WebSocket 交互路径。
- 测试层在 `tests/` 中与 examples 分工：测试负责稳定断言，examples 负责用法模板。
