use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

use crate::elf::{self, Chip, ElfData};
use crate::error::{Error, Result};
use crate::manifest::{Component, FirmwareInfo, PackageManifest};
use crate::verify;

const MANIFEST_FILENAME: &str = "manifest.json";

/// Configuration for creating a `.mayna` archive.
pub struct CreateConfig {
    pub elf_path: PathBuf,
    pub chip: Chip,
    pub extra_args: Option<String>,
    pub partition_table_path: Option<PathBuf>,
    pub changelog_path: Option<PathBuf>,
    pub bootloader_path: Option<PathBuf>,
    pub min_firmware_version: String,
    pub publish_date: Option<String>,
    pub output_dir: PathBuf,
}

/// A named blob of bytes to include in the archive.
struct ArchiveEntry {
    key: String,
    file_name: String,
    data: Vec<u8>,
}

impl ArchiveEntry {
    fn component(&self) -> Component {
        let hash = Sha256::digest(&self.data);
        Component {
            file: self.file_name.clone(),
            size: self.data.len() as u64,
            sha256: hash.iter().map(|b| format!("{b:02x}")).collect(),
        }
    }
}

/// Gather all components (firmware binary, defmt data, optional files) into in-memory entries.
fn collect_entries(config: &CreateConfig, elf_data: &ElfData) -> Result<Vec<ArchiveEntry>> {
    // espflash requires file paths, so write firmware binary to a temp file.
    // TempDir cleans up automatically when dropped.
    let tmp_dir = tempfile::tempdir().map_err(|source| Error::Io {
        path: PathBuf::from("tempdir"),
        source,
    })?;
    let firmware_path = tmp_dir.path().join("firmware.bin");
    elf::extract_firmware_binary(
        &config.elf_path,
        &firmware_path,
        config.chip,
        config.extra_args.as_deref(),
    )?;
    let firmware_data = fs::read(&firmware_path).map_err(|source| Error::Io {
        path: firmware_path,
        source,
    })?;

    let mut entries = vec![
        ArchiveEntry {
            key: "firmware".to_string(),
            file_name: "firmware.bin".to_string(),
            data: firmware_data,
        },
        ArchiveEntry {
            key: "defmt_table".to_string(),
            file_name: "defmt_table.bin".to_string(),
            data: elf_data.serialize_defmt_table()?,
        },
        ArchiveEntry {
            key: "defmt_locations".to_string(),
            file_name: "defmt_locations.bin".to_string(),
            data: elf_data.serialize_defmt_locations()?,
        },
    ];

    if let Some(ref src) = config.partition_table_path {
        entries.push(ArchiveEntry {
            key: "partition_table".to_string(),
            file_name: "partition_table.bin".to_string(),
            data: fs::read(src).map_err(|source| Error::Io {
                path: src.clone(),
                source,
            })?,
        });
    }

    if let Some(ref src) = config.changelog_path {
        entries.push(ArchiveEntry {
            key: "changelog".to_string(),
            file_name: "changelog.txt".to_string(),
            data: fs::read(src).map_err(|source| Error::Io {
                path: src.clone(),
                source,
            })?,
        });
    }

    if let Some(ref src) = config.bootloader_path {
        entries.push(ArchiveEntry {
            key: "bootloader".to_string(),
            file_name: "bootloader.bin".to_string(),
            data: fs::read(src).map_err(|source| Error::Io {
                path: src.clone(),
                source,
            })?,
        });
    }

    Ok(entries)
}

/// Build a `PackageManifest` from firmware metadata and archive entries.
fn build_manifest(
    meta: &FirmwareInfo,
    entries: &[ArchiveEntry],
    min_firmware_version: String,
    publish_date: Option<String>,
) -> PackageManifest {
    let mut components = BTreeMap::new();
    for entry in entries {
        components.insert(entry.key.clone(), entry.component());
    }

    let publish_date =
        publish_date.unwrap_or_else(|| time::OffsetDateTime::now_utc().date().to_string());

    PackageManifest {
        format_version: PackageManifest::FORMAT_VERSION,
        device_type: meta.device_type.clone(),
        firmware_version: meta.firmware_version.clone(),
        protocol_version: meta.protocol_version,
        min_firmware_version,
        publish_date,
        components,
    }
}

/// Write manifest and entries into a ZIP archive (store mode, no compression).
fn write_archive(archive_path: &Path, manifest_json: &str, entries: &[ArchiveEntry]) -> Result<()> {
    let file = File::create(archive_path).map_err(|source| Error::Io {
        path: archive_path.to_path_buf(),
        source,
    })?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip.start_file(MANIFEST_FILENAME, options)
        .map_err(|e| Error::Package {
            reason: format!("failed to add {MANIFEST_FILENAME} to archive: {e}"),
        })?;
    zip.write_all(manifest_json.as_bytes())
        .map_err(|source| Error::Io {
            path: archive_path.to_path_buf(),
            source,
        })?;

    for entry in entries {
        zip.start_file(&entry.file_name, options)
            .map_err(|e| Error::Package {
                reason: format!("failed to add {} to archive: {e}", entry.file_name),
            })?;
        zip.write_all(&entry.data).map_err(|source| Error::Io {
            path: archive_path.to_path_buf(),
            source,
        })?;
    }

    zip.finish().map_err(|e| Error::Package {
        reason: format!("failed to finalize archive: {e}"),
    })?;

    Ok(())
}

/// Create a `.mayna` archive from build artifacts.
///
/// Extracts metadata, firmware binary, defmt table, and defmt locations from the
/// ELF file. Computes SHA256 for each component. Produces a ZIP (store mode)
/// archive named `<device_type>_<firmware_version>.mayna`.
///
/// Returns the path to the created archive.
pub fn create(config: CreateConfig) -> Result<PathBuf> {
    let elf_data = ElfData::parse(&config.elf_path)?;
    let meta = elf_data.mayna_meta()?.clone();

    let entries = collect_entries(&config, &elf_data)?;
    let manifest = build_manifest(
        &meta,
        &entries,
        config.min_firmware_version,
        config.publish_date,
    );
    let manifest_json = serde_json::to_string_pretty(&manifest)?;

    let archive_name = format!("{}_{}.mayna", meta.device_type, meta.firmware_version);
    let archive_path = config.output_dir.join(&archive_name);
    write_archive(&archive_path, &manifest_json, &entries)?;

    Ok(archive_path)
}

/// Parse `manifest.json` from a ZIP archive.
fn read_manifest(archive: &mut ZipArchive<File>) -> Result<PackageManifest> {
    let mut entry = archive
        .by_name(MANIFEST_FILENAME)
        .map_err(|e| Error::Package {
            reason: format!("{MANIFEST_FILENAME} not found in archive: {e}"),
        })?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).map_err(|source| Error::Io {
        path: PathBuf::from(MANIFEST_FILENAME),
        source,
    })?;
    Ok(serde_json::from_str(&buf)?)
}

/// Check that no output file already exists in `output_dir`.
fn check_no_conflicts(output_dir: &Path, manifest: &PackageManifest) -> Result<()> {
    let manifest_out = output_dir.join(MANIFEST_FILENAME);
    if manifest_out.exists() {
        return Err(Error::Package {
            reason: format!("file already exists: {}", manifest_out.display()),
        });
    }
    for component in manifest.components.values() {
        let path = output_dir.join(&component.file);
        if path.exists() {
            return Err(Error::Package {
                reason: format!("file already exists: {}", path.display()),
            });
        }
    }
    Ok(())
}

/// Extract and verify a `.mayna` archive.
///
/// Extracts all components to `output_dir`, verifying each component's size and
/// SHA256 checksum against the manifest. Returns the parsed manifest on success.
///
/// On any verification failure (missing file, size mismatch, checksum mismatch),
/// cleans up extracted files and returns an error.
pub fn extract(pkg_path: &Path, output_dir: &Path) -> Result<PackageManifest> {
    let file = File::open(pkg_path).map_err(|source| Error::Io {
        path: pkg_path.to_path_buf(),
        source,
    })?;
    let mut archive = ZipArchive::new(file).map_err(|e| Error::Package {
        reason: format!("failed to open archive: {e}"),
    })?;

    let manifest = read_manifest(&mut archive)?;
    check_no_conflicts(output_dir, &manifest)?;

    // Track extracted files for cleanup on failure
    let mut extracted: Vec<PathBuf> = Vec::new();

    let result = (|| -> Result<()> {
        let manifest_out = output_dir.join(MANIFEST_FILENAME);
        fs::write(&manifest_out, serde_json::to_string_pretty(&manifest)?).map_err(|source| {
            Error::Io {
                path: manifest_out.clone(),
                source,
            }
        })?;
        extracted.push(manifest_out);

        for (_key, component) in &manifest.components {
            let out_path = output_dir.join(&component.file);
            {
                let mut entry = archive
                    .by_name(&component.file)
                    .map_err(|e| Error::Package {
                        reason: format!("component '{}' not found in archive: {e}", component.file),
                    })?;
                let mut out_file = File::create(&out_path).map_err(|source| Error::Io {
                    path: out_path.clone(),
                    source,
                })?;
                std::io::copy(&mut entry, &mut out_file).map_err(|source| Error::Io {
                    path: out_path.clone(),
                    source,
                })?;
            }
            extracted.push(out_path.clone());
            verify::verify_component(&out_path, component)?;
        }

        Ok(())
    })();

    if let Err(e) = result {
        for path in &extracted {
            let _ = fs::remove_file(path);
        }
        return Err(e);
    }

    Ok(manifest)
}
