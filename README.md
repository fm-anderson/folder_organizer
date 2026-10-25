# Folder Organizer

A small Rust command-line tool that keeps directories such as `~/Downloads` organized by moving older files into subfolders based on their extensions.

By default, files must be **older than 7 days** before they are eligible. You can customize this threshold using `--min-age-days`, preview changes with `--dry-run`, and automate the organizer through `cron`.

## Features

- **Automatic categorization:** Sorts files into `documents`, `images`, `video`, `archives`, `data`, or `misc`.
- **Configurable age threshold:** Only processes files older than a specified number of days (7 days by default). Customize this using `--min-age-days`.
- **Dry-run mode:** Shows planned moves without modifying the files being organized.
- **Collision handling:** Adds numeric suffixes such as `report_1.pdf` rather than deliberately replacing a file with the same name.
- **Non-recursive processing:** Examines regular files directly inside the chosen directory, skipping subdirectories and hidden dotfiles.
- **Persistent logging:** Writes structured logs to a directory configured through `.env`, while also showing log messages in the terminal.
- **Half-year log archives:** Preserves previous logs as `organizer_YEAR_1.log` (January–June) or `organizer_YEAR_2.log` (July–December).

## Requirements

- Rust and Cargo.
- A `.env` file in the project's root directory.
- Filesystem access to the directory you want to organize.

## Installation

Clone the repository and enter its directory:

```bash
git clone https://github.com/fm-anderson/folder_organizer.git
cd folder_organizer
```

Create a `.env` file in the project root containing an **absolute** path for the logs:

```dotenv
ORGANIZER_LOG_DIR=/absolute/path/to/folder_organizer/logs
```

Build the executable:

```bash
cargo build --release
```

The binary will be available at `target/release/folder_organizer_rs`.

**Note:** The application loads `.env` from the project directory recorded at compile time, rather than searching the current working directory. Keep the project directory and its `.env` file available when running the binary, including from `cron`.

## Usage

Provide the directory to organize as a positional argument.

**Preview changes (recommended first):**

```bash
./target/release/folder_organizer_rs "$HOME/Downloads" --dry-run
```

**Organize files:**

```bash
./target/release/folder_organizer_rs "$HOME/Downloads"
```

**Run through Cargo during development:**

```bash
cargo run -- "$HOME/Downloads" --dry-run
```

### Configurable Age Threshold

By default, the organizer only processes files older than **7 days**.

Use the `--min-age-days` option to change this threshold without modifying the source code or rebuilding the binary.

**Organize files older than 30 days:**

```bash
./target/release/folder_organizer_rs "$HOME/Downloads" --min-age-days 30
```

**Preview files older than 14 days:**

```bash
./target/release/folder_organizer_rs "$HOME/Downloads" --min-age-days 14 --dry-run
```

**Use the default threshold of 7 days:**

```bash
./target/release/folder_organizer_rs "$HOME/Downloads"
```

The threshold is measured using each file's last modification time. Files must be strictly older than the specified number of days to qualify.

Run `./target/release/folder_organizer_rs --help` to see all available CLI options.

**Run the test suite:**

```bash
cargo test
```

`--dry-run` prevents file moves and destination-directory creation, but it **does not disable logging or log archiving**.

## File categories

The current extension rules are defined in `category_for()` in `src/organizer.rs`:

| Destination  | Extensions                                                      |
| ------------ | --------------------------------------------------------------- |
| `documents/` | `md`, `pdf`, `docx`, `doc`, `pptx`, `ppt`, `xlsx`, `xls`, `txt` |
| `images/`    | `jpg`, `jpeg`, `png`, `gif`, `webp`, `tiff`, `svg`, `heic`      |
| `video/`     | `mp4`, `mov`, `mkv`, `avi`                                      |
| `archives/`  | `zip`, `rar`, `7z`, `tar`, `gz`                                 |
| `data/`      | `json`, `csv`, `xml`, `yml`                                     |
| `misc/`      | Unknown extensions and files without extensions                 |

Extensions are matched case-insensitively for ASCII letters.

An eligible `report.pdf` is moved to `documents/report.pdf`. If that destination is occupied, the organizer tries `report_1.pdf`, `report_2.pdf`, and so on.

### Customizing File Categories

File categories are defined in the `category_for()` function in `src/organizer.rs`.
To add or modify extension mappings, edit this function and rebuild the binary using `cargo build --release`.
Categories are currently **code-defined** and cannot be changed through `.env` or CLI arguments.

## Logging and archiving

The log directory comes from `ORGANIZER_LOG_DIR` in `.env`.

The active log is named `organizer.log`. On startup, the organizer examines its last entry and compares that entry's year and half-year with the current **UTC** date:

- **Same half-year:** Append to `organizer.log`.
- **Different half-year:** Rename the existing log to `organizer_YEAR_1.log` or `organizer_YEAR_2.log`, then create a fresh `organizer.log`.
- **No existing log:** Create `organizer.log` normally.

For example:

```text
logs/
├── organizer.log
├── organizer_2026_1.log
└── organizer_2026_2.log
```

Archives are labeled according to the period of their previous log entries. Rotation is checked **when the program starts**, not continuously at midnight. If an archive with the target filename already exists, the program returns an error rather than intentionally overwriting it.

## Daily automation with cron (macOS/Linux)

To schedule the organizer for **9:00 PM every day**, first ensure it is built and the configured log directory exists:

```bash
cargo build --release
mkdir -p /absolute/path/to/folder_organizer/logs
```

Open your crontab:

```bash
crontab -e
```

Add an entry using the absolute paths on your machine:

```cron
# Organize Downloads every day at 9:00 PM
0 21 * * * /absolute/path/to/folder_organizer/target/release/folder_organizer_rs /absolute/path/to/Downloads >> /absolute/path/to/folder_organizer/logs/cron.log 2>&1
```

The application writes structured events to `organizer.log`. The `>> .../cron.log 2>&1` portion separately captures terminal output and errors from the scheduled command.

Check the schedule with `crontab -l`. To test the same command manually, run everything **after** the five cron scheduling fields in your terminal. Be aware that this performs real file moves unless you add `--dry-run` before the output redirection.

On macOS, the cron service may need permission to access protected directories such as Downloads. Scheduled runs can also be missed while your Mac is asleep.

**After source changes, rebuild the executable used by cron** (`cargo build --release` for the example above). If your crontab points to `target/debug/folder_organizer_rs` instead, use `cargo build` or update the crontab to the release binary.

## Project structure

```text
src/
├── main.rs       # CLI parsing and application flow
├── organizer.rs  # Classification, age rules, file moving, and unit tests
└── logging.rs    # Logging setup, half-year archiving, and unit tests
```

## License

This project is licensed under the [MIT License](LICENSE.md).
