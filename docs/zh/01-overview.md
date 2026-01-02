# 01 - 项目概述

## 1. 项目简介

**Media Organizer** 是一个 Rust 编写的命令行工具，用于自动整理视频文件（电影和电视剧）。通过本地 AI（Ollama）识别文件名，结合 TMDB API 获取元数据，按照标准格式重命名和组织文件，生成媒体库兼容的 NFO 文件和海报。

### 核心功能

| 功能 | 描述 |
|------|------|
| **AI 文件名解析** | 使用 Ollama 本地 AI 解析中英文混合的视频文件名 |
| **TMDB 元数据获取** | 自动匹配 TMDB 数据库，获取详细信息 |
| **标准化命名** | 按照统一格式重命名文件和目录 |
| **NFO 生成** | 生成 Kodi/Emby/Jellyfin 兼容的 NFO 文件 |
| **海报下载** | 自动下载电影/剧集海报 |
| **中央索引** | 跨多硬盘的离线搜索和系列管理 |
| **配置导出/导入** | 支持备份和迁移 |

---

## 2. 技术栈

| 组件 | 技术 | 说明 |
|------|------|------|
| 开发语言 | Rust | 高性能、内存安全、单二进制分发 |
| 异步运行时 | tokio | 高效处理 I/O 密集操作 |
| CLI 框架 | clap | 命令行参数解析 |
| HTTP 客户端 | reqwest | TMDB API 和 Ollama API 调用 |
| AI 推理 | Ollama (qwen2.5:7b) | 本地运行的 LLM |
| 元数据提取 | ffprobe | 视频技术信息提取 |
| 目标平台 | Linux Only | 不支持 Windows/macOS |

---

## 3. 命名格式

### 电影

**目录格式**：
```
{国家代码}/{[原标题](年份)-ttIMDB-tmdbTMDB}/
```

**文件格式**：
```
[原标题](年份)-ttIMDB-tmdbTMDB-分辨率-格式-编码-位深-音频编码-声道.扩展名
```

**示例**：
```
US_UnitedStates/
└── [Avatar](2009)-tt0499549-tmdb19995/
    ├── [Avatar](2009)-tt0499549-tmdb19995-2160p-BluRay-x265-10bit-TrueHD-7.1.mkv
    ├── movie.nfo
    └── poster.jpg
```

### 电视剧

**剧集目录格式**：
```
{国家代码}/[剧集名](年份)-ttIMDB-tmdbTMDB/
```

**季目录格式**：
```
Season {XX}/
```

**文件格式**：
```
[剧集名]-S{XX}E{XX}-[集标题]-分辨率-格式-编码-位深-音频编码-声道.扩展名
```

**示例**：
```
CN_China/
└── [罚罪2](2025)-tt36771056-tmdb296146/
    ├── tvshow.nfo
    ├── poster.jpg
    └── Season 01/
        ├── [罚罪2]-S01E01-[第1集]-1080p-WEB-DL-h264-8bit-aac-2.0.mp4
        └── [罚罪2]-S01E02-[第2集]-1080p-WEB-DL-h264-8bit-aac-2.0.mp4
```

---

## 4. 工作流程

### 三步模型：Plan → Execute → Rollback

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│    Plan     │───▶│   Execute   │───▶│  Rollback   │
│             │    │             │    │  (可选)     │
│ 生成执行计划  │    │ 执行文件操作  │    │ 恢复原状态   │
│ plan.json   │    │ rollback.json│    │            │
└─────────────┘    └─────────────┘    └─────────────┘
```

1. **Plan**：扫描目录 → AI 解析 → TMDB 匹配 → 生成 `plan.json`
2. **Execute**：读取计划 → 执行操作 → 生成 `rollback.json`
3. **Rollback**：逆向执行 → 恢复原始状态

---

## 5. 命令参考

### 基础命令

```bash
# 生成计划（电影）
media-organizer plan movies /path/to/movies --target /path/to/organized

# 生成计划（电视剧）
media-organizer plan tvshows /path/to/tvshows --target /path/to/organized

# 执行计划
media-organizer execute plan.json

# 回滚操作
media-organizer rollback rollback.json
```

### 索引和搜索

```bash
# 索引目录
media-organizer index /path/to/organized

# 搜索
media-organizer search --actor "演员名"
media-organizer search --collection "系列名"
media-organizer search --title "标题"
```

### 配置导出/导入

```bash
# 导出
media-organizer export backup.zip

# 导入
media-organizer import backup.zip
```

---

## 6. 核心原则

### 6.1 "宁可遗漏，不能错误"

当遇到以下情况时，跳过处理而不是强制分类：
- AI 置信度低于阈值
- TMDB 搜索无精确匹配
- 国家信息不确定

### 6.2 幂等性

- 对同一目录多次执行应产生相同结果
- 已整理的文件能被正确识别和快速处理
- 不会覆盖已存在的文件

### 6.3 可回滚

- 所有操作可逆
- 每次执行生成回滚文件
- 支持完全恢复原始状态

---

## 7. 环境要求

| 依赖 | 要求 | 检查命令 |
|------|------|----------|
| Ollama | 运行中，已加载模型 | `curl http://localhost:11434/api/tags` |
| FFprobe | 已安装 | `ffprobe -version` |
| TMDB API | 有效 API Key | 环境变量 `TMDB_API_KEY` |

### 环境变量

```bash
export TMDB_API_KEY="your_api_key"
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="qwen2.5:7b"
```

---

## 8. 配置文件位置

```
~/.config/media_organizer/
├── config.toml              # 应用配置
├── central_index.json       # 中央索引
├── disk_indexes/            # 单硬盘索引
│   └── {disk_label}.json
└── sessions/                # 会话历史
    └── {timestamp}_{id}/
        ├── plan.json
        └── rollback.json
```

---

## 9. 文档索引

| 文档 | 内容 |
|------|------|
| [02-architecture.md](02-architecture.md) | 系统架构设计 |
| [03-processing-flow.md](03-processing-flow.md) | 核心处理流程 |
| [04-central-index.md](04-central-index.md) | 中央索引系统 |
| [05-export-import.md](05-export-import.md) | 配置导出导入 |
| [06-gpu-setup.md](06-gpu-setup.md) | GPU 配置指南 |

