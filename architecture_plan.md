```
ak-asset-storage/
├── Cargo.toml                          # workspace 配置
├── crates/
│   ├── domain/                         # 🎯 领域层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── entities/               # 领域实体
│   │       │   ├── mod.rs
│   │       │   ├── version.rs
│   │       │   ├── file.rs
│   │       │   └── bundle.rs
│   │       ├── value_objects/          # 值对象
│   │       │   ├── mod.rs
│   │       │   ├── file_hash.rs
│   │       │   ├── version_id.rs
│   │       │   └── file_path.rs
│   │       └── events/                 # 领域事件
│   │           ├── mod.rs
│   │           └── version_events.rs
│   │
│   ├── application/                    # 🚀 应用层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── usecases/               # 用例（包含原 Worker 逻辑）
│   │       │   ├── mod.rs
│   │       │   ├── version_usecases.rs
│   │       │   ├── file_usecases.rs
│   │       │   ├── bundle_usecases.rs
│   │       │   ├── asset_check_usecase.rs      # 包含周期性检查
│   │       │   └── asset_download_usecase.rs   # 事件驱动下载
│   │       ├── services/               # 应用服务
│   │       │   ├── mod.rs
│   │       │   ├── asset_service.rs
│   │       │   └── notification_service.rs
│   │       ├── ports/                  # 端口接口
│   │       │   ├── mod.rs
│   │       │   ├── repositories.rs
│   │       │   ├── external_services.rs
│   │       │   ├── notifications.rs
│   │       │   └── scheduler.rs        # 调度器接口
│   │       └── dto/                    # 数据传输对象
│   │           ├── mod.rs
│   │           ├── version_dto.rs
│   │           ├── file_dto.rs
│   │           └── bundle_dto.rs
│   │
│   ├── infrastructure/                 # 🔧 基础设施层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── persistence/            # 持久化实现
│   │       │   ├── mod.rs
│   │       │   ├── postgres/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── connection.rs
│   │       │   │   ├── version_repo_impl.rs
│   │       │   │   ├── file_repo_impl.rs
│   │       │   │   ├── bundle_repo_impl.rs
│   │       │   │   └── migrations.rs
│   │       │   └── models/
│   │       │       ├── mod.rs
│   │       │       ├── version_model.rs
│   │       │       ├── file_model.rs
│   │       │       └── bundle_model.rs
│   │       ├── external/               # 外部服务实现
│   │       │   ├── mod.rs
│   │       │   ├── s3/
│   │       │   │   ├── mod.rs
│   │       │   │   └── client.rs
│   │       │   ├── ak_client/
│   │       │   │   ├── mod.rs
│   │       │   │   └── api_client.rs
│   │       │   └── mailer/
│   │       │       ├── mod.rs
│   │       │       └── smtp_client.rs
│   │       ├── scheduling/             # 调度技术实现
│   │       │   ├── mod.rs
│   │       │   └── tokio_scheduler.rs
│   │       └── config/                 # 配置管理
│   │           ├── mod.rs
│   │           └── settings.rs
│   │
│   ├── web/                           # 🌐 Web 适配器层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── handlers/               # HTTP 处理器
│   │       │   ├── mod.rs
│   │       │   ├── version_handlers.rs
│   │       │   ├── file_handlers.rs
│   │       │   ├── bundle_handlers.rs
│   │       │   ├── asset_handlers.rs   # 手动触发检查/下载
│   │       │   └── health_handlers.rs
│   │       ├── middleware/             # 中间件
│   │       │   ├── mod.rs
│   │       │   ├── cors.rs
│   │       │   ├── timeout.rs
│   │       │   └── tracing.rs
│   │       ├── routes/                 # 路由定义
│   │       │   ├── mod.rs
│   │       │   └── api_routes.rs
│   │       ├── dto/                    # Web 响应 DTO
│   │       │   ├── mod.rs
│   │       │   ├── responses.rs
│   │       │   └── requests.rs
│   │       ├── extractors/             # 请求提取器
│   │       │   ├── mod.rs
│   │       │   └── query_params.rs
│   │       ├── openapi/                # OpenAPI 文档
│   │       │   ├── mod.rs
│   │       │   └── schemas.rs
│   │       └── server.rs               # 服务器启动逻辑
│   │
│   └── cli/                           # 🖥️ CLI 适配器层
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── main.rs                 # 主入口
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── server.rs           # 启动 Web 服务器
│           │   ├── worker.rs           # 启动 Worker（只有 check）
│           │   ├── seed.rs             # 数据种子
│           │   ├── migrate.rs          # 数据库迁移
│           │   └── version.rs          # 版本信息
│           └── app.rs                  # CLI 应用配置
```

## 🔗 依赖关系图

```
        ┌─────────┐    ┌─────────┐
        │   cli   │    │   web   │
        └────┬────┘    └────┬────┘
             │              │
             ▼              ▼
        ┌─────────────────────────────┐
        │      application           │
        │  ┌─────────────────────┐    │
        │  │     usecases        │    │
        │  │ ┌─ check_usecase    │    │
        │  │ └─► download_usecase│    │
        │  └─────────────────────┘    │
        └─────────────┬───────────────┘
                      │
                      ▼
        ┌─────────────────────────────┐
        │        domain              │
        │        entities            |
        └─────────────┬───────────────┘
                      ▲
                      │
        ┌─────────────────────────────┐
        │     infrastructure         │
        │ postgres │ s3 │ scheduler   │
        └─────────────────────────────┘
```

## 🏗️ Explicit Architecture 层次结构

```
┌─────────────────────────────────────────────────────────────┐
│                    外部接口层 (Adapters)                      │
│  ┌─────────────────┐              ┌─────────────────────┐    │
│  │   Web/HTTP      │              │   CLI/Workers       │    │
│  │   (axum)        │              │   (clap)            │    │
│  │                 │              │                     │    │
│  │ • REST API      │              │ • server 命令       │    │
│  │ • 手动触发      │              │ • worker 命令       │    │
│  │ • OpenAPI       │              │ • migrate 命令      │    │
│  └─────────────────┘              └─────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                    应用层 (Application)                       │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                    UseCases                             │ │
│  │ ┌─────────────────┐  ┌───────────────────────────────┐  │ │
│  │ │ AssetCheckUC    │  │ AssetDownloadUC               │  │ │
│  │ │ • 周期检查      │──┤ • 事件驱动下载                │  │ │
│  │ │ • 定时器业务    │  │ • 递归检查更多                │  │ │
│  │ │ • 触发下载      │  │ • 失败重试                    │  │ │
│  │ └─────────────────┘  └───────────────────────────────┘  │ │
│  │                                                         │ │
│  │ VersionUC │ FileUC │ BundleUC │ Services │ DTOs         │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                     领域层 (Domain)                          │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Entities        │ Services        │ Repositories(trait) │ │
│  │ • Version       │ • VersionSvc    │ • VersionRepo       │ │
│  │ • File          │ • FileSvc       │ • FileRepo          │ │
│  │ • Bundle        │ • AssetSvc      │ • BundleRepo        │ │
│  │                 │                 │                     │ │
│  │ ValueObjects    │ Events          │                     │ │
│  │ • FileHash      │ • VersionEvent  │                     │ │
│  │ • VersionId     │                 │                     │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                  基础设施层 (Infrastructure)                   │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Persistence     │ External Svc    │ Scheduling         │ │
│  │ • PostgreSQL    │ • S3 Client     │ • TokioScheduler   │ │
│  │ • RepoImpl      │ • AK API Client │                    │ │
│  │ • Models        │ • SMTP Client   │                    │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 🎯 核心设计决策

**1. UseCase 层级关系：**

- `AssetCheckUsecase` → `AssetDownloadUsecase` (依赖关系)
- 只有 Check 需要定时器，Download 是事件驱动

**2. Worker 简化：**

- 移除独立的 Worker 目录
- Worker 逻辑集成到 UseCase 中
- CLI 只负责启动和协调

**3. 依赖方向：**

- 严格遵循内向依赖原则
- Domain 层零外部依赖
- Infrastructure 实现 Application 接口

**4. 触发方式：**

- 定时触发：CLI Worker 启动 Check 定时器
- 手动触发：Web API 直接调用 UseCase
- 事件触发：Check 完成后自动触发 Download
