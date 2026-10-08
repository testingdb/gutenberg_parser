---
type: "Reference"
title: "Wikipedia Agent Image Enrichment"
openwiki_generated: true
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-2a737474d86fc75cc9d9694f
    resource: repo://src/config.rs
  - id: openwiki-source-0993c2c0112ac99cf6999828
    resource: repo://src/utils.rs
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---


# Wikipedia Agent Image Enrichment

`--wiki-images` adds an `image` field to each agent that can be matched to a Wikipedia article. It is the only part of the pipeline that performs outbound network I/O during parsing, and the only one whose cost scales with the number of *distinct agents* rather than the number of records.

The feature is gated at two levels: the flag must be set for `parse_agent` to call the lookup at all, and the agent must have a qualifying Wikipedia URL in its `webpages` list (repo://src/xml_parser.rs#L115-L120).

## Resolution flow

`agent_wikipedia_image(webpages)` is the entry point (repo://src/utils.rs#L312-L321):

1. **Match.** The first URL matching `RE_WIKIPEDIA_URL` is selected. That regex accepts an optional scheme, any number of `*.wikipedia.org` subdomains, and requires a `/wiki/` path, so `en.wikipedia.org/wiki/X`, `m.wikipedia.org/wiki/X`, and `https://de.wikipedia.org/wiki/X` all qualify while a personal site or a bare host does not (repo://src/config.rs#L623-L629).
2. **Cache probe.** The URL is looked up in a process-wide cache. A cached `Some` or a cached `None` both return immediately — misses are cached precisely because the dominant workload is repeated agents.
3. **Page name.** Everything after the last slash is taken as the page name; an empty tail yields `None` (repo://src/utils.rs#L228-L231).
4. **Request.** `request_wikipedia_thumbnail` builds `https://en.wikipedia.org/api/rest_v1/page/summary/{page}` and reads `thumbnail.source` out of the JSON response (repo://src/utils.rs#L268-L299).
5. **Cache store.** The result — including `None` — is inserted under the article URL.

## Request discipline

Three mechanisms keep this from hammering the Wikimedia API, all of them process-wide rather than per-thread:

- **A shared, identified client.** A single `ureq::Agent` is built lazily with a `User-Agent` that identifies the tool and its crate version. The Wikimedia REST API rejects generic client identifiers, so this is a functional requirement, not cosmetics — the token is assembled at compile time from `CARGO_PKG_VERSION`, which means a version bump automatically changes the identity sent (repo://src/config.rs#L76-L85, repo://src/utils.rs#L199-L205).
- **A global timeout.** `timeout_global` of 10 seconds bounds a stalled request so it cannot pin a worker thread indefinitely (repo://src/utils.rs#L182-L183).
- **A process-wide minimum interval.** A single `Instant` behind a `Mutex` records the last request time; any thread about to call sleeps until at least 100 ms has elapsed since it. Because the timestamp is shared, N worker threads still serialize to roughly one request per 100 ms overall rather than N requests per 100 ms (repo://src/utils.rs#L190-L192, repo://src/utils.rs#L247-L255).

`request_wikipedia_thumbnail` then retries at most twice. A transport error or an HTTP 429/503 sleeps 500 ms and retries once; any other non-200 returns `None` immediately, as does a second failure (repo://src/utils.rs#L268-L299). The retry is deliberately shallow — enrichment is best-effort, and a missing image must never fail a record.

Both mutexes recover from poisoning via `unwrap_or_else(|p| p.into_inner())` rather than propagating the panic, so one failed lookup cannot make every subsequent worker panic (repo://src/utils.rs#L217-L219, repo://src/utils.rs#L248).

## Operational consequence

Because lookups execute inline on the worker threads, the pipeline's parallelism does not hide the network latency: it *multiplies* request pressure, which is why the global throttle exists. Total requests over a full archive approximate the number of distinct agents with Wikipedia pages — tens of thousands — so wall-clock time under `--wiki-images` is dominated by throttling rather than by parsing. Without the flag, zero requests are made and `image` is simply absent (repo://src/utils.rs#L312-L321).

The resolved URL is stored verbatim, including the `utm_*` tracking parameters the API appends. In output it is `image` on the agent in both parser and bridge mode, omitted entirely when absent (repo://src/models.rs#L60-L65).

Related pages: [Running the Parser: Inputs, Options, and Runtime Cost](../operations/running-the-parser.md), [Pipeline Orchestration and Threading Model](../pipeline/pipeline.md), [Record Model and Bridge Schema Mapping](../records/data-models.md), [Test Suite Layout and Local Verification](../development/verification.md).

Key resources: `repo://src/utils.rs`, `repo://src/config.rs`, `repo://src/xml_parser.rs`.
