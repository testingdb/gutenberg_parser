# Project Gutenberg RDF/XML Archive Parser
![Cover image](.misc/Gemini_Generated_Image_yjcci3yjcci3yjcc.png)

<div align="center">

#### Release & Version

[![Release](https://github.com/testingdb/gutenberg_parser/actions/workflows/release.yml/badge.svg)](https://github.com/testingdb/gutenberg_parser/actions/workflows/release.yml)
[![Version 1.0.3](https://img.shields.io/badge/version-1.0.3-blue.svg)](https://github.com/testingdb/gutenberg_parser/releases)

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
- **Taxonomy Normalization:** Maps LC Classification codes and Gutenberg bookshelves into standardized domains, genres, and hierarchical subtopics.
- **Mirror Rewriting:** Rewrites format and cover image URLs to specified Project Gutenberg mirror sites.
- **Strict Quality Filtering:** Automatically filters out non-text entries, entries without authors/contributors, and entries missing required format types (EPUB and HTML).
- **Flexible Output Handling:** Supports single JSON files, Gzip compression (`.json.gz`), and chunked output files.

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

---

## CLI Usage

```text
Usage: gutenberg_parser [OPTIONS] <ARCHIVE_PATH>

Arguments:
  <ARCHIVE_PATH>  Path to the Project Gutenberg .tar.bz2 archive

Options:
  -o, --output <OUTPUT>          Output file path [default: filtered_ebooks.json]
  -m, --mirror <MIRROR>          Mirror key or base URL [default: gutenberg]
      --max-results <MAX_RESULTS> Maximum number of matched ebooks to output
  -c, --chunk-size <CHUNK_SIZE>  Number of items per chunk file
  -h, --help                     Print help
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
Extract metadata from `rdf-files.tar.bz2` to `filtered_ebooks.json`:
```bash
./target/release/gutenberg_parser rdf-files.tar.bz2
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
    "title": "The Adventures of Sherlock Holmes",
    "issued_date": "1999-03-01",
    "agents": [
      {
        "type": "author",
        "agent_id": 467,
        "name": "Doyle, Arthur Conan",
        "aliases": ["Doyle, A. Conan"],
        "webpage": "https://en.wikipedia.org/wiki/Arthur_Conan_Doyle",
        "birth_date": "1859",
        "death_date": "1930"
      }
    ],
    "description": "A collection of Sherlock Holmes short stories, originally published in The Strand Magazine.",
    "language": "en",
    "formats": [
      {
        "type": "text/html",
        "url": "https://www.gutenberg.org/files/1661/1661-h/1661-h.htm"
      },
      {
        "type": "application/epub+zip",
        "url": "https://www.gutenberg.org/ebooks/1661.epub3.images"
      }
    ],
    "taxonomy": {
      "domain": "Literature & Fiction",
      "genres": [
        "Fiction & Novels",
        "Mystery & Crime"
      ],
      "topics": [
        {
          "heading": "Holmes, Sherlock (Fictional character)",
          "subtopics": ["Detective and mystery stories"]
        }
      ]
    },
    "downloads": 14520,
    "ebook_id": "1661",
    "cover_image": "https://www.gutenberg.org/cache/epub/1661/pg1661.cover.medium.jpg",
    "license": "Public domain in the USA."
  }
]
```