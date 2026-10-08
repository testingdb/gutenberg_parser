---
type: architecture-pipeline
title: Pipeline Orchestration and Threading Model
description: How cli::run wires archive acquisition, a bz2 producer, a worker pool, and a streaming consumer through two bounded channels, and how the run terminates.
tags: [pipeline, threading, concurrency, channels, cli]
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-b55a21a31ede1b56cd31a6a6
    resource: repo://src/main.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---

# Pipeline Orchestration and Threading Model

`main` does nothing but declare six modules and delegate to `cli::run`; all orchestration lives in `run` (repo://src/main.rs#L21-L34). The pipeline is a three-stage streaming design chosen so that neither the archive nor the output set is ever fully resident: bytes flow through in bounded batches and records flow out the same way.

## Stage 1 — acquisition

Before any thread exists, the archive path is decided. With `--download`, `download_rdf_archive` fetches the feed and returns a `NamedTempFile`; the handle is retained in `_downloaded_archive` specifically to keep the file alive for the duration of the run (repo://src/cli.rs#L282-L291). The mirror key is resolved here too, and the run echoes a summary of the effective options before starting work (repo://src/cli.rs#L277-L308).

## Stage 2 — the producer

One spawned thread opens the file, wraps it in a 1 MiB `BufReader`, then in a `BzDecoder`, then in a `tar::Archive`. Entries are streamed one at a time; only names ending in `.xml` or `.rdf` are read, and each is buffered fully into a `Vec<u8>` before being sent on `raw_tx` (repo://src/cli.rs#L314-L334).

Each entry is therefore held in memory in its entirety, but only up to the channel bound. Directory entries, and anything that is not XML or RDF, are skipped without reading. The producer breaks out of the loop as soon as `send` fails, which is how it learns the workers are gone.

## Stage 3 — the worker pool

`available_parallelism()` sets the worker count, falling back to 4 if the platform cannot answer (repo://src/cli.rs#L337-L338). Each worker clones its own receiver and sender and loops on `recv`, calling `process_rdf_xml` and forwarding only successful records (repo://src/cli.rs#L340-L356).

A rejected record produces no message at all. Workers exit when the channel closes, and they also break if their own `send` fails — which happens when the consumer stops receiving, as it does when `--max-results` is hit (repo://src/cli.rs#L350-L352).

## Stage 4 — the consumer

Back on the main thread, the `while let Ok(ebook) = parsed_rx.recv()` loop is the collector and writer. It accumulates, flushes chunks, counts matches, and writes; see [Chunked JSON Output and Serialization Guarantees](json-output.md) for the write path (repo://src/cli.rs#L360-L404).

## Channels and termination

Both hops use `crossbeam_channel::bounded(2048)` (repo://src/cli.rs#L310-L312). The bound is what makes the design streaming rather than buffered: 2048 in-flight XML buffers or parsed records is the entire memory commitment beyond one archive entry and one chunk.

Termination is by sender exhaustion, and the critical detail is one line: after spawning the workers, `drop(parsed_tx)` (repo://src/cli.rs#L357). Each worker holds a clone, so the channel would never close while the original sender lived in `run`'s scope; dropping it means the last worker to exit closes the channel, which is what unblocks the consumer's `recv`. Without that drop the run would hang after all parsing finished.

The converse also holds: the consumer never closes `raw_rx` explicitly. Workers exit when the producer's sender drops as its closure returns, which happens when the archive is exhausted or a send fails.

## Early termination and its consequences

`--max-results` breaks the consumer loop without joining anything (repo://src/cli.rs#L383-L387). No thread handle is kept for the producer or any worker — `std::thread::spawn` results are discarded — so the run simply returns from `run`, and `main` exits the process. In-flight records are dropped rather than written, workers that are mid-parse are killed at process exit, and a partially written chunk is the only on-disk trace of truncation.

The one graceful path is the final-flush block after the loop, which writes whatever remains buffered (repo://src/cli.rs#L390-L404).

## Failure behavior

Two `expect` calls turn infrastructure problems into process panics rather than handled errors: opening the archive file in the producer, and writing a chunk in the consumer (repo://src/cli.rs#L317, repo://src/cli.rs#L371). Per-record failures are handled — a malformed record becomes an error code and is skipped — but the archive being missing or the disk being full aborts everything, including records already collected in memory.

Related pages: [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md), [Chunked JSON Output and Serialization Guarantees](json-output.md), [Running the Parser: Inputs, Options, and Runtime Cost](../operations/running-the-parser.md), [Wikipedia Agent Image Enrichment](../integrations/wikipedia-enrichment.md).

Key resources: `repo://src/cli.rs`, `repo://src/main.rs`.
