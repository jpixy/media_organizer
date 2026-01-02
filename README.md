# Media Organizer

A smart media file organizer that uses AI to parse filenames and fetch metadata from TMDB, automatically renaming and organizing movie/TV show files.

## Features

- **AI-powered filename parsing** - Uses local Ollama + Qwen 2.5 model for intelligent movie/show recognition
- **TMDB metadata** - Auto-fetches movie details, posters, directors, actors, and collection info
- **Smart renaming** - Renames files and folders in standardized format
- **Safe operations** - Generate plan first, preview, then execute with full rollback support
- **GPU acceleration** - Supports NVIDIA GPU for accelerated AI inference
- **Central indexing** - Build searchable index across multiple disks
- **Cross-disk search** - Search by title, actor, director, collection, year, genre, country
- **Export/Import** - Backup and migrate your configuration and indexes
- **Detailed logging** - Complete operation logs and progress display

## System Requirements

- **OS**: Linux (Fedora/Ubuntu/Debian)
- **Rust**: 1.70+
- **Ollama**: 0.13+ (for AI inference)
- **ffprobe**: For extracting video technical info
- **TMDB API Key**: Register at [TMDB](https://www.themoviedb.org/)

### Optional
- **NVIDIA GPU**: Recommended for accelerated AI inference (requires CUDA driver)

## Quick Start

### 1. Install Dependencies

```bash
# Fedora
sudo dnf install ffmpeg

# Ubuntu/Debian
sudo apt install ffmpeg

# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Download AI model
ollama pull qwen2.5:7b
```

### 2. Configure Environment Variables

```bash
export TMDB_API_KEY="your_tmdb_api_key"
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="qwen2.5:7b"
```

### 3. Build and Run

```bash
cd media_organizer
cargo build --release

# View help
./target/release/media-organizer --help
```

### 4. Organize Movies

```bash
# Step 1: Generate organization plan
./target/release/media-organizer plan movies /path/to/movies --target /path/to/organized

# Step 2: Review the plan
cat plan_*.json

# Step 3: Execute the plan
./target/release/media-organizer execute plan_*.json

# Rollback if needed
./target/release/media-organizer rollback rollback_*.json
```

## Commands

### plan - Generate Organization Plan

```bash
media-organizer plan movies <SOURCE> [OPTIONS]
media-organizer plan tvshows <SOURCE> [OPTIONS]

Options:
  -t, --target <TARGET>  Target directory
  -v, --verbose          Verbose output
  -o, --output <OUTPUT>  Plan file output path
      --skip-preflight   Skip preflight checks
```

### execute - Execute Plan

```bash
media-organizer execute <PLAN_FILE> [OPTIONS]

Options:
  -o, --output <OUTPUT>  Rollback file output path
```

### rollback - Rollback Operations

```bash
media-organizer rollback <ROLLBACK_FILE> [OPTIONS]

Options:
  --dry-run  Dry run, show what would be done
```

### index - Build Central Index

Build a searchable index from organized media directories:

```bash
# Scan and index a directory
media-organizer index scan /path/to/movies --media-type movies

# Scan TV shows
media-organizer index scan /path/to/tvshows --media-type tvshows

# Custom disk label
media-organizer index scan /mnt/disk1/movies --disk-label MyDisk1

# Show statistics
media-organizer index stats

# List contents of a disk
media-organizer index list JMedia_M05

# Verify index against files
media-organizer index verify /path/to/movies

# Remove a disk from index
media-organizer index remove OldDisk --confirm

# Find duplicate media by TMDB ID across disks
media-organizer index duplicates

# Find only duplicate movies
media-organizer index duplicates --media-type movies

# Find only duplicate TV shows
media-organizer index duplicates --media-type tvshows

# Output as JSON
media-organizer index duplicates --format json
```

### search - Search Media Collection

Search across all indexed disks:

```bash
# Search by title
media-organizer search -t "Inception"

# Search by actor
media-organizer search -a "Leonardo DiCaprio"

# Search by director
media-organizer search -d "Christopher Nolan"

# Search by collection/series
media-organizer search -c "Pirates of the Caribbean"

# Search by year or year range
media-organizer search -y 2024
media-organizer search -y 2020-2024

# Search by genre
media-organizer search -g "Action"

# Search by country
media-organizer search --country US

# Show disk online/offline status
media-organizer search -t "Avatar" --show-status

# Output as JSON
media-organizer search -t "Avatar" --format json

# Combine filters
media-organizer search -a "Tom Hanks" -y 2000-2020 --country US
```

### export - Export Configuration

Backup your configuration and indexes:

```bash
# Full export with auto-generated filename
media-organizer export --auto-name

# Export to specific file
media-organizer export backup.zip

# Include sensitive data (API keys)
media-organizer export backup.zip --include-secrets

# Only export indexes
media-organizer export backup.zip --only indexes

# Only export specific disk
media-organizer export backup.zip --disk JMedia_M05

# Add description
media-organizer export backup.zip --description "Pre-migration backup"

# Exclude sessions (reduce size)
media-organizer export backup.zip --exclude sessions
```

### import - Import Configuration

Restore configuration and indexes from backup:

```bash
# Preview what will be imported
media-organizer import backup.zip --dry-run

# Full import
media-organizer import backup.zip --force

# Merge with existing data
media-organizer import backup.zip --merge

# Backup existing config first
media-organizer import backup.zip --backup-first --force

# Only import indexes
media-organizer import backup.zip --only indexes
```

### sessions - Manage Sessions

```bash
media-organizer sessions list    # List all sessions
media-organizer sessions show <ID>  # Show session details
```

### verify - Verify Configuration

```bash
media-organizer verify <PATH>    # Verify video files
```

## Output Format

### Movie Folder Structure

```
Movies_organized/
└── CN_China/
    └── [Movie Name](Year)-ttIMDB_ID-tmdbTMDB_ID/
        ├── [Movie Name](Year)-Resolution-Format-Codec-BitDepth-Audio-Channels.mp4
        ├── movie.nfo
        └── poster.jpg
```

### TV Show Folder Structure

```
TV_Shows_organized/
└── US_UnitedStates/
    └── [Show Name](Year)-ttIMDB_ID-tmdbTMDB_ID/
        ├── Season 01/
        │   ├── [Show Name]-S01E01-Episode Name-1080p-WEB-DL.mp4
        │   └── ...
        ├── tvshow.nfo
        └── poster.jpg
```

### Example

```
CN_China/
└── [刺杀小说家2](2025)-tt33095008-tmdb945801/
    ├── [刺杀小说家2](2025)-2160p-BluRay-hevc-8bit-dts-5.1.mp4
    ├── movie.nfo
    └── poster.jpg
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TMDB_API_KEY` | TMDB API key | (required) |
| `TMDB_BEARER_TOKEN` | TMDB Bearer token (v4) | (optional) |
| `OLLAMA_BASE_URL` | Ollama service URL | `http://localhost:11434` |
| `OLLAMA_MODEL` | AI model name | `qwen2.5:7b` |
| `RUST_LOG` | Log level | `info` |

### TMDB API Key

1. Register at [TMDB](https://www.themoviedb.org/signup)
2. Go to [API Settings](https://www.themoviedb.org/settings/api)
3. Apply for API Key (v3 auth)
4. Set environment variable: `export TMDB_API_KEY="your_key"`

## GPU Configuration

If you have an NVIDIA GPU, enable GPU acceleration for faster AI inference:

See [GPU Setup Guide](docs/en/06-gpu-setup.md)

### Quick Check

```bash
# Check GPU
nvidia-smi

# Check Ollama GPU status
ollama serve 2>&1 | grep -i "inference compute"
# Should show: library=CUDA
```

## Performance

| Mode | AI Parse Time (per file) |
|------|--------------------------|
| CPU | 30-60 seconds |
| GPU (RTX 3500) | 1-2 seconds |

## Troubleshooting

### AI Parse Timeout
- Check if Ollama is running: `pgrep ollama`
- Check if GPU is enabled: Look for `library=CUDA` in Ollama logs

### TMDB API Error
- Check if API Key is correct
- Check network connection (may need proxy in some regions)

### Video Info Extraction Failed
- Ensure ffprobe is installed: `which ffprobe`

## Documentation

### English
- [Overview](docs/en/01-overview.md)
- [Architecture](docs/en/02-architecture.md)
- [Processing Flow](docs/en/03-processing-flow.md)
- [Central Index](docs/en/04-central-index.md)
- [Export/Import](docs/en/05-export-import.md)
- [GPU Setup](docs/en/06-gpu-setup.md)

### Chinese (中文)
- [Overview (概述)](docs/zh/01-overview.md)
- [Architecture (架构设计)](docs/zh/02-architecture.md)
- [Processing Flow (处理流程)](docs/zh/03-processing-flow.md)
- [Central Index (中央索引)](docs/zh/04-central-index.md)
- [Export/Import (导入导出)](docs/zh/05-export-import.md)
- [GPU Setup (GPU配置)](docs/zh/06-gpu-setup.md)

## License

MIT License

## Contributing

Issues and Pull Requests are welcome!
