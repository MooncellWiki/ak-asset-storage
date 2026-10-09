# 本地 dev 环境

`tmp/config.toml`、本文件和 `deploy/k3s/dev/` 都已提交。PostgreSQL / RustFS
运行在本机单节点 k3s 的 `ak-dev` namespace,数据保存在 local-path PVC;
`tmp/` 下的资源树等运行数据被 .gitignore 挡在本地。

## 一次性准备

```bash
kubectl apply -k deploy/k3s/dev
kubectl -n ak-dev rollout status deployment/postgres
kubectl -n ak-dev rollout status deployment/rustfs
just init-env          # 资源目录与 S3 桶(幂等,可重复跑)
sqlx migrate run       # .env 的 DATABASE_URL 应使用 localhost:32432
pnpm build             # 前端产物(rust-embed 编译需要)
cargo build
```

需要安装 `kubectl`、`sqlx`、`just`、`plocate` 和 RustFS CLI(`rc`);`just init`
也会安装项目开发用的 Cargo 工具。RustFS API / 控制台分别在
`127.0.0.1:31000` / `127.0.0.1:31001`。k3s 需配置
`kube-proxy-arg: ["nodeport-addresses=127.0.0.1/32"]`,让这些 NodePort 只监听本机。

## 提取 Job 与共享目录

`tmp/config.toml` 已启用 `[torappu.kubernetes]`,在 `ak-dev` 创建
`ak-asset-job-dev`,使用公开镜像 `ghcr.io/mooncellwiki/torappu:main`。
本机运行的 server / worker 使用 `$KUBECONFIG` / `~/.kube/config`,当前凭据
需要有该 namespace 的 Job get/create/delete 权限。无需为公开镜像创建
`image_pull_secret`;若将 server / worker 放进集群,再按 `deploy/k3s/rbac.yaml`
配置同 namespace 的 ServiceAccount。

k3s 自带的 `kubectl` 能隐式读取 `/etc/rancher/k3s/k3s.yaml`,kube-rs 不会。
`just init-env` 在未设置 `KUBECONFIG` 且 `~/.kube/config` 不存在时,会将当前
kubectl 上下文导出到权限为 `0600` 的 `~/.kube/config`;已有配置保持不变。

Torappu 的 storage 根目录同时保存下载缓存和提取结果:

```text
tmp/asset/                  # Torappu storage 根目录,可以是符号链接
├── assetbundle/            # 下载缓存
├── hot_update_list/
└── asset/
    ├── gamedata/           # worker 监听的目录
    └── raw/                # server 提供的资源
```

Job 将同一个 storage 根目录挂到 `/app/storage`,并设置
`STORAGE_DIR=/app/storage`;后端的 `asset_base_path` 为 `tmp/asset/asset`。
本机现有 `tmp/asset` 链接到 `/home/xwbx/Documents/torappu/storage`;
换 checkout 或机器时,先运行 `just init-env`,再将配置里的 `host_path` 改为
`realpath tmp/asset` 输出的绝对路径。hostPath 是 k3s 节点路径,这套配置仅用于
本机单节点开发。不要把 storage 根目录直接当作 `asset_base_path`,否则 worker
会监听错目录。

server 启动后,可通过 `POST /api/v1/docker/launch` 手动触发 Job,请求头
`torappu-auth` 使用配置中的 `torappu.token`。请求需提供 `client_version`、
`res_version`、`prev_client_version` 和 `prev_res_version`,可用 `include`
限制提取任务。接口沿用原路径,实际创建的是 Kubernetes Job。worker 检测到新
版本且数据库已有前一版本时也会自动触发;首次入库不会启动提取。

```bash
kubectl -n ak-dev get jobs,pods
kubectl -n ak-dev logs -f job/ak-asset-job-dev
```

正在运行的同名 Job 会阻止重复启动;终态 Job 在下一次启动时自动替换(连同它的 Pod)。
超过 `active_deadline_seconds`(默认 6 小时)仍未结束的 Job,包括一直卡在拉镜像或
Pending 的,会被集群判为失败,不会永久占住启动槽位。hostPath 以 `Directory` 类型挂载,
路径不存在时 Pod 会停在挂载失败,而不是悄悄建一个空目录。

## 常用命令

```bash
target/debug/ak-asset-storage server -c tmp/config.toml   # :5150,MCP 开启
target/debug/ak-asset-storage --worker-threads 2 worker -c tmp/config.toml --concurrent 5
pnpm dev          # 前端 :25173,代理 /api → 5150
just e2e          # e2e 全套(串行)
```

## 隔离关系

dev 与 e2e 共用同一套 k3s 依赖(PostgreSQL + RustFS),但分别使用
`ak_asset_storage_next` / `ak_asset_storage_e2e` 两个库和
`arknights-assets` / `ak-asset-storage-e2e` 两个桶,互不影响。
e2e 的 Job 启动测试使用进程内假 Kubernetes API,不会实际运行提取镜像;
真实镜像的拉取和共享目录仍需在 dev 环境验证。
