mod app;
mod forward;
mod input;
mod stream;
mod tracer;
mod ui;
mod watch;

use clap::Parser;
use prt_core::core::scanner;
use prt_core::i18n;
use prt_core::model::ExportFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum CliExportFormat {
    Json,
    Csv,
}

impl From<CliExportFormat> for ExportFormat {
    fn from(f: CliExportFormat) -> Self {
        match f {
            CliExportFormat::Json => ExportFormat::Json,
            CliExportFormat::Csv => ExportFormat::Csv,
        }
    }
}

#[derive(Parser)]
#[command(name = "prt", about = "Network port monitor")]
struct Cli {
    /// Export and exit (json or csv)
    #[arg(long, value_enum)]
    export: Option<CliExportFormat>,

    /// Stream NDJSON to stdout (one JSON object per line, every scan cycle)
    #[arg(long)]
    json: bool,

    /// Language (en, ru, zh). Auto-detected if not set.
    #[arg(long)]
    lang: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Watch specific ports — compact UP/DOWN monitor with BEL on changes
    Watch {
        /// Ports to watch
        #[arg(required = true)]
        ports: Vec<u16>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Set language
    let lang = match &cli.lang {
        Some(l) => i18n::parse_lang(l),
        None => i18n::detect_locale(),
    };
    i18n::set_lang(lang);

    if let Some(format) = cli.export {
        let entries = scanner::scan()?;
        let output = scanner::export(&entries, format.into())?;
        print!("{output}");
        return Ok(());
    }

    if cli.json {
        return stream::run_json_stream();
    }

    if let Some(Commands::Watch { ports }) = cli.command {
        return watch::run_watch(ports);
    }

    app::run()
}
