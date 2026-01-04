# 01 - Project Overview

## 1. Introduction

**Media Organizer** is a Rust-based command-line tool for automatically organizing video files (movies and TV shows). It uses local AI (Ollama) to parse filenames, combines with TMDB API for metadata, renames and organizes files in a standardized format, and generates media library compatible NFO files and posters.

### Core Features

| Feature | Description |
|---------|-------------|
| **AI Filename Parsing** | Uses Ollama local AI to parse mixed Chinese/English video filenames |
| **TMDB Metadata Fetching** | Automatically matches TMDB database for detailed information |
| **Standardized Naming** | Renames files and directories in unified format |
| **NFO Generation** | Generates Kodi/Emby/Jellyfin compatible NFO files |
| **Poster Download** | Automatically downloads movie/show posters |
| **Central Index** | Offline searchable index across multiple hard drives |
| **Config Export/Import** | Supports backup and migration |

---

## 2. Tech Stack

| Component | Technology | Description |
|-----------|------------|-------------|
| Language | Rust | High performance, memory safe, single binary distribution |
| Async Runtime | tokio | Efficient I/O intensive operations |
| CLI Framework | clap | Command-line argument parsing |
| HTTP Client | reqwest | TMDB API and Ollama API calls |
| AI Inference | Ollama (qwen2.5:7b) | Locally running LLM |
| Metadata Extraction | ffprobe | Video technical information extraction |
| Target Platform | Linux Only | No Windows/macOS support |

---

## 3. Naming Format

### Movies

**Directory Format**:
```
{CountryCode}/{[OriginalTitle](Year)-ttIMDB-tmdbTMDB}/
```

**File Format**:
```
[OriginalTitle](Year)-ttIMDB-tmdbTMDB-Resolution-Format-Codec-BitDepth-AudioCodec-Channels.ext
```

**Example**:
```
US_UnitedStates/
└── [Avatar](2009)-tt0499549-tmdb19995/
    ├── [Avatar](2009)-tt0499549-tmdb19995-2160p-BluRay-x265-10bit-TrueHD-7.1.mkv
    ├── movie.nfo
    └── poster.jpg
```

### TV Shows

**Show Directory Format**:
```
{CountryCode}/[ShowName](Year)-ttIMDB-tmdbTMDB/
```

**Season Directory Format**:
```
Season {XX}/
```

**File Format**:
```
[ShowName]-S{XX}E{XX}-[EpisodeTitle]-Resolution-Format-Codec-BitDepth-AudioCodec-Channels.ext
```

---

## 4. Workflow

### Three-Step Model: Plan -> Execute -> Rollback

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│    Plan     │───>│   Execute   │───>│  Rollback   │
│             │    │             │    │  (optional) │
│ Generate    │    │ Execute file│    │ Restore     │
│ plan.json   │    │ operations  │    │ original    │
└─────────────┘    └─────────────┘    └─────────────┘
```

1. **Plan**: Scan directory -> AI parse -> TMDB match -> Generate `plan.json`
2. **Execute**: Read plan -> Execute operations -> Generate `rollback.json`
3. **Rollback**: Reverse execute -> Restore original state

---

## 5. Command Reference

### Basic Commands

```bash
# Generate plan (movies)
media-organizer plan movies /path/to/movies --target /path/to/organized

# Generate plan (TV shows)
media-organizer plan tvshows /path/to/tvshows --target /path/to/organized

# Execute plan
media-organizer execute plan.json

# Rollback operations
media-organizer rollback rollback.json
```

### Index and Search

```bash
# Index directory
media-organizer index /path/to/organized

# Search
media-organizer search --actor "Actor Name"
media-organizer search --collection "Series Name"
media-organizer search --title "Title"
```

### Config Export/Import

```bash
# Export
media-organizer export backup.zip

# Import
media-organizer import backup.zip
```

---

## 6. Core Principles

### 6.1 "Better to Miss Than Misprocess"

Skip processing when:
- AI confidence below threshold
- No precise TMDB match
- Country information uncertain

### 6.2 Idempotency

- Multiple runs on same directory should produce same result
- Already organized files are correctly recognized and quickly processed
- Never overwrites existing files

### 6.3 Rollbackable

- All operations are reversible
- Each execution generates rollback file
- Supports complete restoration to original state

---

## 7. Requirements

| Dependency | Requirement | Check Command |
|------------|-------------|---------------|
| Ollama | Running with loaded model | `curl http://localhost:11434/api/tags` |
| FFprobe | Installed | `ffprobe -version` |
| TMDB API | Valid API Key | Environment variable `TMDB_API_KEY` |

### Environment Variables

```bash
export TMDB_API_KEY="your_api_key"
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="qwen2.5:7b"
```

---

## 8. Config File Locations

```
~/.config/media_organizer/
├── config.toml              # Application config
├── central_index.json       # Central index
├── disk_indexes/            # Per-disk indexes
│   └── {disk_label}.json
└── sessions/                # Session history
    └── {timestamp}_{id}/
        ├── plan.json
        └── rollback.json
```

---

## 9. Documentation Index

| Document | Content |
|----------|---------|
| [02-architecture.md](02-architecture.md) | System Architecture Design |
| [03-processing-flow.md](03-processing-flow.md) | Core Processing Flow |
| [04-central-index.md](04-central-index.md) | Central Index System |
| [05-export-import.md](05-export-import.md) | Config Export/Import |
| [06-gpu-setup.md](06-gpu-setup.md) | GPU Setup Guide |


