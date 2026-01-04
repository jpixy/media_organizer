# 05 - 配置导出与导入

## 1. 概述

### 需求背景

用户在使用 `media_organizer` 时，可能需要：

1. **迁移到新机器** - 将配置和索引数据迁移到新电脑
2. **备份配置** - 定期备份所有设置和索引
3. **多机同步** - 在多台电脑间同步配置
4. **恢复环境** - 重装系统后快速恢复工作环境

### 解决方案

提供 `export` 和 `import` 命令，支持：
- 导出/导入中央索引
- 导出/导入应用配置
- 导出/导入会话历史
- 支持选择性导出/导入

---

## 2. 可导出的数据

| 类别 | 路径 | 描述 | 大小估算 |
|------|------|------|----------|
| **中央索引** | `~/.config/media_organizer/central_index.json` | 所有硬盘的电影/剧集索引 | 1-50 MB |
| **硬盘索引** | `~/.config/media_organizer/disk_indexes/*.json` | 单个硬盘索引 | 0.1-5 MB/个 |
| **会话历史** | `~/.config/media_organizer/sessions/` | 历史 plan/rollback 文件 | 1-100 MB |
| **应用配置** | `~/.config/media_organizer/config.toml` | API密钥、默认设置等 | < 1 KB |

### 敏感数据处理

| 数据类型 | 处理方式 |
|----------|----------|
| TMDB API Key | 默认**不导出**，需显式指定 `--include-secrets` |
| 文件路径 | 导出时保留，导入时可自动调整 |
| UUID | 保留（用于硬盘识别） |

---

## 3. 导出格式

### 导出包结构

```
media_organizer_backup_20260101_180000.zip
├── manifest.json           # 导出清单
├── config/
│   └── config.toml         # 应用配置（可选）
├── indexes/
│   ├── central_index.json  # 中央索引
│   └── disk_indexes/
│       ├── JMedia_M01.json
│       └── JMedia_M05.json
└── sessions/               # 会话历史（可选）
    └── 20260101_120000_xxxx/
        ├── plan.json
        └── rollback.json
```

### manifest.json 结构

```json
{
  "version": "1.0",
  "app_version": "0.1.0",
  "created_at": "2026-01-01T18:00:00Z",
  "created_by": "johnny@hostname",
  "description": "Full backup before migration",
  
  "contents": {
    "config": true,
    "central_index": true,
    "disk_indexes": ["JMedia_M01", "JMedia_M05"],
    "sessions": 45,
    "includes_secrets": false
  },
  
  "statistics": {
    "total_movies": 1250,
    "total_tvshows": 180,
    "total_disks": 5,
    "total_sessions": 45,
    "export_size_bytes": 15000000
  }
}
```

---

## 4. 命令参考

### 4.1 export 命令

```bash
# 完整导出（不含敏感数据）
media-organizer export backup.zip

# 完整导出（含敏感数据如 API Key）
media-organizer export backup.zip --include-secrets

# 只导出索引
media-organizer export backup.zip --only indexes

# 只导出配置
media-organizer export backup.zip --only config

# 只导出特定硬盘的索引
media-organizer export backup.zip --disk JMedia_M05

# 排除会话历史（减小体积）
media-organizer export backup.zip --exclude sessions

# 自动命名（时间戳）
media-organizer export --auto-name
# 输出: media_organizer_backup_20260101_180000.zip

# 添加描述
media-organizer export backup.zip --description "迁移前备份"
```

### 4.2 import 命令

```bash
# 完整导入
media-organizer import backup.zip

# 预览导入内容（不实际执行）
media-organizer import backup.zip --dry-run

# 只导入索引
media-organizer import backup.zip --only indexes

# 只导入配置
media-organizer import backup.zip --only config

# 合并索引（不覆盖现有数据）
media-organizer import backup.zip --merge

# 强制覆盖（不询问确认）
media-organizer import backup.zip --force

# 导入前备份现有数据
media-organizer import backup.zip --backup-first
```

---

## 5. 输出示例

### 5.1 导出输出

```
$ media-organizer export backup.zip --description "迁移前备份"

[EXPORT] 正在收集数据...

导出内容:
  [x] 应用配置 (config.toml)
  [x] 中央索引 (1250 电影, 180 剧集)
  [x] 硬盘索引 (5 个硬盘)
  [x] 会话历史 (45 个会话)
  [ ] 敏感数据 (使用 --include-secrets 包含)

[EXPORT] 正在创建压缩包...

[OK] 导出成功!
  文件: /home/johnny/backup.zip
  大小: 14.5 MB
  内容: 1250 电影, 180 剧集, 5 硬盘, 45 会话

提示: 导入命令: media-organizer import backup.zip
```

### 5.2 导入预览

```
$ media-organizer import backup.zip --dry-run

[IMPORT] 分析备份文件...

备份信息:
  创建时间: 2026-01-01 18:00:00
  创建者: johnny@workstation-01
  描述: 迁移前备份
  应用版本: 0.1.0

将要导入:
  应用配置: config.toml
  中央索引: 1250 电影, 180 剧集
  硬盘索引: JMedia_M01, JMedia_M02, JMedia_M03, JMedia_M05
  会话历史: 45 个会话

冲突检测:
  [!] 中央索引已存在 (当前: 800 电影)
      - 使用 --merge 合并
      - 使用 --force 覆盖
  [!] 配置文件已存在
      - 将被覆盖

[DRY-RUN] 未执行任何操作。移除 --dry-run 以执行导入。
```

### 5.3 导入执行

```
$ media-organizer import backup.zip --merge --backup-first

[IMPORT] 正在备份现有配置...
[OK] 备份已保存: ~/.config/media_organizer.backup.20260101_190000/

[IMPORT] 正在导入...
  [1/4] 导入配置文件... OK
  [2/4] 合并中央索引... OK (新增 450 条目)
  [3/4] 导入硬盘索引... OK (5 个硬盘)
  [4/4] 导入会话历史... OK (45 个会话)

[OK] 导入成功!
  新增电影: 450
  新增剧集: 50
  新增硬盘: 2
  合并会话: 45

提示: 原配置已备份至 ~/.config/media_organizer.backup.20260101_190000/
```

---

## 6. 典型使用场景

### 6.1 迁移到新机器

```bash
# 旧机器
media-organizer export migration.zip --include-secrets --description "迁移到新机器"

# 复制 migration.zip 到新机器

# 新机器
media-organizer import migration.zip --force
media-organizer search -t "盗梦空间"  # 立即可用
```

### 6.2 定期备份

```bash
# 每周备份（不含敏感数据）
media-organizer export --auto-name --exclude sessions
# 输出: media_organizer_backup_20260101_180000.zip
```

### 6.3 同步多台电脑

```bash
# 机器 A: 导出索引
media-organizer export sync.zip --only indexes

# 机器 B: 合并导入
media-organizer import sync.zip --merge
```

---

## 7. 边缘情况

| 场景 | 处理方式 |
|------|----------|
| 版本不兼容 | 显示警告，建议使用对应版本 |
| 路径差异 | 询问是否自动调整路径 |
| 部分导入失败 | 继续导入其他内容，显示错误摘要 |
| 导入文件损坏 | 验证 ZIP 完整性，提示错误 |


