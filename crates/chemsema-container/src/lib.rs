//! CCJZ is a deterministic ZIP container around the CCJS semantic document.
//! The container is deliberately independent of the chemistry engine: readers
//! can verify and assemble a document before asking an engine to interpret it.

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;

mod journal;
pub use journal::{Journal, JournalRead, JournalRecord, JOURNAL_SCHEMA};

pub const MIMETYPE: &str = "application/vnd.chemsema.document+zip";
pub const CONTAINER_SCHEMA: &str = "chemsema.container.v1";
pub const DEFAULT_SCENE_CHUNK_RECORDS: usize = 1024;

const MIMETYPE_PATH: &str = "mimetype";
const MANIFEST_PATH: &str = "manifest.json";
const ROOT_PATH: &str = "document/root.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub media_type: String,
    pub document_format: String,
    pub root: EntryDescriptor,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene_chunks: Vec<ChunkDescriptor>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resources: BTreeMap<String, EntryDescriptor>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attachments: BTreeMap<String, EntryDescriptor>,
}

#[derive(Debug, Clone, Copy)]
pub struct Attachment<'a> {
    pub id: &'a str,
    pub media_type: &'a str,
    pub extension: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct FileAttachment<'a> {
    pub id: &'a str,
    pub media_type: &'a str,
    pub extension: &'a str,
    pub path: &'a Path,
}

enum EntryPayload<'a> {
    Owned(Vec<u8>),
    Borrowed(&'a [u8]),
    File(&'a Path),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntryDescriptor {
    pub path: String,
    pub media_type: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChunkDescriptor {
    #[serde(flatten)]
    pub entry: EntryDescriptor,
    pub first_record: u64,
    pub record_count: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DecodeLimits {
    pub max_entries: usize,
    pub max_entry_bytes: u64,
    pub max_total_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredEntryRange {
    pub data_offset: u64,
    pub size: u64,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_entry_bytes: 2 * 1024 * 1024 * 1024,
            max_total_bytes: 4 * 1024 * 1024 * 1024,
        }
    }
}

/// Seek-based reader for inspecting or loading individual CCJZ entries without
/// assembling the complete semantic document in memory.
pub struct CcjzReader<R: Read + std::io::Seek> {
    archive: zip::ZipArchive<R>,
    manifest: Manifest,
    limits: DecodeLimits,
}

impl<R: Read + std::io::Seek> CcjzReader<R> {
    pub fn open(reader: R, limits: DecodeLimits) -> Result<Self, String> {
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|error| format!("Failed to open CCJZ ZIP container: {error}"))?;
        validate_archive_directory(&mut archive, &limits)?;
        if archive.len() == 0
            || archive
                .by_index(0)
                .map_err(|error| error.to_string())?
                .name()
                != MIMETYPE_PATH
        {
            return Err("CCJZ mimetype must be the first ZIP entry".to_string());
        }
        let mimetype = read_archive_entry(&mut archive, MIMETYPE_PATH, &limits)?;
        if mimetype != MIMETYPE.as_bytes() {
            return Err("CCJZ mimetype entry is not canonical".to_string());
        }
        let manifest_bytes = read_archive_entry(&mut archive, MANIFEST_PATH, &limits)?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| format!("Invalid CCJZ manifest: {error}"))?;
        validate_manifest_header(&manifest)?;
        validate_manifest_directory(&mut archive, &manifest)?;
        Ok(Self {
            archive,
            manifest,
            limits,
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn read_root(&mut self) -> Result<Vec<u8>, String> {
        let descriptor = self.manifest.root.clone();
        self.read_descriptor(&descriptor)
    }

    pub fn read_scene_chunk(&mut self, index: usize) -> Result<Vec<u8>, String> {
        let descriptor = self
            .manifest
            .scene_chunks
            .get(index)
            .ok_or_else(|| format!("CCJZ scene chunk index {index} is out of range"))?
            .entry
            .clone();
        self.read_descriptor(&descriptor)
    }

    pub fn read_resource(&mut self, id: &str) -> Result<Vec<u8>, String> {
        let descriptor = self
            .manifest
            .resources
            .get(id)
            .ok_or_else(|| format!("Unknown CCJZ resource id '{id}'"))?
            .clone();
        self.read_descriptor(&descriptor)
    }

    pub fn read_attachment(&mut self, id: &str) -> Result<Vec<u8>, String> {
        let descriptor = self
            .manifest
            .attachments
            .get(id)
            .ok_or_else(|| format!("Unknown CCJZ attachment id '{id}'"))?
            .clone();
        self.read_descriptor(&descriptor)
    }

    /// Return the byte range of a stored attachment for range-based consumers
    /// such as HDF5/Zarr readers. The descriptor hash remains the integrity
    /// authority; callers that need full verification call read_attachment.
    pub fn attachment_range(&mut self, id: &str) -> Result<StoredEntryRange, String> {
        let descriptor = self
            .manifest
            .attachments
            .get(id)
            .ok_or_else(|| format!("Unknown CCJZ attachment id '{id}'"))?;
        let entry = self
            .archive
            .by_name(&descriptor.path)
            .map_err(|error| format!("Missing CCJZ entry {}: {error}", descriptor.path))?;
        if entry.compression() != zip::CompressionMethod::Stored || entry.size() != descriptor.size
        {
            return Err(format!(
                "CCJZ attachment {} is not a canonical stored entry",
                descriptor.path
            ));
        }
        Ok(StoredEntryRange {
            data_offset: entry.data_start(),
            size: entry.size(),
        })
    }

    pub fn read_descriptor(&mut self, descriptor: &EntryDescriptor) -> Result<Vec<u8>, String> {
        read_verified_entry(&mut self.archive, descriptor, &self.limits)
    }
}

/// Encode a CCJS snapshot as deterministic, stored ZIP entries. Stored entries
/// trade archive-level compression for byte stability and true range access;
/// large resource payloads may carry their own compression.
pub fn encode_ccjz(document_json: &str) -> Result<Vec<u8>, String> {
    encode_ccjz_with_attachments(document_json, DEFAULT_SCENE_CHUNK_RECORDS, &[])
}

pub fn encode_ccjz_with_chunk_size(
    document_json: &str,
    scene_chunk_records: usize,
) -> Result<Vec<u8>, String> {
    encode_ccjz_with_attachments(document_json, scene_chunk_records, &[])
}

pub fn encode_ccjz_with_attachments(
    document_json: &str,
    scene_chunk_records: usize,
    attachments: &[Attachment<'_>],
) -> Result<Vec<u8>, String> {
    let mut output = Cursor::new(Vec::new());
    write_ccjz(&mut output, document_json, scene_chunk_records, attachments)?;
    Ok(output.into_inner())
}

pub fn write_ccjz<W: Write + std::io::Seek>(
    writer: &mut W,
    document_json: &str,
    scene_chunk_records: usize,
    attachments: &[Attachment<'_>],
) -> Result<(), String> {
    write_ccjz_with_files(writer, document_json, scene_chunk_records, attachments, &[])
}

pub fn write_ccjz_with_files<W: Write + std::io::Seek>(
    writer: &mut W,
    document_json: &str,
    scene_chunk_records: usize,
    attachments: &[Attachment<'_>],
    file_attachments: &[FileAttachment<'_>],
) -> Result<(), String> {
    if scene_chunk_records == 0 {
        return Err("CCJZ scene chunk size must be greater than zero".to_string());
    }
    let mut root: Value = serde_json::from_str(document_json)
        .map_err(|error| format!("CCJZ source is not valid JSON: {error}"))?;
    validate_ccjs_header(&root)?;

    let scene = root
        .pointer_mut("/entities/scene")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "CCJS v0.2 requires entities.scene to be an array".to_string())?;
    let scene = std::mem::take(scene);
    let resources = root
        .get_mut("resources")
        .and_then(Value::as_object_mut)
        .map(std::mem::take)
        .unwrap_or_default();
    let attachment_ids = attachments
        .iter()
        .map(|attachment| attachment.id)
        .chain(file_attachments.iter().map(|attachment| attachment.id))
        .collect::<BTreeSet<_>>();
    if attachment_ids.len() != attachments.len() + file_attachments.len() {
        return Err("CCJZ attachment ids must be unique".to_string());
    }

    let mut entries = BTreeMap::<String, EntryPayload<'_>>::new();

    let mut chunks = Vec::new();
    for (index, records) in scene.chunks(scene_chunk_records).enumerate() {
        let path = format!("entities/scene-{index:06}.jsonl");
        let mut bytes = Vec::new();
        for record in records {
            bytes.extend(canonical_json_bytes(record)?);
            bytes.push(b'\n');
        }
        chunks.push(ChunkDescriptor {
            entry: descriptor(&path, "application/x-ndjson", &bytes),
            first_record: (index * scene_chunk_records) as u64,
            record_count: records.len() as u64,
        });
        entries.insert(path, EntryPayload::Owned(bytes));
    }

    let mut resource_descriptors = BTreeMap::new();
    for (id, value) in resources {
        if attachment_ids.contains(id.as_str()) {
            root.get_mut("resources")
                .and_then(Value::as_object_mut)
                .expect("resources was an object")
                .insert(id, value);
            continue;
        }
        let bytes = canonical_json_bytes(&value)?;
        let hash = sha256_hex(&bytes);
        let path = format!("resources/{hash}.json");
        resource_descriptors.insert(
            id,
            descriptor(&path, "application/vnd.chemsema.resource+json", &bytes),
        );
        entries.entry(path).or_insert(EntryPayload::Owned(bytes));
    }

    let mut attachment_descriptors = BTreeMap::new();
    for attachment in attachments {
        let resource = root
            .get("resources")
            .and_then(Value::as_object)
            .and_then(|resources| resources.get(attachment.id))
            .ok_or_else(|| {
                format!(
                    "CCJZ attachment '{}' requires a matching document resource descriptor",
                    attachment.id
                )
            })?;
        let extension = normalize_extension(attachment.extension)?;
        let hash = sha256_hex(attachment.bytes);
        validate_attachment_resource(
            resource,
            attachment.id,
            attachment.media_type,
            attachment.bytes.len() as u64,
            &hash,
        )?;
        let path = format!("resources/{hash}.{extension}");
        attachment_descriptors.insert(
            attachment.id.to_string(),
            descriptor(&path, attachment.media_type, attachment.bytes),
        );
        entries
            .entry(path)
            .or_insert(EntryPayload::Borrowed(attachment.bytes));
    }
    for attachment in file_attachments {
        let resource = root
            .get("resources")
            .and_then(Value::as_object)
            .and_then(|resources| resources.get(attachment.id))
            .ok_or_else(|| {
                format!(
                    "CCJZ attachment '{}' requires a matching document resource descriptor",
                    attachment.id
                )
            })?;
        let extension = normalize_extension(attachment.extension)?;
        let (size, hash) = hash_file(attachment.path)?;
        validate_attachment_resource(resource, attachment.id, attachment.media_type, size, &hash)?;
        let path = format!("resources/{hash}.{extension}");
        attachment_descriptors.insert(
            attachment.id.to_string(),
            EntryDescriptor {
                path: path.clone(),
                media_type: attachment.media_type.to_string(),
                sha256: hash,
                size,
            },
        );
        entries
            .entry(path)
            .or_insert(EntryPayload::File(attachment.path));
    }

    let root_bytes = canonical_json_bytes(&root)?;
    entries.insert(
        ROOT_PATH.to_string(),
        EntryPayload::Owned(root_bytes.clone()),
    );
    let root_descriptor = descriptor(ROOT_PATH, "application/json", &root_bytes);

    let manifest = Manifest {
        schema: CONTAINER_SCHEMA.to_string(),
        media_type: MIMETYPE.to_string(),
        document_format: "chemsema/0.2".to_string(),
        root: root_descriptor,
        scene_chunks: chunks,
        resources: resource_descriptors,
        attachments: attachment_descriptors,
    };
    let manifest_bytes =
        canonical_json_bytes(&serde_json::to_value(&manifest).map_err(|error| error.to_string())?)?;

    {
        let mut zip = zip::ZipWriter::new(writer);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        write_entry(&mut zip, options, MIMETYPE_PATH, MIMETYPE.as_bytes())?;
        write_entry(&mut zip, options, MANIFEST_PATH, &manifest_bytes)?;
        for (path, bytes) in entries {
            match bytes {
                EntryPayload::Owned(bytes) => write_entry(&mut zip, options, &path, &bytes)?,
                EntryPayload::Borrowed(bytes) => write_entry(&mut zip, options, &path, bytes)?,
                EntryPayload::File(file_path) => {
                    write_file_entry(&mut zip, options, &path, file_path)?
                }
            }
        }
        zip.finish()
            .map_err(|error| format!("Failed to finish CCJZ container: {error}"))?;
    }
    Ok(())
}

/// Decode either the current ZIP container or a legacy gzip `.ccjz`.
pub fn decode_ccjz(bytes: &[u8]) -> Result<String, String> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(bytes);
        let mut text = String::new();
        decoder
            .read_to_string(&mut text)
            .map_err(|error| format!("Failed to decompress legacy gzip CCJZ: {error}"))?;
        return Ok(text);
    }
    decode_zip_ccjz(bytes, DecodeLimits::default())
}

pub fn decode_zip_ccjz(bytes: &[u8], limits: DecodeLimits) -> Result<String, String> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| format!("Failed to open CCJZ ZIP container: {error}"))?;
    validate_archive_directory(&mut archive, &limits)?;
    if archive.len() == 0
        || archive
            .by_index(0)
            .map_err(|error| error.to_string())?
            .name()
            != MIMETYPE_PATH
    {
        return Err("CCJZ mimetype must be the first ZIP entry".to_string());
    }
    let mimetype = read_archive_entry(&mut archive, MIMETYPE_PATH, &limits)?;
    if mimetype != MIMETYPE.as_bytes() {
        return Err("CCJZ mimetype entry is not canonical".to_string());
    }
    let manifest_bytes = read_archive_entry(&mut archive, MANIFEST_PATH, &limits)?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("Invalid CCJZ manifest: {error}"))?;
    validate_manifest_header(&manifest)?;
    validate_manifest_directory(&mut archive, &manifest)?;

    let root_bytes = read_verified_entry(&mut archive, &manifest.root, &limits)?;
    let mut root: Value = serde_json::from_slice(&root_bytes)
        .map_err(|error| format!("Invalid CCJZ document root: {error}"))?;
    let scene = root
        .pointer_mut("/entities/scene")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "CCJZ document root requires entities.scene".to_string())?;
    if !scene.is_empty() {
        return Err("CCJZ document root entities.scene must be empty".to_string());
    }
    let mut expected_first = 0u64;
    for chunk in &manifest.scene_chunks {
        if chunk.first_record != expected_first {
            return Err("CCJZ scene chunk record ranges are not contiguous".to_string());
        }
        let bytes = read_verified_entry(&mut archive, &chunk.entry, &limits)?;
        let mut count = 0u64;
        for (line_index, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
            if line.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_slice(line).map_err(|error| {
                format!(
                    "Invalid CCJZ scene JSONL record {}: {error}",
                    line_index + 1
                )
            })?;
            scene.push(value);
            count += 1;
        }
        if count != chunk.record_count {
            return Err(format!(
                "CCJZ scene chunk {} record count mismatch",
                chunk.entry.path
            ));
        }
        expected_first += count;
    }

    let resource_map = root
        .get_mut("resources")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "CCJZ document root requires resources".to_string())?;
    for (id, descriptor) in &manifest.resources {
        let bytes = read_verified_entry(&mut archive, descriptor, &limits)?;
        let value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Invalid CCJZ resource '{id}': {error}"))?;
        if resource_map.insert(id.clone(), value).is_some() {
            return Err(format!(
                "CCJZ resource '{id}' exists in both root and manifest resources"
            ));
        }
    }
    for (id, descriptor) in &manifest.attachments {
        let resource = resource_map.get(id).ok_or_else(|| {
            format!("CCJZ attachment '{id}' requires a matching root resource descriptor")
        })?;
        validate_attachment_resource(
            resource,
            id,
            &descriptor.media_type,
            descriptor.size,
            &descriptor.sha256,
        )?;
        read_verified_entry(&mut archive, descriptor, &limits)?;
    }
    validate_ccjs_header(&root)?;
    String::from_utf8(canonical_json_bytes(&root)?)
        .map_err(|error| format!("Canonical CCJS is not UTF-8: {error}"))
}

fn validate_archive_directory<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    limits: &DecodeLimits,
) -> Result<(), String> {
    if archive.len() > limits.max_entries {
        return Err(format!("CCJZ has too many entries: {}", archive.len()));
    }
    let mut names = BTreeSet::new();
    let mut folded_names = BTreeSet::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("Failed to inspect CCJZ entry: {error}"))?;
        validate_entry_name(entry.name())?;
        if entry.compression() != zip::CompressionMethod::Stored {
            return Err(format!(
                "CCJZ v1 entry must use stored compression: {}",
                entry.name()
            ));
        }
        if !names.insert(entry.name().to_string())
            || !folded_names.insert(entry.name().to_ascii_lowercase())
        {
            return Err(format!(
                "CCJZ contains a duplicate entry name: {}",
                entry.name()
            ));
        }
        if entry.size() > limits.max_entry_bytes {
            return Err(format!(
                "CCJZ entry is larger than the configured limit: {}",
                entry.name()
            ));
        }
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| "CCJZ expanded size overflow".to_string())?;
        if total > limits.max_total_bytes {
            return Err("CCJZ expanded size exceeds the configured limit".to_string());
        }
    }
    Ok(())
}

fn validate_manifest_header(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema != CONTAINER_SCHEMA
        || manifest.media_type != MIMETYPE
        || manifest.document_format != "chemsema/0.2"
    {
        return Err("Unsupported CCJZ manifest header".to_string());
    }
    Ok(())
}

fn validate_manifest_directory<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    manifest: &Manifest,
) -> Result<(), String> {
    let mut declared = BTreeSet::from([
        MIMETYPE_PATH.to_string(),
        MANIFEST_PATH.to_string(),
        manifest.root.path.clone(),
    ]);
    declared.extend(
        manifest
            .scene_chunks
            .iter()
            .map(|chunk| chunk.entry.path.clone()),
    );
    declared.extend(manifest.resources.values().map(|entry| entry.path.clone()));
    declared.extend(
        manifest
            .attachments
            .values()
            .map(|entry| entry.path.clone()),
    );
    for index in 0..archive.len() {
        let name = archive
            .by_index(index)
            .map_err(|error| format!("Failed to inspect CCJZ entry: {error}"))?
            .name()
            .to_string();
        if !declared.contains(&name) {
            return Err(format!("CCJZ contains an undeclared entry: {name}"));
        }
    }
    for path in declared {
        archive
            .by_name(&path)
            .map_err(|_| format!("CCJZ manifest declares a missing entry: {path}"))?;
    }
    Ok(())
}

pub fn inspect_manifest(bytes: &[u8]) -> Result<Manifest, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| format!("Failed to open CCJZ ZIP container: {error}"))?;
    validate_archive_directory(&mut archive, &DecodeLimits::default())?;
    let manifest = read_archive_entry(&mut archive, MANIFEST_PATH, &DecodeLimits::default())?;
    let manifest: Manifest = serde_json::from_slice(&manifest)
        .map_err(|error| format!("Invalid CCJZ manifest: {error}"))?;
    validate_manifest_header(&manifest)?;
    validate_manifest_directory(&mut archive, &manifest)?;
    Ok(manifest)
}

fn validate_ccjs_header(value: &Value) -> Result<(), String> {
    let name = value.pointer("/format/name").and_then(Value::as_str);
    let version = value.pointer("/format/version").and_then(Value::as_str);
    let unit = value.pointer("/format/unit").and_then(Value::as_str);
    let profile = value.pointer("/format/profile").and_then(Value::as_str);
    if name != Some("chemsema")
        || version != Some("0.2")
        || unit != Some("pt")
        || profile != Some("snapshot")
    {
        return Err("CCJZ requires canonical chemsema/0.2 pt snapshot input".to_string());
    }
    Ok(())
}

fn normalize_extension(value: &str) -> Result<String, String> {
    let extension = value.trim().trim_start_matches('.').to_ascii_lowercase();
    if extension.is_empty()
        || extension.len() > 16
        || !extension
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(format!("Unsafe CCJZ attachment extension '{value}'"));
    }
    Ok(extension)
}

fn validate_attachment_resource(
    resource: &Value,
    id: &str,
    media_type: &str,
    size: u64,
    hash: &str,
) -> Result<(), String> {
    let data = resource
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "CCJZ attachment resource '{}' requires a data descriptor object",
                id
            )
        })?;
    if data.get("sha256").and_then(Value::as_str) != Some(hash)
        || data.get("mediaType").and_then(Value::as_str) != Some(media_type)
        || data.get("byteLength").and_then(Value::as_u64) != Some(size)
        || data.get("storage").and_then(Value::as_str) != Some("ccjz-attachment")
    {
        return Err(format!(
            "CCJZ attachment resource '{}' descriptor does not match its payload",
            id
        ));
    }
    Ok(())
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&canonical_value(value)).map_err(|error| error.to_string())
}

fn canonical_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort();
            let mut result = Map::new();
            for key in keys {
                result.insert(key.clone(), canonical_value(&object[key]));
            }
            Value::Object(result)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_value).collect()),
        _ => value.clone(),
    }
}

fn descriptor(path: &str, media_type: &str, bytes: &[u8]) -> EntryDescriptor {
    EntryDescriptor {
        path: path.to_string(),
        media_type: media_type.to_string(),
        sha256: sha256_hex(bytes),
        size: bytes.len() as u64,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_entry<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    options: SimpleFileOptions,
    path: &str,
    bytes: &[u8],
) -> Result<(), String> {
    zip.start_file(path, options)
        .map_err(|error| format!("Failed to create CCJZ entry {path}: {error}"))?;
    zip.write_all(bytes)
        .map_err(|error| format!("Failed to write CCJZ entry {path}: {error}"))
}

fn write_file_entry<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    options: SimpleFileOptions,
    path: &str,
    file_path: &Path,
) -> Result<(), String> {
    zip.start_file(path, options)
        .map_err(|error| format!("Failed to create CCJZ entry {path}: {error}"))?;
    let mut file = std::fs::File::open(file_path).map_err(|error| {
        format!(
            "Failed to reopen CCJZ attachment {}: {error}",
            file_path.display()
        )
    })?;
    std::io::copy(&mut file, zip).map_err(|error| {
        format!(
            "Failed to stream CCJZ attachment {}: {error}",
            file_path.display()
        )
    })?;
    Ok(())
}

fn hash_file(path: &Path) -> Result<(u64, String), String> {
    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("Failed to open CCJZ attachment {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut size = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| {
            format!("Failed to hash CCJZ attachment {}: {error}", path.display())
        })?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        size = size
            .checked_add(count as u64)
            .ok_or_else(|| "CCJZ attachment size overflow".to_string())?;
    }
    let hash = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok((size, hash))
}

fn validate_entry_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || name.contains(':')
    {
        return Err(format!("Unsafe or non-canonical CCJZ entry name: {name}"));
    }
    Ok(())
}

fn read_verified_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    descriptor: &EntryDescriptor,
    limits: &DecodeLimits,
) -> Result<Vec<u8>, String> {
    validate_entry_name(&descriptor.path)?;
    let bytes = read_archive_entry(archive, &descriptor.path, limits)?;
    if bytes.len() as u64 != descriptor.size {
        return Err(format!("CCJZ entry {} size mismatch", descriptor.path));
    }
    if sha256_hex(&bytes) != descriptor.sha256 {
        return Err(format!("CCJZ entry {} SHA-256 mismatch", descriptor.path));
    }
    Ok(bytes)
}

fn read_archive_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    path: &str,
    limits: &DecodeLimits,
) -> Result<Vec<u8>, String> {
    let mut entry = archive
        .by_name(path)
        .map_err(|error| format!("Missing CCJZ entry {path}: {error}"))?;
    if entry.size() > limits.max_entry_bytes {
        return Err(format!(
            "CCJZ entry is larger than the configured limit: {path}"
        ));
    }
    let capacity = usize::try_from(entry.size())
        .map_err(|_| format!("CCJZ entry is too large for this platform: {path}"))?;
    let mut bytes = Vec::with_capacity(capacity);
    entry
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Failed to read CCJZ entry {path}: {error}"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};

    fn sample() -> String {
        serde_json::json!({
            "format": {"name": "chemsema", "version": "0.2", "unit": "pt", "profile": "snapshot"},
            "document": {"title": "container test"},
            "entities": {"scene": [
                {"id": "a", "type": "text", "z": 1},
                {"id": "b", "type": "text", "z": 2},
                {"id": "c", "type": "text", "z": 3}
            ]},
            "hierarchy": {"roots": ["a", "b", "c"]},
            "resources": {
                "second": {"encoding": "base64", "data": "BBBB"},
                "first": {"data": "AAAA", "encoding": "base64"}
            }
        })
        .to_string()
    }

    #[test]
    fn deterministic_round_trip_chunks_and_resources() {
        let first = encode_ccjz_with_chunk_size(&sample(), 2).unwrap();
        let second = encode_ccjz_with_chunk_size(&sample(), 2).unwrap();
        assert_eq!(first, second);
        assert_eq!(&first[..2], b"PK");
        let decoded: Value = serde_json::from_str(&decode_ccjz(&first).unwrap()).unwrap();
        let source: Value = serde_json::from_str(&sample()).unwrap();
        assert_eq!(decoded, source);
        let manifest = inspect_manifest(&first).unwrap();
        assert_eq!(manifest.scene_chunks.len(), 2);
        assert_eq!(manifest.resources.len(), 2);
        let mut reader = CcjzReader::open(Cursor::new(&first), DecodeLimits::default()).unwrap();
        assert_eq!(reader.manifest(), &manifest);
        assert!(!reader.read_root().unwrap().is_empty());
        assert!(!reader.read_scene_chunk(1).unwrap().is_empty());
        assert!(!reader.read_resource("first").unwrap().is_empty());
    }

    #[test]
    fn legacy_gzip_remains_readable() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(sample().as_bytes()).unwrap();
        let gzip = encoder.finish().unwrap();
        assert_eq!(decode_ccjz(&gzip).unwrap(), sample());
    }

    #[test]
    fn rejects_non_v02_input() {
        let old = sample().replace("\"0.2\"", "\"0.1\"");
        assert!(encode_ccjz(&old).unwrap_err().contains("chemsema/0.2"));
    }

    #[test]
    fn detects_tampered_content() {
        let encoded = encode_ccjz_with_chunk_size(&sample(), 2).unwrap();
        let mut input = zip::ZipArchive::new(Cursor::new(encoded)).unwrap();
        let mut files = Vec::new();
        for index in 0..input.len() {
            let mut entry = input.by_index(index).unwrap();
            let name = entry.name().to_string();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            if name == ROOT_PATH {
                bytes.push(b' ');
            }
            files.push((name, bytes));
        }
        drop(input);
        let mut output = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut output);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            for (name, bytes) in files {
                write_entry(&mut zip, options, &name, &bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        assert!(decode_ccjz(&output.into_inner())
            .unwrap_err()
            .contains("size mismatch"));
    }

    #[test]
    fn rejects_undeclared_or_compressed_entries() {
        let encoded = encode_ccjz(&sample()).unwrap();
        let mut input = zip::ZipArchive::new(Cursor::new(encoded)).unwrap();
        let mut files = Vec::new();
        for index in 0..input.len() {
            let mut entry = input.by_index(index).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            files.push((entry.name().to_string(), bytes));
        }
        drop(input);

        let archive_with = |extra: Option<(&str, &[u8])>, compression| {
            let mut output = Cursor::new(Vec::new());
            {
                let mut zip = zip::ZipWriter::new(&mut output);
                let options = SimpleFileOptions::default().compression_method(compression);
                for (name, bytes) in &files {
                    write_entry(&mut zip, options, name, bytes).unwrap();
                }
                if let Some((name, bytes)) = extra {
                    write_entry(&mut zip, options, name, bytes).unwrap();
                }
                zip.finish().unwrap();
            }
            output.into_inner()
        };
        assert!(decode_ccjz(&archive_with(
            Some(("untracked.bin", b"smuggled")),
            zip::CompressionMethod::Stored,
        ))
        .unwrap_err()
        .contains("undeclared entry"));
        let mut noncanonical = archive_with(None, zip::CompressionMethod::Stored);
        let central = noncanonical
            .windows(4)
            .position(|window| window == [0x50, 0x4b, 0x01, 0x02])
            .unwrap();
        noncanonical[central + 10..central + 12].copy_from_slice(&8u16.to_le_bytes());
        let error = decode_ccjz(&noncanonical).unwrap_err();
        assert!(
            error.to_ascii_lowercase().contains("compression"),
            "{error}"
        );
    }

    #[test]
    fn opaque_attachment_is_lazy_and_descriptor_bound() {
        let payload = b"raw-nmr-fid-payload";
        let mut document: Value = serde_json::from_str(&sample()).unwrap();
        document["resources"]["fid"] = serde_json::json!({
            "type": "nmr-fid",
            "encoding": "opaque",
            "data": {
                "storage": "ccjz-attachment",
                "mediaType": "application/vnd.chemsema.nmr-fid",
                "byteLength": payload.len(),
                "sha256": sha256_hex(payload)
            }
        });
        let encoded = encode_ccjz_with_attachments(
            &document.to_string(),
            2,
            &[Attachment {
                id: "fid",
                media_type: "application/vnd.chemsema.nmr-fid",
                extension: "fid",
                bytes: payload,
            }],
        )
        .unwrap();
        let mut reader = CcjzReader::open(Cursor::new(&encoded), DecodeLimits::default()).unwrap();
        assert_eq!(reader.read_attachment("fid").unwrap(), payload);
        let range = reader.attachment_range("fid").unwrap();
        assert_eq!(range.size, payload.len() as u64);
        assert_eq!(reader.manifest().attachments.len(), 1);
        let decoded: Value = serde_json::from_str(&decode_ccjz(&encoded).unwrap()).unwrap();
        assert_eq!(decoded, document);
    }

    #[test]
    fn file_attachment_streams_without_materializing_archive_payload() {
        let payload = b"file-backed-fid-payload";
        let path = std::env::temp_dir().join(format!(
            "chemsema-container-file-attachment-{}.fid",
            std::process::id()
        ));
        std::fs::write(&path, payload).unwrap();
        let mut document: Value = serde_json::from_str(&sample()).unwrap();
        document["resources"]["fid-file"] = serde_json::json!({
            "type": "nmr-fid",
            "encoding": "opaque",
            "data": {
                "storage": "ccjz-attachment",
                "mediaType": "application/vnd.chemsema.nmr-fid",
                "byteLength": payload.len(),
                "sha256": sha256_hex(payload)
            }
        });
        let mut output = Cursor::new(Vec::new());
        write_ccjz_with_files(
            &mut output,
            &document.to_string(),
            2,
            &[],
            &[FileAttachment {
                id: "fid-file",
                media_type: "application/vnd.chemsema.nmr-fid",
                extension: "fid",
                path: &path,
            }],
        )
        .unwrap();
        let _ = std::fs::remove_file(&path);
        let mut reader =
            CcjzReader::open(Cursor::new(output.into_inner()), DecodeLimits::default()).unwrap();
        assert_eq!(reader.read_attachment("fid-file").unwrap(), payload);
    }
}
