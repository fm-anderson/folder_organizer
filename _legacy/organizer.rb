require 'fileutils'
require 'logger'

log = Logger.new('organizer.log', 'monthly')
log.info("Script execution started")

dry_run = ARGV.include?("--dry-run")
ARGV.delete("--dry-run")
target_path = ARGV[0]

if target_path.nil?
  log.warn("No directory path provided. Exiting.")
  puts "Usage: ruby organizer.rb <directory_path>"
  exit 1
end

absolute_path = File.expand_path(target_path)
current_time = Time.now
AGE_THRESHOLD_SECONDS = 7 * 24 * 60 * 60
FOLDER_MAPPING = {
  # -- Images & Graphics --
  '.jpg'    => 'images',
  '.jpeg'   => 'images',
  '.png'    => 'images',
  '.gif'    => 'images',
  '.webp'   => 'images',
  '.tiff'   => 'images',
  '.svg'    => 'images',
  '.heic'   => 'images', # Apple's image format
  '.psd'    => 'photoshop', # Adobe Photoshop
  '.ai'     => 'illustrator', # Adobe Illustrator

  # -- Documents --
  '.pdf'    => 'documents',
  '.docx'   => 'documents',
  '.doc'    => 'documents',
  '.pptx'   => 'documents',
  '.ppt'    => 'documents',
  '.xlsx'   => 'documents',
  '.xls'    => 'documents',
  '.txt'    => 'documents',
  '.rtf'    => 'documents',
  '.md'     => 'documents',
  '.pages'  => 'documents', # Apple Pages
  '.key'    => 'documents', # Apple Keynote

  # -- Audio & Video --
  '.mp3'    => 'audio',
  '.m4a'    => 'audio',
  '.wav'    => 'audio',
  '.flac'   => 'audio',
  '.mp4'    => 'video',
  '.mov'    => 'video',
  '.mkv'    => 'video',
  '.avi'    => 'video',

  # -- Archives --
  '.zip'    => 'archives',
  '.rar'    => 'archives',
  '.7z'     => 'archives',
  '.tar'    => 'archives',
  '.gz'     => 'archives',

  # -- Code & Data --
  '.rb'     => 'code',
  '.js'     => 'code',
  '.py'     => 'code',
  '.html'   => 'code',
  '.css'    => 'code',
  '.sql'    => 'code',
  '.json'   => 'data',
  '.csv'    => 'data',
  '.xml'    => 'data',
  '.yml'    => 'data',

  # -- Installers & System --
  '.dmg'    => 'installers', # macOS Application Images
  '.pkg'    => 'installers',
  '.exe'    => 'installers',
  '.iso'    => 'installers',
  '.app'    => 'installers',
  '.ttf'    => 'fonts',
  '.otf'    => 'fonts',
}

if File.directory?(absolute_path)
  log.info("Organizing → #{absolute_path}")
else
  log.error("Directory not found: '#{target_path}'. Exiting.")
  puts "Error: '#{target_path}' is not a valid directory."
  exit 1
end

Dir.glob(File.join(absolute_path, '*')).each do |full_path|
  next unless File.file?(full_path)
  file_age = current_time - File.mtime(full_path)

  if file_age > AGE_THRESHOLD_SECONDS
    file_ext = File.extname(full_path).downcase
    destination_folder = FOLDER_MAPPING[file_ext] || 'misc'
    destination_path = File.join(absolute_path, destination_folder)
    filename = File.basename(full_path)

    final_destination_path = File.join(destination_path, filename)
    count = 1
    while File.exist?(final_destination_path)
      basename = File.basename(filename, ".*")
      extension = File.extname(filename)
      new_filename = "#{basename}_#{count}#{extension}"
      final_destination_path = File.join(destination_path, new_filename)
      count += 1
    end

    final_filename = File.basename(final_destination_path)

    if dry_run
      if filename != final_filename
        puts "[Dry Run] would move '#{filename}' to '#{destination_folder}/' as '#{final_filename}' (collision detected)"
      else
        puts "[Dry Run] would move '#{filename}' to '#{destination_folder}/'"
      end
    else
      FileUtils.mkdir_p(destination_path)
      FileUtils.mv(full_path, final_destination_path)
      log.info("Moved '#{filename}' to '#{destination_folder}/' as '#{final_filename}'")
    end
  end
end

log.info("Script execution finished")
log.info("------------------")