use std::{fs, time::Instant};

use anyhow::{Context as _, Result, bail};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

use pangloss::{
    Config, Context, DataEntry, Definition, Glossary, Reader, ReaderFormat, Writer, WriterFormat,
    cli::Cli,
    transform::{
        EntryTransformerBuilder, PreventDuplicateTerms, RemoveNewlines, ResolveMdictStyles,
    },
};

fn init_logger(verbose: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if verbose {
            // Only we are set to debug. ureq and other libs stay the same.
            EnvFilter::new(format!("{}=debug", env!("CARGO_PKG_NAME")))
        } else {
            // EnvFilter::new("warn")
            EnvFilter::new(format!("warn,{}=debug", env!("CARGO_PKG_NAME"))) // for deving now
        }
    });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%H:%M:%S".to_string(),
        ))
        .init();
}

fn prelude(args: &mut Cli) -> Result<()> {
    if !args.input.exists() {
        bail!("Input path {} does not exist.", args.input.display());
    }

    args.rformat = args
        .rformat
        .or_else(|| ReaderFormat::try_from_path(&args.input))
        .context("Couldn't detect the format. Pass --rformat=FORMAT")
        .map(Some)?;

    args.wformat = args
        .wformat
        .or_else(|| WriterFormat::try_from_path(&args.output))
        .context("Couldn't detect the format. Pass --wformat=FORMAT")
        .map(Some)?;

    // Since this creates a folder, do it last to not leave the folder around
    // if any of the previous checks fail.
    if let Some(parent) = args.output.parent()
        && !parent.exists()
        && !parent.as_os_str().is_empty()
    {
        tracing::info!(
            "Parent of {} does not exist. Creating it at {}.",
            args.output.display(),
            parent.display()
        );
        fs::create_dir_all(parent)?;
    }

    Ok(())
}

fn run(args: &Cli) -> Result<()> {
    println!("Input:        {}", args.input.display());
    println!("Output:       {}", args.output.display());

    let config = Config::from_cli(args);
    let ctx = Context::from_config(config);

    let Some(rformat) = config.rformat else {
        bail!("No reading format was given, and we could not detect it");
    };
    let Some(wformat) = config.wformat else {
        bail!("No writing format was given, and we could not detect it");
    };

    let start = Instant::now();
    let mut glossary = rformat.read_with_context(&args.input, &ctx)?;
    tracing::debug!("Read took: {:.2}s", start.elapsed().as_secs_f64());

    post_read(&mut glossary, args)?;

    glossary.diagnostics();

    #[allow(unused)]
    #[allow(clippy::items_after_statements)]
    fn dbg_state(glossary: &Glossary) {
        // tracing::debug!("{:?}", glossary.info);
        for entry in &glossary.entries {
            let converter = pangloss::HtmlConverter::default();
            let queries = ["patata", "lesen", "全然", "λήμμα"];
            if queries.contains(&entry.term()) {
                println!(
                    "html of {}\n{:?}\n",
                    entry.term(),
                    converter.convert(entry.definition())
                );
                // println!(
                //     "structured content of {}\n{:?}\n",
                //     entry.term(),
                //     entry.definition().to_yomitan(entry.term())
                // );
            }
        }
        // tracing::debug!("{:?}", glossary);
    }
    // dbg_state(&glossary);

    add_extra_files(&mut glossary, &ctx)?;

    let now = Instant::now();
    pre_write(&mut glossary, args);
    tracing::debug!("Pre-write took: {:.2}s", now.elapsed().as_secs_f64());

    // dbg_state(&glossary);

    let now = Instant::now();
    wformat.write_with_context(&args.output, &glossary, &ctx)?;
    tracing::debug!("Write took: {:.2}s", now.elapsed().as_secs_f64());
    tracing::debug!("Total took: {:.2}s", start.elapsed().as_secs_f64());

    Ok(())
}

const EXTRA_CSS_SC: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/yomitan/styles/structured-content.css"
));
const EXTRA_CSS_MATERIAL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/yomitan/styles/material.css"
));
const EXTRA_CSS_DISPLAY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/yomitan/styles/display.css"
));

fn add_extra_files(glossary: &mut Glossary, ctx: &Context) -> Result<()> {
    match (ctx.config.rformat, ctx.config.wformat) {
        (Some(ReaderFormat::Yomitan), Some(WriterFormat::Yomitan)) => (),
        (Some(ReaderFormat::Yomitan), _) => {
            tracing::info!(
                "Detected Yomitan as output format: adding a copy of the Yomitan side css!"
            );
            glossary.data_entries.extend(vec![
                DataEntry::new("structured-content.css", EXTRA_CSS_SC.to_vec())?,
                DataEntry::new("material.css", EXTRA_CSS_MATERIAL.to_vec())?,
                DataEntry::new("display.css", EXTRA_CSS_DISPLAY.to_vec())?,
            ]);
        }
        _ => (),
    }
    Ok(())
}

fn post_read(glossary: &mut Glossary, args: &Cli) -> Result<()> {
    if let Some(name) = &args.name {
        tracing::info!("Overwriting dictionary name with {name}");
        glossary.info.update("name", name.clone());
    }

    if let Some(pattern) = &args.strip_pattern {
        let re = regex::Regex::new(pattern).context("Invalid strip_pattern regex")?;
        tracing::info!("Trying to strip pattern {pattern}");
        for entry in &mut glossary.entries {
            match entry.definition_mut() {
                Definition::Text(s) | Definition::Html(s) => {
                    *s = re.replace_all(s, "").into_owned();
                }
                Definition::Yomitan(_) => {
                    bail!("Yomitan definition does not support strip_pattern")
                }
            }
        }
    }

    Ok(())
}

// TODO: The sorting done in some formats should go here
fn pre_write(glossary: &mut Glossary, args: &Cli) {
    // We need format information for this
    let (Some(rformat), Some(wformat)) = (args.rformat, args.wformat) else {
        return;
    };
    // We don't want to break the roundtrip invariant
    if rformat == wformat {
        return;
    }

    let mut builder = EntryTransformerBuilder::default();

    if rformat == ReaderFormat::Mdict
        && let Some(ref stylesheet) = glossary.metadata.stylesheet
    {
        builder = builder.with(ResolveMdictStyles::new(stylesheet.clone()));
    }

    if wformat == WriterFormat::Yomitan {
        builder = builder.with(RemoveNewlines);
    }

    if wformat == WriterFormat::Json {
        builder = builder.with(PreventDuplicateTerms::new(&glossary.alt_map));
    }

    let transformer = builder.build();
    for entry in &mut glossary.entries {
        transformer.transform(entry);
    }
}

fn main() -> Result<()> {
    let mut cli = Cli::parse_cli();
    init_logger(cli.verbose);
    prelude(&mut cli)?;
    run(&cli)
}
