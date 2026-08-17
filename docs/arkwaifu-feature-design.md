# Arkwaifu 功能迁移设计草案

> 状态：待评审  
> 日期：2026-08-10  
> 范围：在 `ak-asset-storage` 中实现 Arkwaifu 的剧情浏览、剧情资源关联、角色差分查询与剧情全文搜索能力。

## 1. 背景

当前系统已经具备以下基础能力：

- 检测游戏客户端与资源版本；
- 下载并保存 AssetBundle；
- 启动 torappu Docker 容器；
- 监听 torappu 的版本化 gamedata 目录；
- 导入 `resource_manifest_idx.json`；
- 通过 `/assets` 和 `/gamedata` 提供静态文件。

torappu 已经覆盖 Arkwaifu 中成本最高的自动抽取管线，包括：

- GameData 解密和 FlatBuffer 转 JSON；
- story txt 提取；
- AVG 背景、插图和物品图片提取；
- AVG 角色 body/face 提取；
- `FaceRectangle` 等角色组合信息提取；
- 输出 `raw/avg/character.json`。

因此，本项目不再实现 Unity 解包和角色图片处理，而是负责：

1. 监听 torappu 的阶段性输出；
2. 导入 torappu 生成的剧情索引，并为 story txt 建立搜索行和资源引用；
3. 提供剧情入口、剧情播放、资源关联和全文搜索 API；
4. 为 StoryPlayer 提供角色组合元数据。

## 2. 设计原则

### 2.1 只保存最新业务快照

剧情业务表只保存最新一份成功导入的数据，不在每张表中保存 `version_id`。

新数据在一个 PostgreSQL 事务中替换旧数据。事务提交前，API 继续读取旧快照；提交后，所有查询同时切换到新快照。

保留一张只有单行的 `story_dataset` 表，仅记录剧情索引表最后一次成功替换的时间。不记录资源版本或持久化文件指纹。

### 2.2 GameData 和 AVG 分开更新

GameData 是版本化完整快照：

```text
gamedata/<res_version>/
```

AVG 是跨版本累计更新的共享工作区：

```text
raw/avg/
```

因此：

- GameData 完成后，torappu 将累计的 `story_index.json` 写入共享 `raw/` 目录；
- AVG 由前端直接读取静态文件 `raw/avg/character.json`；
- 不给 AVG 或图片资源附加一个并不准确的统一版本号。

### 2.3 PostgreSQL 保存索引，文件系统保存原始资源

- story txt 正文不存入 PostgreSQL，前端直接读取版本化静态文件；
- PostgreSQL 只保存业务入口、可搜索文本行和资源引用；
- 图片继续保存在 torappu volume 中；
- 数据库只保存剧情对图片和角色的逻辑引用；
- 暂不引入图片 bytea、S3 二次复制、缩略图和超分 variant。

### 2.4 脚本与业务入口分离

参考 GameData 快照的实际数据量：

- `story/**/*.txt`：5688 个；
- 其中 `[uc]info/**`：2018 个；
- 排除 `[uc]info` 后的剧情和对话脚本：3670 个；
- `story_review_table.json` 引用剧情：1993 个。

非剧情回顾脚本可能被多个数据源使用，例如：

- `story_table.json` 中的活动剧情和干员密录；
- `handbook_info_table.json` 中的干员密录元数据；
- `roguelike_topic_table.json` 中的集成战略对话；
- `activity_table.json` 和 `story_review_meta_table.json` 中的特殊剧情；
- level JSON 的 wave action 引用的教学剧情；
- `sandbox_perm_table.json` 引用的玩法对话；
- 客户端通过命名约定关联的训练脚本。

因此分成两层：

- 文件系统：保存全部 story txt；
- `story_script_references`：保存可搜索、可点击的业务入口；
- `story_lines`：保存用于全文搜索的可见文本行。

不在后端重建剧情回顾树。前端按 reference 的 `type` 分类，按 `name` 搜索，点击 `link` 跳转。

### 2.5 torappu 负责 GameData 关联，Rust 负责查询模型

GameData 表结构、字段名和玩法间的关联变化频繁。torappu 在解包后生成版本化的 `story_index.json`，统一描述每个 txt 的路径和业务入口。

Rust 后端不再遍历多张 GameData 表推断脚本来源，只负责：

- 校验并导入 `story_index.json`；
- 读取索引指向的 txt；
- 解析剧情文本语法；
- 生成可搜索文本、文本行、图片和角色引用；
- 提供稳定的业务 API。

## 3. 数据来源

### 3.1 剧情索引

主要输入为：

```text
raw/story_index.json
gamedata/<res_version>/story/**/*.txt
```

`story_index.json` 由 torappu 生成，必须覆盖当前 `/gamedata/latest/story/` 下的全部 txt，包括无法识别来源的脚本。

### 3.2 `story_index.json` 结构

建议结构：

```json
{
  "schemaVersion": 1,
  "resVersion": "26-07-08-12-06-40_24a544",
  "scripts": [
    {
      "path": "obt/memory/story_noirc_1_1",
      "references": [
        {
          "type": "operator-memory",
          "link": "operator/char_500_noirc/memory/story_noirc_1",
          "name": "黑角干员密录·在破船上"
        }
      ]
    }
  ]
}
```

约定：

- `path` 是相对 `story/` 的路径，不带 `.txt`，统一使用 `/`；
- 一个 txt 可能有多个业务入口，因此使用 `references` 数组；
- 每个 reference 只包含 `type`、`link`、`name` 三个字段：`type` 是 `main-story`、`side-story`、`operator-memory` 等产品类型，`link` 是应用内稳定的相对链接，`name` 是给用户展示的名称；
- reference 不暴露 `story_table`、`handbook_info_table` 等物理来源表名，也不携带来源表的任意字段；
- `link` 不包含域名或 API 前缀，由前端按类型解析，避免 torappu 与部署地址耦合；
- `[uc]info/**` 与其他 txt 一样作为独立 `path` 输出，不在其他条目上保存 `infoPath`；
- 无业务入口的脚本仍必须输出，其 `references` 为空数组；
- 脚本不单独保存 `kind` 和 `title`，展示类型与名称分别使用 reference 的 `type` 和 `name`；
- `schemaVersion` 不支持时导入必须失败，不得静默忽略字段。

torappu 生成索引时应从以下来源识别元数据，但这些 GameData 表不再是 Rust 导入器的直接输入：

- `story_review_table.json`：正式剧情导航；
- `story_table.json` 和 `handbook_info_table.json`：干员密录及其干员、标题、解锁元数据；
- `sandbox_perm_table.json`：生息演算对话；
- `roguelike_topic_table.json`：集成战略对话与档案；
- `activity_table.json`、`zone_table.json` 和 `story_review_meta_table.json`：特殊活动、章节记录和隐藏剧情；
- `levels/**/*.json` 及必要的玩法表：关卡、教学和战斗内对话。

### 3.3 AVG 角色静态数据

- `raw/avg/character.json`
- `raw/avg/characters/**/*.png`

角色组合元数据和图片都不入库。前端通过 `/assets/avg/character.json` 和 `/assets/avg/characters/**` 直接读取。

### 3.4 暂不实现的图集

Arkwaifu 的独立 gallery 功能依赖：

- `story_review_meta_table.json`
- `retro_table.json`
- `replicate_table.json`
- `roguelike_topic_table.json`

该功能与剧情树无直接依赖，暂不放入首期范围。确认需要独立图集页面后，再增加 `galleries` 和 `gallery_arts`。

## 4. 表结构

以下为逻辑 DDL。正式 migration 需要遵守项目约定，Rust 查询全部使用 SQLx 宏。

`story_index.json` 是一次性导入契约，不作为 JSONB 文档保存。导入映射为：

- `scripts[].references[]` → `story_script_references`。

数据库不再建立专门的脚本父表。各表使用已规范化的 `script_path` 关联，完整性由导入前对 `story_index.json` 和磁盘文件的全量校验、以及单个 PostgreSQL 替换事务保证。

### 4.1 剧情数据更新时间

```sql
CREATE TABLE story_dataset (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    updated_at TIMESTAMPTZ NOT NULL
);
```

该表永远只有一行。`updated_at` 与 `story_script_references`、`story_lines` 和资源引用表的快照替换在同一 PostgreSQL 事务中更新。导入失败时不更新。

### 4.2 脚本业务引用

```sql
CREATE TABLE story_script_references (
    script_path TEXT NOT NULL,

    reference_type TEXT NOT NULL,
    link TEXT NOT NULL,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL,

    PRIMARY KEY (script_path, reference_type, link)
);

CREATE INDEX story_script_references_type_idx
    ON story_script_references(reference_type);

CREATE INDEX story_script_references_link_idx
    ON story_script_references(link);

CREATE UNIQUE INDEX story_script_references_script_sort_idx
    ON story_script_references(script_path, sort_order);

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX story_script_references_name_trgm_idx
    ON story_script_references
    USING GIN (name gin_trgm_ops);
```

`story_index.json` 中每个 `references[]` 元素导入为一行：

- `type` → `reference_type`；
- `link` → `link`；
- `name` → `name`；
- 数组中的位置 → `sort_order`。

`reference_type` 不增加 CHECK，允许 torappu 后续增加新的业务入口类型。`story_index.json` 只是 torappu 与 Rust 之间的交换格式，索引内的业务数据不作为 JSONB 整块落库。

### 4.3 可搜索文本行

```sql
CREATE TABLE story_lines (
    script_path TEXT NOT NULL,

    line_number INTEGER NOT NULL,
    line_kind TEXT NOT NULL CHECK (
        line_kind IN (
            'dialogue',
            'narration',
            'visible-command-text'
        )
    ),

    speaker TEXT,
    text TEXT NOT NULL,

    PRIMARY KEY (script_path, line_number)
);

CREATE INDEX story_lines_script_idx
    ON story_lines(script_path, line_number);

CREATE INDEX story_lines_speaker_idx
    ON story_lines(speaker)
    WHERE speaker IS NOT NULL;
```

纯控制指令不写入该表。命令后存在用户可见文字时，保存为 `visible-command-text`。

### 4.4 全文搜索索引

```sql
CREATE INDEX story_lines_text_trgm_idx
    ON story_lines
    USING GIN (text gin_trgm_ops);

CREATE INDEX story_lines_speaker_trgm_idx
    ON story_lines
    USING GIN (speaker gin_trgm_ops)
    WHERE speaker IS NOT NULL;

```

首期使用 `ILIKE` 配合 trigram GIN 索引。当前约 63.5 MB 文本，无需引入 Elasticsearch 或 Meilisearch。

PostgreSQL 默认 `tsvector` 缺少合适的中文分词，因此不作为首期方案。

### 4.5 剧情图片引用

```sql
CREATE TABLE story_picture_assets (
    script_path TEXT NOT NULL,

    asset_id TEXT NOT NULL,
    category TEXT NOT NULL CHECK (
        category IN ('image', 'background', 'item')
    ),
    title TEXT NOT NULL DEFAULT '',
    subtitle TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL,

    PRIMARY KEY (script_path, category, asset_id)
);

CREATE INDEX story_picture_assets_asset_idx
    ON story_picture_assets(category, asset_id);
```

首期不增加全局 `arts` 表。图片 URL 根据类别、资源 ID 和 torappu 目录约定生成。

### 4.6 剧情角色引用

```sql
CREATE TABLE story_character_assets (
    script_path TEXT NOT NULL,

    character_id TEXT NOT NULL,
    display_names TEXT[] NOT NULL DEFAULT '{}',
    sort_order INTEGER NOT NULL,

    PRIMARY KEY (script_path, character_id)
);

CREATE INDEX story_character_assets_character_idx
    ON story_character_assets(character_id);
```

角色 ID 规范化为：

```text
{base}#{face}${body}
```

## 5. 剧情解析器

现有参考实现：

```text
../prts-widgets/src/widgets/StoryPlayer/engine/parser.ts
```

后端需要将其最小词法行为移植到 Rust，而不是直接复制 Arkwaifu 的正则解析器。

必须覆盖：

- 反斜杠续行；
- 空行和注释过滤；
- command 名称小写化；
- 参数 key 大小写规则；
- 单双引号和转义；
- 嵌套括号和逗号参数；
- dialogue、narration、command 分类；
- 隐式 `endtip`。

导入器基于解析结果：

- 生成 `story_lines`；
- 提取背景、插图、物品引用；
- 提取角色和显示名称。

TypeScript 和 Rust parser 应共享一组 fixture，保证关键输入输出一致。

## 6. API

统一前缀：

```text
/api/v1
```

### 6.1 当前内容状态

```http
GET /api/v1/content-status
```

响应示例：

```json
{
  "story": {
    "updatedAt": "2026-08-09T10:00:00Z"
  }
}
```

### 6.2 脚本业务入口

```http
GET /api/v1/story-references
GET /api/v1/story-references?type=main-story
GET /api/v1/story-references?type=operator-memory&q=阿米娅
```

参数：

- `type`：可选 reference 类型；
- `q`：可选名称模糊搜索；
- `limit`：默认 50，设置上限；
- `cursor`：游标分页。

响应条目包含 `type`、`link`、`name`、`scriptPath`。前端按 `type` 组织界面，点击时使用 `link`。

### 6.3 story 脚本与解析结果

```http
GET /gamedata/latest/story/{path}.txt
GET /api/v1/story-data/{*path}/lines
GET /api/v1/story-data/{*path}/assets
```

前端始终通过 `/gamedata/latest/` 读取完整 txt，不关心数据库快照对应的具体 `resVersion`。正文不经过业务 API 或 PostgreSQL。

lines 和 assets 是 Rust 导入时生成的解析结果。assets 响应使用：

```json
{
  "pictures": [],
  "characters": []
}
```

路径必须拒绝：

- `..`；
- 绝对路径；
- 编码后的路径穿越。

API 仅允许查询当前 `story_index.json` 中已校验的 `path`。静态文件服务应正确返回 `ETag` 或 `Last-Modified`。

### 6.4 全文搜索

```http
GET /api/v1/story-search?q=阿米娅
GET /api/v1/story-search?q=罗德岛&speaker=凯尔希
GET /api/v1/story-search?q=博士&referenceType=main-story
```

参数：

- `q`：必填搜索文本；
- `speaker`：可选说话人；
- `referenceType`：可选业务入口类型；
- `limit`：默认 20，设置上限；
- `cursor`：游标分页。

响应示例：

```json
{
  "items": [
    {
      "scriptPath": "activities/a001/level_a001_01_beg",
      "lineNumber": 42,
      "lineKind": "dialogue",
      "speaker": "凯尔希",
      "text": "……罗德岛……",
      "references": [
        {
          "type": "side-story",
          "link": "story/1stact_level_a001_01_beg",
          "name": "GT-1 日正当中·行动前"
        }
      ]
    }
  ],
  "nextCursor": null
}
```

### 6.5 AVG 角色

不提供 AVG 角色业务 API。前端一次读取：

```http
GET /assets/avg/character.json
```

角色详情、基础 ID 和 siblings 在前端内存中解析。图片 URL 直接指向 `/assets/avg/characters/**`。

## 7. 文件监听设计

### 7.1 `story_index.json` 监听

路径：

```text
raw/story_index.json
```

torappu 必须按以下顺序写入：

1. 写完全部 GameData 和 story txt；
2. 更新 `/gamedata/latest/`；
3. 最后生成并写入 `raw/story_index.json`。

`story_index.json` 是最后一个写入的产物。torappu 不保证原子替换，因此 watcher 不在刚发现变化时立即导入。

watcher 复用 `ItemDemandWatcher` 的模式：每 10 秒扫描，使用 `mtime + size` fingerprint，变化后 debounce 30 秒再导入。新变化会覆盖尚未到期的 pending import。

首次扫描会把已存在的文件视为新文件，并在 debounce 后导入。`raw/story_index.json` 不是符号链接，因此可以采用 `ItemDemandWatcher` 现有的 canonicalize-or-preserve-missing 路径处理。

### 7.2 `character.json` 静态发布

路径：

```text
raw/avg/character.json
```

torappu 必须将当前的直接 `write_text()` 改成：

1. 写 `character.json.tmp`；
2. 关闭临时文件；
3. `os.replace(tmp, character.json)`。

Rust 后端只作为静态文件服务器，不监听、解析或导入该文件。响应应允许浏览器通过 `ETag` 或 `Last-Modified` 重新验证，不应为该可变 URL 设置长时间 immutable 缓存。

## 8. GameData 更新时序

```text
VersionCheckWorker
    │
    ├── 发现新版本
    └── 启动 torappu
            │
            ├── 解密并写入 GameData
            ├── 写入 story txt
            ├── 更新 gamedata/latest
            └── 最后写入 raw/story_index.json
                         │
                         ▼
              StoryIndexWatcher
                         │
             每 10 秒扫描 raw/story_index.json
                         │
检查 mtime + size fingerprint
                         │
变化后 debounce 30 秒
                         │
                  获取 advisory lock
                                      │
                                      ▼
                            在数据库事务外构建快照
                            ├── 校验并解析 story_index.json
                            ├── 按索引读取全部 story txt
                            ├── 解析剧情文本
                            ├── 生成 story_lines
                            ├── 提取图片引用
                            └── 提取角色引用
                                      │
                                      ▼
                                  完整性校验
                                      │
                                      ▼
                              PostgreSQL 事务
                            ├── 删除旧业务数据
                            ├── 插入新业务数据
                            └── 更新 story_dataset.updated_at
                                      │
                                      ▼
                                    COMMIT
```

完整性校验至少包括：

- `story_index.json.schemaVersion` 受支持；
- 索引覆盖磁盘上全部 story txt，没有重复 `path`；
- 每个 reference 恰好包含非空的 `type`、`link`、`name`，且 `link` 不是绝对 URL；
- 所有 `path` 指向的 txt 文件存在；
- 同一脚本下没有重复的 `(type, link)` reference；
- script path 无路径穿越；
- `story_script_references`、`story_lines` 和资源引用中的每个 `script_path` 都属于当前已校验索引。

大量文件读取和解析必须在数据库事务外完成，避免长事务。

## 9. 失败与重试

### 9.1 GameData

不保存持久化 fingerprint。worker 每次启动时，首次扫描会重新导入已存在的 `raw/story_index.json`。

自动导入失败后记录完整错误，但不保留 pending 或自动退避重试。管理员修正文件后通过 seed/import 命令调用同一导入 service 人工重跑。实现时需要为现有 `seed` 命令增加剧情索引导入入口。

## 10. 暂不实现

首期明确不实现：

- Arkwaifu gallery 页面；
- 全局 `arts` 资源目录；
- 图片上传和 UUID 鉴权；
- 图片 bytea 存储；
- Real-ESRGAN 超分；
- thumbnail variant；
- AVG 资源版本化；
- Elasticsearch 或 Meilisearch；
- 多服务器和多语言数据源。

## 11. 推荐实施顺序

1. torappu 在 GameData、story txt 和 `/gamedata/latest/` 完成后，最后写入 `raw/story_index.json`；
2. torappu 原子替换 `character.json`；
3. 为 `story_index.json` 建立跨项目 fixture 和 schema 兼容测试；
4. 增加剧情相关 migration；
5. 实现 Rust story parser 和共享 fixture；
6. 实现 `StoryIndexWatcher` 和基于索引的 GameData 快照导入；
7. 实现 reference 列表、脚本和资源关联 API；
8. 实现 `story_lines` 与 PostgreSQL 搜索；
9. 根据产品需求决定是否增加 gallery 和全局 arts 目录。

## 12. 待评审问题

请重点确认以下事项：

1. 产品是否确认只保存最新业务快照，不提供历史版本剧情查询？
2. 是否需要把全部 story txt 都纳入搜索，还是只搜索至少有一个 reference 的脚本？
3. `[uc]info/**` 是否应进入全文搜索结果？默认建议不进入。
4. `story_lines` 是否需要保存 command 原文，以支持后续精确跳转或调试？
5. `story_index.json` 的生产者与 Rust 消费者是否需要独立 JSON Schema 作为版本化契约？
6. 是否需要首期实现独立 gallery 功能？
7. 图片 URL 是否可以长期依赖当前 `raw/avg` 目录约定？
8. 全文搜索是否需要支持繁简转换、拼音、错别字或相关性排序？若不需要，`pg_trgm` 足够。
