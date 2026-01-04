# Quick Start Guide

## Prerequisites

- Linux system
- 8GB+ RAM
- [Ollama](https://ollama.ai) running with `qwen2.5:7b` model
- [TMDB API Key](https://www.themoviedb.org/settings/api)
- ffprobe (installed with ffmpeg)

## 1. Environment Setup

```bash
# Set environment variables
export TMDB_API_KEY="your_tmdb_api_key"
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="qwen2.5:7b"
```

## 2. Installation

```bash
# Build from source
git clone https://github.com/jpixy/media_organizer.git
cd media_organizer
cargo build --release

# Binary is at target/release/media-organizer
```

## 3. Quick Test

```bash
# Check prerequisites
media-organizer plan movies /path/to/source --dry-run

# Should show:
# [OK] ffprobe: installed
# [OK] Ollama: running
# [OK] TMDB API: connected
```

## 4. Basic Workflow

### Organize Movies

```bash
# 1. Generate plan (no files moved)
media-organizer plan movies /source -t /target

# 2. Review plan
cat /target/plan_*.json | jq '.items | length'

# 3. Execute plan
media-organizer execute /target/plan_*.json

# 4. Rollback if needed
media-organizer rollback /target/rollback_*.json
```

### Organize TV Shows

```bash
media-organizer plan tvshows /source -t /target
media-organizer execute /target/plan_*.json
```

## 5. Build Index

```bash
# Index movies
media-organizer index scan /target --media-type movies --disk-label MyDisk

# Index TV shows
media-organizer index scan /target --media-type tvshows --disk-label MyDisk

# View statistics
media-organizer index stats
```

## 6. Search

```bash
# Search by title (searches both movies and TV shows)
media-organizer search --title "Inception"

# Search by actor
media-organizer search --actor "Johnny Depp"

# Search by year range
media-organizer search --year 2020-2024

# Show online/offline status
media-organizer search --title "Avatar" --show-status
```

## Command Reference

| Command | Description |
|---------|-------------|
| `plan movies <src> -t <dst>` | Generate movie organization plan |
| `plan tvshows <src> -t <dst>` | Generate TV show organization plan |
| `execute <plan.json>` | Execute plan |
| `rollback <rollback.json>` | Rollback operations |
| `index scan <path>` | Scan directory to build index |
| `index stats` | Show collection statistics |
| `search --title <title>` | Search by title |
| `search --actor <actor>` | Search by actor |
| `export` | Export config and indexes |
| `import <backup>` | Import config and indexes |

## Next Steps

- Read the [User Manual](USER-MANUAL.md)
- Learn about [Central Index System](04-central-index.md)
- Learn about [Export/Import](05-export-import.md)

