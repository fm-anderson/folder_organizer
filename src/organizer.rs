use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

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
    let mut filename: OsString = stem.to_os_string();

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

    let mut count: usize = 1;

    loop {
        let filename: OsString = numbered_filename(stem, destination.extension(), count);
        let candidate: PathBuf = destination.with_file_name(filename);

        if !candidate.try_exists()? {
            return Ok(candidate);
        }

        count += 1;
    }
}

fn move_file(source: &Path, destination: &Path) -> io::Result<()> {
    let parent: &Path = destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination has no parent directory",
        )
    })?;

    std::fs::create_dir_all(parent)?;
    std::fs::rename(source, destination)
}

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;
fn is_older_than_threshold(age: Duration, threshold: Duration) -> bool {
    age > threshold
}

pub(crate) fn organize_directory(
    directory: &Path,
    dry_run: bool,
    min_age_days: u64,
) -> io::Result<usize> {
    let seconds: u64 = min_age_days
        .checked_mul(SECONDS_PER_DAY)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Minimum age is too large"))?;

    let threshold: Duration = Duration::from_secs(seconds);
    let now: SystemTime = SystemTime::now();
    let mut eligible_count: usize = 0;

    for entry in std::fs::read_dir(directory)? {
        let entry: std::fs::DirEntry = entry?;

        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        if entry.file_type()?.is_file() {
            let metadata: std::fs::Metadata = entry.metadata()?;
            let modified: SystemTime = metadata.modified()?;
            let age: Result<Duration, std::time::SystemTimeError> = now.duration_since(modified);

            match age {
                Ok(duration) => {
                    if is_older_than_threshold(duration, threshold) {
                        eligible_count += 1;
                        let path: PathBuf = entry.path();

                        println!("Old file: {}", path.display());
                        println!("Age: {} days", duration.as_secs() / 86_400);

                        let category: &str = match path.extension() {
                            Some(extension) => {
                                let extension: String =
                                    extension.to_string_lossy().to_ascii_lowercase();

                                println!("Extension: {}", extension);

                                category_for(extension.as_str())
                            }
                            None => {
                                println!("Extension: (none)");

                                category_for("")
                            }
                        };

                        println!("Category: {}", category);
                        let destination_dir: PathBuf = directory.join(category);
                        let destination_path: PathBuf = destination_dir.join(entry.file_name());
                        let available_path: PathBuf = available_destination(&destination_path)?;

                        if available_path != destination_path {
                            tracing::info!(
                                original = %destination_path.display(),
                                alternative = %available_path.display(),
                                "Filename collision resolved"
                            );
                        }

                        if dry_run {
                            tracing::info!(
                                source = %path.display(),
                                destination = %available_path.display(),
                                "Would move file"
                            );
                        } else {
                            move_file(&path, &available_path)?;
                            tracing::info!(
                                source = %path.display(),
                                destination = %available_path.display(),
                                "File moved"
                            );
                        }
                    }
                }
                Err(_) => {
                    tracing::warn!(
                        file = %entry.path().display(),
                        "Skipping file with future modification time"
                    );
                }
            }
        }
    }

    Ok(eligible_count)
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
    #[test]
    fn test_move_file() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let source = temp_dir.path().join("notes.txt");
        let destination = temp_dir.path().join("documents/notes.txt");
        std::fs::write(&source, "Hello from Rust!")?;
        move_file(&source, &destination)?;
        assert!(!source.exists());
        assert!(destination.is_file());
        let content = std::fs::read_to_string(&destination)?;
        assert_eq!(content, "Hello from Rust!");

        Ok(())
    }
    #[test]
    fn test_move_file_with_collision() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let source = temp_dir.path().join("report.pdf");
        let destination_dir = temp_dir.path().join("documents");
        let destination = destination_dir.join("report.pdf");
        std::fs::create_dir_all(&destination_dir)?;
        std::fs::write(&source, "New report")?;
        std::fs::write(&destination, "Existing report")?;
        let available_path = available_destination(&destination)?;
        move_file(&source, &available_path)?;
        assert_eq!(available_path, destination_dir.join("report_1.pdf"));
        assert!(!source.exists());
        assert_eq!(std::fs::read_to_string(&destination)?, "Existing report");
        assert_eq!(std::fs::read_to_string(&available_path)?, "New report");

        Ok(())
    }
    #[test]
    fn test_seven_day_boundary() {
        let threshold = Duration::from_secs(7 * SECONDS_PER_DAY);
        let before = threshold - Duration::from_secs(1);
        let exact = threshold;
        let after = threshold + Duration::from_secs(1);
        assert!(!is_older_than_threshold(before, threshold));
        assert!(!is_older_than_threshold(exact, threshold));
        assert!(is_older_than_threshold(after, threshold));
    }
}
