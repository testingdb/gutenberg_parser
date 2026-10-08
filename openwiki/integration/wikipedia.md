---
type: integration-wikipedia
title: Wikipedia Image Resolution
description: Wikipedia REST thumbnail lookup for agents with Wikipedia pages, with process-wide memoization and rate-limiting throttle.
tags: [wikipedia, image, integration, memoization]
verified:
  - by: openwiki/0.7.0
    at: 2026-10-08T08:19:49.366Z
sources:
  - id: openwiki-source-2a737474d86fc75cc9d9694f
    resource: repo://src/config.rs
  - id: openwiki-source-0993c2c0112ac99cf6999828
    resource: repo://src/utils.rs
generated: { by: "pi", at: "2026-10-08T08:19:49.366Z" }
---

When `--wiki-images` (`-w`) is enabled, `agent_wikipedia_image` in `utils.rs` resolves thumbnails for agents whose `webpages` include a Wikipedia URL (`repo://src/utils.rs#L312-L330`, `repo://src/xml_parser.rs#L117`).

## Lookup Process

- `RE_WIKIPEDIA_URL` matches `wikipedia.org/wiki/` URLs.
- `wikipedia_page_name` extracts the page name after the last slash.
- `wikipedia_summary_url` builds `https://en.wikipedia.org/api/rest_v1/page/summary/<name>`.
- `request_wikipedia_thumbnail` fetches JSON and reads `thumbnail.source`.

## Rate Limiting and Retries

`throttle_wikipedia_request` enforces `WIKIPEDIA_MIN_INTERVAL` between requests using a process-wide `LazyLock` mutex (`WIKIPEDIA_LAST_REQUEST`). Requests retry once on `429` or `503` status with a 500ms backoff (repo://src/utils.rs#L247-L295).

## Memoization

`wiki_image_cache()` (process-wide `LazyLock<HashMap<String, Option<String>>`) caches results per Wikipedia URL. Repeated lookups of the same agent page (common across ebooks) issue no further network requests (repo://src/utils.rs#L312-L330).

Key resources: `repo://src/utils.rs`, `repo://src/config.rs`. Related: [Parsing](../architecture/parsing.md), [Pipeline](../features/pipeline.md), [Data Models](../architecture/models.md), [Taxonomy](../architecture/taxonomy.md).
