# 02 - System Architecture

## 1. Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                  User                                        │
│                                   │                                          │
│                                   ▼                                          │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                              CLI Entry                                 │  │
│  │                                                                        │  │
│  │   plan movies/tvshows <source> [--target]                             │  │
│  │   execute <plan.json>                                                  │  │
│  │   rollback <rollback.json>                                            │  │
│  │   index <path> | search <query> | export/import                       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                   │                                          │
│                                   ▼                                          │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                          Preflight Checker                             │  │
│  │                                                                        │  │
│  │   [x] ffprobe available  [x] Ollama running  [x] TMDB API available   │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              Core Processing                                 │
│                                                                              │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│   │ Scanner  │───>│ Metadata │───>│  TMDB    │───>│ Planner  │             │
│   │          │    │ Extractor│    │  Client  │    │          │             │
│   │ Dir Scan │    │ Info     │    │ Metadata │    │ Plan Gen │             │
│   └──────────┘    └────┬─────┘    └──────────┘    └────┬─────┘             │
│                        │                               │                    │
│                        │ (on demand)                   │                    │
│                        ▼                               ▼                    │
│                   ┌──────────┐                   ┌──────────┐              │
│                   │  Ollama  │                   │plan.json │              │
│                   │  AI Parse│                   │          │              │
│                   └──────────┘                   └────┬─────┘              │
│                                                       │                     │
│                                                       ▼                     │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│   │ rollback │<───│   NFO    │<───│  File    │<───│ Executor │             │
│   │   .json  │    │ Generator│    │  Mover   │    │          │             │
│   │          │    │          │    │          │    │ Execute  │             │
│   └──────────┘    └──────────┘    └──────────┘    └──────────┘             │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                             External Services                                │
│                                                                              │
│   ┌────────────────────┐    ┌────────────────────┐    ┌─────────────────┐  │
│   │   Ollama Service   │    │     TMDB API       │    │    ffprobe      │  │
│   │                    │    │                    │    │                 │  │
│   │   localhost:11434  │    │ api.themoviedb.org │    │  Video metadata │  │
│   │   qwen2.5:7b       │    │                    │    │                 │  │
│   └────────────────────┘    └────────────────────┘    └─────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Module Structure

```
media_organizer/
├── src/
│   ├── main.rs                 # Program entry
│   ├── lib.rs                  # Library entry
│   │
│   ├── cli/                    # Command-line interface
│   │   ├── mod.rs
│   │   ├── args.rs             # clap argument definitions
│   │   └── commands/           # Subcommand implementations
│   │       ├── plan.rs
│   │       ├── execute.rs
│   │       ├── rollback.rs
│   │       ├── index.rs
│   │       ├── search.rs
│   │       └── export_import.rs
│   │
│   ├── core/                   # Core business logic
│   │   ├── mod.rs
│   │   ├── scanner.rs          # Directory scanning
│   │   ├── parser.rs           # AI filename parsing
│   │   ├── metadata.rs         # Metadata extraction (pending)
│   │   ├── planner.rs          # Plan generation
│   │   ├── executor.rs         # Plan execution
│   │   ├── rollback.rs         # Rollback handling
│   │   └── indexer.rs          # Central indexing
│   │
│   ├── services/               # External service clients
│   │   ├── mod.rs
│   │   ├── ollama.rs           # Ollama API client
│   │   ├── tmdb.rs             # TMDB API client
│   │   └── ffprobe.rs          # ffprobe wrapper
│   │
│   ├── models/                 # Data models
│   │   ├── mod.rs
│   │   ├── media.rs            # Media info models
│   │   ├── plan.rs             # Plan file model
│   │   ├── rollback.rs         # Rollback file model
│   │   ├── index.rs            # Index model
│   │   └── config.rs           # Config model
│   │
│   ├── generators/             # Generators
│   │   ├── mod.rs
│   │   ├── nfo.rs              # NFO file generation
│   │   ├── filename.rs         # Filename generation
│   │   └── folder.rs           # Folder name generation
│   │
│   └── preflight/              # Preflight checks
│       ├── mod.rs
│       ├── ffprobe.rs
│       ├── ollama.rs
│       └── tmdb.rs
│
├── docs/                       # Documentation
│   ├── zh/                     # Chinese docs
│   └── en/                     # English docs
│
└── tests/                      # Tests
```

---

## 3. Data Flow

### 3.1 Plan Phase

```
Source Directory
  │
  ▼
┌────────────────────────────────────────────────────────────────────┐
│ Scanner: Recursively scan directory                                │
│                                                                     │
│ Input: Directory path                                               │
│ Output: Video file list (path, size, parent dir)                   │
│ Filter: .mkv, .mp4, .avi, .mov, .wmv, .m4v, .ts, .flv, .webm, .rmvb│
└────────────────────────────────────────────────────────────────────┘
  │
  ▼
┌────────────────────────────────────────────────────────────────────┐
│ Metadata Extractor: Extract candidate metadata                      │
│                                                                     │
│ 1. Check if filename is organized format -> Extract TMDB ID        │
│ 2. Check if ancestor dir is organized -> Extract TMDB ID           │
│ 3. Extract from filename/dirname: title, year, season/episode      │
│ 4. Call AI parsing if needed                                       │
└────────────────────────────────────────────────────────────────────┘
  │
  ▼
┌────────────────────────────────────────────────────────────────────┐
│ TMDB Client: Query metadata                                         │
│                                                                     │
│ Fast path: Has TMDB ID -> Direct detail query                      │
│ Search path: No TMDB ID -> Search match -> Verify -> Get details   │
│                                                                     │
│ Search priority:                                                    │
│   1. Chinese + English title + Year (intersection match)           │
│   2. English title + Year                                           │
│   3. Chinese title + Year                                           │
└────────────────────────────────────────────────────────────────────┘
  │
  ▼
┌────────────────────────────────────────────────────────────────────┐
│ ffprobe: Extract video metadata                                     │
│                                                                     │
│ Input: Video file path                                              │
│ Output: Resolution, codec, bit depth, audio codec, channels        │
│ Concurrent: Multi-file parallel processing                         │
└────────────────────────────────────────────────────────────────────┘
  │
  ▼
┌────────────────────────────────────────────────────────────────────┐
│ Planner: Generate execution plan                                    │
│                                                                     │
│ - Generate target folder and filename                               │
│ - Plan operation list (mkdir, move, create, download)              │
│ - Safety check: Detect duplicate target paths                       │
│ - Output plan.json                                                  │
└────────────────────────────────────────────────────────────────────┘
  │
  ▼
plan.json
```

---

## 4. Concurrency and Performance

| Operation | Concurrency Strategy |
|-----------|---------------------|
| ffprobe calls | Parallel using `futures::join_all` |
| Poster downloads | Parallel using `futures::join_all` |
| TMDB API | Sequential (API rate limited) |
| AI parsing | First file per directory, reuse for rest |
| TV Show season cache | Query once per season, cache all episodes |

---

## 5. Error Handling Strategy

| Scenario | Handling |
|----------|----------|
| AI parsing failed | Mark as unknown, skip |
| TMDB no match | Mark as unknown, skip |
| TMDB uncertain match | Skip (better to miss) |
| ffprobe failed | Use filename-parsed metadata |
| File operation failed | Abort, preserve rollback record |
| Target path conflict | Detect at Plan phase, error |


