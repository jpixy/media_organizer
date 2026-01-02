# 05 - Config Export and Import

## 1. Overview

### Background

Users of `media_organizer` may need to:

1. **Migrate to new machine** - Transfer config and index data to new computer
2. **Backup config** - Regular backup of all settings and indexes
3. **Multi-machine sync** - Sync configuration across multiple computers
4. **Restore environment** - Quickly restore working environment after OS reinstall

### Solution

Provide `export` and `import` commands supporting:
- Export/import central index
- Export/import application config
- Export/import session history
- Selective export/import

---

## 2. Exportable Data

| Category | Path | Description | Size Estimate |
|----------|------|-------------|---------------|
| **Central Index** | `~/.config/media_organizer/central_index.json` | All disk movie/show indexes | 1-50 MB |
| **Disk Indexes** | `~/.config/media_organizer/disk_indexes/*.json` | Per-disk indexes | 0.1-5 MB each |
| **Session History** | `~/.config/media_organizer/sessions/` | Historical plan/rollback files | 1-100 MB |
| **App Config** | `~/.config/media_organizer/config.toml` | API keys, default settings | < 1 KB |

### Sensitive Data Handling

| Data Type | Handling |
|-----------|----------|
| TMDB API Key | **Not exported** by default, requires `--include-secrets` |
| File paths | Exported as-is, can be auto-adjusted on import |
| UUID | Preserved (for disk identification) |

---

## 3. Export Format

### Export Package Structure

```
media_organizer_backup_20260101_180000.zip
├── manifest.json           # Export manifest
├── config/
│   └── config.toml         # App config (optional)
├── indexes/
│   ├── central_index.json  # Central index
│   └── disk_indexes/
│       ├── JMedia_M01.json
│       └── JMedia_M05.json
└── sessions/               # Session history (optional)
    └── 20260101_120000_xxxx/
        ├── plan.json
        └── rollback.json
```

### manifest.json Structure

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

## 4. Command Reference

### 4.1 export Command

```bash
# Full export (without sensitive data)
media-organizer export backup.zip

# Full export (with secrets like API Key)
media-organizer export backup.zip --include-secrets

# Export only indexes
media-organizer export backup.zip --only indexes

# Export only config
media-organizer export backup.zip --only config

# Export specific disk only
media-organizer export backup.zip --disk JMedia_M05

# Exclude session history (reduce size)
media-organizer export backup.zip --exclude sessions

# Auto-name (timestamp)
media-organizer export --auto-name
# Output: media_organizer_backup_20260101_180000.zip

# Add description
media-organizer export backup.zip --description "Pre-migration backup"
```

### 4.2 import Command

```bash
# Full import
media-organizer import backup.zip

# Preview import (no actual execution)
media-organizer import backup.zip --dry-run

# Import only indexes
media-organizer import backup.zip --only indexes

# Import only config
media-organizer import backup.zip --only config

# Merge indexes (don't overwrite existing)
media-organizer import backup.zip --merge

# Force overwrite (no confirmation)
media-organizer import backup.zip --force

# Backup existing before import
media-organizer import backup.zip --backup-first
```

---

## 5. Output Examples

### 5.1 Export Output

```
$ media-organizer export backup.zip --description "Pre-migration backup"

[EXPORT] Collecting data...

Export contents:
  [x] App config (config.toml)
  [x] Central index (1250 movies, 180 shows)
  [x] Disk indexes (5 disks)
  [x] Session history (45 sessions)
  [ ] Sensitive data (use --include-secrets to include)

[EXPORT] Creating archive...

[OK] Export successful!
  File: /home/johnny/backup.zip
  Size: 14.5 MB
  Contents: 1250 movies, 180 shows, 5 disks, 45 sessions

Tip: Import command: media-organizer import backup.zip
```

### 5.2 Import Preview

```
$ media-organizer import backup.zip --dry-run

[IMPORT] Analyzing backup...

Backup info:
  Created: 2026-01-01 18:00:00
  Created by: johnny@workstation-01
  Description: Pre-migration backup
  App version: 0.1.0

Will import:
  App config: config.toml
  Central index: 1250 movies, 180 shows
  Disk indexes: JMedia_M01, JMedia_M02, JMedia_M03, JMedia_M05
  Session history: 45 sessions

Conflicts detected:
  [!] Central index exists (current: 800 movies)
      - Use --merge to merge
      - Use --force to overwrite
  [!] Config file exists
      - Will be overwritten

[DRY-RUN] No operations performed. Remove --dry-run to import.
```

### 5.3 Import Execution

```
$ media-organizer import backup.zip --merge --backup-first

[IMPORT] Backing up existing config...
[OK] Backup saved: ~/.config/media_organizer.backup.20260101_190000/

[IMPORT] Importing...
  [1/4] Importing config... OK
  [2/4] Merging central index... OK (added 450 entries)
  [3/4] Importing disk indexes... OK (5 disks)
  [4/4] Importing session history... OK (45 sessions)

[OK] Import successful!
  New movies: 450
  New shows: 50
  New disks: 2
  Merged sessions: 45

Tip: Original config backed up at ~/.config/media_organizer.backup.20260101_190000/
```

---

## 6. Typical Use Cases

### 6.1 Migrate to New Machine

```bash
# Old machine
media-organizer export migration.zip --include-secrets --description "Migration to new machine"

# Copy migration.zip to new machine

# New machine
media-organizer import migration.zip --force
media-organizer search -t "Inception"  # Works immediately
```

### 6.2 Regular Backup

```bash
# Weekly backup (without secrets)
media-organizer export --auto-name --exclude sessions
# Output: media_organizer_backup_20260101_180000.zip
```

### 6.3 Sync Multiple Machines

```bash
# Machine A: Export indexes
media-organizer export sync.zip --only indexes

# Machine B: Merge import
media-organizer import sync.zip --merge
```

---

## 7. Edge Cases

| Scenario | Handling |
|----------|----------|
| Version incompatible | Show warning, suggest using matching version |
| Path differences | Ask whether to auto-adjust paths |
| Partial import failure | Continue importing other content, show error summary |
| Corrupted import file | Verify ZIP integrity, show error |

