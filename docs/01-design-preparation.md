# 设计前准备

本文档整理了 Media Organizer 项目在正式开发前的讨论和决策。

---

## 项目概述

Media Organizer 是一个命令行工具，用于自动整理视频文件（电影和电视剧）。通过 AI 识别文件名中的标题和年份，结合 TMDB API 获取元数据，按照标准格式重命名和组织文件。

### 核心功能

1. 扫描指定目录下的视频文件
2. 使用本地 AI（Ollama）解析文件名，提取标题、年份等信息
3. 调用 TMDB API 获取详细元数据
4. 按照标准格式重命名文件和目录
5. 生成 NFO 文件（Kodi 兼容格式）
6. 下载电影海报（1-3 张）
7. 处理 Sample 文件夹

---

## 技术决策

### 开发语言：Rust

选择 Rust 的理由：
- 性能优秀，编译后单二进制分发
- 内存安全，长期运行稳定
- tokio 异步生态成熟，处理大量文件 I/O 效率高
- serde 处理 JSON 非常方便

### 目标平台：Linux Only

仅支持 Linux 平台，不考虑 Windows 和 macOS。

### AI 服务：local-ai-starter

- `local-ai-starter` 是独立的通用服务
- 只负责启动 Ollama 并暴露标准 REST API
- 不为任何特定项目定制
- Media Organizer 作为客户端调用其 API

### 视频元数据提取：ffprobe

选择 ffprobe 的理由：
- 预装率高（FFmpeg 通常预装）
- 速度快
- JSON 输出格式方便解析
- 信息覆盖足够（分辨率、编码、音轨）

### 不需要的技术

以下技术经评估后决定不引入：

| 技术 | 不需要的原因 |
|------|-------------|
| RAG | 无需查询外部知识库，TMDB 信息通过 API 直接获取 |
| MCP | 工作流是确定性的，不需要 LLM 自主决策调用工具 |
| LangChain | 单轮调用场景，引入反而增加复杂度；且项目使用 Rust |

---

## 命名格式

### 电影

**文件夹格式：**
```
[${originalTitle}]-[${title}](${edition})-${year}-${imdb}-${tmdb}
```

**文件名格式：**
```
[${originalTitle}]-[${title}](${edition})-${year}-${videoResolution}-${videoFormat}-${videoCodec}-${videoBitDepth}bit-${audioCodec}-${audioChannelsAsString}
```

**说明：**
- 如果是中文电影（originalTitle 与 title 相同或为繁简体差异），只保留一个 title
- edition 如 Director's Cut、Extended Edition 等，可选字段

**示例：**
```
[Avatar]-[阿凡达](2009)-tt0499549-tmdb19995/
  ├── [Avatar]-[阿凡达](2009)-2160p-BluRay-x265-10bit-TrueHD-7.1.mkv
  ├── movie.nfo
  ├── poster.jpg
  └── Sample/
```

### 电视剧

**剧集文件夹格式：**
```
[${showOriginalTitle}]-[${showTitle}]-${showImdb}-${showTmdb}
```

**季文件夹格式：**
```
S${seasonNr2}.${showYear}
```

**剧集文件名格式：**
```
[${showOriginalTitle}]-S${seasonNr2}E${episodeNr2}-[${originalTitle}]-[${title}]-${videoFormat}-${videoCodec}-${videoBitDepth}bit-${audioCodec}-${audioChannelsAsString}
```

---

## CLI 命令设计

### 命令结构

```
media-organizer
├── plan <type> <source> [--target <path>]    # 生成执行计划
├── execute <plan.json>                        # 执行计划
├── rollback <rollback.json>                   # 回滚操作
├── sessions                                   # 会话管理
│   ├── list                                   # 列出所有会话
│   └── show <session-id>                      # 显示会话详情
└── verify <path>                              # 视频完整性校验
```

### 媒体类型区分

用户通过位置参数明确指定媒体类型，程序不自动检测：

- `movies` - 处理电影文件
- `tvshows` - 处理电视剧文件

### 命令示例

**Plan 阶段 - 需要指定类型：**
```bash
# 电影
media-organizer plan movies /mnt/downloads/movies --target /mnt/media/movies

# 电视剧
media-organizer plan tvshows /mnt/downloads/shows --target /mnt/media/tvshows
```

**Execute 阶段 - 不需要类型（从 plan.json 读取）：**
```bash
media-organizer execute ./plan.json
```

**Rollback 阶段 - 不需要类型（从 rollback.json 读取）：**
```bash
media-organizer rollback ./rollback.json
```

**会话管理：**
```bash
media-organizer sessions list
media-organizer sessions show 2024-12-24_abc123
```

**视频校验：**
```bash
media-organizer verify /path/to/videos
```

### plan.json 中的类型记录

```json
{
  "version": "1.0",
  "media_type": "movies",
  "source_path": "/mnt/downloads/movies",
  "target_path": "/mnt/media/movies",
  ...
}
```

Execute 和 Rollback 命令直接读取 JSON 文件中的 `media_type` 字段，无需用户再次指定。

---

## 工作流设计

### 三步模型：Plan → Execute → Rollback

#### Step 1: Plan

- 扫描目录
- 解析文件名（调用 Ollama）
- 查询 TMDB
- 生成 `plan.json`

用户可在执行前审核、修改 plan.json。

#### Step 2: Execute

- 读取 `plan.json`
- 逐条执行操作
- 生成 `rollback.json`
- 创建 NFO 文件、下载海报

#### Step 3: Rollback（可选）

- 读取 `rollback.json`
- 逆向执行操作
- 恢复原始目录结构

### 回滚功能分阶段实现

**第一阶段（v0.1）：**
- `--dry-run` 预览模式
- 操作日志记录
- `sessions list` 查看历史

**第二阶段（v0.2+）：**
- `rollback` 命令
- 冲突检测
- 部分回滚支持

---

## 目录策略

### 目标目录

- 支持 `--target` 参数指定目标目录
- 使用移动操作（不复制），节省空间
- rollback.json 记录原始路径，支持回滚

### 默认行为

如果不指定 `--target`，在源目录同级创建 `_organized` 目录：
```
/mnt/downloads/movies/           ← 源目录
/mnt/downloads/movies_organized/ ← 默认目标
```

### 解析失败处理

当 AI 无法解析文件名或 TMDB 查无结果时，文件放入 `unknown/` 文件夹，后续手动处理。

---

## 前置检查

程序运行前进行以下检查，任一失败则终止并提示：

| 检查项 | 检查方式 | 失败提示 |
|--------|---------|---------|
| ffprobe | 执行 `ffprobe -version` | Install FFmpeg: `sudo apt install ffmpeg` |
| Ollama | HTTP GET `/api/tags` | Start Ollama: `ollama serve` |
| TMDB API | HTTP GET 测试端点 | Set `TMDB_API_KEY` environment variable |

---

## 幂等性保证

针对同一目录，无论整理多少次，结果都应该是唯一的。

实现方式：
- 在目标目录生成元数据文件，记录源目录结构 hash 和 TMDB ID
- 每次运行前检查，如果已处理且无变化则跳过

---

## 可选功能

### 视频完整性校验

提供 CLI 命令校验视频文件是否完整，但默认不开启以提高效率。

```bash
media-organizer verify /path/to/videos
```

---

## 配置管理

### TMDB API Key

支持以下方式配置（按优先级）：
1. 命令行参数 `--api-key`
2. 环境变量 `TMDB_API_KEY`
3. 配置文件 `~/.config/media_organizer/config.toml`

---

## 待确认事项

- [ ] Ollama 使用的具体模型（如 qwen2.5:7b）
- [ ] TMDB API Key 的具体值
- [ ] 海报下载的具体数量和尺寸偏好

