# 剧情资源反查一期设计

> 状态：待评审  
> 日期：2026-08-17  
> 本文档取代 `arkwaifu-feature-design.md` 中的一期剧情范围。

## 1. 目标

一期只实现一个业务能力：查询某个资源被哪些剧情 txt 使用。当资源是角色时，同时返回该角色在每个 txt 中出现过的显示名。

```http
GET /api/v1/story-resource-usages?type={type}&id={id}
```

剧情 txt path 与 PRTS 页面名、页面显示名的关系已由 PRTS MediaWiki 中的 Cargo 数据维护。前端自行请求 MediaWiki API 完成 path 到页面的关联，Rust 后端不再生成、监听或导入 `story_index.json`。

## 2. 一期不实现

- `story_index.json`；
- `story_script_references`；
- `/api/v1/story-references`；
- PRTS 页面名、显示名在本地数据库的副本；
- 剧情全文搜索和 `story_lines`；
- 剧情正文业务 API；
- 剧情分组树；
- AVG `character.json` 入库或角色详情 API；
- gallery、缩略图、超分和图片二次存储；
- 历史版本剧情查询。

前端如需加载完整剧情，直接请求：

```text
/gamedata/latest/story/<script_path>.txt
```

## 3. GameData 完成 marker

### 3.1 路径

torappu 在每个版本的 GameData 和 story txt 全部就绪后，最后写入：

```text
gamedata/<res_version>/.gamedata-ready.json
```

`gamedata/latest` 必须在 marker 发布后指向该完整版本。后端监听的逻辑路径是：

```text
gamedata/latest/.gamedata-ready.json
```

### 3.2 内容

```json
{
  "schemaVersion": 1,
  "task": "GameData",
  "clientVersion": "2.6.01",
  "resVersion": "26-07-08-12-06-40_24a544",
  "completedAt": "2026-08-17T10:00:00Z",
  "producer": {
    "name": "torappu",
    "revision": "git-commit-or-image-tag"
  }
}
```

### 3.3 发布顺序

```text
写完 GameData
→ 写完 story/**/*.txt
→ 写入版本目录的 .gamedata-ready.json
→ 将 gamedata/latest 切换到该版本
```

marker 是完成信号，普通 GameData JSON 或某个 story txt 的变化不单独触发导入。

torappu 如果能保证 marker 原子替换，watcher 可以使用较短 debounce。一期为容忍直接写入，沿用现有文件 watcher 的 30 秒 debounce。

## 4. 数据库设计

### 4.1 最后成功更新时间

```sql
CREATE TABLE story_dataset (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    updated_at TIMESTAMPTZ NOT NULL
);
```

该表永远只有一行。一期不保存 `version_id`、marker hash 或其他持久化 fingerprint。

### 4.2 剧情资源使用

```sql
CREATE TABLE story_resource_usages (
    script_path TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    display_names TEXT[] NOT NULL DEFAULT '{}',
    sort_order INTEGER NOT NULL,

    PRIMARY KEY (script_path, resource_type, resource_id)
);

CREATE INDEX story_resource_usages_resource_idx
    ON story_resource_usages(resource_type, resource_id);

CREATE INDEX story_resource_usages_script_idx
    ON story_resource_usages(script_path, sort_order);
```

`resource_type` 一期支持：

- `background`；
- `image`；
- `item`；
- `character`。

`display_names` 只对 `character` 有业务意义，其他类型使用空数组。同一角色在一个 txt 中以多个名称出现时，去重后按首次出现顺序保存。

`sort_order` 是资源在该 txt 中首次出现的顺序。

### 4.3 一致性

导入事务必须：

1. 删除旧 `story_resource_usages`；
2. 插入新数据；
3. upsert `story_dataset.updated_at = now()`；
4. 提交。

解析或事务失败时，API 继续读取上一份完整快照。

## 5. 文件监听

### 5.1 扫描方式

`GameDataReadyWatcher` 复用现有 `ItemDemandWatcher` 的单文件轮询模式：

- 逻辑路径是 `gamedata/latest/.gamedata-ready.json`；
- 每 10 秒读取 fingerprint；
- fingerprint 至少包含 marker 的 `mtime + size`；
- 首次扫描已存在 marker 时也发出信号；
- 变化后 debounce 30 秒；
- debounce 期间的新变化覆盖旧 pending；
- 自动导入失败后记录错误，不自动重试。

### 5.2 `latest` 符号链接

`latest` 可能在新版本完成时被替换。watcher 必须保存未解析的逻辑路径，并在每次扫描时重新解析。

不能在 watcher 启动时执行：

```rust
std::fs::canonicalize("gamedata/latest/.gamedata-ready.json")
```

否则 watcher 会永久监听启动时的版本目录。

为了可靠识别 `latest` 切换，fingerprint 建议包含每次扫描时解析出的 marker 真实路径：

```text
resolved_path + mtime + size
```

### 5.3 失败处理

一期不保存持久化 fingerprint。worker 重启后，首次扫描会重新导入当前 `latest` 版本。

自动导入失败后记录完整错误，但不保留 pending 或自动退避重试。管理员修正数据或 parser 后，通过 seed/import 命令调用同一 import service 人工重跑。

## 6. 导入与解析

### 6.1 marker 校验

导入前必须校验：

- marker JSON 可完整解析；
- `schemaVersion` 受支持；
- `task == "GameData"`；
- `resVersion` 非空；
- marker 真实路径的版本目录名与 `resVersion` 一致；
- `gamedata/latest/story/` 存在且可读。

任一校验失败时不开启数据库替换事务。

### 6.2 脚本发现

导入器递归扫描：

```text
gamedata/latest/story/**/*.txt
```

`script_path` 是相对 `story/` 的路径，不带 `.txt`，统一使用 `/`。

一期默认不解析 `[uc]info/**`，因为它们是剧情简介而不是播放脚本。

### 6.3 逐文件解析

不将全部 txt 同时读入内存，而是逐文件处理：

```text
读取一个 txt
  → 解析 command
  → 提取背景、插图、物品和角色
  → 在当前脚本内聚合和去重
  → 产生 story_resource_usages rows
  → 释放当前 txt 正文
```

当前脚本内使用：

```text
HashMap<(resource_type, resource_id), ResourceUsage>
```

`ResourceUsage` 记录资源首次出现的顺序。角色额外使用保留插入顺序的集合收集 display name。因此峰值内存与最大单个 txt 和其资源数量相关，不与全部剧情总大小相关。

### 6.4 角色归一化

角色资源 ID 使用 StoryPlayer 可识别的规范形式：

```text
{base}#{face}${body}
```

解析器从角色显示指令中同时提取：

- 规范化后的 `resource_id`；
- 当次显示名。

空名、纯空白名不写入 `display_names`。同名去重，但保留首次出现顺序。

角色 ID 解析、face/body 组合、别名和空名行为必须使用 fixture 测试。

### 6.5 人工导入

将剧情资源导入暴露为 seed/import 命令，调用与 watcher 完全相同的 import service。

用于：

- marker 不完整或内容错误；
- 脚本语法解析失败；
- 数据库临时失败；
- 修复 parser 后重建。

## 7. API 设计

```http
GET /api/v1/story-resource-usages?type=character&id=amiya%231%241
```

查询参数：

- `type`：必填，`background | image | item | character`；
- `id`：必填，规范化资源 ID；
- `limit`：默认 50，设置最大值；
- `cursor`：游标分页。

资源 ID 可能包含 `/`、`#` 或 `$`，因此使用 query parameter。`id` 必须正确百分号编码，服务端拒绝包含 NUL 的值。

响应示例：

```json
{
  "resource": {
    "type": "character",
    "id": "amiya#1$1"
  },
  "items": [
    {
      "scriptPath": "activities/a001/level_a001_01_beg",
      "displayNames": ["阿米娅", "罗德岛的领袖"]
    }
  ],
  "nextCursor": null
}
```

PRTS 页面名和显示名不在该 API 中返回。前端使用 `scriptPath` 请求 MediaWiki Cargo API 完成关联。

## 8. 测试要求

至少覆盖：

- marker schema、task 和 `resVersion` 校验；
- watcher 首次扫描、debounce 和 pending 覆盖；
- `latest` 符号链接切换后能发现新 marker；
- watcher 导入失败后不自动重试；
- story txt 递归发现和 `[uc]info/**` 排除；
- background、image、item 指令提取；
- 角色 ID 归一化；
- 角色 display name 去重与顺序；
- 导入事务失败后保留旧快照；
- 人工 seed/import 调用与 watcher 相同的 service；
- API 资源类型、ID 解码、分页和空结果。

## 9. 实施顺序

1. torappu 在版本 GameData 完成后生成 `.gamedata-ready.json` 并切换 `gamedata/latest`；
2. 增加一期 migration；
3. 实现最小剧情 command parser 和资源提取器；
4. 实现 story txt 发现和逐文件 snapshot builder；
5. 实现事务替换 import service；
6. 实现人工 seed/import 入口；
7. 复用单文件 watcher 模式实现 `GameDataReadyWatcher`；
8. 实现资源反查 API 及查询索引；
9. 补齐 parser、导入事务、watcher 和 API 测试。

## 10. 待确认事项

1. 一期资源类型是否确定为 `background | image | item | character`？
   - 已按此实现。指令口径沿用 arkwaifu：`background(image=)`、`largebg/gridbg/verticalbg(imagegroup= 按 / 拆分)` → background；`image(image=)` → image；`showitem(image=)` → item。`additem`/`deliveritem`/`cgitem` 是玩法奖励或 CG 图层，不计入 item。
2. 角色 `resource_id` 是否确定使用 `{base}#{face}${body}`？
   - 已实现，face/body 缺省为 1（与 arkwaifu `NormalizeCharacterID` 一致）。
3. `[uc]info/**` 是否确定一期不解析？
   - 已实现为不解析。
4. MediaWiki Cargo API 是否已提供按 `scriptPath` 批量查询的接口？
   - 前端侧事项，与本次后端实现无关，保持待确认。

### 实现补充决定

- `limit` 默认 50，最大 200，越界返回 400；`id` 含 NUL 返回 400。
- 游标分页按 `script_path` 升序，`nextCursor` 即最后一页尾条的 `scriptPath`，下一页原样回传 `cursor` 参数。
- 批量插入使用 `jsonb_to_recordset`（`text[]` 列无法直接通过 UNNEST 逐行批量传入）。
- torappu 侧 marker 已同步修改：`GameDataReady` 补全 `task/clientVersion/resVersion/producer` 并以 camelCase 序列化；写入顺序为先原子替换 marker（tmp + `os.replace`）再切换 `gamedata/latest`。
