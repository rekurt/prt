mod app;
mod forward;
mod input;
mod stream;
mod tracer;
mod ui;
mod views;
mod watch;

use clap::{CommandFactory, Parser};
use prt_core::core::scanner;
use prt_core::i18n;
use prt_core::model::ExportFormat;
use std::io::{self, Write};

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
#[command(name = "prt", about = "Network port monitor", version)]
struct Cli {
    /// Export and exit (json or csv)
    #[arg(long, value_enum, conflicts_with = "json")]
    export: Option<CliExportFormat>,

    /// Stream NDJSON to stdout (one JSON object per line, every scan cycle)
    #[arg(long)]
    json: bool,

    /// Language (en, ru, zh). Auto-detected if not set.
    #[arg(long, global = true)]
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
    cli.validate_modes().unwrap_or_else(|error| error.exit());

    // Set language
    let lang = match &cli.lang {
        Some(l) => i18n::parse_lang(l),
        None => i18n::detect_locale(),
    };
    i18n::set_lang(lang);

    if let Some(format) = cli.export {
        let entries = scanner::scan()?;
        let output = scanner::export(&entries, format.into())?;
        return write_export(&mut io::stdout().lock(), output.as_bytes()).map_err(Into::into);
    }

    if cli.json {
        return stream::run_json_stream();
    }

    if let Some(Commands::Watch { ports }) = cli.command {
        return watch::run_watch(ports);
    }

    app::run()
}

impl Cli {
    fn validate_modes(&self) -> Result<(), clap::Error> {
        if self.command.is_some() && (self.export.is_some() || self.json) {
            return Err(Self::command().error(
                clap::error::ErrorKind::ArgumentConflict,
                "watch cannot be combined with --export or --json",
            ));
        }
        Ok(())
    }
}

fn write_export(writer: &mut impl Write, output: &[u8]) -> io::Result<()> {
    match writer.write_all(output).and_then(|()| writer.flush()) {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn output_modes_conflict() {
        for args in [
            vec!["prt", "--export", "json", "--json"],
            vec!["prt", "--export", "csv", "watch", "80"],
            vec!["prt", "--json", "watch", "80"],
        ] {
            let error = Cli::try_parse_from(args)
                .and_then(|cli| cli.validate_modes())
                .expect_err("modes must conflict");
            assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
        }
    }

    #[test]
    fn individual_modes_and_watch_language_are_accepted() {
        for args in [
            vec!["prt"],
            vec!["prt", "--export", "json"],
            vec!["prt", "--export", "csv"],
            vec!["prt", "--json"],
            vec!["prt", "watch", "80", "443"],
            vec!["prt", "--lang", "ru", "watch", "80"],
            vec!["prt", "watch", "80", "--lang", "ru"],
        ] {
            Cli::try_parse_from(args)
                .and_then(|cli| cli.validate_modes())
                .unwrap();
        }
    }

    struct FailingWriter {
        kind: io::ErrorKind,
        fail_on_flush: bool,
    }

    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.fail_on_flush {
                Ok(bytes.len())
            } else {
                Err(io::Error::from(self.kind))
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(self.kind))
        }
    }

    #[test]
    fn export_handles_closed_pipes_and_preserves_other_errors() {
        for fail_on_flush in [false, true] {
            for kind in [io::ErrorKind::BrokenPipe, io::ErrorKind::PermissionDenied] {
                let result = write_export(
                    &mut FailingWriter {
                        kind,
                        fail_on_flush,
                    },
                    b"[]",
                );
                if kind == io::ErrorKind::BrokenPipe {
                    assert!(result.is_ok());
                } else {
                    assert_eq!(result.unwrap_err().kind(), kind);
                }
            }
        }
    }

    #[test]
    fn export_preserves_output_bytes() {
        let mut output = Vec::new();
        write_export(&mut output, b"pid,port\n42,8080\n").unwrap();
        assert_eq!(output, b"pid,port\n42,8080\n");
    }
}
