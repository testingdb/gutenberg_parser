---
type: feature-pipeline
title: Pipeline, CLI, and Multi-Threaded Output
description: CLI argument parsing, archive download/acquisition, bounded crossbeam channel pipeline (producer/workers/consumer), chunked/gzip output, and bridge schema conversion.
tags: [pipeline, cli, threading, output, download]
verified:
  - by: openwiki/0.7.0
    at: 2026-10-08T08:19:49.366Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
generated: { by: "pi", at: "2026-10-08T08:19:49.366Z" }
---

The binary (`main.rs`) delegates to `cli::run()` (`src/cli.rs`), which orchestrates the full multi-threaded pipeline (repo://README.md, repo://src/cli.rs#L1-L50).

## Argument Parsing

`Args` (`clap`) defines `archive_path` (required unless `--download`), `--output`, `--mirror`, `--chunk_size`, `--max_results`, `--bridge`, `--include-licensed`, `--wiki-images`, and `--download`. Conflicts between `archive_path` and `--download` are enforced by `clap` (repo://src/cli.rs#L40-L110).

## Archive Acquisition

`--download` streams `rdf-files.tar.bz2` from `RDF_FEED_URL` to a temporary file. The producer thread opens the archive, decompresses `bz2`, and sends raw XML buffers (`Vec<u8>`) into a bounded `crossbeam_channel` (`bounded(2048)`) (repo://src/cli.rs#L120-L220).

## Threading Model

- **Producer (1)**: Decompresses `bz2` archive and pushes raw XML buffers to `raw_tx`.
- **Workers (`available_parallelism()`)**: Consume `raw_rx`, call `process_rdf_xml`, and push parsed `Ebook` objects to `parsed_tx`.
- **Consumer (1)**: Collects from `parsed_rx` into chunks and writes JSON files. `drop(parsed_tx)` ensures clean shutdown when all workers finish (repo://src/cli.rs#L250-L350).

## Chunked Output

`write_chunk` writes arrays sorted by numeric `ebook_id` (fallback `u64::MAX`). When `path` ends with `.gz`, `flate2::GzEncoder` is used. `bridge` converts each `Ebook` to `BridgeEbook` before serialization. `get_chunk_path` inserts chunk indices into filenames (repo://src/cli.rs#L80-L120, repo://src/models.rs#L140-L266).

## Pipeline Timing

`run()` reports download progress, worker count, mirror base, filtering mode, chunk flush events, and final timing statistics.

Key resources: `repo://src/cli.rs`, `repo://README.md`, `repo://src/main.rs`.
