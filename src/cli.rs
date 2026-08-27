//! CLI Pipeline & Multi-Threaded Orchestration
//! ---------------------------------------------------------
//! This module defines the command-line interface (`Args`), the archive
//! download logic (`download_rdf_archive`), chunked JSON serialization
//! (`write_chunk`, `get_chunk_path`), and the main pipeline entry point
//! (`run`).
//!
//! ## Pipeline Architecture
//! 1. **Argument Parsing**: `clap` parses `Args` and validates conflicts.
//! 2. **Archive Acquisition**: Either loads an existing `.tar.bz2` or
//!    downloads `rdf-files.tar.bz2` from the Gutenberg mirror.
//! 3. **Producer Thread**: Decompresses `bz2` stream and feeds raw XML
//!    buffers into a bounded `crossbeam_channel`.
//! 4. **Worker Pool**: Spawns `available_parallelism()` threads; each
//!    consumes raw XML buffers, calls `process_rdf_xml`, and pushes
//!    parsed `Ebook` objects to a second bounded channel.
//! 5. **Consumer Thread**: Collects `Ebook` objects into chunks, writes
//!    JSON arrays (with optional gzip compression), and optionally applies
//!    the `bridge` schema conversion (`BridgeEbook`).
//!
//! ## Key Data Flows
//! - `raw_tx` / `raw_rx`: `Vec<u8>` (XML buffers)
//! - `parsed_tx` / `parsed_rx`: `Ebook` (structured metadata)

use crate::config::*;
use crate::models::*;
use bzip2::read::BzDecoder;
use clap::Parser;
use crossbeam_channel::{bounded, Receiver, Sender};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::time::Instant;
use tar::Archive;
use tempfile::NamedTempFile;

use crate::xml_parser::*;
/// Command-line arguments for the Gutenberg archive extractor.
///
/// The parser enforces mutual exclusion between `archive_path` and
/// `--download`, and provides optional chunking, result limits,
/// mirror selection, bridge-mode schema conversion, and licensed-content
/// inclusion.
#[derive(Parser, Debug)]
#[command(author, version, about = "Ultra-fast Multi-threaded Gutenberg Archive Extractor")]
pub struct Args {
    /// Path to the Project Gutenberg `.tar.bz2` archive.
    /// Required unless `--download` is specified.
    #[arg(
        required_unless_present = "download",
        conflicts_with = "download",
        help = "Path to the Project Gutenberg .tar.bz2 archive (required unless --download is used)"
    )]
    archive_path: Option<String>,

    /// Output JSON file path.
    #[arg(short, long, default_value = "filtered_ebooks.json")]
    output: String,

    /// Mirror base URL key (`gutenberg`, `pglaf`, `odu`, etc.).
    #[arg(short, long, default_value = "gutenberg")]
    mirror: String,

    /// Maximum number of matched ebooks to emit.
    #[arg(long)]
    max_results: Option<usize>,

    /// Chunk size for streaming JSON output.
    #[arg(short, long)]
    chunk_size: Option<usize>,

    /// Rename output fields to match the target database schema.
    #[arg(
        long,
        help = "Rename output object fields to match the target database schema (alt-target-schema.md)"
    )]
    bridge: bool,

    /// Include non-Public-Domain (copyrighted / licensed) ebooks.
    #[arg(
        long,
        help = "Also include ebooks that are NOT Public Domain (copyrighted or otherwise licensed)"
    )]
    include_licensed: bool,

    /// Automatically download the RDF archive, parse it, and delete it.
    #[arg(
        long,
        help = "Automatically download rdf-files.tar.bz2 from Project Gutenberg, parse it, then delete the archive afterwards"
    )]
    download: bool,
}

/// Writes a sorted array of `Ebook` objects to JSON, optionally compressed.
///
/// # Arguments
/// * `data` — Mutable slice of `Ebook` objects to serialize.
/// * `path` — Destination file path; `.gz` suffix triggers gzip encoding.
/// * `bridge` — When `true`, converts each `Ebook` to `BridgeEbook` before
///   serialization, aligning field names with the external target schema.
///
/// # Sorting
/// Records are sorted deterministically by numeric `ebook_id` (falling back
/// to `u64::MAX` for non-numeric IDs) to ensure reproducible chunk order.
pub fn write_chunk(data: &mut [Ebook], path: &str, bridge: bool) -> std::io::Result<()> {
    // Sort deterministically by numeric ebook_id
    data.sort_by_key(|e| e.ebook_id.parse::<u64>().unwrap_or(u64::MAX));

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    // Inner writer that serializes the array with optional bridge mapping.
    let write_array = |w: &mut dyn Write| -> std::io::Result<()> {
        w.write_all(b"[\n")?;
        let total = data.len();
        for (i, ebook) in data.iter().enumerate() {
            if bridge {
                serde_json::to_writer(&mut *w, &BridgeEbook::from(ebook))?;
            } else {
                serde_json::to_writer(&mut *w, ebook)?;
            }
            if i + 1 < total {
                w.write_all(b",\n")?;
            } else {
                w.write_all(b"\n")?;
            }
        }
        w.write_all(b"]\n")?;
        Ok(())
    };

    if path.ends_with(".gz") {
        let mut encoder = GzEncoder::new(writer, Compression::default());
        write_array(&mut encoder)?;
        encoder.finish()?;
    } else {
        write_array(&mut writer)?;
        writer.flush()?;
    }
    Ok(())
}

/// Generates a chunked file name based on the base output path and index.
///
/// Handles `.json.gz` suffix stripping, then inserts the chunk index before
/// the extension (or before the file stem when no extension exists).
pub fn get_chunk_path(base_path: &str, chunk_index: usize) -> String {
    if let Some(stripped) = base_path.strip_suffix(".json.gz") {
        return format!("{}_{}.json.gz", stripped, chunk_index);
    }
    let path = std::path::Path::new(base_path);
    let parent = path.parent().unwrap_or(std::path::Path::new(""));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        parent
            .join(format!("{}_{}.{}", stem, chunk_index, ext))
            .to_string_lossy()
            .to_string()
    } else {
        parent
            .join(format!("{}_{}", stem, chunk_index))
            .to_string_lossy()
            .to_string()
    }
}

/// Downloads the RDF feed archive from the Gutenberg mirror.
///
/// Streams the response to a temporary `.tar.bz2` file with progress
/// reporting every 2 seconds. Returns the `NamedTempFile` handle, which
/// will be cleaned up by the OS when the variable goes out of scope (or
/// explicitly deleted in `run()` after parsing).
///
/// # Errors
/// Returns `Err(String)` for network failures, non-200 HTTP status,
/// or temporary-file I/O errors.
pub fn download_rdf_archive() -> Result<NamedTempFile, String> {
    let start = Instant::now();
    println!("[INFO] Downloading rdf-files.tar.bz2 from {}", RDF_FEED_URL);

    let mut response = ureq::get(RDF_FEED_URL)
        .call()
        .map_err(|e| format!("Failed to download archive: {}", e))?;

    if response.status() != 200 {
        return Err(format!(
            "Failed to download archive: HTTP {} from {}",
            response.status(),
            RDF_FEED_URL
        ));
    }

    let total_bytes = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    let mut temp_file = tempfile::Builder::new()
        .prefix("rdf-files-")
        .suffix(".tar.bz2")
        .tempfile_in(".")
        .map_err(|e| format!("Failed to create temporary archive file: {}", e))?;

    let mut reader = response.body_mut().as_reader();
    let mut buffer = [0u8; 128 * 1024];
    let mut downloaded: u64 = 0;
    let mut last_progress = Instant::now();

    loop {
        let n = reader
            .read(&mut buffer)
            .map_err(|e| format!("Failed reading download stream: {}", e))?;
        if n == 0 {
            break;
        }
        temp_file
            .write_all(&buffer[..n])
            .map_err(|e| format!("Failed writing archive to disk: {}", e))?;
        downloaded += n as u64;

        if last_progress.elapsed().as_secs() >= 2 {
            let progress = match total_bytes {
                Some(total) if total > 0 => format!(
                    "{:.1} / {:.1} MB",
                    downloaded as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0
                ),
                _ => format!("{:.1} MB", downloaded as f64 / 1_048_576.0),
            };
            println!("[INFO] Downloaded {}", progress);
            last_progress = Instant::now();
        }
    }

    temp_file
        .flush()
        .map_err(|e| format!("Failed to flush archive to disk: {}", e))?;

    println!(
        "[INFO] Download complete: {:.1} MB in {:.2}s -> {}",
        downloaded as f64 / 1_048_576.0,
        start.elapsed().as_secs_f64(),
        temp_file.path().display()
    );

    Ok(temp_file)
}

/// Main pipeline execution: parses arguments, acquires archive, spawns
/// producer / worker / consumer threads, writes output chunks, and reports
/// timing statistics.
///
/// ## Threading Model
/// - **Producer (1)**: Decompresses `bz2` archive and sends raw XML buffers.
/// - **Workers (N)**: `available_parallelism()` threads; parse XML and send
///   `Ebook` objects.
/// - **Consumer (1)**: Collects ebooks into chunks and writes JSON files.
///
/// The `parsed_tx` sender is explicitly dropped (`drop(parsed_tx)`) after
/// spawning workers to close the remaining reference, ensuring the consumer
/// exits cleanly when all workers have finished.
pub fn run() {
    let args = Args::parse();
    let start_time = Instant::now();

    let mirror_base = GUTENBERG_MIRRORS
        .get(args.mirror.to_lowercase().as_str())
        .unwrap_or(&args.mirror.as_str())
        .to_string();

    let (_downloaded_archive, archive_path) = if args.download {
        let temp = download_rdf_archive().unwrap_or_else(|e| {
            eprintln!("[ERROR] {}", e);
            std::process::exit(1);
        });
        let path = temp.path().to_path_buf();
        (Some(temp), path)
    } else {
        (None, PathBuf::from(args.archive_path.clone().unwrap()))
    };

    println!("[INFO] Opening archive: {}", archive_path.display());
    if args.download {
        println!("[INFO] Downloaded archive will be deleted after parsing");
    }
    println!("[INFO] Using mirror base: {}", mirror_base);
    if args.bridge {
        println!("[INFO] Bridge output mode enabled: field names match the target database schema");
    }
    if args.include_licensed {
        println!("[INFO] Including non-Public-Domain (licensed) ebooks");
    } else {
        println!("[INFO] Filtering to Public Domain ebooks only");
    }

    // Bounded channels link producer → workers → consumer.
    let (raw_tx, raw_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = bounded(2048);
    let (parsed_tx, parsed_rx): (Sender<Ebook>, Receiver<Ebook>) = bounded(2048);

    // Producer Thread: Single-pass bz2 stream decompression
    let producer_archive_path = archive_path.clone();
    std::thread::spawn(move || {
        let file = File::open(&producer_archive_path).expect("Failed to open archive file");
        let buf_reader = BufReader::with_capacity(1024 * 1024, file);
        let bz_decoder = BzDecoder::new(buf_reader);
        let mut archive = Archive::new(bz_decoder);

        if let Ok(entries) = archive.entries() {
            for entry in entries.flatten() {
                let name = entry.path().unwrap_or_default().to_string_lossy().to_string();
                if name.ends_with(".xml") || name.ends_with(".rdf") {
                    let mut buffer = Vec::new();
                    let mut entry = entry;
                    if entry.read_to_end(&mut buffer).is_ok() && raw_tx.send(buffer).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Worker Threads (std::thread Pool)
    let num_workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    println!("[INFO] Spawning {} parallel worker threads", num_workers);

    for _ in 0..num_workers {
        let rx = raw_rx.clone();
        let tx = parsed_tx.clone();
        let mirror = mirror_base.clone();
        let include_licensed = args.include_licensed;

        std::thread::spawn(move || {
            while let Ok(raw_bytes) = rx.recv() {
                if let Ok(ebook) = process_rdf_xml(&raw_bytes, &mirror, include_licensed) {
                    if tx.send(ebook).is_err() {
                        break;
                    }
                }
            }
        });
    }
    drop(parsed_tx); // Close remaining producer reference

    // Consumer Thread: Streaming Output Collector & Writer
    let mut current_chunk = Vec::new();
    let mut chunk_index = 1;
    let mut total_matched = 0;

    while let Ok(ebook) = parsed_rx.recv() {
        current_chunk.push(ebook);
        total_matched += 1;

        if let Some(c_size) = args.chunk_size {
            if current_chunk.len() >= c_size {
                let path = get_chunk_path(&args.output, chunk_index);
                write_chunk(&mut current_chunk, &path, args.bridge).expect("Failed to write chunk");
                println!(
                    "[INFO] Flushed chunk {} ({} items) -> {}",
                    chunk_index,
                    current_chunk.len(),
                    path
                );
                chunk_index += 1;
                current_chunk.clear();
            }
        }

        if let Some(max) = args.max_results {
            if total_matched >= max {
                break;
            }
        }
    }

    if !current_chunk.is_empty() {
        if args.chunk_size.is_some() {
            let path = get_chunk_path(&args.output, chunk_index);
            write_chunk(&mut current_chunk, &path, args.bridge).expect("Failed to write final chunk");
            println!(
                "[INFO] Flushed final chunk {} ({} items) -> {}",
                chunk_index,
                current_chunk.len(),
                path
            );
        } else {
            write_chunk(&mut current_chunk, &args.output, args.bridge).expect("Failed to write output");
            println!("[INFO] Wrote {} matched items -> {}", total_matched, args.output);
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    println!(
        "[INFO] Pipeline complete in {:.2}s. Total Matched: {}",
        elapsed, total_matched
    );

    if args.download {
        println!("[INFO] Deleting downloaded archive: {}", archive_path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms that `get_chunk_path` inserts the chunk index into the
    /// output file stem.
    #[test]
    fn chunk_path_generates_suffix() {
        assert!(get_chunk_path("out.json", 1).contains("_1"));
    }
}
