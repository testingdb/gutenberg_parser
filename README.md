# Project Gutenberg RDF/XML Archive Parser
![Cover image](.misc/Gemini_Generated_Image_yjcci3yjcci3yjcc.png)

<div align="center">

#### Release & Version

[![Release](https://github.com/testingdb/gutenberg_parser/actions/workflows/release.yml/badge.svg)](https://github.com/testingdb/gutenberg_parser/actions/workflows/release.yml)
[![Version 1.3.0](https://img.shields.io/badge/version-1.3.0-blue.svg)](https://github.com/testingdb/gutenberg_parser/releases)

#### Code Quality

[![Reliability Rating](https://sonarcloud.io/api/project_badges/measure?project=testingdb_gutenberg_parser&metric=reliability_rating)](https://sonarcloud.io/summary/new_code?id=testingdb_gutenberg_parser)
[![Maintainability Rating](https://sonarcloud.io/api/project_badges/measure?project=testingdb_gutenberg_parser&metric=sqale_rating)](https://sonarcloud.io/summary/new_code?id=testingdb_gutenberg_parser)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=testingdb_gutenberg_parser&metric=vulnerabilities)](https://sonarcloud.io/summary/new_code?id=testingdb_gutenberg_parser)
[![Duplicated Lines (%)](https://sonarcloud.io/api/project_badges/measure?project=testingdb_gutenberg_parser&metric=duplicated_lines_density)](https://sonarcloud.io/summary/new_code?id=testingdb_gutenberg_parser)
[![Technical Debt](https://sonarcloud.io/api/project_badges/measure?project=testingdb_gutenberg_parser&metric=sqale_index)](https://sonarcloud.io/summary/new_code?id=testingdb_gutenberg_parser)
[![Lines of Code](https://sonarcloud.io/api/project_badges/measure?project=testingdb_gutenberg_parser&metric=ncloc)](https://sonarcloud.io/summary/new_code?id=testingdb_gutenberg_parser)

</div>

A high-performance, multi-threaded Rust CLI utility designed to parse, transform, and index Project Gutenberg RDF/XML metadata archives (`.tar.bz2`). 

The parser extracts author metadata, taxonomy structures (Library of Congress codes, genres, subjects, topics), format links, and download metrics, outputting clean, normalized JSON or Gzip-compressed JSON.

---

## Features

- **Multi-Threaded Pipeline:** Utilizes `crossbeam-channel` and worker thread pools to extract and process XML files concurrently.
- **Automatic Archive Download:** Pass `--download` to fetch the latest `rdf-files.tar.bz2` directly from Project Gutenberg, parse it, and delete the archive afterwards — no manual provisioning needed.
- **Taxonomy Normalization:** Maps LC Classification codes and Gutenberg bookshelves into standardized domains, genres, and hierarchical subtopics.
- **Mirror Rewriting:** Rewrites format and cover image URLs to specified Project Gutenberg mirror sites.
- **Strict Quality Filtering:** Automatically filters out non-text entries, entries without authors/contributors, and entries missing required format types (EPUB and HTML).
- **Public Domain Filtering:** Only Public Domain ebooks are included by default; pass `--include-licensed` to also keep copyrighted or otherwise licensed ebooks.
- **Flexible Output Handling:** Supports single JSON files, Gzip compression (`.json.gz`), and chunked output files.

---

## CI/CD

The repository uses a GitHub Actions pipeline covering quality gates, dependency
security, SonarCloud analysis, and automated release builds. See
**[WORKFLOW.md](WORKFLOW.md)** for the full guide (triggers, required checks,
secrets, troubleshooting, and how to extend the pipeline).

---

## Prerequisites & Installation

Ensure you have Rust and Cargo installed on your system.

### Build from Source

Clone the repository and build the binary in release mode:

```bash
cargo build --release
```

The optimized binary will be located at `./target/release/gutenberg_parser`.

---

## Obtaining Gutenberg Metadata Archives

Project Gutenberg provides raw RDF catalog archives updated daily. Download the latest archive (`rdf-files.tar.bz2`) using `curl` or `wget`:

```bash
wget https://www.gutenberg.org/cache/epub/feeds/rdf-files.tar.bz2
```

Alternatively, skip manual provisioning entirely and let the parser fetch the archive itself with `--download` (see [CLI Usage](#cli-usage)); the downloaded archive is automatically deleted after parsing.

---

## CLI Usage

```text
Usage: gutenberg_parser [OPTIONS] [ARCHIVE_PATH]

Arguments:
  [ARCHIVE_PATH]  Path to the Project Gutenberg .tar.bz2 archive (required unless --download is used)

Options:
  -o, --output <OUTPUT>          Output file path [default: filtered_ebooks.json]
  -m, --mirror <MIRROR>          Mirror key or base URL [default: gutenberg]
      --max-results <MAX_RESULTS> Maximum number of matched ebooks to output
  -c, --chunk-size <CHUNK_SIZE>  Number of items per chunk file
      --bridge                   Rename output object fields to match the target database schema (alt-target-schema.md)
      --include-licensed         Also include ebooks that are NOT Public Domain (copyrighted or otherwise licensed)
      --download                 Automatically download rdf-files.tar.bz2 from Project Gutenberg, parse it, then delete the archive afterwards
  -h, --help                     Print helpẑ
  -V, --version                  Print version
```

### Predefined Mirrors

The `--mirror` option accepts custom URLs or any of the following predefined mirror keys:

| Key | Base URL |
| :--- | :--- |
| `gutenberg` | `https://www.gutenberg.org/` |
| `pglaf` | `https://gutenberg.pglaf.org/` |
| `odu` | `https://mirror.cs.odu.edu/gutenberg/` |
| `waterloo` | `http://mirror.csclub.uwaterloo.ca/gutenberg/` |
| `uk` | `http://www.mirrorservice.org/sites/ftp.ibiblio.org/pub/docs/books/gutenberg/` |
| `xmission` | `http://mirrors.xmission.com/gutenberg/` |

---

## Examples

### 1. Basic Processing
Extract Public Domain metadata from `rdf-files.tar.bz2` to `filtered_ebooks.json`:
```bash
./target/release/gutenberg_parser rdf-files.tar.bz2
```

### 1b. Including Licensed Books
Parse an archive while keeping copyrighted or otherwise licensed ebooks:
```bash
./target/release/gutenberg_parser rdf-files.tar.bz2 \
  --include-licensed
```

### 1c. Auto-Download the Archive
Fetch the latest `rdf-files.tar.bz2` from Project Gutenberg, parse it, and delete the archive afterwards:
```bash
./target/release/gutenberg_parser --download
```

### 2. Compressed Output with a Custom Mirror
Parse an archive, map download links to the Waterloo mirror, and write directly to a Gzip-compressed file:
```bash
./target/release/gutenberg_parser rdf-files.tar.bz2 \
  --output catalog.json.gz \
  --mirror waterloo
```

### 3. Chunked Output and Result Limit
Limit processing to 5,000 matches and split the output into chunks of 1,000 items per file (`chunk_1.json`, `chunk_2.json`, etc.):
```bash
./target/release/gutenberg_parser rdf-files.tar.bz2 \
  --output chunk.json \
  --max-results 5000 \
  --chunk-size 1000
```

---

## JSON Output Schema Example

```json
[
  {
    "title": "Alice's Adventures in Wonderland",
    "issued_date": "1994-10-01",
    "alternative_titles": [
      "Alice in Wonderland"
    ],
    "agents": [
      {
        "type": "author",
        "agent_id": 18,
        "name": "Carroll, Lewis",
        "aliases": ["Dodgson, Charles Lutwidge"],
        "webpage": "https://en.wikipedia.org/wiki/Lewis_Carroll",
        "birth_date": "1832",
        "death_date": "1898"
      },
      {
        "type": "illustrator",
        "agent_id": 100,
        "name": "Tenniel, John",
        "aliases": ["Tenniel, Sir John"],
        "webpage": "https://en.wikipedia.org/wiki/John_Tenniel",
        "birth_date": "1820",
        "death_date": "1914"
      }
    ],
    "description": "Alice's Adventures in Wonderland (commonly shortened to Alice in Wonderland) is an 1865 novel written by English author Charles Lutwidge Dodgson under the pseudonym Lewis Carroll. It tells of a girl named Alice falling through a rabbit hole into a fantasy world populated by peculiar, anthropomorphic creatures.",
    "language": "en",
    "formats": [
      {
        "type": "text/html",
        "url": "https://www.gutenberg.org/files/11/11-h/11-h.htm"
      },
      {
        "type": "application/epub+zip",
        "url": "https://www.gutenberg.org/ebooks/11.epub3.images"
      }
    ],
    "taxonomy": {
      "domain": "Literature & Fiction",
      "genres": [
        "Children's Literature",
        "Classics",
        "Fantasy"
      ],
      "topics": [
        {
          "heading": "Fantasy literature",
          "subtopics": ["Children's stories", "Imaginary places"]
        },
        {
          "heading": "Animals",
          "subtopics": ["Rabbits", "Cats"]
        }
      ]
    },
    "downloads": 62000,
    "ebook_id": "11",
    "cover_image": "https://www.gutenberg.org/cache/epub/11/pg11.cover.medium.jpg",
    "license": "Public domain in the USA."
  }
]
```

---

## Bridge Mode

Passing `--bridge` keeps the same one-object-per-book structure but renames output object fields:

| Parser field | Bridge field | Target column |
| :--- | :--- | :--- |
| `ebook_id` | `pg_id` | `books.pg_id` |
| `language` | `lang_code` | `books.lang_code` |
| `downloads` | `pg_download_count` | `books.pg_download_count` |
| `cover_image` | `md_cover_image_url` | `books.md_cover_image_url` |
| `license` | `license_statement` | `books.license_statement` |
| `formats[].type` | `formats[].mime_type` | `formats.mime_type` |
| `formats[].url` | `formats[].file_url` | `formats.file_url` |
| `agents[].type` | `agents[].role` | `books_contributions.role` |
| `agents[].agent_id` | `agents[].pg_id` | `agents.pg_id` |
| `agents[].webpage` | `agents[].external_urls` | `agents.external_urls` |

### 4. Bridge Output Example

```bash
./target/release/gutenberg_parser rdf-files.tar.bz2 \
  --output catalog_bridge.json \
  --bridge
```

```json
[
  {
    "title": "Around the World in Eighty Days",
    "issued_date": "1995-11-01",
    "alternative_titles": [
      "Le tour du monde en quatre-vingts jours. English",
      "Around the World in 80 Days"
    ],
    "agents": [
      {
        "role": "author",
        "pg_id": 31,
        "name": "Verne, Jules",
        "external_urls": ["https://en.wikipedia.org/wiki/Jules_Verne"],
        "aliases": ["Verne, Jules Gabriel"],
        "birth_date": "1828",
        "death_date": "1905"
      }
    ],
    "lang_code": "en",
    "formats": [
      {
        "mime_type": "text/html",
        "file_url": "https://www.gutenberg.org/files/103/103-h/103-h.htm"
      },
      {
        "mime_type": "application/epub+zip",
        "file_url": "https://www.gutenberg.org/ebooks/103.epub3.images"
      }
    ],
    "taxonomy": {
      "domain": "Literature & Fiction",
      "genres": [
        "Action & Adventure",
        "Classics",
        "Fiction & Novels"
      ],
      "topics": [
        {
          "heading": "Voyages around the world",
          "subtopics": ["Adventure stories"]
        },
        {
          "heading": "Ballooning",
          "subtopics": []
        }
      ]
    },
    "pg_download_count": 45000,
    "pg_id": 103,
    "md_cover_image_url": "https://www.gutenberg.org/cache/epub/103/pg103.cover.medium.jpg",
    "license_statement": "Public domain in the USA."
  }
]
```

---

## JSON Output Contract

The parser guarantees a **stable JSON schema** for all output modes. This section documents field presence rules so downstream consumers can rely on consistent structure.

### Array Fields (Always Present, Never Omitted)

These fields are **always serialized as arrays** — empty `[]` when no data exists, never `null` or missing:

| Field | Description |
|-------|-------------|
| `taxonomy.genres` | Mapped genre labels (e.g., `["Fiction & Novels", "Mystery & Crime"]`) |
| `taxonomy.topics` | Hierarchical topic objects with `heading` and `subtopics` |
| `taxonomy.topics[].subtopics` | Subtopic strings for each topic heading |
| `agents[].aliases` | Alternative name forms for each agent (author, translator, etc.) |
| `agents[].external_urls` | *(Bridge mode only)* Agent webpage URLs as array |
| `alternative_titles` | Alternate title strings for the work |
| `agents` | Contributor objects (authors, translators, illustrators, etc.) |

**Example — Book with No Genres/Topics:**
```json
{
  "taxonomy": {
    "domain": "General & Uncategorized",
    "genres": [],
    "topics": []
  },
  "alternative_titles": ["Alternate Title Example"],
  "agents": [
    { "type": "author", "name": "Anonymous", "aliases": [], ... }
  ]
}
```

### Optional Scalar Fields (Omitted When `null`)

These fields are **only present when a value exists** — absent from JSON when `None`/`null`:

| Field | Context |
|-------|---------|
| `issued_date` | Publication date (may be unknown) |
| `description` | Summary/notes (MARC 520/500) |
| `webpage` | *(Parser mode)* Agent homepage URL |
| `birth_date` / `death_date` | Agent lifespan dates |
| `agent_id` / `pg_id` | Project Gutenberg agent/book numeric IDs |

### Required Non-Null Fields (Always Present, Non-Empty)

These fields are **guaranteed present and non-null** for every output object:

| Parser Mode | Bridge Mode | Description |
|-------------|-------------|-------------|
| `title` | `title` | Work title |
| `language` | `lang_code` | ISO 639 language code |
| `formats` | `formats` | Array of format objects (HTML + EPUB minimum) |
| `taxonomy.domain` | `taxonomy.domain` | Primary domain classification |
| `ebook_id` | `pg_id` | Project Gutenberg book ID |
| `cover_image` | `md_cover_image_url` | Cover image URL |
| `license` | `license_statement` | License text |
| `downloads` | `pg_download_count` | Download counter |
| `agents[]` | `agents[]` | At least one contributor (required by filter) |

### Minimal Valid Output Example

A book with only required fields and all optional arrays empty:

```json
{
  "title": "Untitled Work",
  "alternative_titles": ["Also Known As"],
  "issued_date": null,
  "agents": [
    { "type": "author", "agent_id": null, "name": "Unknown", "aliases": [], "webpage": null, "birth_date": null, "death_date": null }
  ],
  "description": null,
  "language": "en",
  "formats": [
    { "type": "text/html", "url": "https://www.gutenberg.org/files/12345/12345-h/12345-h.htm" },
    { "type": "application/epub+zip", "url": "https://www.gutenberg.org/ebooks/12345.epub3.images" }
  ],
  "taxonomy": {
    "domain": "General & Uncategorized",
    "genres": [],
    "topics": []
  },
  "downloads": 0,
  "ebook_id": "12345",
  "cover_image": "https://www.gutenberg.org/cache/epub/12345/pg12345.cover.medium.jpg",
  "license": "Public domain in the USA."
}
```

> **Note:** In Bridge mode (`--bridge`), field names follow the target database schema (see [Bridge Mode](#bridge-mode) table). The presence rules above apply identically — array fields are always present, optional scalars omitted when null.