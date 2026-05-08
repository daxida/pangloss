//! Html format that can be read by a browser.
//!
//! The entries are split in chunks. It has a dark/light mode toggle and a redirection
//! to see the glossary info at the top, and a search bar, which is basically a
//! huge javascript hashmap in the back.

use std::{fs, path::Path};

use anyhow::{Result, bail};

use crate::{
    Context, HtmlConverter, Writer,
    glossary::{Glossary, TERM_SEPARATOR},
};

pub struct HtmlFormat;

impl Writer for HtmlFormat {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
        write_with_context(path, glossary, ctx)
    }
}

// TODO: Links are completely broken

struct HtmlInfo<'a> {
    title: &'a str,
    description: &'a str,
}

impl<'a> HtmlInfo<'a> {
    fn new(title: &'a str, description: &'a str) -> Self {
        Self { title, description }
    }
}

const MAX_FILE_SIZE: usize = 102400; // 100KB per file

fn make_anchor(term: &str) -> String {
    term.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

// https://github.com/ilius/pyglossary/blob/master/pyglossary/plugins/html_dir/writer.py
// https://github.com/ilius/pyglossary/blob/master/doc/p/html_dir.md
fn write_with_context(path: &Path, glossary: &Glossary, _: &Context) -> Result<()> {
    if path.extension().and_then(|e| e.to_str()) != Some("hdir") {
        bail!(
            "Expected a file with .hdir extension but got {}",
            path.display()
        );
    }

    // Prelude only creates the parents.
    tracing::debug!("Creating path: {}", path.display());
    let _ = fs::create_dir_all(&path);

    let title = glossary.info.get("title").unwrap_or("Glossary");
    let description = glossary.info.get("description").unwrap_or_default();
    let html_info = HtmlInfo::new(title, description);

    write_pages(path, glossary, &html_info)?;
    write_info(path, glossary, &html_info)?;

    Ok(())
}

fn write_pages(path: &Path, glossary: &Glossary, html_info: &HtmlInfo) -> Result<()> {
    let HtmlInfo { title, .. } = html_info;
    let alt_map = &glossary.alt_map;

    let mut css_links = String::new();
    for data_entry in glossary.css_files() {
        fs::write(path.join(&data_entry.fname), &data_entry.bytes)?;
        css_links.push_str(&format!(
            r#"<link rel="stylesheet" href="./{}" />"#,
            data_entry.fname.display()
        ));
    }

    let mut file_index = 0usize;
    let mut current_size = 0usize;
    let mut file_entries: Vec<Vec<String>> = vec![Vec::new()];
    let mut search_entries = Vec::new();

    let converter = HtmlConverter::new(glossary);

    for entry in &glossary.entries {
        let raw_terms = entry.s_terms(alt_map);
        // Convert "term|syn1|syn2" into "term | syn1 | syn2"
        let terms = raw_terms
            .split(TERM_SEPARATOR)
            .collect::<Vec<_>>()
            .join(&format!(" {TERM_SEPARATOR} "));
        let term = entry.term();
        let anchor = make_anchor(term);

        search_entries.push(format!(
            r#"{{ term: {:?}, page: "{:05}.html", anchor: {:?} }}"#,
            term, file_index, anchor
        ));

        let text = format!(
            r#"<div class="entry" id="{anchor}"><div class="terms">{}</div><div class="defi">{}</div></div>
"#,
            terms,
            converter.convert(entry.definition())
        );
        if current_size + text.len() > MAX_FILE_SIZE && !file_entries[file_index].is_empty() {
            file_index += 1;
            file_entries.push(Vec::new());
            current_size = 0;
        }
        current_size += text.len();
        file_entries[file_index].push(text);
    }

    fs::write(
        path.join("search.js"),
        format!("const SEARCH_INDEX = [{}];", search_entries.join(",")),
    )?;

    let total_pages = file_entries.len();

    for (i, entries) in file_entries.into_iter().enumerate() {
        let prev_link = if i > 0 {
            format!("<a href=\"./{:05}.html\">&#9664;</a>", i - 1)
        } else {
            String::new()
        };
        let next_link = if i + 1 < total_pages {
            format!("<a href=\"./{:05}.html\">&#9654;</a>", i + 1)
        } else {
            String::new()
        };
        let nav = format!(
            r#"<nav style="text-align: center; font-size: 2.5em;">{prev_link}&nbsp;&nbsp;&nbsp;{next_link}&nbsp;&nbsp;&nbsp;<a href="./info.html">ℹ️</a></nav>"#
        );

        let entries_html = entries.join("");

        // It's a bit verbose due to the light/dark mode toggle
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} — Page {page} of {total_pages}</title>
    <!-- User CSS is loaded first so that our default styles below take priority -->
    {css_links}
    <style>
        /* Don't let user css overwrite the scrollbar... */
        html, body, #root, .entries {{
            overflow: visible !important;
            overflow-y: auto !important;
            max-height: none !important;
        }}

        body {{ margin: 0; background: #ddd; }}
        body:has(#theme-toggle:checked) {{ background: #222; }}
        #theme-toggle:checked ~ label ~ body, #theme-toggle:checked ~ body {{ background: #222; }}
        #root {{ font-family: sans-serif; max-width: 1200px; margin: 0 auto; padding: 0 1rem; background: #f5f5f5; color: #222; min-height: 100vh; }}
        #theme-toggle:checked ~ #root {{ background: #373737; color: #eee; }}
        #root a {{ color: #0066cc; }}
        #theme-toggle:checked ~ #root a {{ color: #aaaaff; }}
        #root nav {{ text-align: center; font-size: 2.5em; padding: 0.5rem 0; }}
        #root .entry {{ border-bottom: 1px solid #ccc; padding: 0.6rem 0; }}
        #theme-toggle:checked ~ #root .entry {{ border-bottom: 1px solid #555; }}
        #root .terms {{ font-size: 1.1em; font-weight: bold; color: #2a6e2a; margin-bottom: 0.3rem; }}
        #theme-toggle:checked ~ #root .terms {{ color: #c7ffb9; }}
        #root .defi {{ color: #222; }}
        #theme-toggle:checked ~ #root .defi {{ color: #eee; }}
        #theme-label {{ position: fixed; top: 1rem; right: 1rem; cursor: pointer; font-size: 1.5em; }}

        /* Search bar */
        #search {{ position: sticky; top: 0; background: inherit; padding: 0.5rem 0; z-index: 100; }}
        #search-input {{ width: 100%; box-sizing: border-box; padding: 0.7rem; font-size: 1rem; }}
        #search-results {{ background: inherit; max-height: 300px; overflow-y: auto; }}
        #search-results a {{ display: block; padding: 0.4rem; text-decoration: none; }}
        #search-results a:hover {{ background: rgba(127,127,127,0.15); }}
    </style>
</head>
<body>
    <input type="checkbox" id="theme-toggle" hidden>
    <label for="theme-toggle" id="theme-label" title="Toggle light/dark mode">🌙</label>
    <div id="root">
        <div id="search">
            <input id="search-input" placeholder="Search...">
            <div id="search-results"></div>
        </div>
        {nav}
        <div class="entries">
{entries_html}        </div>
        {nav}
    </div>
    <script src="./search.js"></script>
    <script>
        const input = document.getElementById("search-input");
        const results = document.getElementById("search-results");

        input.addEventListener("input", () => {{
            const q = input.value.toLowerCase().trim();

            if (!q) {{
                results.innerHTML = "";
                return;
            }}

            const matches = SEARCH_INDEX
                .filter(e => e.term.toLowerCase().includes(q))
                .slice(0, 30);

            results.innerHTML = matches.map(e =>
                `<a href="./${{e.page}}#${{e.anchor}}">${{e.term}}</a>`
            ).join("");
        }});
    </script>
</body>
</html>"#,
            page = i + 1,
        );

        fs::write(path.join(format!("{i:05}.html")), html)?;
    }

    Ok(())
}

// Write info page (only dark mode)
fn write_info(path: &Path, glossary: &Glossary, html_info: &HtmlInfo) -> Result<()> {
    let info_path = path.join("info.html");
    let HtmlInfo { title, description } = html_info;

    let mut info_rows = String::new();
    for (key, value) in &glossary.info {
        info_rows.push_str(&format!("<tr><td>{key}</td><td>{value}</td></tr>\n"));
    }
    let info_html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Info: {title}</title>
    <style>
        body {{ font-family: sans-serif; max-width: 900px; margin: 2rem auto; padding: 0 1rem; background: #373737; color: #eee; }}
        table, th, td {{ border: 1px solid #888; border-collapse: collapse; padding: 5px; }}
        a {{ color: #aaaaff; }}
    </style>
</head>
<body>
    <p><a href="./00000.html">&#9664; Back</a></p>
    <h1>Info: {title}</h1>
    <p>{description}</p>
    <table>
        <tr><th>Key</th><th>Value</th></tr>
        {info_rows}
    </table>
</body>
</html>"#
    );

    fs::write(info_path, info_html)?;
    Ok(())
}
