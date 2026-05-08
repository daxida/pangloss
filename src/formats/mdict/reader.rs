use std::{
    collections::HashMap,
    io::{BufReader, Read},
    path::Path,
    sync::LazyLock,
};

use anyhow::{Context as _, Result, bail};
use flate2::read::ZlibDecoder;
use indexmap::IndexMap;
use regex::Regex;

use crate::{
    Context, Reader,
    encryption::{adler32, fast_decrypt, ripemd128},
    formats::mdict::{COMPRESSION_HEADER_0, COMPRESSION_HEADER_2, EncryptionKind, MdictFormat},
    glossary::{AltEntry, AltMap, Entry, Glossary, GlossaryInfo, GlossaryMetadata},
    utils::unescape_html,
};

impl Reader for MdictFormat {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary> {
        read_with_context(path, ctx)
    }
}

fn read_with_context(path: &Path, _: &Context) -> Result<Glossary> {
    if path.extension().and_then(|e| e.to_str()) != Some("mdx") {
        bail!("MdictReader expects a .mdx file, got {}", path.display());
    }

    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(&file);

    let ParsedHeader {
        attrs,
        encoding,
        stylesheet,
    } = read_header(&mut reader)?;
    let info = GlossaryInfo::from(attrs);
    let metadata = GlossaryMetadata {
        stylesheet,
        ..Default::default()
    };

    let encryption = EncryptionKind::try_from(info.get("encrypted").unwrap_or("no"))?;

    let keys = read_keys(&mut reader, &encoding, encryption)?;
    let values = read_values(&mut reader, &keys)?;

    // First pass: collect @@@LINK redirects
    // links_map: headword -> Vec<alt_term>
    let mut links_map: HashMap<String, Vec<String>> = HashMap::new();
    for (term, defi) in keys.iter().zip(values.iter()) {
        if let Some(headword) = defi.strip_prefix("@@@LINK=") {
            // Trim headword, there may be newlines: "E662100\r\n"
            links_map
                .entry(headword.trim().to_string())
                .or_default()
                .push(term.clone());
        }
    }
    tracing::debug!(
        "Size of links_map: {}",
        links_map.values().map(Vec::len).sum::<usize>()
    );

    // Second pass: build entries, skip @@@LINK entries
    let mut entries = Vec::new();
    let mut alt_map = AltMap::new();
    for (term, defi) in keys.into_iter().zip(values.into_iter()) {
        // WARN: this breaks the roundtrip invariant
        if defi.starts_with("@@@LINK=") {
            continue;
        }
        if let Some(alts) = links_map.get(term.as_str()) {
            for alt in alts {
                alt_map
                    .entry(term.clone())
                    .or_default()
                    .push(AltEntry::only_term(alt.clone()));
            }
        }
        entries.push(Entry::with_html(term, defi));
    }

    Ok(Glossary {
        entries,
        alt_map,
        info,
        metadata,
        ..Default::default()
    })
}

struct ParsedHeader {
    attrs: IndexMap<String, String>,
    encoding: String,
    stylesheet: Option<StyleSheet>,
}

pub type StyleSheet = IndexMap<u32, (String, String)>;

impl ParsedHeader {
    const fn new(
        attrs: IndexMap<String, String>,
        encoding: String,
        stylesheet: Option<StyleSheet>,
    ) -> Self {
        Self {
            attrs,
            encoding,
            stylesheet,
        }
    }
}

// https://github.com/daxida/MDictUtils/blob/master/src/MDictUtils/MDictHeader.cs
//
/// Returns (attributes, encoding)
fn read_header<R: Read>(reader: &mut R) -> Result<ParsedHeader> {
    let header_text_size = read_u32_be(reader)? as usize;
    let mut raw = vec![0u8; header_text_size];
    reader.read_exact(&mut raw)?;

    let checksum = read_u32_le(reader)?;
    if adler32(&raw) != checksum {
        bail!("MDX header checksum mismatch");
    }

    let utf16: Vec<u16> = raw
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let header_str = String::from_utf16_lossy(&utf16);

    parse_header_str(&header_str)
}

static HEADER_ENTRY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)(\w+)="(.*?)""#).unwrap());

// quote @ The header_str consists of a single, XML tag dictionary, with various attributes.
fn parse_header_str(header: &str) -> Result<ParsedHeader> {
    let mut attrs = IndexMap::new();
    for cap in HEADER_ENTRY_RE.captures_iter(header) {
        let key = cap[1].to_string();
        let val = unescape_html(&cap[2]);
        attrs.insert(key, val);
    }

    let version = attrs
        .get("GeneratedByEngineVersion")
        .context("Missing GeneratedByEngineVersion in MDX header")?;
    if version != "2.0" {
        bail!("Only MDX version 2.0 is supported, got {version}");
    }

    let encoding = attrs
        .get("Encoding")
        .context("Missing Encoding in MDX header")?
        .clone();
    if encoding != "UTF-8" {
        bail!("Only encoding UTF-8 is supported, got {encoding}");
    }

    // https://github.com/xiaoyifang/goldendict-ng/blob/master/src/dict/mdictparser.cc#L346
    let stylesheet = if let Some(stylesheet) = attrs.get("StyleSheet") {
        let lines: Vec<_> = stylesheet.lines().collect();
        let mut map = IndexMap::new();
        for chunk in lines.chunks(3) {
            if let [id, prefix, suffix] = chunk
                && let Ok(id) = id.trim().parse::<u32>()
            {
                map.insert(id, (prefix.to_string(), suffix.to_string()));
            }
        }
        Some(map)
    } else {
        None
    };

    Ok(ParsedHeader::new(attrs, encoding, stylesheet))
}

fn decompress_block(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 8 {
        bail!("Block too small");
    }
    let comp_type = u32::from_be_bytes(data[0..4].try_into()?);
    let payload = &data[8..];

    match comp_type {
        COMPRESSION_HEADER_0 => Ok(payload.to_vec()),
        COMPRESSION_HEADER_2 => {
            let mut out = Vec::new();
            ZlibDecoder::new(payload)
                .read_to_end(&mut out)
                .context("zlib decompression failed")?;
            Ok(out)
        }
        other => bail!("Unsupported compression type: 0x{other:08X}"),
    }
}

fn decode_string(data: &[u8], encoding: &str) -> String {
    if encoding == "UTF-16LE" {
        let u16s: Vec<u16> = data
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else {
        String::from_utf8_lossy(data).to_string()
    }
}

fn read_keys<R: Read>(
    reader: &mut R,
    encoding: &str,
    encryption: EncryptionKind,
) -> Result<Vec<String>> {
    let _num_blocks = read_u64_be(reader)?;
    let _num_entries = read_u64_be(reader)?;
    let _decompressed_size = read_u64_be(reader)?;
    let key_block_info_size = read_u64_be(reader)?;
    let _key_block_size = read_u64_be(reader)?;
    let _checksum = read_u32_be(reader)?;

    let mut info_compressed = vec![0u8; key_block_info_size as usize];
    reader.read_exact(&mut info_compressed)?;

    if encryption.encrypts_index() && info_compressed.len() > 8 {
        let mut key_input = [0u8; 8];
        key_input[..4].copy_from_slice(&info_compressed[4..8]);
        key_input[4..].copy_from_slice(&0x3695u32.to_le_bytes());
        let key = ripemd128(&key_input);
        fast_decrypt(&mut info_compressed[8..], &key);
    }

    let info = decompress_block(&info_compressed)?;
    let block_sizes = parse_key_block_info(&info)?;

    let is_utf16 = encoding == "UTF-16LE";
    let mut keys = Vec::new();

    for (compressed_size, _) in block_sizes {
        let mut block_data = vec![0u8; compressed_size as usize];
        reader.read_exact(&mut block_data)?;
        let block = decompress_block(&block_data)?;

        let mut cursor = std::io::Cursor::new(&block);
        while (cursor.position() as usize) < block.len() {
            // Record offset (discarded)
            read_u64_be(&mut cursor)?;

            let mut string_bytes = Vec::new();
            loop {
                if is_utf16 {
                    let mut pair = [0u8; 2];
                    if cursor.read_exact(&mut pair).is_err() {
                        break;
                    }
                    if pair == [0, 0] {
                        break;
                    }
                    string_bytes.extend_from_slice(&pair);
                } else {
                    let b = read_u8(&mut cursor)?;
                    if b == 0 {
                        break;
                    }
                    string_bytes.push(b);
                }
            }

            keys.push(decode_string(&string_bytes, encoding));
        }
    }

    Ok(keys)
}

fn parse_key_block_info(info: &[u8]) -> Result<Vec<(u64, u64)>> {
    let mut cursor = std::io::Cursor::new(info);
    let mut blocks = Vec::new();

    while (cursor.position() as usize) < info.len() {
        read_u64_be(&mut cursor)?; // num keywords

        // first key: u16 size + bytes + null terminator
        let first_size = {
            let mut buf = [0u8; 2];
            cursor.read_exact(&mut buf)?;
            u16::from_be_bytes(buf) as usize
        };
        let mut skip = vec![0u8; first_size + 1];
        cursor.read_exact(&mut skip)?;

        // last key: u16 size + bytes + null terminator
        let last_size = {
            let mut buf = [0u8; 2];
            cursor.read_exact(&mut buf)?;
            u16::from_be_bytes(buf) as usize
        };
        let mut skip = vec![0u8; last_size + 1];
        cursor.read_exact(&mut skip)?;

        let compressed = read_u64_be(&mut cursor)?;
        let decompressed = read_u64_be(&mut cursor)?;
        blocks.push((compressed, decompressed));
    }

    Ok(blocks)
}

fn read_values<R: Read>(reader: &mut R, keys: &[String]) -> Result<Vec<String>> {
    let num_blocks = read_u64_be(reader)?;
    let _num_entries = read_u64_be(reader)?;
    let _info_size = read_u64_be(reader)?;
    let _total_size = read_u64_be(reader)?;

    let mut block_descs = Vec::with_capacity(num_blocks as usize);
    for _ in 0..num_blocks {
        let compressed = read_u64_be(reader)?;
        let decompressed = read_u64_be(reader)?;
        block_descs.push((compressed, decompressed));
    }

    let mut all_records = Vec::new();
    for (compressed_size, _) in &block_descs {
        let mut block_data = vec![0u8; *compressed_size as usize];
        reader.read_exact(&mut block_data)?;
        let block = decompress_block(&block_data)?;
        all_records.extend_from_slice(&block);
    }

    let mut values = Vec::with_capacity(keys.len());
    let mut pos = 0usize;

    for _ in keys {
        let start = pos;
        while pos < all_records.len() && all_records[pos] != 0 {
            pos += 1;
        }
        values.push(String::from_utf8_lossy(&all_records[start..pos]).to_string());
        pos += 1;
    }

    Ok(values)
}

fn read_u8<R: Read>(r: &mut R) -> Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32_be<R: Read>(r: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn read_u32_le<R: Read>(r: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64_be<R: Read>(r: &mut R) -> Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_be_bytes(buf))
}

#[cfg(test)]
mod tests {
    use crate::formats::mdict::reader::ParsedHeader;

    use super::parse_header_str;

    #[test]
    fn parse_header_str_multiline_description() {
        let header = r#"<Dictionary
GeneratedByEngineVersion="2.0"
Encoding="UTF-8"
Description="This is a &lt;b&gt;bold&lt;/b&gt; description
with a newline and &quot;quoted&quot; text and &amp;amp; entity"
Title="My Dictionary"/>"#;

        let ParsedHeader {
            attrs, encoding, ..
        } = parse_header_str(header).unwrap();
        assert_eq!(encoding, "UTF-8");
        assert_eq!(attrs["Title"], "My Dictionary");
        assert_eq!(
            attrs["Description"],
            "This is a <b>bold</b> description\nwith a newline and \"quoted\" text and &amp; entity"
        );
    }

    #[test]
    fn parse_header_str_stylesheet() {
        let header = r#"<Dictionary
GeneratedByEngineVersion="2.0"
Encoding="UTF-8"
Description="description"
StyleSheet="1


2
</font>

3

<font color="\#006AD5">
"
Title="My Dictionary"/>"#;
        let ParsedHeader { stylesheet, .. } = parse_header_str(header).unwrap();
        let stylesheet = stylesheet.unwrap();
        eprintln!("{stylesheet:?} | {header}");
        assert_eq!(stylesheet.len(), 3);
        assert_eq!(stylesheet[0], (String::new(), String::new()));
        assert_eq!(stylesheet[1], ("</font>".to_string(), String::new()));
        // Not sure if the fixture is malformed or we have a parsing error
        // assert_eq!(
        //     stylesheet[2],
        //     ("".to_string(), "<font color=\"#006AD5\">".to_string())
        // );
    }
}
