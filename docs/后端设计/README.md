# AgentSecret 后端设计

后端的三个能力面同进程运行：

- `/api/v1`：面向 Web 前端，登录会话鉴权。
- `/v1`：OpenAI 兼容模型网关，访问令牌鉴权。
- `/mcp`：MCP 工具，访问令牌鉴权。

## 模块导航

| 文档 | 内容 |
| --- | --- |
| [后端接口.md](后端接口.md) | 管理接口契约、鉴权与错误语义 |
| [模型网关.md](模型网关.md) | `/v1` 路由、上游代理、配额与敏感信息过滤 |
| [MCP工具.md](MCP工具.md) | MCP 工具、沙盒执行与权限矩阵 |
| [../数据库设计/数据库设计.md](../数据库设计/数据库设计.md) | 数据库表结构 |
| [../项目结构.md](../项目结构.md) | SeaORM entity / schema / controller 分层 |

## 1. 技术选型

- Web：Actix Web 4。
- ORM：SeaORM 2.0，MySQL 驱动使用 `sqlx-mysql` 与 `runtime-tokio-rustls`。
- 实体：`src/entity` 一表一文件，控制器和服务直接使用 `Entity::find()`、`ActiveModel` 与关系查询。
- 契约：HTTP 请求与响应结构放在 `src/schema`，不与实体模型混用。
- 错误：统一业务码；`/api/v1` HTTP 200 + `{code,message,data}`，`/v1` 与 `/mcp` 按协议映射标准状态或错误对象。

## 2. 迁移与启动

1. 解析 `--database-url` 与环境变量。
2. 建立迁移连接。
3. 执行唯一的 `migrations/0001_init.sql`。
4. 迁移成功后建立 SeaORM `DatabaseConnection` 并注入 Actix `Data`。
5. 绑定 HTTP 地址；迁移失败不监听端口。

第一版不拆多个迁移文件；初始化脚本内部仍按「用户认证 → 模型网关 → 项目脚本 → 权限系统」分语义块并留空行。后续新增结构时再追加 `0002_*`，不修改 `0001_init.sql`。

## 3. 代码风格

- 控制器按参考工程风格：路由函数处理请求，数据库查询在 entity 中封装或在控制器中直接使用 SeaORM。
- 复杂流程用中文编号注释，例如登录失败计数、配额刷新、模型路由重试。
- 实体字段使用 `///` 注释说明业务含义，敏感字段 `#[serde(skip)]`。
- 不添加 repository/store 薄层，不为了形式化拆分增加转发代码。
