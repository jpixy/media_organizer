# 实施计划

本文档将架构设计拆解为细粒度的实施任务，按照依赖关系和逻辑顺序排列。

---

## 任务总览

```
Phase 1: 项目初始化
    └── Task 1.1 ~ 1.5

Phase 2: 前置检查模块
    └── Task 2.1 ~ 2.4

Phase 3: 数据模型定义
    └── Task 3.1 ~ 3.6

Phase 4: 外部服务客户端
    └── Task 4.1 ~ 4.6

Phase 5: 核心业务逻辑 - Scanner
    └── Task 5.1 ~ 5.4

Phase 6: 核心业务逻辑 - Parser
    └── Task 6.1 ~ 6.4

Phase 7: 核心业务逻辑 - Planner
    └── Task 7.1 ~ 7.5

Phase 8: 生成器模块
    └── Task 8.1 ~ 8.4

Phase 9: 核心业务逻辑 - Executor
    └── Task 9.1 ~ 9.5

Phase 10: 核心业务逻辑 - Rollback
    └── Task 10.1 ~ 10.4

Phase 11: CLI 命令实现
    └── Task 11.1 ~ 11.7

Phase 12: 集成与测试
    └── Task 12.1 ~ 12.5
```

---

## Phase 1: 项目初始化

### Task 1.1: 创建 Cargo 项目

**描述**：初始化 Rust 项目，配置基本信息

**产出**：
- `Cargo.toml` 包含项目名称、版本、作者
- `src/main.rs` 基础入口
- `src/lib.rs` 库入口

**依赖**：无

---

### Task 1.2: 配置项目依赖

**描述**：在 Cargo.toml 中添加所有必要依赖

**依赖列表**：
| 依赖 | 版本 | 用途 |
|------|------|------|
| clap | 4.x | CLI 参数解析 |
| tokio | 1.x | 异步运行时 |
| reqwest | 0.11.x | HTTP 客户端 |
| serde | 1.x | 序列化框架 |
| serde_json | 1.x | JSON 处理 |
| walkdir | 2.x | 目录遍历 |
| indicatif | 0.17.x | 进度条 |
| colored | 2.x | 彩色输出 |
| sha2 | 0.10.x | 校验和 |
| uuid | 1.x | UUID 生成 |
| chrono | 0.4.x | 时间处理 |
| toml | 0.8.x | 配置解析 |
| thiserror | 1.x | 错误处理 |
| tracing | 0.1.x | 日志 |
| tracing-subscriber | 0.3.x | 日志订阅 |

**依赖**：Task 1.1

---

### Task 1.3: 创建模块目录结构

**描述**：按照架构设计创建所有模块目录和 mod.rs 文件

**目录结构**：
```
src/
├── cli/
│   ├── mod.rs
│   ├── args.rs
│   └── commands/
│       └── mod.rs
├── core/
│   └── mod.rs
├── services/
│   └── mod.rs
├── models/
│   └── mod.rs
├── generators/
│   └── mod.rs
├── utils/
│   └── mod.rs
└── preflight/
    └── mod.rs
```

**依赖**：Task 1.1

---

### Task 1.4: 配置错误处理框架

**描述**：定义项目统一的错误类型

**产出**：
- `src/error.rs` 定义 `Error` 和 `Result` 类型
- 使用 thiserror 派生宏

**依赖**：Task 1.2

---

### Task 1.5: 配置日志框架

**描述**：初始化 tracing 日志系统

**产出**：
- 日志初始化函数
- 支持不同日志级别
- 支持彩色输出

**依赖**：Task 1.2

---

## Phase 2: 前置检查模块

### Task 2.1: 实现 ffprobe 检查

**描述**：检查系统是否安装 ffprobe

**逻辑**：
1. 执行 `ffprobe -version`
2. 解析输出获取版本号
3. 返回检查结果

**产出**：`src/preflight/ffprobe.rs`

**依赖**：Task 1.3, Task 1.4

---

### Task 2.2: 实现 Ollama 检查

**描述**：检查 Ollama 服务是否运行

**逻辑**：
1. HTTP GET `http://localhost:11434/api/tags`
2. 验证返回状态码
3. 解析可用模型列表

**产出**：`src/preflight/ollama.rs`

**依赖**：Task 1.3, Task 1.4

---

### Task 2.3: 实现 TMDB API 检查

**描述**：验证 TMDB API Key 有效性

**逻辑**：
1. 读取 API Key（环境变量/配置文件）
2. 调用 TMDB 测试端点
3. 验证返回结果

**产出**：`src/preflight/tmdb.rs`

**依赖**：Task 1.3, Task 1.4

---

### Task 2.4: 整合前置检查入口

**描述**：提供统一的前置检查入口函数

**逻辑**：
1. 依次执行三项检查
2. 汇总结果
3. 格式化输出（✓/✗）
4. 任一失败则返回错误

**产出**：`src/preflight/mod.rs` 中的 `run_preflight_checks()` 函数

**依赖**：Task 2.1, Task 2.2, Task 2.3

---

## Phase 3: 数据模型定义

### Task 3.1: 定义媒体类型枚举

**描述**：定义电影和电视剧的类型枚举

**产出**：
```
MediaType::Movies
MediaType::TvShows
```

**文件**：`src/models/media.rs`

**依赖**：Task 1.3

---

### Task 3.2: 定义视频文件信息模型

**描述**：定义扫描阶段获取的视频文件信息

**字段**：
- 文件路径
- 文件大小
- 修改时间
- 是否为 Sample
- 所属父目录

**文件**：`src/models/media.rs`

**依赖**：Task 3.1

---

### Task 3.3: 定义视频元数据模型

**描述**：定义 ffprobe 提取的视频技术信息

**字段**：
- 分辨率 (resolution)
- 视频格式 (format)
- 视频编码 (video_codec)
- 位深 (bit_depth)
- 音频编码 (audio_codec)
- 声道 (audio_channels)

**文件**：`src/models/media.rs`

**依赖**：Task 3.1

---

### Task 3.4: 定义 TMDB 元数据模型

**描述**：定义从 TMDB 获取的元数据

**字段（电影）**：
- tmdb_id
- imdb_id
- original_title
- title (本地化)
- year
- overview
- directors
- actors
- poster_urls

**字段（电视剧）**：
- 剧集级别信息
- 季级别信息
- 单集级别信息

**文件**：`src/models/media.rs`

**依赖**：Task 3.1

---

### Task 3.5: 定义 Plan 模型

**描述**：定义 plan.json 的完整结构

**结构**：
```
Plan
├── version
├── created_at
├── media_type
├── source_path
├── target_path
├── items[]
│   ├── id
│   ├── status
│   ├── source (文件信息)
│   ├── parsed (AI 解析结果)
│   ├── tmdb (TMDB 元数据)
│   ├── media_info (视频元数据)
│   ├── target (目标路径)
│   └── operations[]
├── samples[]
└── unknown[]
```

**文件**：`src/models/plan.rs`

**依赖**：Task 3.2, Task 3.3, Task 3.4

---

### Task 3.6: 定义 Rollback 模型

**描述**：定义 rollback.json 的完整结构

**结构**：
```
Rollback
├── version
├── plan_id
├── executed_at
└── operations[]
    ├── seq
    ├── op_type
    ├── from
    ├── to
    ├── checksum
    ├── rollback_op
    └── executed
```

**文件**：`src/models/rollback.rs`

**依赖**：Task 1.3

---

## Phase 4: 外部服务客户端

### Task 4.1: 实现 Ollama 客户端基础结构

**描述**：创建 Ollama API 客户端

**功能**：
- 配置 base URL
- HTTP 客户端初始化
- 错误处理

**文件**：`src/services/ollama.rs`

**依赖**：Task 1.2, Task 1.4

---

### Task 4.2: 实现 Ollama 文件名解析接口

**描述**：实现调用 Ollama 解析视频文件名的功能

**功能**：
- 构造 Prompt 模板
- 发送请求到 `/api/generate`
- 解析 JSON 响应
- 提取标题、年份、置信度

**文件**：`src/services/ollama.rs`

**依赖**：Task 4.1

---

### Task 4.3: 实现 TMDB 客户端基础结构

**描述**：创建 TMDB API 客户端

**功能**：
- 配置 API Key
- 配置 base URL
- HTTP 客户端初始化
- 速率限制处理

**文件**：`src/services/tmdb.rs`

**依赖**：Task 1.2, Task 1.4

---

### Task 4.4: 实现 TMDB 搜索接口

**描述**：实现电影和电视剧搜索功能

**接口**：
- `search_movie(title, year)` → 搜索电影
- `search_tv(title, year)` → 搜索电视剧

**文件**：`src/services/tmdb.rs`

**依赖**：Task 4.3, Task 3.4

---

### Task 4.5: 实现 TMDB 详情接口

**描述**：实现获取详细信息的功能

**接口**：
- `get_movie_details(tmdb_id)` → 电影详情
- `get_tv_details(tmdb_id)` → 剧集详情
- `get_season_details(tmdb_id, season)` → 季详情
- `get_episode_details(tmdb_id, season, episode)` → 单集详情

**文件**：`src/services/tmdb.rs`

**依赖**：Task 4.3, Task 3.4

---

### Task 4.6: 实现 ffprobe 服务

**描述**：封装 ffprobe 调用

**功能**：
- 执行 ffprobe 命令
- 解析 JSON 输出
- 提取视频/音频信息
- 映射到 VideoMetadata 模型

**文件**：`src/services/ffprobe.rs`

**依赖**：Task 3.3

---

## Phase 5: 核心业务逻辑 - Scanner

### Task 5.1: 实现目录递归扫描

**描述**：递归遍历指定目录

**功能**：
- 使用 walkdir 遍历目录
- 收集所有文件路径

**文件**：`src/core/scanner.rs`

**依赖**：Task 1.2

---

### Task 5.2: 实现视频文件过滤

**描述**：根据扩展名过滤视频文件

**支持格式**：
- .mkv, .mp4, .avi, .mov, .wmv
- .m4v, .ts, .m2ts, .flv, .webm

**文件**：`src/core/scanner.rs`

**依赖**：Task 5.1

---

### Task 5.3: 实现 Sample 识别

**描述**：识别 Sample 文件和文件夹

**规则**：
- 文件夹名包含 "sample"（不区分大小写）
- 文件名包含 "sample"（不区分大小写）

**文件**：`src/core/scanner.rs`

**依赖**：Task 5.1

---

### Task 5.4: 整合 Scanner 模块

**描述**：提供统一的扫描入口

**接口**：
- `scan_directory(path)` → `ScanResult`
- `ScanResult` 包含：视频文件列表、Sample 列表、空目录列表

**文件**：`src/core/scanner.rs`

**依赖**：Task 5.1, Task 5.2, Task 5.3, Task 3.2

---

## Phase 6: 核心业务逻辑 - Parser

### Task 6.1: 设计 Prompt 模板

**描述**：设计用于文件名解析的 Prompt

**要求**：
- 支持中英文混合文件名
- 提取标题、年份
- 识别分辨率、编码等信息
- 返回结构化 JSON

**文件**：`src/core/parser.rs`

**依赖**：无

---

### Task 6.2: 实现单文件解析

**描述**：解析单个视频文件名

**流程**：
1. 构造 Prompt
2. 调用 Ollama API
3. 解析响应
4. 返回解析结果

**文件**：`src/core/parser.rs`

**依赖**：Task 6.1, Task 4.2

---

### Task 6.3: 实现批量解析

**描述**：批量解析多个文件名

**功能**：
- 控制并发数
- 进度显示
- 错误收集

**文件**：`src/core/parser.rs`

**依赖**：Task 6.2

---

### Task 6.4: 实现解析结果验证

**描述**：验证 AI 解析结果的合理性

**验证规则**：
- 年份范围检查（1900-当前年份）
- 标题非空检查
- 置信度阈值检查

**文件**：`src/core/parser.rs`

**依赖**：Task 6.2

---

## Phase 7: 核心业务逻辑 - Planner

### Task 7.1: 实现信息聚合

**描述**：聚合所有来源的信息

**输入**：
- 扫描结果
- AI 解析结果
- TMDB 元数据
- ffprobe 元数据

**输出**：统一的 PlanItem 结构

**文件**：`src/core/planner.rs`

**依赖**：Task 3.5

---

### Task 7.2: 实现文件名生成逻辑

**描述**：根据信息生成目标文件名

**调用**：generators/filename 模块

**文件**：`src/core/planner.rs`

**依赖**：Task 7.1

---

### Task 7.3: 实现文件夹名生成逻辑

**描述**：根据信息生成目标文件夹名

**调用**：generators/folder 模块

**文件**：`src/core/planner.rs`

**依赖**：Task 7.1

---

### Task 7.4: 实现操作列表生成

**描述**：为每个文件生成操作列表

**操作类型**：
- mkdir：创建目录
- move：移动文件
- create：创建文件（NFO）
- download：下载文件（海报）

**文件**：`src/core/planner.rs`

**依赖**：Task 7.2, Task 7.3

---

### Task 7.5: 实现 plan.json 输出

**描述**：将计划序列化为 JSON 文件

**功能**：
- 格式化 JSON 输出
- 写入文件
- 同时保存到 sessions 目录

**文件**：`src/core/planner.rs`

**依赖**：Task 7.4, Task 3.5

---

## Phase 8: 生成器模块

### Task 8.1: 实现电影文件夹名生成器

**描述**：生成电影文件夹名

**格式**：`[${originalTitle}]-[${title}](${edition})-${year}-${imdb}-${tmdb}`

**特殊处理**：
- 中文电影去重标题
- 繁简体判断

**文件**：`src/generators/folder.rs`

**依赖**：Task 3.4

---

### Task 8.2: 实现电影文件名生成器

**描述**：生成电影文件名

**格式**：`[${originalTitle}]-[${title}](${edition})-${year}-${resolution}-${format}-${codec}-${bitDepth}bit-${audioCodec}-${audioChannels}`

**文件**：`src/generators/filename.rs`

**依赖**：Task 3.3, Task 3.4

---

### Task 8.3: 实现电视剧文件夹/文件名生成器

**描述**：生成电视剧相关命名

**格式**：
- 剧集文件夹：`[${showOriginalTitle}]-[${showTitle}]-${showImdb}-${showTmdb}`
- 季文件夹：`S${seasonNr2}.${showYear}`
- 文件名：`[${showOriginalTitle}]-S${seasonNr2}E${episodeNr2}-[${originalTitle}]-[${title}]-...`

**文件**：`src/generators/folder.rs`, `src/generators/filename.rs`

**依赖**：Task 3.4

---

### Task 8.4: 实现 NFO 生成器

**描述**：生成 Kodi 兼容的 NFO 文件

**格式**：XML

**内容**：
- 电影：movie.nfo
- 电视剧：tvshow.nfo
- 单集：episode.nfo

**文件**：`src/generators/nfo.rs`

**依赖**：Task 3.4

---

## Phase 9: 核心业务逻辑 - Executor

### Task 9.1: 实现计划验证

**描述**：执行前验证计划有效性

**检查项**：
- 源文件存在性
- 目标路径冲突检测
- 磁盘空间检查

**文件**：`src/core/executor.rs`

**依赖**：Task 3.5

---

### Task 9.2: 实现目录创建操作

**描述**：执行 mkdir 操作

**功能**：
- 递归创建目录
- 记录到 rollback

**文件**：`src/core/executor.rs`

**依赖**：Task 3.6

---

### Task 9.3: 实现文件移动操作

**描述**：执行 move 操作

**功能**：
- 计算源文件 checksum
- 移动文件
- 验证目标文件 checksum
- 记录到 rollback

**文件**：`src/core/executor.rs`

**依赖**：Task 3.6

---

### Task 9.4: 实现文件创建操作

**描述**：执行 create 操作（NFO 文件）

**功能**：
- 生成 NFO 内容
- 写入文件
- 记录到 rollback

**文件**：`src/core/executor.rs`

**依赖**：Task 8.4, Task 3.6

---

### Task 9.5: 实现海报下载操作

**描述**：执行 download 操作

**功能**：
- 从 TMDB 下载海报
- 保存到目标目录
- 记录到 rollback

**文件**：`src/core/executor.rs`

**依赖**：Task 4.3, Task 3.6

---

## Phase 10: 核心业务逻辑 - Rollback

### Task 10.1: 实现 rollback.json 解析

**描述**：读取并解析 rollback 文件

**文件**：`src/core/rollback.rs`

**依赖**：Task 3.6

---

### Task 10.2: 实现冲突检测

**描述**：检测回滚前的冲突

**检查项**：
- 文件是否被修改（checksum 比对）
- 文件是否被删除
- 原路径是否已被占用

**文件**：`src/core/rollback.rs`

**依赖**：Task 10.1

---

### Task 10.3: 实现逆向操作执行

**描述**：执行回滚操作

**逻辑**：
- 逆序遍历操作列表
- delete → move → rmdir

**文件**：`src/core/rollback.rs`

**依赖**：Task 10.1, Task 10.2

---

### Task 10.4: 实现回滚结果报告

**描述**：输出回滚执行结果

**内容**：
- 成功恢复的文件数
- 失败的操作及原因
- 最终状态

**文件**：`src/core/rollback.rs`

**依赖**：Task 10.3

---

## Phase 11: CLI 命令实现

### Task 11.1: 实现 CLI 参数定义

**描述**：使用 clap 定义所有命令和参数

**命令**：
- plan movies/tvshows
- execute
- rollback
- sessions list/show
- verify

**文件**：`src/cli/args.rs`

**依赖**：Task 1.2

---

### Task 11.2: 实现 plan 命令

**描述**：实现 plan 子命令

**流程**：
1. 前置检查
2. 扫描目录
3. 解析文件名
4. 查询 TMDB
5. 提取视频元数据
6. 生成计划
7. 输出 plan.json

**文件**：`src/cli/commands/plan.rs`

**依赖**：Task 2.4, Task 5.4, Task 6.3, Task 4.4, Task 4.6, Task 7.5

---

### Task 11.3: 实现 execute 命令

**描述**：实现 execute 子命令

**流程**：
1. 读取 plan.json
2. 验证计划
3. 执行操作
4. 生成 rollback.json

**文件**：`src/cli/commands/execute.rs`

**依赖**：Task 9.1 ~ 9.5

---

### Task 11.4: 实现 rollback 命令

**描述**：实现 rollback 子命令

**流程**：
1. 读取 rollback.json
2. 冲突检测
3. 执行回滚
4. 输出结果

**文件**：`src/cli/commands/rollback.rs`

**依赖**：Task 10.1 ~ 10.4

---

### Task 11.5: 实现 sessions 命令

**描述**：实现 sessions 子命令

**功能**：
- list：列出所有历史会话
- show：显示会话详情

**文件**：`src/cli/commands/sessions.rs`

**依赖**：Task 1.3

---

### Task 11.6: 实现 verify 命令

**描述**：实现视频完整性校验

**功能**：
- 使用 ffprobe 验证视频可播放性
- 输出校验结果

**文件**：`src/cli/commands/verify.rs`

**依赖**：Task 4.6

---

### Task 11.7: 实现 main 入口

**描述**：整合所有命令到 main 函数

**功能**：
- 解析命令行参数
- 路由到对应命令处理函数
- 统一错误处理

**文件**：`src/main.rs`

**依赖**：Task 11.1 ~ 11.6

---

## Phase 12: 集成与测试

### Task 12.1: 创建测试数据集

**描述**：准备测试用的视频文件名样本

**内容**：
- 常见命名格式样本
- 中英文混合样本
- 边界情况样本

**目录**：`tests/fixtures/`

**依赖**：无

---

### Task 12.2: 编写 Scanner 单元测试

**描述**：测试目录扫描功能

**文件**：`tests/scanner_test.rs`

**依赖**：Task 5.4, Task 12.1

---

### Task 12.3: 编写 Parser 单元测试

**描述**：测试文件名解析功能

**文件**：`tests/parser_test.rs`

**依赖**：Task 6.4, Task 12.1

---

### Task 12.4: 编写端到端测试

**描述**：测试完整工作流程

**场景**：
- Plan → Execute 流程
- Rollback 流程

**文件**：`tests/e2e_test.rs`

**依赖**：Task 11.7

---

### Task 12.5: 编写文档和使用说明

**描述**：完善 README 和使用文档

**内容**：
- 安装说明
- 快速开始
- 命令参考
- 配置说明

**文件**：`README.md`

**依赖**：Task 11.7

---

## 任务依赖图

```
Phase 1 (初始化)
    │
    ├──────────────────────────────────────┐
    ▼                                      ▼
Phase 2 (前置检查)                    Phase 3 (数据模型)
    │                                      │
    │                    ┌─────────────────┼─────────────────┐
    │                    ▼                 ▼                 ▼
    │              Phase 4 (服务)    Phase 5 (Scanner)  Phase 8 (生成器)
    │                    │                 │                 │
    │                    ▼                 │                 │
    │              Phase 6 (Parser) ◀──────┘                 │
    │                    │                                   │
    │                    ▼                                   │
    │              Phase 7 (Planner) ◀───────────────────────┘
    │                    │
    │                    ▼
    │              Phase 9 (Executor)
    │                    │
    │                    ▼
    │              Phase 10 (Rollback)
    │                    │
    └───────────────────▶│
                         ▼
                   Phase 11 (CLI)
                         │
                         ▼
                   Phase 12 (测试)
```

---

## 预估工时

| Phase | 任务数 | 预估工时 |
|-------|--------|----------|
| Phase 1: 项目初始化 | 5 | 2h |
| Phase 2: 前置检查 | 4 | 3h |
| Phase 3: 数据模型 | 6 | 4h |
| Phase 4: 外部服务 | 6 | 8h |
| Phase 5: Scanner | 4 | 3h |
| Phase 6: Parser | 4 | 6h |
| Phase 7: Planner | 5 | 6h |
| Phase 8: 生成器 | 4 | 4h |
| Phase 9: Executor | 5 | 6h |
| Phase 10: Rollback | 4 | 4h |
| Phase 11: CLI | 7 | 6h |
| Phase 12: 测试 | 5 | 6h |
| **总计** | **59** | **58h** |

---

## 里程碑

| 里程碑 | 完成 Phase | 可验证产出 |
|--------|-----------|-----------|
| M1: 基础框架 | Phase 1-3 | 项目可编译，模块结构完整 |
| M2: 服务集成 | Phase 4 | 可调用 Ollama、TMDB、ffprobe |
| M3: Plan 功能 | Phase 5-7 | 可生成 plan.json |
| M4: Execute 功能 | Phase 8-9 | 可执行计划，生成 rollback.json |
| M5: 完整 CLI | Phase 10-11 | 所有命令可用 |
| M6: 发布就绪 | Phase 12 | 测试通过，文档完善 |

