use std::fmt;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use std::process::Command;

use defmt_decoder::Locations;
use object::read::Object as _;
use object::read::ObjectSection as _;

use crate::error::{Error, Result};
use crate::manifest::FirmwareInfo;

const MAYNA_META_SECTION: &str = ".mayna_meta";

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Chip {
    Esp32,
    Esp32c2,
    Esp32c3,
    Esp32c5,
    Esp32c6,
    Esp32h2,
    Esp32p4,
    Esp32s2,
    Esp32s3,
}

impl fmt::Display for Chip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Chip::Esp32 => "esp32",
            Chip::Esp32c2 => "esp32c2",
            Chip::Esp32c3 => "esp32c3",
            Chip::Esp32c5 => "esp32c5",
            Chip::Esp32c6 => "esp32c6",
            Chip::Esp32h2 => "esp32h2",
            Chip::Esp32p4 => "esp32p4",
            Chip::Esp32s2 => "esp32s2",
            Chip::Esp32s3 => "esp32s3",
        };
        f.write_str(s)
    }
}

/// Parsed ELF file data. Reads and parses the ELF once; individual components
/// can then be written out without re-reading or re-parsing.
pub struct ElfData {
    mayna_meta: Option<FirmwareInfo>,
    defmt_table: Option<defmt_decoder::Table>,
    defmt_locations: Option<Locations>,
}

impl ElfData {
    /// Read and parse an ELF file in a single pass.
    pub fn parse(elf_path: &Path) -> Result<Self> {
        let raw = fs::read(elf_path).map_err(|source| Error::Io {
            path: elf_path.to_path_buf(),
            source,
        })?;

        let obj = object::File::parse(&*raw).map_err(|e| Error::InvalidElf {
            reason: e.to_string(),
        })?;

        let mayna_meta = match obj.section_by_name(MAYNA_META_SECTION) {
            Some(section) => {
                let data = section.data().map_err(|e| Error::InvalidElf {
                    reason: format!("failed to read {MAYNA_META_SECTION} section: {e}"),
                })?;
                Some(serde_json::from_slice(data)?)
            }
            None => None,
        };

        let defmt_table = defmt_decoder::Table::parse(&raw).map_err(|e| Error::Defmt {
            reason: e.to_string(),
        })?;

        let defmt_locations = match &defmt_table {
            Some(table) => Some(table.get_locations(&raw).map_err(|e| Error::Defmt {
                reason: e.to_string(),
            })?),
            None => None,
        };

        Ok(Self {
            mayna_meta,
            defmt_table,
            defmt_locations,
        })
    }

    /// Return the parsed `.mayna_meta` firmware info.
    pub fn mayna_meta(&self) -> Result<&FirmwareInfo> {
        self.mayna_meta.as_ref().ok_or(Error::SectionNotFound {
            section: MAYNA_META_SECTION.to_string(),
        })
    }

    /// Serialize the defmt string table to a byte vector with postcard.
    pub fn serialize_defmt_table(&self) -> Result<Vec<u8>> {
        let table = self.defmt_table.as_ref().ok_or(Error::Defmt {
            reason: "no defmt data found in ELF".to_string(),
        })?;
        Ok(postcard::to_allocvec(table)?)
    }

    /// Serialize the defmt location data to a byte vector with postcard.
    pub fn serialize_defmt_locations(&self) -> Result<Vec<u8>> {
        let locations = self.defmt_locations.as_ref().ok_or(Error::Defmt {
            reason: "no defmt data found in ELF".to_string(),
        })?;
        Ok(postcard::to_allocvec(locations)?)
    }

    /// Serialize the defmt string table with postcard and write to a file.
    pub fn write_defmt_table(&self, output_path: &Path) -> Result<()> {
        let table = self.defmt_table.as_ref().ok_or(Error::Defmt {
            reason: "no defmt data found in ELF".to_string(),
        })?;

        let writer = BufWriter::new(File::create(output_path).map_err(|source| Error::Io {
            path: output_path.to_path_buf(),
            source,
        })?);

        postcard::to_io(table, writer)?;
        Ok(())
    }

    /// Serialize the defmt location data with postcard and write to a file.
    pub fn write_defmt_locations(&self, output_path: &Path) -> Result<()> {
        let locations = self.defmt_locations.as_ref().ok_or(Error::Defmt {
            reason: "no defmt data found in ELF".to_string(),
        })?;

        let writer = BufWriter::new(File::create(output_path).map_err(|source| Error::Io {
            path: output_path.to_path_buf(),
            source,
        })?);

        postcard::to_io(locations, writer)?;
        Ok(())
    }
}

// Convenience free functions for single-extraction use (e.g. CLI subcommands).

/// Extract the `.mayna_meta` JSON section from an ELF file and parse it.
pub fn extract_mayna_meta(elf_path: &Path) -> Result<FirmwareInfo> {
    ElfData::parse(elf_path)?.mayna_meta().cloned()
}

/// Build the argument list for an `espflash save-image` invocation.
fn espflash_save_image_args(
    elf_path: &Path,
    output_path: &Path,
    chip: Chip,
    extra_args: Option<&str>,
) -> Result<Vec<String>> {
    let mut args = vec![
        "save-image".to_string(),
        "--chip".to_string(),
        chip.to_string(),
    ];

    if let Some(extra) = extra_args {
        let parsed = shell_words::split(extra).map_err(|e| Error::Espflash {
            reason: format!("failed to parse espflash extra args: {e}"),
        })?;
        args.extend(parsed);
    }

    args.push(elf_path.to_string_lossy().into_owned());
    args.push(output_path.to_string_lossy().into_owned());

    Ok(args)
}

/// Convert an ELF file to a flashable firmware binary using `espflash save-image`.
pub fn extract_firmware_binary(
    elf_path: &Path,
    output_path: &Path,
    chip: Chip,
    extra_args: Option<&str>,
) -> Result<()> {
    let args = espflash_save_image_args(elf_path, output_path, chip, extra_args)?;

    let output = Command::new("espflash")
        .args(&args)
        .output()
        .map_err(|e| Error::Espflash {
            reason: format!("failed to run espflash: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Espflash {
            reason: format!("espflash exited with {}: {stderr}", output.status),
        });
    }

    Ok(())
}

/// Extract the defmt string table from the ELF file, serialize it with postcard,
/// and write to the output path.
pub fn extract_defmt_table(elf_path: &Path, output_path: &Path) -> Result<()> {
    ElfData::parse(elf_path)?.write_defmt_table(output_path)
}

/// Extract defmt location data (log line → source file mapping) from DWARF info,
/// serialize it with postcard, and write to the output path.
pub fn extract_defmt_locations(elf_path: &Path, output_path: &Path) -> Result<()> {
    ElfData::parse(elf_path)?.write_defmt_locations(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn espflash_args_no_extra() {
        let args = espflash_save_image_args(
            Path::new("fw.elf"),
            Path::new("fw.bin"),
            Chip::Esp32,
            None,
        )
        .unwrap();
        assert_eq!(
            args,
            ["save-image", "--chip", "esp32", "fw.elf", "fw.bin"]
        );
    }

    #[test]
    fn espflash_args_simple_extra() {
        let args = espflash_save_image_args(
            Path::new("fw.elf"),
            Path::new("fw.bin"),
            Chip::Esp32,
            Some("--flash-mode dio --flash-size 4mb"),
        )
        .unwrap();
        assert_eq!(
            args,
            [
                "save-image",
                "--chip",
                "esp32",
                "--flash-mode",
                "dio",
                "--flash-size",
                "4mb",
                "fw.elf",
                "fw.bin",
            ]
        );
    }

    #[test]
    fn espflash_args_quoted_value_with_spaces() {
        let args = espflash_save_image_args(
            Path::new("fw.elf"),
            Path::new("fw.bin"),
            Chip::Esp32s3,
            Some("--flash-size \"4 mb\""),
        )
        .unwrap();
        assert_eq!(
            args,
            [
                "save-image",
                "--chip",
                "esp32s3",
                "--flash-size",
                "4 mb",
                "fw.elf",
                "fw.bin",
            ]
        );
    }

    #[test]
    fn espflash_args_with_hyphen_prefixed_values() {
        let args = espflash_save_image_args(
            Path::new("fw.elf"),
            Path::new("fw.bin"),
            Chip::Esp32c6,
            Some("--help"),
        )
        .unwrap();
        assert_eq!(
            args,
            ["save-image", "--chip", "esp32c6", "--help", "fw.elf", "fw.bin"]
        );
    }
}
