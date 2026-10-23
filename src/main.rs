use clap::Parser;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Parser, Debug)]
#[command(version, about = "Organize files by type")]
struct Cli {
    directory: PathBuf,

    #[arg(long)]
    dry_run: bool,
}

fn category_for(extension: &str) -> &'static str {
    match extension {
        //
        "md" | "pdf" | "docx" | "doc" | "pptx" | "ppt" | "xlsx" | "xls" | "txt" => "documents",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "tiff" | "svg" | "heic" => "images",
        "mp4" | "mov" | "mkv" | "avi" => "video",
        "zip" | "rar" | "7z" | "tar" | "gz" => "archives",
        "json" | "csv" | "xml" | "yml" => "data",
        _ => "misc",
    }
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

    let now = SystemTime::now();

    for entry in std::fs::read_dir(&cli.directory)? {
        let entry = entry?;

        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        if entry.file_type()?.is_file() {
            let metadata = entry.metadata()?;
            let modified = metadata.modified()?;
            let age = now.duration_since(modified);

            match age {
                Ok(duration) => {
                    if duration > std::time::Duration::from_secs(7 * 24 * 60 * 60) {
                        let path = entry.path();

                        println!("Old file: {}", path.display());
                        println!("Age: {} days", duration.as_secs() / 86_400);

                        let category = match path.extension() {
                            Some(extension) => {
                                let extension = extension.to_string_lossy().to_ascii_lowercase();

                                println!("Extension: {}", extension);

                                category_for(extension.as_str())
                            }
                            None => {
                                println!("Extension: (none)");

                                category_for("")
                            }
                        };

                        println!("Category: {}", category);
                    }
                }
                Err(_) => {
                    println!("Age: File modification time is in the future");
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_category() {
        assert_eq!(category_for("md"), "documents");
    }
    #[test]
    fn test_unknown_extension() {
        assert_eq!(category_for("xyz"), "misc");
    }
    #[test]
    fn test_document_extensions() {
        assert_eq!(category_for("pdf"), "documents");
        assert_eq!(category_for("docx"), "documents");
        assert_eq!(category_for("md"), "documents");
    }
    #[test]
    fn test_image_extensions() {
        assert_eq!(category_for("jpg"), "images");
        assert_eq!(category_for("jpeg"), "images");
        assert_eq!(category_for("png"), "images");
    }
    #[test]
    fn test_remaining_categories() {
        assert_eq!(category_for("mp4"), "video");
        assert_eq!(category_for("zip"), "archives");
        assert_eq!(category_for("json"), "data");
    }
}
