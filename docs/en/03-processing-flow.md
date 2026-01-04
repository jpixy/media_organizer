# 03 - Core Processing Flow

## 1. Overview

This document describes the core processing flow of Media Organizer, including metadata extraction, TMDB matching, and file processing logic.

---

## 2. File Processing Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                         File Processing Entry                        │
│                      process_video(file)                            │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Phase 1: File Type Detection                      │
│                                                                     │
│  ┌─────────────────┐                                                │
│  │ Is Organized    │──Yes──> Extract TMDB ID ──> Phase 4 (Direct)   │
│  │ Format?         │                                                │
│  │ [Title]-S01E01  │                                                │
│  │ [Title](Year)-  │                                                │
│  └────────┬────────┘                                                │
│           │ No                                                      │
│           ▼                                                         │
│  ┌─────────────────┐                                                │
│  │ In Organized    │──Yes──> Extract TMDB ID from Dir ──> Phase 4   │
│  │ Directory?      │                                                │
│  │ (ancestor has   │                                                │
│  │  tmdb marker)   │                                                │
│  └────────┬────────┘                                                │
│           │ No                                                      │
│           ▼                                                         │
│       New File ──────────────> Phase 2 (Full Parsing Flow)          │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Phase 2: Information Collection (New Files Only)  │
│                                                                     │
│  2.1 Extract from Filename                                          │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  extract_from_filename(filename) -> FilenameInfo            │   │
│  │    ├── Title (Chinese/English)                              │   │
│  │    ├── Year                                                  │   │
│  │    ├── Season/Episode (TV Shows)                            │   │
│  │    └── Technical info (resolution, codec, etc.)             │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  2.2 Directory Type Classification                                  │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  classify_directory(dir_name) -> DirectoryType              │   │
│  │    ├── TitleDirectory  (work directory: contains title)     │   │
│  │    ├── SeasonDirectory (Season 01, etc.)                    │   │
│  │    ├── QualityDirectory (4K, 1080P, etc.)                   │   │
│  │    ├── CategoryDirectory (actor/series/year/region)         │   │
│  │    └── Unknown                                               │   │
│  │                                                              │   │
│  │  Traverse upward, find first TitleDirectory                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  2.3 Merge Information                                              │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  merge_info(filename_info, dir_info) -> CandidateMetadata   │   │
│  │    Priority: Filename info > Directory info (more specific) │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Phase 3: AI Augmentation (On Demand)              │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  if candidate.needs_ai_parsing():                           │   │
│  │      # AI needed when:                                       │   │
│  │      # - No Chinese title AND no English title               │   │
│  │      # - Title looks like codec/technical info               │   │
│  │                                                              │   │
│  │      ai_result = parser.parse(filename + context)           │   │
│  │      candidate.merge_ai_result(ai_result)                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Phase 4: TMDB Matching                            │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  4A: If TMDB ID exists (from organized format)              │   │
│  │      ├── Directly call get_movie_details(tmdb_id) or        │   │
│  │      │                get_tv_details(tmdb_id)               │   │
│  │      └── Fast path, no search needed                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  4B: If no TMDB ID (new file)                                │   │
│  │      ├── Search using candidate metadata                     │   │
│  │      ├── Priority:                                           │   │
│  │      │   1. Chinese + English title + Year (most precise)   │   │
│  │      │   2. English title + Year                             │   │
│  │      │   3. Chinese title + Year                             │   │
│  │      │   4. Title only (no year)                             │   │
│  │      └── Validate match quality, skip if uncertain          │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Phase 5: Validation & Decision                    │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  validate_match(tmdb_result, candidate) -> MatchQuality     │   │
│  │                                                              │   │
│  │  MatchQuality:                                               │   │
│  │    ├── Exact     (title+year match perfectly)               │   │
│  │    ├── High      (title similar, year matches)              │   │
│  │    ├── Medium    (title similar or year close)              │   │
│  │    ├── Low       (partial match only)                       │   │
│  │    └── NoMatch                                               │   │
│  │                                                              │   │
│  │  Decision rules:                                             │   │
│  │    ├── Exact/High  -> Process                                │   │
│  │    ├── Medium      -> Based on config (default: skip)       │   │
│  │    └── Low/NoMatch -> Skip (better to miss than misprocess) │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Directory Type Classification

### 3.1 Directory Types

| Type | Description | Example |
|------|-------------|---------|
| **TitleDirectory** | Work directory with title | `Guillermo del Toro (2022)` |
| **SeasonDirectory** | Season directory | `Season 01`, `S01` |
| **QualityDirectory** | Quality/technical | `4K`, `1080P`, `BluRay` |
| **CategoryDirectory** | Category directory | `Marvel`, `2024`, `Korean Drama` |
| **OrganizedDirectory** | Already organized | `[Title](2025)-tt12345-tmdb67890` |

---

## 4. Organized Format Recognition

### 4.1 Organized Filename Formats

**Movies**:
```
[Title](Year)-ttIMDB-tmdbTMDB-resolution-format-codec-bitdepth-audio-channels.ext
```

Regex:
```regex
^\[.+\]\(\d{4}\)-tt\d+-tmdb\d+-.+\.(mp4|mkv|avi)$
```

**TV Shows**:
```
[ShowTitle]-SXXEXX-[EpisodeTitle]-resolution-format-codec-bitdepth-audio-channels.ext
```

Regex:
```regex
^\[.+\]-S\d{1,2}E\d{1,3}-\[.+\]-.+\.(mp4|mkv|avi)$
```

---

## 5. TV Show Special Processing

### 5.1 Season Caching

For multiple files in the same directory, use caching to reduce API calls:

```
Directory: [Show](2025)-tmdb296146/Season 01/
  │
  ├── E01.mp4 ──> Query TMDB + Get all season episodes ──> Cache
  ├── E02.mp4 ──> Use cache
  ├── E03.mp4 ──> Use cache
  └── E04.mp4 ──> Use cache
```

### 5.2 Episode Number Extraction

Use regex extraction first, AI parsing as fallback:

```rust
// Regex patterns
S01E01, S1E1
E01, E1, EP01
Episode 01, Episode 1
01.mp4, 1.mp4 (pure numbers)
```

---

## 6. TMDB Matching Strategy

### 6.1 Search Priority

1. **Chinese+English Intersection Match** (most reliable)
   - Search both Chinese and English titles
   - Get intersection of results
   - Select best match from intersection

2. **English Title Match**
   - Search with English title only
   - Select best match

3. **Chinese Title Match**
   - Search with Chinese title only
   - Select best match

### 6.2 Match Validation

Validation criteria:
- Title similarity > 0.7
- Year matches (allow +/- 1 year)
- Country info consistent (if available)

---

## 7. Incremental Processing

### 7.1 Scenario: New Episodes Added

```
Organized directory:
[Show](2025)-tt36771056-tmdb296146/
├── Season 01/
│   ├── [Show]-S01E01-[Ep1]-1080p.mp4   (already organized)
│   ├── [Show]-S01E02-[Ep2]-1080p.mp4   (already organized)
│   └── Show.E03.1080p.mp4               (new file)
```

Processing flow:
1. Recognize new file `Show.E03.1080p.mp4`
2. Detect it's in organized directory
3. Extract TMDB ID (296146) from directory name
4. Extract episode number (E03) with regex
5. Directly query TMDB for episode 3 details
6. Generate target filename

### 7.2 Scenario: Re-organize

```
Organized file:
[Avatar](2009)-tt0499549-tmdb19995-1080p.mp4
```

Processing flow:
1. Recognize filename as organized format
2. Extract TMDB ID (19995)
3. Directly query TMDB for latest details
4. Use new details to generate target (may update NFO)

---

## 8. Error Handling and Logging

### 8.1 Error Classification

| Error Type | Handling | Log Level |
|------------|----------|-----------|
| AI parsing failed | Skip, mark as unknown | WARN |
| TMDB no match | Skip, mark as unknown | WARN |
| TMDB uncertain match | Skip, mark as unknown | INFO |
| ffprobe failed | Use filename metadata | WARN |
| Network error | Retry or abort | ERROR |

### 8.2 Log Examples

```
[SCAN] Found 276 video files in /path/to/source
[AI] Parsing: movie.mp4 (CPU inference may take 1-3 min)
[TMDB] Found match: Avatar (2009) - tmdb19995
[ORGANIZED] Re-indexing: [Show]-S01E01-... (using tmdb296146)
[WARN] Skipped: unknown.mp4 - No TMDB match
[OK] Plan generated: 268 items, 8 unknown
```

---

## 9. Performance Optimizations

### 9.1 Parallel Processing

| Operation | Parallelization |
|-----------|-----------------|
| ffprobe calls | Parallel with `futures::join_all` |
| Poster downloads | Parallel with `futures::join_all` |
| TMDB API | Sequential (API rate limiting) |

### 9.2 Caching Strategy

| Cache Type | Scope | Content |
|------------|-------|---------|
| TV Show cache | Same directory | Show metadata |
| Season cache | Same show/season | All episode details |
| TMDB search cache | Optional | Search results (not implemented) |

### 9.3 Organized File Fast Path

For already-organized files, the system:

1. **Skips AI parsing** - Extracts TMDB ID directly from filename/folder
2. **Uses cache** - Multiple files from same show share metadata cache
3. **Direct TMDB query** - Uses ID to get details, no search needed

**Performance improvement**:
- 40 organized files: ~90s -> ~3s (approximately 30x faster)
- Avoids redundant AI calls and TMDB searches

### 9.4 TMDB ID Extraction from Parent

When filename doesn't contain TMDB ID, system extracts from parent folder:

```
Movies_organized/EN_English/[Avatar](2009)-tt0499549-tmdb19995/
  |-- [Avatar](2009)-1080p-WEB-DL.mp4  <- Gets tmdb19995 from parent
```

### 9.5 Parent Directory ID Fallback for TV Shows

When a season directory's IMDB ID is not recognized by TMDB (e.g., season-specific IDs), 
the system automatically searches parent directories for the show's main ID:

```
L_Slow Horses.tt5875444/           <- Main show IMDB ID
  |-- S02.tt13660696/              <- Season-specific ID (not recognized by TMDB)
  |     |-- episode.mp4            <- Will use tt5875444 from parent
  |-- S03.tt20778346/
        |-- episode.mp4            <- Will use tt5875444 from parent
```

**Implementation**:
- `try_parent_directory_id_lookup()` in `planner.rs`
- `extract_ids_from_path_starting_at()` in `metadata.rs`

### 9.6 CJK Parent Directory Context

When parent directory contains CJK characters (Chinese/Japanese/Korean) but filename 
uses romanized/Latin characters, the parent dir name is added to AI parsing context:

```
逃避虽可耻但有用/                           <- Parent has CJK title
  |-- NIGEHAJI.E01.720p.FIX字幕侠/         <- Per-episode folder (skipped)
        |-- [V2]NIGEHAJI.E01.720p.mkv      <- Romanized filename

AI input becomes: "逃避虽可耻但有用 - [V2]NIGEHAJI.E01.720p.mkv"
```

This fixes cases like:
- **NIGEHAJI** (逃げ恥) - Japanese romanized abbreviation
- Other shows using romanized names in filenames but proper CJK titles in directories

**Detection logic**:
1. Check if parent directory contains CJK characters
2. Check if filename has few CJK chars but many Latin chars
3. If both conditions met, include parent dir name in AI context

---

## 10. Implementation Status

### 10.1 Unified Metadata Extraction (Implemented)

`CandidateMetadata` struct in `src/core/metadata.rs`:

```rust
pub struct CandidateMetadata {
    pub chinese_title: Option<String>,
    pub english_title: Option<String>,
    pub year: Option<u16>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub tmdb_id: Option<u64>,
    pub imdb_id: Option<String>,
    pub source: Option<MetadataSource>,
    pub confidence: f32,
    // ... other fields
}
```

### 10.2 Directory Type Classification (Implemented)

Smart directory classification in `src/core/metadata.rs`:

```rust
pub enum DirectoryType {
    TitleDirectory(TitleInfo),
    SeasonDirectory(u32),
    QualityDirectory,
    CategoryDirectory,
    OrganizedDirectory(OrganizedInfo),
    Unknown,
}

