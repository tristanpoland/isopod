use std::io::{Seek, SeekFrom, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::{volume_type, ISO_STANDARD_ID, SECTOR_SIZE};
use crate::directory::{Directory, DirectoryEntry};
use crate::utils;
use crate::Error;
use crate::Result;

/// A trait for volume descriptors in ISO 9660
pub trait VolumeDescriptor {
  /// Write the volume descriptor to the given writer
  fn write_to<W: Write + Seek>(&self, writer: &mut W) -> Result<()>;
}

/// Primary Volume Descriptor as defined in ISO 9660
#[derive(Debug, Clone)]
pub struct PrimaryVolumeDescriptor {
  /// Volume identifier
  volume_id: String,

  /// Publisher identifier
  publisher_id: String,

  /// Data preparer identifier
  preparer_id: String,

  /// Application identifier
  application_id: String,

  /// Volume creation date/time
  creation_time: SystemTime,

  /// Volume modification date/time
  modification_time: SystemTime,

  /// Root directory entry
  root_directory_entry: DirectoryEntry,

  /// Volume space size (in sectors)
  volume_space_size: u32,

  /// Block size (usually 2048)
  block_size: u16,

  /// Path table size in bytes
  path_table_size: u32,

  /// Location of the L path table
  path_table_location_l: u32,

  /// Location of the optional L path table
  optional_path_table_location_l: u32,

  /// Location of the M path table
  path_table_location_m: u32,

  /// Location of the optional M path table
  optional_path_table_location_m: u32,
}

impl PrimaryVolumeDescriptor {
  /// Create a new primary volume descriptor
  pub fn new(
    volume_id: &str,
    publisher_id: &str,
    preparer_id: &str,
    application_id: &str,
    root_directory: &Directory,
  ) -> Self {
    let now = SystemTime::now();

    // Create root directory entry
    let root_directory_entry = DirectoryEntry::new_directory(
      root_directory.name(),
      0, // Will be updated later
      0, // Will be updated later
    );

    // Calculate path table locations
    // Type L path table (little endian) - typically at sector 18
    let path_table_location_l = 18;
    
    // Optional Type L path table
    let optional_path_table_location_l = 0;
    
    // Type M path table (big endian) - typically at sector 19
    let path_table_location_m = 19;
    
    // Optional Type M path table
    let optional_path_table_location_m = 0;
    
    // Size for just root directory
    let path_table_size = 10;

    Self {
      volume_id: volume_id.to_string(),
      publisher_id: publisher_id.to_string(),
      preparer_id: preparer_id.to_string(),
      application_id: application_id.to_string(),
      creation_time: now,
      modification_time: now,
      root_directory_entry,
      volume_space_size: 0, // Will be calculated during write
      block_size: SECTOR_SIZE as u16,
      path_table_size: 0,       // Will be calculated during write
      path_table_location_l: 0, // Will be set during write
      optional_path_table_location_l: 0,
      path_table_location_m: 0, // Will be set during write
      optional_path_table_location_m: 0,
    }
  }

  /// Parse a primary volume descriptor from a buffer
  pub fn parse_from_buffer(buffer: &[u8]) -> Option<Self> {
    for sector in 0..16 {
      let offset = sector * SECTOR_SIZE;

      // Check if we have a primary volume descriptor
      if offset + 7 <= buffer.len()
        && buffer[offset] == volume_type::PRIMARY_VOLUME_DESCRIPTOR
        && &buffer[offset + 1..offset + 6] == ISO_STANDARD_ID
      {
        // Parse fields
        let volume_id = utils::parse_iso_string(&buffer[offset + 40..offset + 40 + 32]);
        let publisher_id = utils::parse_iso_string(&buffer[offset + 318..offset + 318 + 128]);
        let preparer_id = utils::parse_iso_string(&buffer[offset + 446..offset + 446 + 128]);
        let application_id = utils::parse_iso_string(&buffer[offset + 574..offset + 574 + 128]);

        // Parse dates
        let creation_time = utils::parse_iso_date(&buffer[offset + 813..offset + 813 + 17])
          .unwrap_or_else(|| SystemTime::now());
        let modification_time = utils::parse_iso_date(&buffer[offset + 830..offset + 830 + 17])
          .unwrap_or_else(|| SystemTime::now());

        // Parse sizes and locations
        let volume_space_size = utils::parse_u32_both(&buffer[offset + 80..offset + 80 + 8]);
        let block_size = utils::parse_u16_both(&buffer[offset + 128..offset + 128 + 4]);
        let path_table_size = utils::parse_u32_both(&buffer[offset + 132..offset + 132 + 8]);
        let path_table_location_l = u32::from_le_bytes([
          buffer[offset + 140],
          buffer[offset + 141],
          buffer[offset + 142],
          buffer[offset + 143],
        ]);
        let optional_path_table_location_l = u32::from_le_bytes([
          buffer[offset + 144],
          buffer[offset + 145],
          buffer[offset + 146],
          buffer[offset + 147],
        ]);
        let path_table_location_m = u32::from_be_bytes([
          buffer[offset + 148],
          buffer[offset + 149],
          buffer[offset + 150],
          buffer[offset + 151],
        ]);
        let optional_path_table_location_m = u32::from_be_bytes([
          buffer[offset + 152],
          buffer[offset + 153],
          buffer[offset + 154],
          buffer[offset + 155],
        ]);

        // Parse root directory entry
        let root_directory_entry =
          DirectoryEntry::parse_from_buffer(&buffer[offset + 156..offset + 156 + 34])
            .unwrap_or_else(|| DirectoryEntry::new_directory("ROOT", 0, 0));

        return Some(Self {
          volume_id,
          publisher_id,
          preparer_id,
          application_id,
          creation_time,
          modification_time,
          root_directory_entry,
          volume_space_size,
          block_size,
          path_table_size,
          path_table_location_l,
          optional_path_table_location_l,
          path_table_location_m,
          optional_path_table_location_m,
        });
      }
    }

    None
  }

  /// Get the volume ID
  pub fn volume_id(&self) -> &str {
    &self.volume_id
  }

  /// Set the volume ID
  pub fn set_volume_id(&mut self, volume_id: String) {
    self.volume_id = volume_id;
  }

  /// Get the publisher ID
  pub fn publisher_id(&self) -> &str {
    &self.publisher_id
  }

  /// Set the publisher ID
  pub fn set_publisher_id(&mut self, publisher_id: String) {
    self.publisher_id = publisher_id;
  }

  /// Get the preparer ID
  pub fn preparer_id(&self) -> &str {
    &self.preparer_id
  }

  /// Set the preparer ID
  pub fn set_preparer_id(&mut self, preparer_id: String) {
    self.preparer_id = preparer_id;
  }

  /// Get the application ID
  pub fn application_id(&self) -> &str {
    &self.application_id
  }

  /// Set the application ID
  pub fn set_application_id(&mut self, application_id: String) {
    self.application_id = application_id;
  }

  /// Get the creation time
  pub fn creation_time(&self) -> SystemTime {
    self.creation_time
  }

  /// Set the creation time
  pub fn set_creation_time(&mut self, time: SystemTime) {
    self.creation_time = time;
  }

  /// Get the modification time
  pub fn modification_time(&self) -> SystemTime {
    self.modification_time
  }

  /// Set the modification time
  pub fn set_modification_time(&mut self, time: SystemTime) {
    self.modification_time = time;
  }

  /// Get the root directory entry
  pub fn root_directory_entry(&self) -> &DirectoryEntry {
    &self.root_directory_entry
  }

  /// Get a mutable reference to the root directory entry
  pub fn root_directory_entry_mut(&mut self) -> &mut DirectoryEntry {
    &mut self.root_directory_entry
  }

  /// Get the volume space size (in sectors)
  pub fn volume_space_size(&self) -> u32 {
    self.volume_space_size
  }

  /// Set the volume space size
  pub fn set_volume_space_size(&mut self, size: u32) {
    self.volume_space_size = size;
  }

  /// Get the block size
  pub fn block_size(&self) -> u16 {
    self.block_size
  }

  /// Update volume descriptor with directory information
  pub fn update_with_directory(&mut self, root_dir: &Directory) {
    // Update root directory entry
    self.root_directory_entry = root_dir.to_entry();
  }
}

impl VolumeDescriptor for PrimaryVolumeDescriptor {
  fn write_to<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
    // Position at the 16th sector (after system area)
    let sector_position = SECTOR_SIZE as u64 * 16;
    writer.seek(SeekFrom::Start(sector_position))?;

    let mut buffer = [0u8; SECTOR_SIZE];

    // Type code
    buffer[0] = volume_type::PRIMARY_VOLUME_DESCRIPTOR;

    // Standard identifier
    buffer[1..6].copy_from_slice(ISO_STANDARD_ID);

    // Version
    buffer[6] = 1;

    // Unused field
    buffer[7] = 0;

    // System identifier (32 bytes, padded with spaces)
    utils::write_iso_string(&mut buffer[8..40], "");

    // Volume identifier (32 bytes, padded with spaces)
    utils::write_iso_string(&mut buffer[40..72], &self.volume_id);

    // Unused field (8 bytes)
    // buffer[72..80] already zeroed

    // Volume space size (both little and big endian)
    utils::write_u32_both(&mut buffer[80..88], self.volume_space_size);

    // Unused field (32 bytes)
    // buffer[88..120] already zeroed

    // Volume set size (both little and big endian)
    utils::write_u16_both(&mut buffer[120..124], 1);

    // Volume sequence number (both little and big endian)
    utils::write_u16_both(&mut buffer[124..128], 1);

    // Logical block size (both little and big endian)
    utils::write_u16_both(&mut buffer[128..132], self.block_size);

    // Path table size (both little and big endian)
    utils::write_u32_both(&mut buffer[132..140], self.path_table_size);

    // Path table locations
    buffer[140..144].copy_from_slice(&self.path_table_location_l.to_le_bytes());
    buffer[144..148].copy_from_slice(&self.optional_path_table_location_l.to_le_bytes());
    buffer[148..152].copy_from_slice(&self.path_table_location_m.to_be_bytes());
    buffer[152..156].copy_from_slice(&self.optional_path_table_location_m.to_be_bytes());

    // Root directory entry needs more space (40 bytes instead of 34)
    let mut root_dir_buffer = vec![0u8; 40];
    self
      .root_directory_entry
      .write_to_buffer(&mut root_dir_buffer[0..40])?;
    buffer[156..196].copy_from_slice(&root_dir_buffer[0..40]);

    // Volume set identifier (128 bytes, padded with spaces)
    utils::write_iso_string(&mut buffer[190..318], "");

    // Publisher identifier (128 bytes, padded with spaces)
    utils::write_iso_string(&mut buffer[318..446], &self.publisher_id);

    // Data preparer identifier (128 bytes, padded with spaces)
    utils::write_iso_string(&mut buffer[446..574], &self.preparer_id);

    // Application identifier (128 bytes, padded with spaces)
    utils::write_iso_string(&mut buffer[574..702], &self.application_id);

    // Copyright file identifier (38 bytes, padded with spaces)
    utils::write_iso_string(&mut buffer[702..740], "");

    // Abstract file identifier (36 bytes, padded with spaces)
    utils::write_iso_string(&mut buffer[740..776], "");

    // Bibliographic file identifier (37 bytes, padded with spaces)
    utils::write_iso_string(&mut buffer[776..813], "");

    // Volume creation date/time (17 bytes)
    utils::write_iso_date(&mut buffer[813..830], self.creation_time);

    // Volume modification date/time (17 bytes)
    utils::write_iso_date(&mut buffer[830..847], self.modification_time);

    // Volume expiration date/time (17 bytes)
    utils::write_iso_date(&mut buffer[847..864], SystemTime::UNIX_EPOCH);

    // Volume effective date/time (17 bytes)
    utils::write_iso_date(&mut buffer[864..881], self.creation_time);

    // File structure version
    buffer[881] = 1;

    // Reserved for future (1 byte)
    buffer[882] = 0;

    // Application use (512 bytes)
    // buffer[883..1395] already zeroed

    // Reserved for future (653 bytes)
    // buffer[1395..2048] already zeroed

    writer.write_all(&buffer)?;

    Ok(())
  }
}
