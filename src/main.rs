use clap::Parser;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
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

fn numbered_filename(stem: &OsStr, extension: Option<&OsStr>, count: usize) -> OsString {
    let mut filename = stem.to_os_string();

    filename.push(format!("_{count}"));

    if let Some(extension) = extension {
        filename.push(".");
        filename.push(extension);
    }

    filename
}

fn available_destination(destination: &Path) -> io::Result<PathBuf> {
    if !destination.try_exists()? {
        return Ok(destination.to_path_buf());
    }

    let Some(stem) = destination.file_stem() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination has no filename stem",
        ));
    };

    let mut count = 1;

    loop {
        let filename = numbered_filename(stem, destination.extension(), count);
        let candidate = destination.with_file_name(filename);

        if !candidate.try_exists()? {
            return Ok(candidate);
        }

        count += 1;
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
                        let destination_dir = cli.directory.join(category);
                        let destination_path = destination_dir.join(entry.file_name());

                        println!("Destination: {}", destination_path.display());
                        let available_path = available_destination(&destination_path)?;

                        if available_path != destination_path {
                            println!("Collision detected: {}", destination_path.display());
                            println!("Alternative: {}", available_path.display());
                        }
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
    #[test]
    fn test_numbered_filename_with_extension() {
        let filename = numbered_filename(OsStr::new("report"), Some(OsStr::new("pdf")), 2);
        assert_eq!(filename, OsStr::new("report_2.pdf"));
    }
    #[test]
    fn test_numbered_filename_without_extension() {
        let filename = numbered_filename(OsStr::new("README"), None, 1);
        assert_eq!(filename, OsStr::new("README_1"));
    }
    #[test]
    fn test_available_destination_without_collision() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let destination = temp_dir.path().join("report.pdf");
        let result = available_destination(&destination)?;
        assert_eq!(result, destination);

        Ok(())
    }
    #[test]
    fn test_available_destination_with_collision() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let destination = temp_dir.path().join("report.pdf");
        std::fs::write(&destination, "Existing file")?;
        let result = available_destination(&destination)?;
        let expected = temp_dir.path().join("report_1.pdf");
        assert_eq!(result, expected);

        Ok(())
    }
    #[test]
    fn test_available_destination_with_multiple_collisions() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let destination = temp_dir.path().join("report.pdf");
        std::fs::write(&destination, "Original file")?;
        std::fs::write(temp_dir.path().join("report_1.pdf"), "First collision")?;
        std::fs::write(temp_dir.path().join("report_2.pdf"), "Second collision")?;
        let result = available_destination(&destination)?;
        let expected = temp_dir.path().join("report_3.pdf");
        assert_eq!(result, expected);

        Ok(())
    }
    #[test]
    fn test_available_destination_without_extension() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let destination = temp_dir.path().join("README");
        std::fs::write(&destination, "Original file")?;
        std::fs::write(temp_dir.path().join("README_1"), "First collision")?;
        let result = available_destination(&destination)?;
        let expected = temp_dir.path().join("README_2");
        assert_eq!(result, expected);

        Ok(())
    }
}
