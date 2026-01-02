# 04 - 中央索引系统

## 1. 概述

### 问题描述

用户拥有大量媒体收藏，由于容量限制，电影通常分散存储在多个外置硬盘上。由于一次只能挂载一个硬盘，因此很难：

1. 跨所有硬盘按演员、导演或系列搜索电影
2. 知道特定电影在哪个硬盘上
3. 识别哪些电影属于同一系列（例如：加勒比海盗1-5）
4. 获得整个收藏的概览

### 解决方案

**持久化的中央索引**：
- 将所有已处理硬盘的元数据存储在本地
- 支持离线搜索（硬盘无需挂载）
- 追踪电影系列/合集信息
- 提供按演员、导演、系列或标题的快速查找

---

## 2. 文件结构

```
~/.config/media_organizer/
├── central_index.json          # 主索引（所有硬盘合并）
├── central_index.json.backup   # 更新前自动备份
└── disk_indexes/
    ├── JMedia_M01.json         # 单个硬盘索引
    ├── JMedia_M02.json
    └── JMedia_M05.json
```

---

## 3. 数据结构

### 3.1 中央索引结构

```json
{
  "version": "1.0",
  "created_at": "2026-01-01T10:00:00Z",
  "updated_at": "2026-01-01T18:00:00Z",
  
  "disks": {
    "JMedia_M01": {
      "label": "JMedia_M01",
      "uuid": "1234-5678-ABCD",
      "last_indexed": "2026-01-01T12:00:00Z",
      "movie_count": 280,
      "tvshow_count": 45,
      "total_size_bytes": 1288490188800,
      "base_path": "/run/media/johnny/JMedia_M01/Movies_organized"
    }
  },
  
  "movies": [
    {
      "id": "uuid-xxxx",
      "disk": "JMedia_M01",
      "relative_path": "US_UnitedStates/[Pirates of the Caribbean...]/movie.mkv",
      "title": "加勒比海盗：黑珍珠号的诅咒",
      "original_title": "Pirates of the Caribbean: The Curse of the Black Pearl",
      "year": 2003,
      "tmdb_id": 22,
      "imdb_id": "tt0325980",
      "collection_id": 295,
      "collection_name": "Pirates of the Caribbean Collection",
      "country": "US",
      "genres": ["冒险", "奇幻", "动作"],
      "actors": ["约翰尼·德普", "奥兰多·布鲁姆"],
      "directors": ["戈尔·维宾斯基"],
      "runtime": 143,
      "rating": 7.8
    }
  ],
  
  "tvshows": [...],
  
  "collections": {
    "295": {
      "id": 295,
      "name": "加勒比海盗系列",
      "movies": [
        {"tmdb_id": 22, "title": "黑珍珠号的诅咒", "year": 2003, "disk": "JMedia_M01"},
        {"tmdb_id": 58, "title": "聚魂棺", "year": 2006, "disk": "JMedia_M02"}
      ],
      "total_in_collection": 5,
      "owned_count": 2
    }
  },
  
  "indexes": {
    "by_actor": {"约翰尼·德普": ["uuid-xxxx"]},
    "by_director": {"戈尔·维宾斯基": ["uuid-xxxx"]},
    "by_genre": {"动作": ["uuid-xxxx"]},
    "by_year": {"2003": ["uuid-xxxx"]},
    "by_country": {"US": ["uuid-xxxx"]}
  },
  
  "statistics": {
    "total_movies": 1250,
    "total_tvshows": 180,
    "total_disks": 5,
    "total_size_bytes": 6500000000000
  }
}
```

---

## 4. 命令参考

### 4.1 index 命令

```bash
# 基本用法 - 索引当前硬盘
media-organizer index /run/media/johnny/JMedia_M05/Movies_organized

# 使用自定义硬盘标签
media-organizer index /path/to/movies --disk-label "Archive_2024"

# 索引电视剧
media-organizer index /path/to/tvshows --media-type tvshows

# 强制重新索引
media-organizer index /path/to/movies --force

# 试运行
media-organizer index /path/to/movies --dry-run
```

### 4.2 search 命令

```bash
# 按演员搜索
media-organizer search --actor "约翰尼·德普"
media-organizer search -a "Johnny Depp"

# 按导演搜索
media-organizer search --director "克里斯托弗·诺兰"
media-organizer search -d "Nolan"

# 按系列搜索
media-organizer search --collection "加勒比海盗"
media-organizer search -c "Marvel"

# 按标题搜索
media-organizer search --title "加勒比"
media-organizer search -t "Inception"

# 按年份搜索
media-organizer search --year 2024
media-organizer search --year 2020-2024

# 按类型搜索
media-organizer search --genre "动作"

# 按国家搜索
media-organizer search --country US

# 组合筛选
media-organizer search --actor "德普" --year 2000-2010

# 显示硬盘状态
media-organizer search --actor "Depp" --show-status
```

### 4.3 统计和管理

```bash
# 显示收藏统计
media-organizer index --stats

# 列出硬盘内容
media-organizer index --list JMedia_M05

# 从索引移除硬盘
media-organizer index --remove JMedia_OLD

# 验证索引与文件
media-organizer index --verify /run/media/johnny/JMedia_M05
```

---

## 5. 输出示例

### 5.1 搜索输出

```
$ media-organizer search --actor "约翰尼·德普"

找到 8 部包含演员 "约翰尼·德普" 的电影：

 #  | 年份 | 标题                                    | 硬盘        | 状态
----|------|------------------------------------------|-------------|--------
 1  | 2003 | 加勒比海盗：黑珍珠号的诅咒                  | JMedia_M01  | 离线
 2  | 2006 | 加勒比海盗：聚魂棺                         | JMedia_M02  | 离线
 3  | 2007 | 加勒比海盗：世界的尽头                      | JMedia_M05  | 在线
 4  | 2024 | 僵尸喜欢黑夜                               | JMedia_M05  | 在线

系列信息:
  - 加勒比海盗系列: 已拥有 5/5 部（分布在 3 个硬盘）
```

### 5.2 统计输出

```
$ media-organizer index --stats

媒体收藏统计
============================

硬盘:
  JMedia_M01  | 280 部电影 |  45 部剧集 | 1.2 TB | 最后索引: 2026-01-01
  JMedia_M02  | 310 部电影 |  52 部剧集 | 1.5 TB | 最后索引: 2025-12-28
  JMedia_M05  | 150 部电影 |  30 部剧集 | 0.8 TB | 最后索引: 2026-01-01
  -------------------------------------------------------------------------
  合计        | 740 部电影 | 127 部剧集 | 3.5 TB

按国家:
  US (美国)    ████████████████████  350 (47%)
  CN (中国)    ████████████          220 (30%)
  KR (韩国)    ████                  85  (11%)
  JP (日本)    ██                    45  (6%)
  其他         ██                    40  (6%)

系列收藏:
  完整:    35 个系列（所有电影已拥有）
  不完整:  18 个系列（部分电影缺失）
```

---

## 6. 系列（Collection）支持

### 6.1 TMDB Collections API

```
GET /collection/{collection_id}?api_key=xxx&language=zh-CN
```

### 6.2 NFO 中的系列信息

```xml
<movie>
  <title>加勒比海盗：黑珍珠号的诅咒</title>
  ...
  <set>
    <name>加勒比海盗系列</name>
    <overview>...</overview>
  </set>
  <tmdbcollectionid>295</tmdbcollectionid>
</movie>
```

---

## 7. 边缘情况处理

| 场景 | 处理方式 |
|------|----------|
| 硬盘标签冲突 | 使用 UUID 作为主要标识符 |
| 电影在硬盘间移动 | 索引时自动检测并更新 |
| 重复电影 | 允许重复，搜索显示所有副本 |
| 硬盘重命名 | 提供 `--rename` 命令 |
| 索引损坏 | 自动备份，支持从硬盘索引重建 |

---

## 8. 工作流程

### 8.1 初始设置

```bash
# 1. 处理第一个硬盘
media-organizer plan movies /run/media/johnny/JMedia_M01/Movies
media-organizer execute plan.json

# 2. 自动创建索引（execute 后自动触发）
# 或手动执行:
media-organizer index /run/media/johnny/JMedia_M01/Movies_organized

# 3. 对所有硬盘重复
```

### 8.2 日常使用

```bash
# 查找电影
media-organizer search -t "盗梦空间"
# 输出: [JMedia_M02] Inception (2010) - 离线

# 查找不完整的系列
media-organizer search -c "" --incomplete
```

