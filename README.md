test command

touch -t 202301011200 old_image.jpg && touch -t 202301011200 old_document.txt && touch -t 202301011200 old_archive.zip && touch -t 202301011200 old_file.unknown && touch new_file.txt

ruby organizer.rb ~/test_downloads_organizer

---

# 📂 Folder Organizer

A command-line Ruby script that automatically organizes files in a specified directory into subfolders based on their file type and age. This tool is designed to keep directories like `~/Downloads` clean and manageable with minimal effort.

---

## ✨ Features

- **Automatic Categorization:** Moves files into subfolders (e.g., `images`, `documents`, `archives`) based on their extension.
- **Configurable Rules:** Easily customize which file types go into which folders by editing a simple mapping hash.
- **Time-Based Delay:** Ignores new files by default (e.g., files newer than 7 days) so you can easily access recent downloads.
- **Safe Dry Run Mode:** Preview all changes with the `--dry-run` flag before any files are actually moved.
- **Collision Handling:** Automatically renames files by appending a suffix (e.g., `file_1.txt`) if a file with the same name already exists in the destination.

---

## ✅ Requirements

- A working installation of **Ruby**.

---

## ⚙️ Installation

1.  Save the code as `organizer.rb` in a directory of your choice.
2.  No other installation steps or gems are required.

---

## 🚀 Usage

You run the script from your terminal, providing the path to the directory you want to organize as a command-line argument.

### Basic Usage

```bash
ruby organizer.rb <path_to_directory>
```

### Example

To organize your main Downloads folder:

```bash
ruby organizer.rb ~/Downloads
```

### Dry Run Mode

To see a preview of what the script _would_ do without moving any files, use the `--dry-run` flag. This is highly recommended before running it on a directory for the first time.

```bash
# Preview changes for your Downloads folder
ruby organizer.rb --dry-run ~/Downloads
```

---

## 🔧 Configuration

You can easily customize the script's behavior by editing the constants at the top of the `organizer.rb` file.

### File & Folder Mapping

The `FOLDER_MAPPING` hash controls where each file type is moved. You can add new extensions or change the destination folders to fit your needs.

```ruby
FOLDER_MAPPING = {
  # Add your own rules here
  '.zip'    => 'archives',
  '.pdf'    => 'documents',
  '.jpg'    => 'images',
  # ... etc.
}
```

### Age Threshold

The `AGE_THRESHOLD_SECONDS` constant defines how old a file must be before it is moved. The calculation is `days * hours * minutes * seconds`.

```ruby
# To ignore files newer than 14 days
AGE_THRESHOLD_SECONDS = 14 * 24 * 60 * 60

# To ignore files newer than 30 days
AGE_THRESHOLD_SECONDS = 30 * 24 * 60 * 60
```

---

## 🤖 Automation (macOS/Linux)

You can schedule this script to run automatically using `cron`.

1.  Find the full path to your Ruby installation by running `which ruby`.
2.  Open your crontab for editing by running `crontab -e`.
3.  Add the following line, replacing the paths with your own. This example runs the script every day at 9:00 PM.

<!-- end list -->

```bash
# Runs every day at 9 PM
0 21 * * * cd /path/to/your/script/folder && /full/path/to/ruby organizer.rb $HOME/Downloads
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE.md).
