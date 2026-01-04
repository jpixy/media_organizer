# Media Organizer 完整使用手册

## 目录

1. [概述](#1-概述)
2. [安装与配置](#2-安装与配置)
3. [命令参考](#3-命令参考)
4. [工作流程](#4-工作流程)
5. [中央索引系统](#5-中央索引系统)
6. [搜索功能](#6-搜索功能)
7. [备份与恢复](#7-备份与恢复)
8. [常见问题](#8-常见问题)

---

## 1. 概述

**Media Organizer** 是一个 Rust 编写的命令行工具，用于自动整理视频文件（电影和电视剧）。

### 核心功能

| 功能 | 描述 |
|------|------|
| **AI 文件名解析** | 使用 Ollama 本地 AI 解析中英文混合的视频文件名 |
| **TMDB 元数据获取** | 自动匹配 TMDB 数据库，获取详细信息 |
| **标准化命名** | 按照统一格式重命名文件和目录 |
| **NFO 生成** | 生成 Kodi/Emby/Jellyfin 兼容的 NFO 文件 |
| **海报下载** | 自动下载电影/剧集海报 |
| **中央索引** | 跨多硬盘的离线搜索，同时支持电影和电视剧 |
| **配置导出/导入** | 支持备份和迁移 |

### 支持的媒体类型

- **movies** - 电影
- **tvshows** - 电视剧

---

## 2. 安装与配置

### 2.1 系统要求

- Linux 操作系统
- 8GB+ RAM（推荐 16GB+）
- Rust 工具链（编译需要）

### 2.2 依赖服务

| 服务 | 用途 | 安装 |
|------|------|------|
| **Ollama** | AI 文件名解析 | `curl -fsSL https://ollama.ai/install.sh \| sh` |
| **qwen2.5:7b** | AI 模型 | `ollama pull qwen2.5:7b` |
| **ffprobe** | 视频信息提取 | `sudo apt install ffmpeg` |
| **TMDB API** | 元数据获取 | 在 themoviedb.org 注册获取 |

### 2.3 环境变量

```bash
# 必需
export TMDB_API_KEY="你的API密钥"

# 可选（有默认值）
export OLLAMA_BASE_URL="http://localhost:11434"  # 默认
export OLLAMA_MODEL="qwen2.5:7b"                  # 默认
```

### 2.4 编译安装

```bash
git clone https://github.com/jpixy/media_organizer.git
cd media_organizer
cargo build --release

# 可选：添加到 PATH
sudo cp target/release/media-organizer /usr/local/bin/
```

---

## 3. 命令参考

### 3.1 全局选项

```bash
media-organizer [OPTIONS] <COMMAND>

Options:
  -v, --verbose         详细输出
      --skip-preflight  跳过前置检查
  -h, --help            显示帮助
  -V, --version         显示版本
```

### 3.2 plan - 生成整理计划

生成文件组织计划，不实际移动文件。

```bash
# 电影
media-organizer plan movies <源目录> [OPTIONS]

# 电视剧
media-organizer plan tvshows <源目录> [OPTIONS]

Options:
  -t, --target <目标目录>  目标目录（默认：源目录_organized）
      --dry-run           仅检查，不生成计划
```

**示例：**

```bash
# 整理电影
media-organizer plan movies /mnt/downloads/movies -t /mnt/library/movies

# 整理电视剧
media-organizer plan tvshows /mnt/downloads/tvshows -t /mnt/library/tvshows
```

### 3.3 execute - 执行计划

执行 plan 命令生成的计划文件。

```bash
media-organizer execute <plan.json> [OPTIONS]

Options:
  -o, --output <路径>  rollback 文件输出路径
```

**示例：**

```bash
media-organizer execute /mnt/library/movies/plan_20260104_123456.json
```

### 3.4 rollback - 回滚操作

回滚之前的执行操作，将文件移回原位置。

```bash
media-organizer rollback <rollback.json> [OPTIONS]

Options:
  --dry-run  预览回滚操作，不实际执行
```

**示例：**

```bash
# 预览回滚
media-organizer rollback /mnt/library/movies/rollback_20260104_123456.json --dry-run

# 执行回滚
media-organizer rollback /mnt/library/movies/rollback_20260104_123456.json
```

### 3.5 index - 索引管理

管理中央媒体索引。

```bash
media-organizer index <SUBCOMMAND>

Subcommands:
  scan         扫描目录建立索引
  stats        显示收藏统计
  list         列出指定硬盘内容
  verify       验证索引与文件一致性
  remove       从索引移除硬盘
  duplicates   查找重复项
  collections  列出电影合集
```

#### scan - 扫描目录

```bash
media-organizer index scan <路径> [OPTIONS]

Options:
  --media-type <类型>    movies 或 tvshows（默认：movies）
  --disk-label <标签>    硬盘标签（自动检测）
  --force                强制重新索引
```

**示例：**

```bash
# 索引电影
media-organizer index scan /mnt/library/movies --media-type movies --disk-label MyDisk

# 索引电视剧（同一硬盘）
media-organizer index scan /mnt/library/tvshows --media-type tvshows --disk-label MyDisk

# 单个硬盘可同时包含电影和电视剧，使用相同的 disk-label
```

#### stats - 显示统计

```bash
media-organizer index stats
```

**输出示例：**

```
Media Collection Statistics
==================================================

Disks:
  JMedia_M05 | 154 movies | 100 TV shows | 2959.9 GB | Online
      movies -> /mnt/library/movies
      tvshows -> /mnt/library/tvshows
--------------------------------------------------
  Total | 154 movies | 100 TV shows | 2959.9 GB

By Language:
  EN ████████████████ 82 (32%)
  ZH  ██████████████ 73 (29%)
  KO           █████ 28 (11%)

By Decade:
  2020s      ██████████ 131 (52%)
  2010s ████████████████ 82 (32%)
  2000s         ███████ 22 (9%)

Collections:
  Complete: 7 collections
  Incomplete: 31 collections
```

**注意：** By Language 和 By Decade 统计同时包含电影和电视剧。

#### list - 列出内容

```bash
media-organizer index list [OPTIONS]

Options:
  --disk-label <标签>   指定硬盘
  --media-type <类型>   movies 或 tvshows
```

#### remove - 移除硬盘

```bash
media-organizer index remove <disk-label>
```

### 3.6 search - 搜索

搜索媒体收藏。**同时搜索电影和电视剧**。

```bash
media-organizer search [OPTIONS]

Options:
  -t, --title <标题>        按标题搜索
  -a, --actor <演员>        按演员搜索
  -d, --director <导演>     按导演搜索（仅电影）
  -c, --collection <系列>   按系列搜索（仅电影）
  -y, --year <年份>         按年份搜索（支持范围：2020-2024）
  -g, --genre <类型>        按类型搜索
  --language <语言代码>     按语言搜索（en, zh, ja, ko 等）
  --show-status             显示在线/离线状态
  --format <格式>           输出格式：table, simple, json
```

**示例：**

```bash
# 按标题搜索（同时搜索电影和电视剧）
media-organizer search --title "盗梦空间"

# 按演员搜索
media-organizer search --actor "约翰尼·德普"

# 按年份范围搜索
media-organizer search --year 2020-2024

# 组合搜索
media-organizer search --actor "莱昂纳多" --year 2010-2020

# JSON 输出
media-organizer search --title "黑镜" --format json
```

**输出示例：**

```
Found 5 results:

Movies (3):
   # | Year | Title                                    | Disk         | Country
--------------------------------------------------------------------------------
   1 | 2010 | 盗梦空间                                 | JMedia_M05   | US
   2 | 2014 | 星际穿越                                 | JMedia_M05   | US
   3 | 2017 | 敦刻尔克                                 | JMedia_M02   | UK

TV Shows (2):
   # | Year | Title                                    | Disk         | Episodes
--------------------------------------------------------------------------------
   1 | 2016 | 西部世界                                 | JMedia_M05   | 36
   2 | 2019 | 切尔诺贝利                               | JMedia_M05   | 5
```

### 3.7 export - 导出

导出配置和索引用于备份。

```bash
media-organizer export [OUTPUT] [OPTIONS]

Options:
  --include-secrets    包含敏感数据（API 密钥）
  --only <类型>        仅导出：indexes, config, sessions
  --exclude <类型>     排除：indexes, config, sessions
  --disk <标签>        仅导出指定硬盘索引
  --auto-name          自动生成带时间戳的文件名
```

**示例：**

```bash
# 完整备份
media-organizer export --auto-name

# 仅备份索引
media-organizer export --only indexes --auto-name

# 备份到指定文件
media-organizer export /path/to/backup.zip
```

### 3.8 import - 导入

从备份文件导入配置和索引。

```bash
media-organizer import <备份文件> [OPTIONS]

Options:
  --dry-run       预览导入内容
  --only <类型>   仅导入：indexes, config, sessions
  --merge         合并（不覆盖现有数据）
  --force         强制覆盖
  --backup-first  导入前先备份现有配置
```

**示例：**

```bash
# 预览
media-organizer import backup_20260104.zip --dry-run

# 合并导入
media-organizer import backup_20260104.zip --merge

# 强制覆盖
media-organizer import backup_20260104.zip --force --backup-first
```

---

## 4. 工作流程

### 4.1 完整工作流程

```
┌─────────────────┐
│   源视频文件     │
│  (杂乱的命名)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   plan 命令     │  ← AI 解析 + TMDB 匹配
│  生成整理计划    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   检查计划       │  ← 人工审核
│  (可选)          │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  execute 命令   │  ← 移动文件 + 生成 NFO + 下载海报
│   执行计划       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  index scan     │  ← 建立中央索引
│   建立索引       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    search       │  ← 跨硬盘搜索
│   搜索媒体       │
└─────────────────┘
```

### 4.2 目录结构示例

**整理前：**
```
/downloads/
├── [2024]加勒比海盗.mkv
├── Black.Mirror.S01E01.720p.mkv
├── 盗梦空间.2010.1080p.BluRay.mkv
└── 西游记之大圣归来 (2015).mp4
```

**整理后：**
```
/library/
├── movies/
│   ├── ZH_Chinese/
│   │   └── [西游记之大圣归来](2015)-tt4040840-tmdb166589/
│   │       ├── [西游记之大圣归来](2015)-1920x1080(1080p)-BluRay-h264-8bit-aac-2.0.mp4
│   │       ├── movie.nfo
│   │       └── poster.jpg
│   └── EN_English/
│       └── [Inception][盗梦空间](2010)-tt1375666-tmdb27205/
│           ├── [Inception][盗梦空间](2010)-1920x1080(1080p)-BluRay-h264-8bit-dts-5.1.mkv
│           ├── movie.nfo
│           └── poster.jpg
└── tvshows/
    └── GB_UnitedKingdom/
        └── [Black Mirror][黑镜](2011)-tt2085059-tmdb42009/
            └── Season 01/
                ├── [黑镜]-S01E01-[国歌]-720p-WEB-h264-8bit-aac-2.0.mkv
                └── episode.nfo
```

---

## 5. 中央索引系统

### 5.1 设计理念

- **跨硬盘搜索**：即使硬盘未挂载也能搜索
- **统一管理**：一个硬盘可同时包含电影和电视剧
- **离线浏览**：无需挂载硬盘即可查看收藏

### 5.2 存储结构

```
~/.config/media_organizer/
├── central_index.json          # 主索引
├── central_index.json.backup   # 自动备份
└── disk_indexes/
    ├── JMedia_M01.json
    ├── JMedia_M02.json
    └── JMedia_M05.json
```

### 5.3 复合存储

单个 disk-label 可同时存储多种媒体类型：

```bash
# 同一硬盘，不同媒体类型
media-organizer index scan /mnt/disk/movies --media-type movies --disk-label MyDisk
media-organizer index scan /mnt/disk/tvshows --media-type tvshows --disk-label MyDisk
```

索引中会记录：
```json
{
  "disks": {
    "MyDisk": {
      "paths": {
        "movies": "/mnt/disk/movies",
        "tvshows": "/mnt/disk/tvshows"
      }
    }
  }
}
```

---

## 6. 搜索功能

### 6.1 搜索范围

| 搜索条件 | 电影 | 电视剧 |
|---------|------|--------|
| --title | ✅ | ✅ |
| --actor | ✅ | ✅ |
| --director | ✅ | ❌ (用 creators) |
| --collection | ✅ | ❌ |
| --year | ✅ | ✅ |
| --genre | ✅ | ✅ |
| --language | ✅ | ✅ |

### 6.2 输出格式

```bash
# 表格格式（默认）
media-organizer search --title "黑镜"

# 简洁格式
media-organizer search --title "黑镜" --format simple

# JSON 格式（适合脚本处理）
media-organizer search --title "黑镜" --format json
```

---

## 7. 备份与恢复

### 7.1 推荐备份策略

```bash
# 每周完整备份
media-organizer export --auto-name

# 仅备份索引（更频繁）
media-organizer export --only indexes --auto-name
```

### 7.2 恢复流程

```bash
# 1. 预览备份内容
media-organizer import backup.zip --dry-run

# 2. 备份当前配置后导入
media-organizer import backup.zip --backup-first --force
```

---

## 8. 常见问题

### Q: 如何处理 AI 解析失败的文件？

A: 检查 plan 输出的 "Unknown Files" 列表，可以：
1. 手动重命名文件后重新 plan
2. 确保 Ollama 服务运行正常

### Q: 如何更新已整理文件的索引？

A: 使用 `--force` 重新扫描：
```bash
media-organizer index scan /path --force
```

### Q: 搜索只显示电影，没有电视剧？

A: 确保电视剧也已建立索引：
```bash
media-organizer index scan /path/to/tvshows --media-type tvshows
```

### Q: 如何查看哪些硬盘在线？

A: 使用 `--show-status` 选项：
```bash
media-organizer search --title "test" --show-status
media-organizer index stats
```

---

## 附录

### A. 国家代码参考

| 代码 | 国家 |
|------|------|
| US | 美国 |
| CN | 中国 |
| KR | 韩国 |
| JP | 日本 |
| GB | 英国 |
| FR | 法国 |
| DE | 德国 |
| TW | 台湾 |
| HK | 香港 |

### B. 环境变量

| 变量 | 描述 | 默认值 |
|------|------|--------|
| TMDB_API_KEY | TMDB API 密钥 | (必需) |
| OLLAMA_BASE_URL | Ollama 服务地址 | http://localhost:11434 |
| OLLAMA_MODEL | AI 模型名称 | qwen2.5:7b |

