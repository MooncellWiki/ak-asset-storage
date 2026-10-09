# 本地 dev 环境

开发环境是机器本地、不进 git 的:`tmp/` 整个目录被 .gitignore 忽略,配置与
RustFS 清单由 justfile 生成(`just init-env` / `just k3s-apply`)。PostgreSQL
跑在本机单节点 k3s 的 `ak-dev` namespace;RustFS 的数据卷是 hostPath,直接
用仓库里的 `tmp/rustfs-data`,数据随 checkout 走、重建集群也不丢。

## 一次性准备

```bash
just init-env        # kubeconfig 导出、目录准备(rustfs-data 属主/SELinux)、
                     # 生成 tmp/config.toml、建 S3 桶(幂等,可重复跑)
just k3s-apply       # apply deploy/k3s/dev(PostgreSQL)+ 生成的 tmp/k3s/rustfs.yaml
just up              # sqlx migrate run;.env 的 DATABASE_URL 应使用 localhost:32432
pnpm build           # 前端产物(rust-embed 编译需要)
cargo build
```

需要安装 `kubectl`、`sqlx`、`just`、`plocate` 和 RustFS CLI(`rc`);`just init`
也会安装项目开发用的 Cargo 工具。RustFS API / 控制台分别在
`127.0.0.1:31000` / `127.0.0.1:31001`。k3s 需配置
`kube-proxy-arg: ["nodeport-addresses=127.0.0.1/32"]`,让这些 NodePort 只监听本机。

## 生成物

- `tmp/config.toml`:`just init-env` 在文件不存在时生成。torappu 的 `host_path`
  会用 `realpath tmp/asset` 的结果填好,不需要手动维护绝对路径;生成后随便改
  (整目录已 ignore,真实 token 不会被提交)。挪了 checkout 或改了 `tmp/asset`
  符号链接后,删掉它重跑 `just init-env` 即可。
- `tmp/k3s/rustfs.yaml`:`just k3s-apply` 每次 apply 前由 `just gen-rustfs`
  重新生成,e2e 也会先调它,不要手改。hostPath 指向本 checkout 的
  `tmp/rustfs-data`(`type: Directory`,目录不存在时 Pod 停在挂载失败,先跑
  `just init-env`)。

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
本机 `tmp/asset` 若要指向别处的 Torappu checkout,建好符号链接后删掉
`tmp/config.toml` 重跑 `just init-env`,`host_path` 会按新链接重新解析。
不要把 storage 根目录直接当作 `asset_base_path`,否则 worker 会监听错目录。

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
Pending 的,会被集群判为失败,不会永久占住启动槽位。

## 常用命令

```bash
target/debug/ak-asset-storage server -c tmp/config.toml   # :5150,MCP 开启
target/debug/ak-asset-storage --worker-threads 2 worker -c tmp/config.toml --concurrent 5
pnpm dev          # 前端 :25173,代理 /api → 5150
just e2e          # e2e 全套(串行)
```

## 隔离关系

dev 与 e2e 共用同一套 k3s 依赖(PostgreSQL + RustFS,同一个
`tmp/rustfs-data`),但分别使用 `ak_asset_storage_next` / `ak_asset_storage_e2e`
两个库和 `arknights-assets` / `ak-asset-storage-e2e` 两个桶,互不影响。
e2e 启动时会先调用 `just gen-rustfs` 再 apply,确保 RustFS 清单与当前 checkout
一致;e2e 的 Job 启动测试使用进程内假 Kubernetes API,不会实际运行提取镜像。
