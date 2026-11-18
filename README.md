# AgentSecret

按设计文档实现的前端、后端、模型网关与 MCP 单二进制项目。

## 架构

- 后端：Actix Web + SeaORM entity，`controller` / `schema` / `entity` / `service` 分层。
- 前端：React + Vite，构建产物由 RustEmbed 嵌入二进制。
- 数据库：MySQL 8.0，第一版仅一个 `server/migrations/0001_init.sql`。
- 模型网关：`/v1/models`、`/v1/responses`，访问令牌鉴权。
- MCP：`/mcp` Streamable HTTP JSON-RPC，访问令牌鉴权。
- 沙盒：Bollard 通过 Docker API 创建一次性受限容器，输出过滤后落库。

## 常用命令

```bash
make lint
make test
make build
make test-integration
```

集成测试需要指向可清空的测试库：

```bash
AGENTSECRET_TEST_DATABASE_URL=mysql://user:password@127.0.0.1:3306/agentsecret TEST_RELEASE_CREDENTIAL=fake-release-token make test-integration
```

本地 MySQL 可使用 `deploy/docker-compose.yml` 中的隔离数据库；Docker 生命周期由使用者控制。
