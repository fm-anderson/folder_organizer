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

    if !cli.directory.is_dir() {
        eprintln!(
            "Error: '{}' is not a valid directory.",
            cli.directory.display()
        );

        std::process::exit(1);
    }

    println!("Dry run: {}", cli.dry_run);
    println!("Directory: {:?}", cli.directory);

    for entry in std::fs::read_dir(&cli.directory)? {
        let entry = entry?;

        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        if entry.file_type()?.is_file() {
            println!("Found file: {}", entry.path().display());
        }
    }

    Ok(())
}
