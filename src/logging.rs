use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use tracing_subscriber::fmt::writer::MakeWriterExt;

fn log_period(line: &str) -> Option<(u16, u8)> {
    let timestamp: &str = line.split_whitespace().next()?;

    let mut parts = timestamp.split('-');

    let year: u16 = parts.next()?.parse::<u16>().ok()?;
    let month: u8 = parts.next()?.parse::<u8>().ok()?;

    let half: u8 = match month {
        1..=6 => 1,
        7..=12 => 2,
        _ => return None,
    };

    Some((year, half))
}

fn last_log_line(path: &Path) -> io::Result<Option<String>> {
    let file: std::fs::File = std::fs::File::open(path)?;
    let reader: io::BufReader<std::fs::File> = io::BufReader::new(file);

    reader.lines().last().transpose()
}

fn should_rotate_log(previous: Option<(u16, u8)>, current: (u16, u8)) -> bool {
    match previous {
        Some(period) => period != current,
        None => false,
    }
}

fn current_log_period() -> io::Result<(u16, u8)> {
    let now: time::OffsetDateTime = time::OffsetDateTime::now_utc();
    let year: u16 =
        u16::try_from(now.year()).map_err(|_| io::Error::other("UTC year is out of range"))?;
    let month: u8 = u8::from(now.month());
    let half: u8 = if month <= 6 { 1 } else { 2 };

    Ok((year, half))
}

fn archive_log(path: &Path, period: (u16, u8)) -> io::Result<PathBuf> {
    let (year, half) = period;
    let archive_path: PathBuf = path.with_file_name(format!("organizer_{year}_{half}.log"));
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
    let directory: std::ffi::OsString = std::env::var_os("ORGANIZER_LOG_DIR").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "ORGANIZER_LOG_DIR environment variable is not set",
        )
    })?;

    Ok(PathBuf::from(directory))
}

pub(crate) fn initialize_logging() -> io::Result<()> {
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).map_err(io::Error::other)?;

    let stable_log_dir: PathBuf = log_directory()?;

    println!("Log directory: {}", stable_log_dir.display());

    let log_path: PathBuf = stable_log_dir.join("organizer.log");

    let previous_period: Option<(u16, u8)> = if log_path.try_exists()? {
        let last_line: Option<String> = last_log_line(&log_path)?;

        last_line.as_deref().and_then(log_period)
    } else {
        None
    };

    let current_period: (u16, u8) = current_log_period()?;

    let rotate: bool = should_rotate_log(previous_period, current_period);

    println!("Last log period: {:?}", previous_period);
    println!("Current UTC period: {:?}", current_period);
    println!("Log rotation needed: {}", rotate);

    if let Some(period) = previous_period
        && rotate
    {
        let archive_path: PathBuf = archive_log(&log_path, period)?;

        println!("Archived log: {}", archive_path.display());
    }

    let file_appender = tracing_appender::rolling::never(&stable_log_dir, "organizer.log");

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(std::io::stderr.and(file_appender))
        .init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
