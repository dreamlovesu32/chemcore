use chemsema_container::{
    write_ccjz_reusing, write_ccjz_with_files, CcjzReader, DecodeLimits, FileAttachment,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::time::Instant;

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn document(scene_count: usize, resource: Option<Value>) -> String {
    let scene = (0..scene_count)
        .map(|index| json!({"id": format!("obj_{index:09}"), "type": "text"}))
        .collect::<Vec<_>>();
    let roots = (0..scene_count)
        .map(|index| format!("obj_{index:09}"))
        .collect::<Vec<_>>();
    let mut resources = serde_json::Map::new();
    if let Some(resource) = resource {
        resources.insert("large".to_string(), resource);
    }
    json!({
        "format": {"name": "chemsema", "version": "0.2", "unit": "pt", "profile": "snapshot"},
        "document": {"id": "performance", "title": "CCJZ performance"},
        "entities": {"scene": scene},
        "hierarchy": {"roots": roots},
        "resources": resources,
    })
    .to_string()
}

fn main() -> Result<(), String> {
    let full = std::env::var("CHEMSEMA_CCJZ_FULL").ok().as_deref() == Some("1");
    let scene_counts = if full {
        vec![10_000usize, 100_000, 1_000_000]
    } else {
        vec![10_000]
    };
    let attachment_sizes = if full {
        vec![10u64 << 20, 100u64 << 20, 1u64 << 30]
    } else {
        vec![10u64 << 20]
    };
    let directory =
        std::env::temp_dir().join(format!("chemsema-ccjz-performance-{}", std::process::id()));
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let result = (|| {
        let mut scene_results = Vec::new();
        for count in scene_counts {
            let source = document(count, None);
            let archive_path = directory.join(format!("scene-{count}.ccjz"));
            let start = Instant::now();
            let mut output = File::create(&archive_path).map_err(|error| error.to_string())?;
            write_ccjz_with_files(&mut output, &source, 1024, &[], &[])?;
            output.sync_all().map_err(|error| error.to_string())?;
            let write_ms = elapsed_ms(start);
            let allowed_ms = 5_000.0 + count as f64 * 0.08;
            if write_ms > allowed_ms {
                return Err(format!(
                    "CCJZ {count}-entity write took {write_ms:.1} ms; limit is {allowed_ms:.1} ms"
                ));
            }
            let start = Instant::now();
            let mut reader = CcjzReader::open(
                File::open(&archive_path).map_err(|error| error.to_string())?,
                DecodeLimits::default(),
            )?;
            let open_ms = elapsed_ms(start);
            let start = Instant::now();
            let first_chunk_bytes = reader.read_scene_chunk(0)?.len();
            let first_chunk_ms = elapsed_ms(start);
            if open_ms > 1_000.0 || first_chunk_ms > 1_000.0 {
                return Err(format!(
                    "CCJZ lazy read exceeded 1000 ms: manifest={open_ms:.1}, chunk={first_chunk_ms:.1}"
                ));
            }
            let edited = source.replacen(
                &format!(r#""id":"obj_{:09}","type":"text""#, count - 1),
                &format!(r#""id":"obj_{:09}","type":"note""#, count - 1),
                1,
            );
            if edited == source {
                return Err("CCJZ copy-on-write fixture edit did not apply".to_string());
            }
            let rewritten_path = directory.join(format!("scene-{count}-edited.ccjz"));
            let mut previous = CcjzReader::open(
                File::open(&archive_path).map_err(|error| error.to_string())?,
                DecodeLimits::default(),
            )?;
            let mut rewritten = File::create(&rewritten_path).map_err(|error| error.to_string())?;
            let start = Instant::now();
            let reuse = write_ccjz_reusing(&mut previous, &mut rewritten, &edited, 1024, &[], &[])?;
            rewritten.sync_all().map_err(|error| error.to_string())?;
            let copy_on_write_ms = elapsed_ms(start);
            let reuse_ratio = reuse.reused_bytes as f64
                / (reuse.reused_bytes + reuse.written_bytes).max(1) as f64;
            if reuse.reused_entries == 0
                || reuse.written_entries == 0
                || reuse_ratio < 0.70
                || copy_on_write_ms > allowed_ms
            {
                return Err(format!(
                    "CCJZ copy-on-write gate failed: {reuse:?}, ratio={reuse_ratio:.3}, time={copy_on_write_ms:.1} ms"
                ));
            }
            scene_results.push(json!({
                "entities": count,
                "sourceBytes": source.len(),
                "archiveBytes": fs::metadata(&archive_path).map_err(|error| error.to_string())?.len(),
                "writeMs": write_ms,
                "manifestOpenMs": open_ms,
                "firstChunkMs": first_chunk_ms,
                "firstChunkBytes": first_chunk_bytes,
                "copyOnWriteMs": copy_on_write_ms,
                "reusedEntries": reuse.reused_entries,
                "writtenEntries": reuse.written_entries,
                "reusedBytes": reuse.reused_bytes,
                "writtenBytes": reuse.written_bytes,
                "reuseRatio": reuse_ratio,
            }));
        }

        let mut attachment_results = Vec::new();
        for size in attachment_sizes {
            let payload_path = directory.join(format!("payload-{size}.bin"));
            let payload = File::create(&payload_path).map_err(|error| error.to_string())?;
            payload.set_len(size).map_err(|error| error.to_string())?;
            drop(payload);
            let hash = hash_file(&payload_path)?;
            let source = document(
                0,
                Some(json!({
                    "type": "opaque-array",
                    "encoding": "opaque",
                    "data": {
                        "storage": "ccjz-attachment",
                        "mediaType": "application/octet-stream",
                        "byteLength": size,
                        "sha256": hash,
                    }
                })),
            );
            let archive_path = directory.join(format!("attachment-{size}.ccjz"));
            let start = Instant::now();
            let mut output = File::create(&archive_path).map_err(|error| error.to_string())?;
            write_ccjz_with_files(
                &mut output,
                &source,
                1024,
                &[],
                &[FileAttachment {
                    id: "large",
                    media_type: "application/octet-stream",
                    extension: "bin",
                    path: &payload_path,
                }],
            )?;
            output.sync_all().map_err(|error| error.to_string())?;
            let write_ms = elapsed_ms(start);
            let minimum_mib_per_second = 20.0;
            let throughput = (size as f64 / (1u64 << 20) as f64) / (write_ms / 1000.0);
            if throughput < minimum_mib_per_second {
                return Err(format!(
                    "CCJZ attachment throughput was {throughput:.1} MiB/s; minimum is {minimum_mib_per_second:.1} MiB/s"
                ));
            }
            let start = Instant::now();
            let mut reader = CcjzReader::open(
                File::open(&archive_path).map_err(|error| error.to_string())?,
                DecodeLimits::default(),
            )?;
            let open_ms = elapsed_ms(start);
            let range = reader.attachment_range("large")?;
            if range.size != size || open_ms > 1_000.0 {
                return Err("CCJZ large attachment range gate failed".to_string());
            }
            attachment_results.push(json!({
                "bytes": size,
                "archiveBytes": fs::metadata(&archive_path).map_err(|error| error.to_string())?.len(),
                "writeMs": write_ms,
                "throughputMiBPerSecond": throughput,
                "manifestOpenMs": open_ms,
                "storedRangeBytes": range.size,
            }));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "schema": "chemsema.ccjz-performance.v1",
                "mode": if full { "full" } else { "smoke" },
                "scene": scene_results,
                "attachments": attachment_results,
            }))
            .map_err(|error| error.to_string())?
        );
        Ok(())
    })();
    let _ = fs::remove_dir_all(&directory);
    result
}
