# 快速开始指南

## 前置要求

- Linux 系统
- 8GB+ 内存
- [Ollama](https://ollama.ai) 运行中，已安装 `qwen2.5:7b` 模型
- [TMDB API Key](https://www.themoviedb.org/settings/api)
- ffprobe (随 ffmpeg 安装)

## 1. 环境配置

```bash
# 设置环境变量
export TMDB_API_KEY="你的TMDB_API_KEY"
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="qwen2.5:7b"
```

## 2. 安装

```bash
# 从源码编译
git clone https://github.com/jpixy/media_organizer.git
cd media_organizer
cargo build --release

# 二进制文件位于 target/release/media-organizer
```

## 3. 快速测试

```bash
# 检查前置条件
media-organizer plan movies /path/to/source --dry-run

# 应显示：
# [OK] ffprobe: installed
# [OK] Ollama: running
# [OK] TMDB API: connected
```

## 4. 基本工作流程

### 整理电影

```bash
# 1. 生成计划（不移动文件）
media-organizer plan movies /源目录 -t /目标目录

# 2. 检查计划
cat /目标目录/plan_*.json | jq '.items | length'

# 3. 执行计划
media-organizer execute /目标目录/plan_*.json

# 4. 如需回滚
media-organizer rollback /目标目录/rollback_*.json
```

### 整理电视剧

```bash
media-organizer plan tvshows /源目录 -t /目标目录
media-organizer execute /目标目录/plan_*.json
```

## 5. 建立索引

```bash
# 索引电影目录
media-organizer index scan /目标目录 --media-type movies --disk-label MyDisk

# 索引电视剧目录
media-organizer index scan /目标目录 --media-type tvshows --disk-label MyDisk

# 查看统计
media-organizer index stats
```

## 6. 搜索

```bash
# 按标题搜索（同时搜索电影和电视剧）
media-organizer search --title "加勒比海盗"

# 按演员搜索
media-organizer search --actor "约翰尼·德普"

# 按年份搜索
media-organizer search --year 2020-2024

# 显示在线/离线状态
media-organizer search --title "盗梦空间" --show-status
```

## 常用命令速查

| 命令 | 描述 |
|------|------|
| `plan movies <源> -t <目标>` | 生成电影整理计划 |
| `plan tvshows <源> -t <目标>` | 生成电视剧整理计划 |
| `execute <plan.json>` | 执行计划 |
| `rollback <rollback.json>` | 回滚操作 |
| `index scan <路径>` | 扫描目录建立索引 |
| `index stats` | 显示收藏统计 |
| `search --title <标题>` | 搜索标题 |
| `search --actor <演员>` | 搜索演员 |
| `export` | 导出配置和索引 |
| `import <备份文件>` | 导入配置和索引 |

## 下一步

- 阅读 [完整使用手册](USER-MANUAL.md)
- 了解 [中央索引系统](04-central-index.md)
- 了解 [配置导出导入](05-export-import.md)

