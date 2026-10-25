use clap::Parser;
use std::ffi::{OsStr, OsString};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing_subscriber::fmt::writer::MakeWriterExt;

const AGE_THRESHOLD: Duration = Duration::from_secs(7 * 24 * 60 * 60);

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

fn move_file(source: &Path, destination: &Path) -> io::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination has no parent directory",
        )
    })?;

    std::fs::create_dir_all(parent)?;
    std::fs::rename(source, destination)
}

fn is_older_than_threshold(age: Duration) -> bool {
    age > AGE_THRESHOLD
}

fn log_period(line: &str) -> Option<(u16, u8)> {
    let timestamp = line.split_whitespace().next()?;

    let mut parts = timestamp.split('-');

    let year = parts.next()?.parse::<u16>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;

    let half = match month {
        1..=6 => 1,
        7..=12 => 2,
        _ => return None,
    };

    Some((year, half))
}

fn last_log_line(path: &Path) -> io::Result<Option<String>> {
    let file = std::fs::File::open(path)?;
    let reader = io::BufReader::new(file);

    reader.lines().last().transpose()
}

fn should_rotate_log(previous: Option<(u16, u8)>, current: (u16, u8)) -> bool {
    match previous {
        Some(period) => period != current,
        None => false,
    }
}

fn current_log_period() -> io::Result<(u16, u8)> {
    let now = time::OffsetDateTime::now_utc();
    let year =
        u16::try_from(now.year()).map_err(|_| io::Error::other("UTC year is out of range"))?;
    let month = u8::from(now.month());
    let half = if month <= 6 { 1 } else { 2 };

    Ok((year, half))
}

fn archive_log(path: &Path, period: (u16, u8)) -> io::Result<PathBuf> {
    let (year, half) = period;
    let archive_path = path.with_file_name(format!("organizer_{year}_{half}.log"));
    if archive_path.try_exists()? {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Log archive already exists",
        ));
    }
    std::fs::rename(path, &archive_path)?;

    Ok(archive_path)
}

fn log_directory() -> io::Result<PathBuf> {
    let directory = std::env::var_os("ORGANIZER_LOG_DIR").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "ORGANIZER_LOG_DIR environment variable is not set",
        )
    })?;

    Ok(PathBuf::from(directory))
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).map_err(io::Error::other)?;
    let stable_log_dir = log_directory()?;
    println!("Log directory: {}", stable_log_dir.display());

    let log_path = stable_log_dir.join("organizer.log");
    let previous_period = if log_path.try_exists()? {
        let last_line = last_log_line(&log_path)?;
        last_line.as_deref().and_then(log_period)
    } else {
        None
    };
    let current_period = current_log_period()?;
    let rotate = should_rotate_log(previous_period, current_period);

    println!("Last log period: {:?}", previous_period);
    println!("Current UTC period: {:?}", current_period);
    println!("Log rotation needed: {}", rotate);

    if let Some(period) = previous_period
        && rotate
    {
        let archive_path = archive_log(&log_path, period)?;

        println!("Archived log: {}", archive_path.display());
    }

    let file_appender = tracing_appender::rolling::never(&stable_log_dir, "organizer.log");

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(std::io::stderr.and(file_appender))
        .init();

    tracing::info!("Organizer started");

    if !cli.directory.is_dir() {
        tracing::error!(
            directory = %cli.directory.display(),
            "Invalid directory"
        );

        std::process::exit(1);
    }

    println!("Dry run: {}", cli.dry_run);
    println!("Directory: {:?}", cli.directory);

    let now = SystemTime::now();
    let mut eligible_count = 0;

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
                    if is_older_than_threshold(duration) {
                        eligible_count += 1;
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
                        let available_path = available_destination(&destination_path)?;

                        if available_path != destination_path {
                            tracing::info!(
                                original = %destination_path.display(),
                                alternative = %available_path.display(),
                                "Filename collision resolved"
                            );
                        }

                        if cli.dry_run {
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
    println!("Eligible files: {}", eligible_count);

    tracing::info!(
        eligible_files = eligible_count,
        dry_run = cli.dry_run,
        "Organizer finished"
    );

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
        let before = AGE_THRESHOLD - Duration::from_secs(1);
        let exact = AGE_THRESHOLD;
        let after = AGE_THRESHOLD + Duration::from_secs(1);
        assert!(!is_older_than_threshold(before));
        assert!(!is_older_than_threshold(exact));
        assert!(is_older_than_threshold(after));
    }
    #[test]
    fn test_log_period() {
        assert_eq!(
            log_period("2026-01-15T10:00:00Z INFO Started"),
            Some((2026, 1))
        );
        assert_eq!(
            log_period("2026-06-30T23:59:59Z INFO Finished"),
            Some((2026, 1))
        );
        assert_eq!(
            log_period("2026-07-01T00:00:00Z INFO Started"),
            Some((2026, 2))
        );
        assert_eq!(
            log_period("2026-12-31T23:59:59Z INFO Finished"),
            Some((2026, 2))
        );
        assert_eq!(log_period("Invalid log entry"), None);
    }
    #[test]
    fn test_last_log_line() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let log_path = temp_dir.path().join("organizer.log");
        std::fs::write(
            &log_path,
            "2026-06-30T23:59:59Z INFO First\n\
         2026-07-01T00:00:00Z INFO Second\n",
        )?;
        let last_line = last_log_line(&log_path)?;
        assert_eq!(
            last_line.as_deref(),
            Some("2026-07-01T00:00:00Z INFO Second")
        );

        Ok(())
    }
    #[test]
    fn test_should_rotate_log() {
        assert!(!should_rotate_log(Some((2026, 2)), (2026, 2)));
        assert!(should_rotate_log(Some((2026, 1)), (2026, 2)));
        assert!(should_rotate_log(Some((2026, 2)), (2027, 1)));
        assert!(!should_rotate_log(None, (2026, 2)));
    }
    #[test]
    fn test_rotation_needed_for_old_log_file() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let log_path = temp_dir.path().join("organizer.log");
        std::fs::write(&log_path, "2026-06-30T23:59:59Z INFO Organizer finished\n")?;
        let last_line = last_log_line(&log_path)?;
        let previous_period = last_line.as_deref().and_then(log_period);
        let current_period = (2026, 2);
        assert_eq!(previous_period, Some((2026, 1)));
        assert!(should_rotate_log(previous_period, current_period));

        Ok(())
    }
    #[test]
    fn test_archive_log() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let log_path = temp_dir.path().join("organizer.log");
        let content = "2026-06-30T23:59:59Z INFO Organizer finished\n";
        std::fs::write(&log_path, content)?;
        let archive_path = archive_log(&log_path, (2026, 1))?;
        let expected = temp_dir.path().join("organizer_2026_1.log");
        assert_eq!(archive_path, expected);
        assert!(!log_path.exists());
        assert!(archive_path.is_file());
        assert_eq!(std::fs::read_to_string(&archive_path)?, content);

        Ok(())
    }
    #[test]
    fn test_archive_log_does_not_overwrite() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let log_path = temp_dir.path().join("organizer.log");
        let archive_path = temp_dir.path().join("organizer_2026_1.log");
        std::fs::write(&log_path, "New log content")?;
        std::fs::write(&archive_path, "Existing archive content")?;
        let result = archive_log(&log_path, (2026, 1));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            std::fs::read_to_string(&archive_path)?,
            "Existing archive content"
        );
        assert_eq!(std::fs::read_to_string(&log_path)?, "New log content");

        Ok(())
    }
    #[test]
    fn test_complete_log_rotation() -> std::io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let log_path = temp_dir.path().join("organizer.log");
        let old_content = "2026-06-30T23:59:59Z INFO Old entry\n";
        std::fs::write(&log_path, old_content)?;

        // Read the previous period
        let last_line = last_log_line(&log_path)?;
        let previous_period = last_line.as_deref().and_then(log_period);
        // Simulate July 2026
        let current_period = (2026, 2);
        // Rotate if necessary
        if let Some(period) = previous_period
            && should_rotate_log(previous_period, current_period)
        {
            archive_log(&log_path, period)?;
        }

        let archive_path = temp_dir.path().join("organizer_2026_1.log");
        assert!(!log_path.exists());
        assert_eq!(std::fs::read_to_string(&archive_path)?, old_content);
        let new_content = "2026-07-01T00:00:00Z INFO New entry\n";
        std::fs::write(&log_path, new_content)?;

        // Both files should exist with the correct content
        assert_eq!(std::fs::read_to_string(&archive_path)?, old_content);
        assert_eq!(std::fs::read_to_string(&log_path)?, new_content);

        Ok(())
    }
}
