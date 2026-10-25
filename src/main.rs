mod logging;
mod organizer;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "Organize files by type")]
struct Cli {
    directory: PathBuf,

    #[arg(long)]
    dry_run: bool,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    logging::initialize_logging()?;
    tracing::info!("Organizer started");

    if !cli.directory.is_dir() {
        tracing::error!(
            directory = %cli.directory.display(),
            "Invalid directory"
        );

        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Not a directory: {}", cli.directory.display()),
        ));
    }

    println!("Dry run: {}", cli.dry_run);
    println!("Directory: {:?}", cli.directory);

    let eligible_count = organizer::organize_directory(&cli.directory, cli.dry_run)?;

    println!("Eligible files: {}", eligible_count);

    tracing::info!(
        eligible_files = eligible_count,
        dry_run = cli.dry_run,
        "Organizer finished"
    );

    Ok(())
}
