# ROMM Metrics Exporter - TODO: Pending Metrics

These metrics are identified from the ROMM OpenAPI spec but not yet implemented in `src/main.rs`.

## Option 3: Category from Name Prefix (from `/api/roms`) - DONE
- Parse category from ROM name prefix (e.g., `~category~name`)
- Metric: `romm_category{category="game"} 5000`

## Option 4: File Structure (from `/api/roms`) - DONE
- Count ROMs by storage format (single file, multiple files, nested folder, easyrpg, other)
- Metric: `romm_file_structure{type="single"} 8900`

## Option 5: Hash Coverage (from `/api/roms`) - DONE
- Count ROMs by available hash checksum type (CRC, MD5, SHA1, RHash)
- Metric: `romm_hash_coverage{hash_type="crc"} 7500`

## Option 6: ROM Manual Flag (from `/api/roms`) - DONE
- Count ROMs by has_manual boolean flag
- Metric: `romm_roms_with_manual{has_manual="true"} 5000`

## Option 7: Integrity State (from `/api/roms`) - DONE
- Count ROMs by integrity state (identified, unidentified, physical, missing)
- Metric: `romm_rom_integrity{state="identified"} 12000`

## Option 8: ROM Metadata Flags (from `/api/roms`)
- Aggregate boolean metadata flags from ROM entries
- Metric: `romm_metadata{flag="some_flag"} 3000`

## Option 9: Missing / Unidentified ROMs (from `/api/roms` filter)
- Count ROMs by integrity state (missing_from_fs, unidentified, identified, physical)
- Metric: `romm_rom_integrity{state="missing_from_fs"} 150`

## Option 9: User Ratings Distribution (from `/api/roms`)
- Aggregate ROM ratings into histogram/bucket buckets
- Metric: `romm_rating_distribution{rating="5"} 2100`

## Option 10: Genres / Companies / Developers (from `/api/roms/filters`)
- Use filter enumeration for genre/company/developer counts
- Metric: `romm_genres{genre="Action"} 8500`

## Option 11: Collection Counts (from `/api/collections`, `/api/collections/smart`, `/api/collections/virtual`)
- Count manual, smart, virtual, and favorites collections
- Metric: `romm_collections{type="manual"} 245`

## Option 12: Active Sessions / Devices (from `/api/streaming/sessions`, `/api/devices/online`)
- Track active streaming sessions and online devices
- Metric: `romm_streaming_sessions{status="active"} 3`

## Option 13: Task / Scan Status (from `/api/tasks`)
- Monitor scan, update, and cleanup job statuses
- Metric: `romm_tasks_status{task="scan" status="completed"} 42`

## Option 14: Audio/Soundtrack / Media Flags (from `/api/roms`)
- Count ROMs with associated media: soundtrack screenshot, notes, youtube video, video
- Metric: `romm_has_screenshot 8900`

## Option 15: Language & Age Distribution (from `/api/roms`)
- Aggregate ROMs by language and age rating
- Metric: `romm_languages{language="English"} 18500`

## Option 16: Save/State Size Metrics (from `/api/roms` + save data)
- Total saves size by storage type
- Metric: `romm_saves_size_bytes 8500000000`

## Option 17: Memory Card / Container Metrics (from `/api/memory-cards`)
- Count memory cards by emulator type
- Metric: `romm_memory_cards{emulator="RomBox"} 42`

## Option 18: Play Session Aggregates (from `/api/play-sessions`)
- Total play sessions, average duration, per-user counts
- Metric: `romm_play_sessions_total 15800`

## Option 19: Music Library Stats (from `/api/music/stats`)
- Track music tracks, artists, albums, playlists, favorites
- Metric: `romm_music_tracks 5200`

---

## Implementation Notes

- Most ROM-level metrics (options 4-15) require paginated `/api/roms` fetches or filter enumeration
- The `/api/roms/filters` endpoint provides the full set of label values for genres, companies, developers, regions, languages
- Consider aggregating per-ROM flags during pagination rather than making separate API calls
- Keep label cardinality in check: genre/company filters may have hundreds of values, which is fine for Prometheus gauges
- Priority order: 4-7 (core ROM metrics first), then 8-15 (engagement & categorization), then 16-19 (niche metrics)
