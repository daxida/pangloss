use std::{
    fmt::Write as _,
    fs,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::Result;
#[allow(unused)]
use flate2::{Compression, write::ZlibEncoder};

use crate::{
    Context, Writer,
    encryption::adler32,
    formats::mdict::{
        ATTR_ORDER, COMPRESSION_HEADER_0, COMPRESSION_HEADER_2, CompressionKind, MdictFormat,
        default_attr,
    },
    glossary::{DataEntry, Glossary, GlossaryInfo, HtmlConverter},
    utils::escape_html,
};

impl Writer for MdictFormat {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
        write_with_context(path, glossary, ctx, self.compression)
    }
}

type Pairs = Vec<(String, String)>;

fn write_with_context(
    path: &Path,
    glossary: &Glossary,
    _: &Context,
    compression: CompressionKind,
) -> Result<()> {
    let file = fs::File::create(path)?;
    let mut writer = BufWriter::new(file);

    write_header(&mut writer, &glossary.info)?;

    let pairs = collect_pairs(glossary);

    write_key_blocks(&mut writer, &pairs, compression)?;
    write_record_blocks(&mut writer, &pairs, compression)?;

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

// Collect (term, definition) pairs including alts
fn collect_pairs(glossary: &Glossary) -> Pairs {
    let mut pairs: Vec<_> = Vec::new();
    let converter = HtmlConverter::new(glossary);

    for entry in &glossary.entries {
        let term = entry.term().to_string();
        let defi = converter.convert(entry.definition());
        // Alts become @@@LINK entries
        for alt in entry.alts() {
            pairs.push((alt.term().to_string(), format!("@@@LINK={term}")));
        }
        pairs.push((term, defi));
    }

    // Sort by term (MDX keys must be sorted)
    // TODO: This sorting is scuffed!
    pairs.sort_by(|a, b| {
        let strip = |s: &str| {
            s.trim_start_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        };
        strip(&a.0).cmp(&strip(&b.0))
    });

    pairs
}

fn write_key_blocks<W: Write>(
    writer: &mut W,
    pairs: &[(String, String)],
    compression: CompressionKind,
) -> Result<()> {
    // Build one key block containing all entries
    let mut block_data = Vec::new();
    let mut record_offset = 0u64;
    for (term, defi) in pairs {
        block_data.extend_from_slice(&record_offset.to_be_bytes());
        block_data.extend_from_slice(term.as_bytes());
        block_data.push(0); // null terminator
        record_offset += defi.len() as u64 + 1; // +1 for null terminator
    }

    let compressed = compress_block(&block_data, compression)?;

    let first_term = pairs.first().map_or("", |(t, _)| t.as_str());
    let last_term = pairs.last().map_or("", |(t, _)| t.as_str());

    // Key block info: one entry per block
    let mut info = Vec::new();
    info.extend_from_slice(&(pairs.len() as u64).to_be_bytes()); // num keywords
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
        h.extend_from_slice(&(pairs.len() as u64).to_be_bytes()); // num_entries
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

fn write_record_blocks<W: Write>(
    writer: &mut W,
    pairs: &[(String, String)],
    compression: CompressionKind,
) -> Result<()> {
    // Build one record block containing all definitions
    let mut block_data = Vec::new();
    for (_, defi) in pairs {
        block_data.extend_from_slice(defi.as_bytes());
        block_data.push(0); // null terminator
    }

    let compressed = compress_block(&block_data, compression)?;

    // Record block info header (4 x u64)
    writer.write_all(&1u64.to_be_bytes())?; // num_blocks
    writer.write_all(&(pairs.len() as u64).to_be_bytes())?; // num_entries
    writer.write_all(&(16u64).to_be_bytes())?; // info size (1 block = 2 x u64)
    // writer.write_all(&(block_data.len() as u64).to_be_bytes())?; // total decompressed size
    writer.write_all(&(compressed.len() as u64).to_be_bytes())?; // blocks_len — total size of rec_blocks

    // One record block descriptor: compressed + decompressed sizes
    writer.write_all(&(compressed.len() as u64).to_be_bytes())?;
    writer.write_all(&(block_data.len() as u64).to_be_bytes())?;

    writer.write_all(&compressed)?;

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
