# Quick Start Guide

Get started with Media Organizer in 5 minutes.

## Prerequisites

- Linux system (Fedora/Ubuntu/Debian)
- Rust development environment
- Network proxy (if in mainland China, TMDB requires proxy access)

## Step 1: Install Dependencies (2 minutes)

```bash
# 1. Install ffmpeg (includes ffprobe)
# Fedora:
sudo dnf install ffmpeg

# Ubuntu/Debian:
sudo apt install ffmpeg

# 2. Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# 3. Start Ollama and download model
ollama serve &
ollama pull qwen2.5:7b
```

## Step 2: Get TMDB API Key (1 minute)

1. Visit https://www.themoviedb.org/signup to register
2. After login, visit https://www.themoviedb.org/settings/api
3. Click "Create" to create API Key
4. Copy "API Key (v3 auth)"

## Step 3: Build Project (1 minute)

```bash
cd media_organizer
cargo build --release
```

## Step 4: Organize Your Movies (1 minute)

```bash
# Set environment variables
export TMDB_API_KEY="your_api_key"

# Generate organization plan
./target/release/media-organizer plan movies ~/Videos/Movies --target ~/Videos/Movies_Organized -v

# Review the plan
cat plan_*.json

# Execute the plan (move files, download posters, generate NFO)
./target/release/media-organizer execute plan_*.json
```

## Complete Example

```bash
# Assuming your movies are in ~/Downloads/Movies
# And you want to organize them to ~/Videos/Movies

# 1. Set environment variables
export TMDB_API_KEY="your_api_key_here"
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="qwen2.5:7b"

# 2. Ensure Ollama is running
pgrep ollama || ollama serve &

# 3. Generate plan
./target/release/media-organizer plan movies \
  ~/Downloads/Movies \
  --target ~/Videos/Movies \
  --verbose

# 4. Review plan
less plan_*.json

# 5. Execute plan
./target/release/media-organizer execute plan_*.json

# 6. View results
ls -la ~/Videos/Movies/
```

## Command Quick Reference

| Action | Command |
|--------|---------|
| Organize movies | `media-organizer plan movies <source> -t <target>` |
| Organize TV shows | `media-organizer plan tvshows <source> -t <target>` |
| Execute plan | `media-organizer execute <plan.json>` |
| Rollback | `media-organizer rollback <rollback.json>` |
| View sessions | `media-organizer sessions list` |
| Index stats | `media-organizer index stats` |
| Find duplicates | `media-organizer index duplicates` |
| Search | `media-organizer search -t <title>` |
| Verify files | `media-organizer verify <path>` |
| Help | `media-organizer --help` |

## Indexing & Search

After organizing, build a searchable index:

```bash
# Index your organized movies
media-organizer index scan ~/Videos/Movies_organized --media-type movies

# Index TV shows
media-organizer index scan ~/Videos/TV_Shows_organized --media-type tvshows

# View statistics
media-organizer index stats

# Search by title
media-organizer search -t "Avatar"

# Search by actor
media-organizer search -a "Tom Hanks"

# Search by country with disk status
media-organizer search --country CN --show-status

# Search by year range
media-organizer search -y 2020-2024

# Find duplicates across disks
media-organizer index duplicates
```

## Export & Import

Backup your configuration for migration:

```bash
# Export everything (auto-named with timestamp)
media-organizer export --auto-name

# Export with description
media-organizer export backup.zip --description "Before migration"

# Preview import
media-organizer import backup.zip --dry-run

# Import and merge with existing data
media-organizer import backup.zip --merge --force
```

## Output Example

After execution, your movies will be organized as:

```
Movies_organized/
├── CN_China/
│   └── [刺杀小说家2](2025)-tt33095008-tmdb945801/
│       ├── [刺杀小说家2](2025)-2160p-BluRay-hevc-8bit-dts-5.1.mp4
│       ├── movie.nfo
│       └── poster.jpg
├── US_UnitedStates/
│   └── [Inception](2010)-tt1375666-tmdb27205/
│       ├── [Inception](2010)-1080p-BluRay-h264-8bit-dts-5.1.mp4
│       ├── movie.nfo
│       └── poster.jpg
└── ...
```

## Search Example

```bash
$ media-organizer index stats

Media Collection Statistics
==================================================

Disks:
  JMedia_M05 | 39 movies | 0 TV shows | 122.1 GB | Online
  JMedia_03  | 120 movies | 45 TV shows | 450.0 GB | Offline
--------------------------------------------------
  Total | 159 movies | 45 TV shows | 572.1 GB

By Country:
  CN ████████████████████ 65 (41%)
  US      ██████████ 35 (22%)
  KR           █████ 20 (13%)
  ...

$ media-organizer search -t "Avatar" --show-status

Found 2 results:

Movies (2):
    # | Year | Title            | Disk       | Status
------------------------------------------------------
    1 | 2022 | Avatar: TWOW     | JMedia_M05 | Online
    2 | 2009 | Avatar           | JMedia_03  | Offline
```

## FAQ

### Q: AI parsing is slow?
A: Check if GPU is enabled. Run `nvidia-smi` to confirm GPU is available, see [GPU Setup Guide](docs/zh/04-ollama-gpu-setup.md).

### Q: TMDB connection timeout?
A: In mainland China, you need a proxy to access TMDB. Ensure your proxy is enabled.

### Q: Movie info not found?
A: The filename may be too complex or the movie doesn't exist in TMDB. Check the `unknown` section in `plan.json`.

### Q: How to rollback?
A: Run `media-organizer rollback <rollback.json>` using the rollback file generated during execution.

### Q: How to search across multiple disks?
A: Index each disk with `media-organizer index scan`, then use `media-organizer search` to find media even on offline disks.

### Q: How to migrate to a new machine?
A: Use `media-organizer export` on the old machine, copy the backup file, then `media-organizer import` on the new machine.

## Next Steps

- See [README.md](README.md) for full features
- See [GPU Setup Guide](docs/zh/04-ollama-gpu-setup.md) to accelerate AI inference
- See [Central Index Design](docs/zh/06-central-index-design.md) for multi-disk management
- See [Export/Import Design](docs/zh/07-config-export-import-design.md) for backup/migration
