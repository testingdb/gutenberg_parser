---
type: "Reference"
title: "Chunked JSON Output and Serialization Guarantees"
openwiki_generated: true
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-a5d2f77c02ea33206b391c3a
    resource: repo://src/models.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---


# Chunked JSON Output and Serialization Guarantees

Two functions in `cli.rs` own everything between an assembled `Vec<Ebook>` and a file on disk: `write_chunk` serializes one batch, and `get_chunk_path` names the file. Nothing else in the pipeline touches output.

## Deterministic ordering

`write_chunk` sorts the batch in place before serializing, by `ebook_id` parsed as `u64`, with `unwrap_or(u64::MAX)` for anything non-numeric (repo://src/cli.rs#L115-L117). Two consequences:

- Numeric IDs sort numerically, so `2` precedes `10` rather than following lexicographic order.
- Non-numeric IDs all collapse to `u64::MAX` and therefore sort together at the end. Among themselves their relative order follows `sort_by_key`'s stability guarantee over the incoming order — which, because records arrive from concurrent workers, is *not* deterministic.

So a chunk is reproducible when IDs are numeric, and reproducible up to a trailing group of malformed IDs otherwise. The sort is per chunk, not global: with chunking enabled the batches themselves were formed in arrival order, which varies between runs. Chunking therefore trades global determinism for bounded memory.

## Hand-written array emission

The array is written byte by byte rather than through `serde_json::to_string`, opening with `[\n`, emitting each record, separating with `,\n`, closing with `]\n` (repo://src/cli.rs#L123-L140). Each record goes through `serde_json::to_writer` into the same writer, so record bodies use normal serde escaping while the array framing is fixed. The trailing separator is written *before* the newline that follows the last element, which is what keeps the output valid JSON without a trailing comma. The framing is identical in both modes, so a consumer cannot tell bridge output apart by array shape.

## Bridge mode is a write-time substitution

The `bridge` flag selects one of two serialization calls per record: `BridgeEbook::from(ebook)` or the `Ebook` itself. No record is pre-converted and no parsing decision depends on the flag, so switching modes costs only the field renames at write time (repo://src/cli.rs#L126-L131).

## gzip is chosen by filename

If the output path ends with `.gz`, the array is written through a `GzEncoder` with default compression and then finished; otherwise it is written to a `BufWriter` and flushed (repo://src/cli.rs#L142-L149). Compression is a property of the *path*, not a separate flag, so `--output catalog.json.gz` implies gzip and no other spelling does.

## Chunk file naming

`get_chunk_path(base_path, chunk_index)` inserts the index into the file name (repo://src/cli.rs#L157-L175):

| Base path | Chunk 1 | Chunk 2 |
| :--- | :--- | :--- |
| `out.json` | `out_1.json` | `out_2.json` |
| `catalog.json.gz` | `catalog_1.json.gz` | `catalog_2.json.gz` |
| `data/results.json` | `data/results_1.json` | `data/results_2.json` |

The `.json.gz` case is special-cased explicitly because the generic path logic would otherwise split the stem at the last dot and produce `catalog.json_1.gz`. For everything else the function keeps the parent directory and inserts the index before the extension, falling back to appending it after the stem when there is no extension. Chunk numbering starts at 1, so no un-suffixed file is ever produced when chunking is on.

## Chunk size, max results, and the final partial chunk

The consumer loop pushes each record into `current_chunk` and flushes when the length reaches `--chunk-size`, incrementing the index and clearing the buffer; separately, it breaks out of the loop when `--max-results` is reached (repo://src/cli.rs#L360-L388).

After the loop, a non-empty `current_chunk` is flushed. With chunking enabled that is a final *partial* chunk numbered after the full ones — so `--max-results 5000 --chunk-size 1000` yields exactly five files, whereas a count that is not a multiple of the chunk size yields a short trailing file (repo://src/cli.rs#L390-L399). Without `--chunk-size`, `chunk_size` is `None`, no flush ever happens inside the loop, and the single accumulated buffer is written to `args.output` verbatim (repo://src/cli.rs#L400-L403).

Because the flush test happens before the `max_results` test, a record that fills the final chunk and also reaches the limit is flushed and the loop then breaks with an empty buffer, so no trailing empty file is written.

Related pages: [Pipeline Orchestration and Threading Model](pipeline.md), [Record Model and Bridge Schema Mapping](../records/data-models.md), [Running the Parser: Inputs, Options, and Runtime Cost](../operations/running-the-parser.md).

Key resources: `repo://src/cli.rs`, `repo://src/models.rs`, `repo://README.md`.
