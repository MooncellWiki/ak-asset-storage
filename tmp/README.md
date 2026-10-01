# 本地 dev 环境

`tmp/config.toml`、本文件和顶层的 `docker-compose.yaml`(rustfs bind 到
`tmp/rustfs-data`)都已提交;其余内容(S3 数据、资源树、索引)被 .gitignore
挡在本地。

## 一次性准备

```bash
just init              # 目录、属主、btrfs 子卷、SELinux 标签、S3 桶(幂等,可重复跑)
docker compose up -d   # 基础设施
sqlx migrate run       # 迁移(sqlx 宏的编译期校验也依赖它)
pnpm build             # 前端产物(rust-embed 编译需要)
```

`just init` 里 rustfs 的桶创建需要容器先在跑;顺序反过来它也只会提示你补一句
`mc mb --ignore-existing rustfs/arknights-assets`。

## 常用命令

```bash
target/debug/ak-asset-storage server -c tmp/config.toml   # :5150,MCP 开启
target/debug/ak-asset-storage --worker-threads 2 worker -c tmp/config.toml --concurrent 5
pnpm dev          # 前端 :25173,代理 /api → 5150
just e2e          # e2e 全套(串行)
```

## 隔离关系

dev 与 e2e 共用同一套容器(db + rustfs),但分别使用
`ak_asset_storage_next` / `ak_asset_storage_e2e` 两个库和
`arknights-assets` / `ak-asset-storage-e2e` 两个桶,互不影响。
