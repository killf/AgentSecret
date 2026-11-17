# AgentSecret 后端设计

后端的三个能力面同进程运行于单二进制 `agentsecret-server`，共用同一套 service 层与数据库访问层：

- **HTTP API 服务**：面向 Web 前端，路由前缀 `/api/v1`，登录会话（JWT）鉴权。
- **模型网关**：与 HTTP API 同进程，挂载在 `/v1` 路径；面向 OpenAI 兼容客户端，访问令牌（`b_access_tokens`）鉴权。
- **MCP 工具**：与 HTTP API 同进程，挂载在 `/mcp` 路径；面向 AI Agent，访问令牌（`b_access_tokens`）鉴权。

鉴权方式：HTTP API 用登录会话；模型网关与 MCP 共用访问令牌鉴权链（详见《模型网关.md》§1 与《MCP工具.md》§3）。

## 模块导航

| 文档 | 内容 |
| --- | --- |
| [后端接口.md](后端接口.md) | 认证与鉴权机制、全部 /api/v1 接口契约（请求/响应/执行流程）、安全基线 |
| [模型网关.md](模型网关.md) | OpenAI 兼容网关：/v1 路由、访问令牌鉴权、路由选商、配额、错误响应 |
| [MCP工具.md](MCP工具.md) | MCP 工具定位、传输与鉴权、工具清单、沙盒执行、权限矩阵、调用限制 |
| [../数据库设计/数据库设计.md](../数据库设计/数据库设计.md) | 数据库表结构与通用约定 |
| [../前端设计/前端设计.md](../前端设计/前端设计.md) | 前端页面结构与交互流程 |
| [../部署设计/部署设计.md](../部署设计/部署设计.md) | 构建、编排、健康检查与生命周期 |
| [../项目结构.md](../项目结构.md) | 单 crate 工程结构与代码目录 |

## 1. 技术选型与工程结构

- 语言与运行时：Rust 1.100+（edition 2024，MSRV 1.100），异步运行时 actix-rt 2（基于 tokio）。
- Web 框架：actix-web 4 + actix-web-httpauth（Bearer 提取）+ actix-service（自定义中间件）。
- 数据库：MySQL 8.0，访问层用 sqlx 0.8（`mysql` 驱动，编译期校验 SQL，迁移用 `sqlx migrate`，脚本放 `migrations/`）。
- 序列化：serde + serde_json。
- 认证：jsonwebtoken；加密：aes-gcm + rand；口令哈希：sha1（存储加盐，详见《后端接口.md》）。
- 上游调用：reqwest（rustls，不依赖系统 OpenSSL）。客户端在 `HttpServer` 启动上下文内构造并放入 `web::Data`，全进程复用；actix-rt 基于 tokio，reqwest 可直接使用。
- 错误处理：thiserror 定义领域错误（core 层不依赖 actix）；HTTP 映射在 server 层用 `impl ResponseError for AppError` 实现。
- 日志与链路：tracing + tracing-subscriber，结构化 JSON 输出；配合 actix-web 的 `tracing-actix-web` 中间件记录请求。

工程结构为单 crate（`agentsecret-server`），详见《项目结构.md》。

## 2. 数据库迁移与启动流程

### 2.1 总体策略

- 迁移在 server 启动时自动执行，脚本由 `sqlx::migrate!` 在编译期内嵌，运行时不依赖外部迁移目录。
- 当前只支持单实例部署，不做副本间的迁移协调：执行迁移的就是当前启动的实例。
- 保留 sqlx 内置的迁移锁（MySQL 会话级 advisory lock），作为防止并发执行的保险，不额外引入协调机制。
- 迁移逻辑放在 store 模块，由 server 启动时调用（MCP 与 HTTP API 同进程，不单独迁移）；锁保证同一时刻只有一个执行者。

### 2.2 启动时序

1. 解析启动参数并加载配置（`core::config::Config::from_env()`，连接串可由 `--database-url` 覆盖），必填项缺失直接退出。
2. 建立独立迁移连接，不复用业务连接池。
3. 执行 `Migrator::run(&migrate_conn)`：取迁移锁 → 建 `_sqlx_migrations` 表 → 检查 dirty 状态 → 校验已应用迁移的 checksum → 执行待应用迁移 → 释放锁。
4. 迁移成功后建立业务连接池，再启动 HTTP 监听与后台任务。
5. 迁移失败或 schema 版本不匹配时记录错误日志并退出，不进入监听状态，不带病提供服务。

### 2.3 迁移锁与并发

- sqlx 0.8 对 MySQL 使用 `SELECT GET_LOCK(?, -1)`，锁 ID 由数据库名生成；`-1` 表示持续等待直到拿到锁。锁是会话级的，连接断开即释放。
- 单实例部署不会产生竞争；该锁用于防止本地开发多进程、手工多实例或将来扩副本时的误并发。
- 等待锁的实例在迁移完成前不得进入 Ready，避免以旧 schema 对外提供服务。

### 2.4 失败与 dirty 状态处置

- MySQL 的 DDL 隐式提交，sqlx 无法用事务真正包裹迁移，采用先写 `success=FALSE` 记账行、执行脚本、再改 `success=TRUE` 的方式。
- 进程在迁移中途被杀（OOM、节点故障、探针误杀）会留下 `success=FALSE` 的行，库进入 dirty 状态；此后所有实例启动都会报 `MigrateError::Dirty(version)` 并退出。
- 处置流程写进 runbook 并演练：
  1. 确认失败迁移的版本号与库中实际结构；
  2. 手工补齐或回退 DDL，使库与该版本脚本的预期一致；
  3. 执行 `sqlx migrate resolve --version <n>` 标记成功，或 `sqlx migrate revert` 撤销后重放。
- 容器侧为迁移期留足健康检查启动窗口（compose healthcheck 的 `start_period`），避免探针在迁移中途把进程判为失败。

### 2.5 迁移脚本约束

- 迁移文件只追加、不修改：sqlx 校验已应用迁移的 checksum，改写旧文件会触发 `VersionMismatch`，实例直接启动失败。
- 一个文件只做一步，粒度尽量小，避免单次大改动。
- MySQL 8 不支持 `ADD COLUMN IF NOT EXISTS`，脚本幂等要靠 `CREATE TABLE IF NOT EXISTS` 一类语法或人工处理，重跑不会自动跳过。
- 面向滚动更新做 expand/contract 拆分：新增列、新增表在当前版本发布，删除列、改名、加非空约束延后到下一版本。即使单副本部署，滚动更新期间新旧容器会短暂并存，旧容器仍按旧结构访问数据库。
- 大表变更避免长时间阻塞业务读写：优先使用 `ALGORITHM=INSTANT`（MySQL 8.0.12+ 的加列），并评估 DDL 期间的元数据锁影响。

### 2.6 权限与连接

- 应用账号需要 DDL 权限（CREATE/ALTER/INDEX 等）才能在启动时执行迁移；建库与建角色由 `deploy/sql/init.sql` 交由 DBA 或运维预先执行，应用账号不负责建库。
- 迁移使用独立连接：DDL 期间的元数据锁会阻塞业务查询，独立连接可避免迁移占用业务连接池，也便于区分迁移失败与业务连接故障。
- 迁移日志按结构化字段输出（版本号、耗时、成功与否），便于在日志系统中检索。
