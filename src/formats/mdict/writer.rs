use std::{
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
    glossary::{Glossary, GlossaryInfo, HtmlConverter},
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

    // SAFETY: There should always be a parent by main.rs logic.
    let parent = path.parent().unwrap();
    // mdict convention: write on the same folder
    let opath = parent;
    let _ = fs::create_dir(opath);
    for data_entry in glossary.css_files() {
        let fname = opath.join(data_entry.fname());
        fs::write(&fname, data_entry.bytes())?;
    }

    Ok(())
}

fn write_header<W: Write>(writer: &mut W, info: &GlossaryInfo) -> Result<()> {
    let mut xml = String::from("<Dictionary ");
    for key in ATTR_ORDER {
        let val = info.get(key).unwrap_or_else(|| default_attr(key));
        let row = format!("{}=\"{}\" ", key, escape_html(val));
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
        if let Some(alts) = glossary.alt_map.get(&term) {
            for alt in alts {
                pairs.push((alt.term().to_string(), format!("@@@LINK={term}")));
            }
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
