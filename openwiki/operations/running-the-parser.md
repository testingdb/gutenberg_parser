---
type: "Reference"
title: "Running the Parser: Inputs, Options, and Runtime Cost"
openwiki_generated: true
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-a5d2f77c02ea33206b391c3a
    resource: repo://src/models.rs
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---


# Running the Parser: Inputs, Options, and Runtime Cost

The binary takes at most one positional argument and eight options. Understanding what each one changes — and what it costs — is enough to plan a full-archive run.

## Getting an archive

There are two provisioning paths. Either download `rdf-files.tar.bz2` (Gutenberg republishes it daily) yourself and pass its path, or pass `--download` with no positional argument and let the tool fetch it (repo://src/cli.rs#L49-L56, repo://src/cli.rs#L96-L101).

`--download` streams the response through `ureq` into a `tempfile::Builder` temporary file created **in the current working directory**, not the system temp dir, and prints progress every two seconds with megabytes downloaded against `content-length` when the server supplies it (repo://src/cli.rs#L187-L258). The `NamedTempFile` handle is kept alive for the whole run and deleted when it drops at the end of `run()`; the final log line announces the deletion. Any download failure — transport error, non-200 status, or local I/O error — prints `[ERROR]` and exits with status 1 rather than continuing (repo://src/cli.rs#L282-L291).

The practical consequence: `--download` needs write permission in the current directory, needs roughly the archive's full size on local disk, and leaves nothing behind afterwards.

## Options and defaults

| Flag | Default | Effect |
| :--- | :--- | :--- |
| `[ARCHIVE_PATH]` | required unless `--download` | Path to the `.tar.bz2` archive |
| `-o, --output` | `filtered_ebooks.json` | Output path; a `.gz` suffix enables gzip |
| `-m, --mirror` | `gutenberg` | Mirror key or literal base URL |
| `--max-results` | none | Stop after N matched records |
| `-c, --chunk-size` | none | Records per output file |
| `--bridge` | off | Rename fields to the target schema |
| `--include-licensed` | off | Keep non-public-domain records |
| `-w, --wiki-images` | off | Resolve agent thumbnails from Wikipedia |

`archive_path` and `--download` are mutually exclusive and one is mandatory, enforced by clap rather than by runtime checks, so a missing archive is a usage error before any work starts (repo://src/cli.rs#L49-L56).

An unrecognized `--mirror` value is treated as a literal base URL rather than rejected, which is what allows an arbitrary mirror without a code change (repo://src/cli.rs#L277-L280). See [Mirror Selection and URL Transformation](../ingestion/mirror-urls.md) for how the value is applied.

## Cost and behavior tradeoffs

**Filtering is on by default.** Without `--include-licensed`, the run keeps only records whose license text contains "public domain", so adding the flag materially increases output size and changes the population represented. This is the default precisely because the target corpus is public-domain works (repo://src/xml_parser.rs#L211-L220).

**Chunking changes write behavior, not parse behavior.** With `--chunk-size`, records accumulate in memory until the threshold and each chunk is flushed to its own file; without it, the entire matched set is held and written once at the end. Chunking bounds peak memory and gives incremental progress, at the cost of many files (repo://src/cli.rs#L360-L404). See [Chunked JSON Output and Serialization Guarantees](../pipeline/json-output.md).

**`--max-results` truncates, it does not sample.** The consumer loop breaks as soon as the count is reached, so which records survive depends on worker scheduling order rather than on any ordering guarantee — although each written chunk is sorted by ebook ID. Producers and workers keep running until their channels close, so a capped run does not stop all work immediately (repo://src/cli.rs#L383-L388).

**`--bridge` is a pure rename at serialization time.** No parsing behavior changes; each record is converted through `BridgeEbook::from` as it is written, and `ebook_id` is re-parsed as an integer that falls back to `0` on failure (repo://src/cli.rs#L123-L140, repo://src/models.rs#L301-L319).

**`--wiki-images` dominates wall-clock time.** Every lookup runs inline on a worker thread and is paced by a process-wide 100 ms minimum interval, so a full archive performs on the order of one request per distinct agent — tens of thousands. Expect the run to take noticeably longer than a plain parse; without the flag, no network calls are made at all. Details in [Wikipedia Agent Image Enrichment](../integrations/wikipedia-enrichment.md).

## Failure modes to expect

The pipeline is fail-soft for records and fail-fast for the run. A malformed or filtered-out record is dropped by the worker without aborting anything. A missing or unreadable archive file, however, is an `expect` in the producer thread and a chunk write failure is an `expect` in the consumer — both panic the process rather than producing a partial-file error message (repo://src/cli.rs#L317, repo://src/cli.rs#L371).

Related pages: [Pipeline Orchestration and Threading Model](../pipeline/pipeline.md), [Chunked JSON Output and Serialization Guarantees](../pipeline/json-output.md), [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md).

Key resources: `repo://src/cli.rs`, `repo://README.md`, `repo://src/models.rs`.
