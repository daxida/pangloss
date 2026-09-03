use std::{
    fmt::Write as _,
    fs,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::Result;
#[allow(unused)]
use flate2::{Compression, write::ZlibEncoder};
use rayon::prelude::*;

use crate::{
    Context, Writer,
    encryption::adler32,
    formats::mdict::{
        ATTR_ORDER, COMPRESSION_HEADER_0, COMPRESSION_HEADER_2, CompressionKind, MdictFormat,
        default_attr,
    },
    glossary::{DataEntry, Entry, Glossary, GlossaryInfo, HtmlConverter},
    utils::escape_html,
};

impl Writer for MdictFormat {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
        write_with_context(path, glossary, ctx, self.compression)
    }
}

fn write_with_context(
    path: &Path,
    glossary: &Glossary,
    _: &Context,
    compression: CompressionKind,
) -> Result<()> {
    let file = fs::File::create(path)?;
    let mut writer = BufWriter::new(file);

    write_header(&mut writer, &glossary.info)?;

    let keys = collect_keys(glossary);
    let (records, offsets) = build_records(glossary, &keys);

    write_key_blocks(&mut writer, &keys, &offsets, compression)?;
    write_record_blocks(&mut writer, &records, keys.len(), compression)?;

    // The resources go in the companion .mdd, including css
    // (even though we support reading the css files standalone)
    if !glossary.data_entries.is_empty() {
        write_mdd(
            &path.with_extension("mdd"),
            &glossary.data_entries,
            compression,
        )?;
    }

    Ok(())
}

/// The header of a .mdd, which names a different root and fills in no encoding.
const MDD_ATTR_ORDER: [(&str, &str); 10] = [
    ("GeneratedByEngineVersion", "2.0"),
    ("RequiredEngineVersion", "2.0"),
    ("Format", ""),
    ("KeyCaseSensitive", "No"),
    ("StripKey", "No"),
    ("Encrypted", "No"),
    ("RegisterBy", ""),
    ("Title", ""),
    ("Encoding", ""),
    ("CreationDate", ""),
];

/// Write the resources to a .mdd beside the .mdx.
fn write_mdd(path: &Path, data_entries: &[DataEntry], compression: CompressionKind) -> Result<()> {
    let file = fs::File::create(path)?;
    let mut writer = BufWriter::new(file);

    let title = path.file_stem().unwrap_or_default().to_string_lossy();
    let mut xml = String::from("<Library_Data ");
    for (key, value) in MDD_ATTR_ORDER {
        let value = if key == "Title" { &title } else { value };
        let _ = write!(xml, "{key}=\"{}\" ", escape_html(value));
    }
    xml.push_str("/>\r\n\0");

    let mut raw = Vec::with_capacity(xml.len() * 2);
    for unit in xml.encode_utf16() {
        raw.extend_from_slice(&unit.to_le_bytes());
    }
    writer.write_all(&(raw.len() as u32).to_be_bytes())?;
    writer.write_all(&raw)?;
    writer.write_all(&adler32(&raw).to_le_bytes())?;

    // Key block: the offset a file's bytes start at, then its path.
    let names: Vec<Vec<u16>> = data_entries
        .iter()
        .map(|entry| resource_key(&entry.fname().to_string_lossy()))
        .collect();
    let mut key_block = Vec::new();
    let mut offset = 0u64;
    for (name, entry) in names.iter().zip(data_entries) {
        key_block.extend_from_slice(&offset.to_be_bytes());
        for unit in name {
            key_block.extend_from_slice(&unit.to_le_bytes());
        }
        key_block.extend_from_slice(&[0, 0]);
        offset += entry.bytes().len() as u64;
    }
    let compressed = compress_block(&key_block, compression)?;

    let empty = Vec::new();
    let first = names.first().unwrap_or(&empty);
    let last = names.last().unwrap_or(&empty);
    let mut info = Vec::new();
    info.extend_from_slice(&(data_entries.len() as u64).to_be_bytes());
    for name in [first, last] {
        // The length is in characters; the reader skips (length + 1) * 2 bytes.
        info.extend_from_slice(&(name.len() as u16).to_be_bytes());
        for unit in name {
            info.extend_from_slice(&unit.to_le_bytes());
        }
        info.extend_from_slice(&[0, 0]);
    }
    info.extend_from_slice(&(compressed.len() as u64).to_be_bytes());
    info.extend_from_slice(&(key_block.len() as u64).to_be_bytes());
    let info_compressed = compress_block(&info, compression)?;

    let mut header_buf = Vec::new();
    header_buf.extend_from_slice(&1u64.to_be_bytes()); // num_blocks
    header_buf.extend_from_slice(&(data_entries.len() as u64).to_be_bytes());
    header_buf.extend_from_slice(&(info.len() as u64).to_be_bytes());
    header_buf.extend_from_slice(&(info_compressed.len() as u64).to_be_bytes());
    header_buf.extend_from_slice(&(compressed.len() as u64).to_be_bytes());
    writer.write_all(&header_buf)?;
    writer.write_all(&adler32(&header_buf).to_be_bytes())?;
    writer.write_all(&info_compressed)?;
    writer.write_all(&compressed)?;

    // Record block: the files themselves, one after another.
    let records: Vec<u8> = data_entries
        .iter()
        .flat_map(|entry| entry.bytes().iter().copied())
        .collect();
    let compressed = compress_block(&records, compression)?;

    writer.write_all(&1u64.to_be_bytes())?; // num_blocks
    writer.write_all(&(data_entries.len() as u64).to_be_bytes())?;
    writer.write_all(&16u64.to_be_bytes())?; // info size: one descriptor
    writer.write_all(&(compressed.len() as u64).to_be_bytes())?;
    writer.write_all(&(compressed.len() as u64).to_be_bytes())?;
    writer.write_all(&(records.len() as u64).to_be_bytes())?;
    writer.write_all(&compressed)?;

    Ok(())
}

/// `apple.png` is stored as `\apple.png`, the way a definition's src resolves.
fn resource_key(name: &str) -> Vec<u16> {
    format!("\\{}", name.replace('/', "\\"))
        .encode_utf16()
        .collect()
}

fn write_header<W: Write>(writer: &mut W, info: &GlossaryInfo) -> Result<()> {
    let mut xml = String::from("<Dictionary ");
    for key in ATTR_ORDER {
        // These two describe the bytes we are about to write, not the ones the
        // source dictionary held, and as of now, only UTF-8 plain is supported.
        let val = match key {
            "Encoding" => "UTF-8",
            "Encrypted" => "No",
            _ => info.get(key).unwrap_or_else(|| default_attr(key)),
        };
        let row = format!("{key}=\"{}\" ", escape_html(val));
        xml.push_str(&row);
    }
    xml.push_str("/>\r\n\0");

    // Encode as UTF-16LE
    let utf16: Vec<u16> = xml.encode_utf16().collect();
    let mut raw = Vec::with_capacity(utf16.len() * 2);
    for unit in &utf16 {
        raw.extend_from_slice(&unit.to_le_bytes());
    }

    // The \0 terminator is part of the string but not checksummed/sized
    // Strip the last 2 bytes (UTF-16LE \0) before size+checksum
    let payload = &raw;

    let checksum = adler32(payload);
    writer.write_all(&(payload.len() as u32).to_be_bytes())?;
    writer.write_all(payload)?;
    writer.write_all(&checksum.to_le_bytes())?;
    Ok(())
}

/// What a key points at: an entry to render, or the headword an alt redirects to.
enum Record<'a> {
    Entry(&'a Entry),
    Link(&'a str), // @@@LINK=...
}

/// The keys, sorted.
fn collect_keys(glossary: &Glossary) -> Vec<(&str, Record<'_>)> {
    let alts: usize = glossary.entries.iter().map(|e| e.alts().len()).sum();
    let mut keys = Vec::with_capacity(glossary.entries.len() + alts);

    for entry in &glossary.entries {
        // Alts become @@@LINK entries
        for alt in entry.alts() {
            keys.push((alt.term(), Record::Link(entry.term())));
        }
        keys.push((entry.term(), Record::Entry(entry)));
    }

    // Sort by term (MDX keys must be sorted)
    keys.sort_by_cached_key(|(term, _)| {
        term.trim_start_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase()
    });

    keys
}

/// Below this many entries a render chunk is not worth handing to another core.
const MIN_RECORD_CHUNK_LEN: usize = 8192;

/// The record block, and where each key's bytes start in it.
///
/// Rendering dominates the write, so chunks of keys are rendered in parallel
/// and stitched back together, rebasing each chunk's offsets onto the whole.
fn build_records(glossary: &Glossary, keys: &[(&str, Record<'_>)]) -> (Vec<u8>, Vec<u64>) {
    let converter = HtmlConverter::new(glossary);

    // Enough chunks to keep every core busy even when entry sizes are uneven,
    // but never so few entries per chunk that rayon costs more than it saves.
    let chunk_len = keys
        .len()
        .div_ceil(rayon::current_num_threads() * 4)
        .max(MIN_RECORD_CHUNK_LEN);
    let chunks: Vec<(Vec<u8>, Vec<u64>)> = keys
        .par_chunks(chunk_len)
        .map(|chunk| render_chunk(&converter, chunk))
        .collect();

    let total = chunks.iter().map(|(records, _)| records.len()).sum();
    let mut records = Vec::with_capacity(total);
    let mut offsets = Vec::with_capacity(keys.len());
    for (chunk_records, chunk_offsets) in chunks {
        let base = records.len() as u64;
        offsets.extend(chunk_offsets.into_iter().map(|offset| offset + base));
        records.extend_from_slice(&chunk_records);
    }

    (records, offsets)
}

/// Render one chunk of keys, with offsets relative to the chunk's own start.
fn render_chunk(converter: &HtmlConverter, keys: &[(&str, Record<'_>)]) -> (Vec<u8>, Vec<u64>) {
    let mut records = Vec::new();
    let mut offsets = Vec::with_capacity(keys.len());
    let mut rendered = String::with_capacity(4096);

    for (_, record) in keys {
        offsets.push(records.len() as u64);
        match record {
            Record::Entry(entry) => {
                rendered.clear();
                converter.write_into(entry.definition(), &mut rendered);
                records.extend_from_slice(rendered.as_bytes());
            }
            Record::Link(term) => {
                records.extend_from_slice(b"@@@LINK=");
                records.extend_from_slice(term.as_bytes());
            }
        }
        records.push(0); // null terminator
    }

    (records, offsets)
}

fn write_key_blocks<W: Write>(
    writer: &mut W,
    keys: &[(&str, Record<'_>)],
    offsets: &[u64],
    compression: CompressionKind,
) -> Result<()> {
    // Build one key block containing all entries
    let size: usize = keys.iter().map(|(term, _)| term.len() + 9).sum();
    let mut block_data = Vec::with_capacity(size);
    for ((term, _), offset) in keys.iter().zip(offsets) {
        block_data.extend_from_slice(&offset.to_be_bytes());
        block_data.extend_from_slice(term.as_bytes());
        block_data.push(0); // null terminator
    }

    let compressed = compress_block(&block_data, compression)?;

    let first_term = keys.first().map_or("", |(term, _)| *term);
    let last_term = keys.last().map_or("", |(term, _)| *term);

    // Key block info: one entry per block
    let mut info = Vec::new();
    info.extend_from_slice(&(keys.len() as u64).to_be_bytes()); // num keywords
    // first key
    info.extend_from_slice(&(first_term.len() as u16).to_be_bytes());
    info.extend_from_slice(first_term.as_bytes());
    info.push(0);
    // last key
    info.extend_from_slice(&(last_term.len() as u16).to_be_bytes());
    info.extend_from_slice(last_term.as_bytes());
    info.push(0);
    // compressed/decompressed sizes
    info.extend_from_slice(&(compressed.len() as u64).to_be_bytes());
    info.extend_from_slice(&(block_data.len() as u64).to_be_bytes());

    let info_compressed = compress_block(&info, compression)?;

    // Key section header (5 x u64 + u32 checksum)
    let header_buf = {
        let mut h = Vec::new();
        h.extend_from_slice(&1u64.to_be_bytes()); // num_blocks
        h.extend_from_slice(&(keys.len() as u64).to_be_bytes()); // num_entries
        h.extend_from_slice(&(info.len() as u64).to_be_bytes()); // decompressed info size
        h.extend_from_slice(&(info_compressed.len() as u64).to_be_bytes()); // compressed info size
        h.extend_from_slice(&(compressed.len() as u64).to_be_bytes()); // key block size
        h
    };
    let header_checksum = adler32(&header_buf);

    writer.write_all(&header_buf)?;
    writer.write_all(&header_checksum.to_be_bytes())?;
    writer.write_all(&info_compressed)?;
    writer.write_all(&compressed)?;

    Ok(())
}

/// Records are cut into blocks of this size before (parallel) compressing.
const RECORD_BLOCK_SIZE: usize = 4 << 20;

fn write_record_blocks<W: Write>(
    writer: &mut W,
    records: &[u8],
    count: usize,
    compression: CompressionKind,
) -> Result<()> {
    let blocks: Vec<&[u8]> = records.chunks(RECORD_BLOCK_SIZE).collect();
    let compressed: Vec<Vec<u8>> = blocks
        .par_iter()
        .map(|block| compress_block(block, compression))
        .collect::<Result<_>>()?;

    let blocks_len: usize = compressed.iter().map(Vec::len).sum();

    // Record section header (4 x u64)
    writer.write_all(&(compressed.len() as u64).to_be_bytes())?; // num_blocks
    writer.write_all(&(count as u64).to_be_bytes())?; // num_entries
    // info size: two u64 per block descriptor
    writer.write_all(&((compressed.len() * 16) as u64).to_be_bytes())?;
    writer.write_all(&(blocks_len as u64).to_be_bytes())?;

    // One descriptor per block: compressed + decompressed sizes
    for (block, compressed) in blocks.iter().zip(&compressed) {
        writer.write_all(&(compressed.len() as u64).to_be_bytes())?;
        writer.write_all(&(block.len() as u64).to_be_bytes())?;
    }
    for compressed in &compressed {
        writer.write_all(compressed)?;
    }

    Ok(())
}

/// No compression version
fn compress_block(data: &[u8], compression: CompressionKind) -> Result<Vec<u8>> {
    match compression {
        CompressionKind::None => Ok(compress_block_none(data)),
        CompressionKind::Lzo => unimplemented!(),
        CompressionKind::Zip => compress_block_zlib(data),
    }
}

fn compress_block_none(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + data.len());
    out.extend_from_slice(&COMPRESSION_HEADER_0.to_be_bytes()); // no compression
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out.extend_from_slice(data);
    out
}

fn compress_block_zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;

    let mut out = Vec::with_capacity(8 + compressed.len());
    out.extend_from_slice(&COMPRESSION_HEADER_2.to_be_bytes()); // zlib compression
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}
