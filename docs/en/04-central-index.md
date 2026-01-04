# 04 - Central Index System

## 1. Overview

### Problem Description

Users with large media collections often store movies across multiple external hard drives due to capacity limits. Since only one drive can be mounted at a time, it's difficult to:

1. Search movies by actor, director, or series across all drives
2. Know which drive a specific movie is on
3. Identify which movies belong to the same series
4. Get an overview of the entire collection

### Solution

**Persistent Central Index**:
- Store metadata from all processed drives locally
- Support offline search (drives don't need to be mounted)
- Track movie series/collection information
- Provide quick lookup by actor, director, series, or title

---

## 2. File Structure

```
~/.config/media_organizer/
├── central_index.json          # Main index (all drives merged)
├── central_index.json.backup   # Auto backup before updates
└── disk_indexes/
    ├── JMedia_M01.json         # Per-drive index
    ├── JMedia_M02.json
    └── JMedia_M05.json
```

---

## 3. Data Structure

### 3.1 Central Index Structure

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
      "title": "Pirates of the Caribbean: The Curse of the Black Pearl",
      "original_title": "Pirates of the Caribbean: The Curse of the Black Pearl",
      "year": 2003,
      "tmdb_id": 22,
      "imdb_id": "tt0325980",
      "collection_id": 295,
      "collection_name": "Pirates of the Caribbean Collection",
      "country": "US",
      "genres": ["Adventure", "Fantasy", "Action"],
      "actors": ["Johnny Depp", "Orlando Bloom"],
      "directors": ["Gore Verbinski"],
      "runtime": 143,
      "rating": 7.8
    }
  ],
  
  "tvshows": [...],
  
  "collections": {
    "295": {
      "id": 295,
      "name": "Pirates of the Caribbean Collection",
      "movies": [
        {"tmdb_id": 22, "title": "The Curse of the Black Pearl", "year": 2003, "disk": "JMedia_M01"},
        {"tmdb_id": 58, "title": "Dead Man's Chest", "year": 2006, "disk": "JMedia_M02"}
      ],
      "total_in_collection": 5,
      "owned_count": 2
    }
  },
  
  "indexes": {
    "by_actor": {"Johnny Depp": ["uuid-xxxx"]},
    "by_director": {"Gore Verbinski": ["uuid-xxxx"]},
    "by_genre": {"Action": ["uuid-xxxx"]},
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

## 4. Command Reference

### 4.1 index Command

```bash
# Basic usage - index current drive
media-organizer index /run/media/johnny/JMedia_M05/Movies_organized

# Use custom disk label
media-organizer index /path/to/movies --disk-label "Archive_2024"

# Index TV shows
media-organizer index /path/to/tvshows --media-type tvshows

# Force re-index
media-organizer index /path/to/movies --force

# Dry run
media-organizer index /path/to/movies --dry-run
```

### 4.2 search Command

```bash
# Search by actor
media-organizer search --actor "Johnny Depp"
media-organizer search -a "Depp"

# Search by director
media-organizer search --director "Christopher Nolan"
media-organizer search -d "Nolan"

# Search by collection
media-organizer search --collection "Pirates"
media-organizer search -c "Marvel"

# Search by title
media-organizer search --title "Caribbean"
media-organizer search -t "Inception"

# Search by year
media-organizer search --year 2024
media-organizer search --year 2020-2024

# Search by genre
media-organizer search --genre "Action"

# Search by country
media-organizer search --country US

# Combined filters
media-organizer search --actor "Depp" --year 2000-2010

# Show disk status
media-organizer search --actor "Depp" --show-status
```

### 4.3 Statistics and Management

```bash
# Show collection statistics
media-organizer index --stats

# List disk contents
media-organizer index --list JMedia_M05

# Remove disk from index
media-organizer index --remove JMedia_OLD

# Verify index against files
media-organizer index --verify /run/media/johnny/JMedia_M05
```

---

## 5. Output Examples

### 5.1 Search Output

```
$ media-organizer search --actor "Johnny Depp"

Found 8 movies featuring "Johnny Depp":

 #  | Year | Title                                    | Disk        | Status
----|------|------------------------------------------|-------------|--------
 1  | 2003 | Pirates of the Caribbean: Curse of BP    | JMedia_M01  | Offline
 2  | 2006 | Pirates of the Caribbean: Dead Man's     | JMedia_M02  | Offline
 3  | 2007 | Pirates of the Caribbean: At World's End | JMedia_M05  | Online
 4  | 2024 | Some Movie                               | JMedia_M05  | Online

Collection info:
  - Pirates of the Caribbean: Own 5/5 (across 3 disks)
```

### 5.2 Statistics Output

```
$ media-organizer index --stats

Media Collection Statistics
============================

Disks:
  JMedia_M01  | 280 movies |  45 shows | 1.2 TB | Last indexed: 2026-01-01
  JMedia_M02  | 310 movies |  52 shows | 1.5 TB | Last indexed: 2025-12-28
  JMedia_M05  | 150 movies |  30 shows | 0.8 TB | Last indexed: 2026-01-01
  -------------------------------------------------------------------------
  Total       | 740 movies | 127 shows | 3.5 TB

By Country:
  US (United States)  ████████████████████  350 (47%)
  CN (China)          ████████████          220 (30%)
  KR (Korea)          ████                  85  (11%)
  JP (Japan)          ██                    45  (6%)
  Other               ██                    40  (6%)

Collection Status:
  Complete:    35 collections (all movies owned)
  Incomplete:  18 collections (some movies missing)
```

---

## 6. Collection Support

### 6.1 TMDB Collections API

```
GET /collection/{collection_id}?api_key=xxx&language=en-US
```

### 6.2 Collection Info in NFO

```xml
<movie>
  <title>Pirates of the Caribbean: The Curse of the Black Pearl</title>
  ...
  <set>
    <name>Pirates of the Caribbean Collection</name>
    <overview>...</overview>
  </set>
  <tmdbcollectionid>295</tmdbcollectionid>
</movie>
```

---

## 7. Edge Cases

| Scenario | Handling |
|----------|----------|
| Disk label conflict | Use UUID as primary identifier |
| Movie moved between disks | Auto-detect and update on index |
| Duplicate movies | Allow duplicates, show all copies in search |
| Disk renamed | Provide `--rename` command |
| Index corruption | Auto backup, support rebuild from disk indexes |


