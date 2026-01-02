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

