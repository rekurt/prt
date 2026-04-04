mod app;
mod input;
mod ui;

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

    /// Language (en, ru, zh). Auto-detected if not set.
    #[arg(long)]
    lang: Option<String>,
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

    app::run()
}
