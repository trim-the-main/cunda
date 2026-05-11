use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mayna::elf::Chip;

#[derive(Parser)]
#[command(name = "mayna")]
#[command(about = "Firmware distribution package tool for Cunda project")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a .mayna archive from a firmware ELF file
    /// if all is successful a new file is created with filename `device-type_firmware-version.mayna`
    Create {
        /// Path to the firmware ELF file
        #[arg(long)]
        elf: PathBuf,

        /// Target chip
        #[arg(long)]
        chip: Chip,

        /// Extra arguments passed through to espflash save-image
        #[arg(long, allow_hyphen_values = true)]
        espflash_args: Option<String>,

        /// Path to the partition table file (optional)
        #[arg(long)]
        partition_table: Option<PathBuf>,

        /// Path to the changelog text file (optional)
        #[arg(long)]
        changelog: Option<PathBuf>,

        /// Path to the bootloader binary (optional)
        #[arg(long)]
        bootloader: Option<PathBuf>,

        /// Minimum firmware version required for OTA update (default: "0.0.0")
        #[arg(long, default_value = "0.0.0")]
        min_firmware_version: String,

        /// Publish date in ISO 8601 format (default: today)
        #[arg(long)]
        publish_date: Option<String>,

        /// Output directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },

    /// Dump the .mayna_meta JSON from an ELF file
    DumpMeta {
        /// Path to the firmware ELF file
        elf: PathBuf,
    },

    /// Dump a flashable firmware binary from an ELF file
    DumpFirmware {
        /// Path to the firmware ELF file
        elf: PathBuf,

        /// Target chip
        #[arg(long)]
        chip: Chip,

        /// Extra arguments passed through to espflash save-image
        #[arg(long, allow_hyphen_values = true)]
        espflash_args: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Dump the defmt string table from an ELF file
    DumpDefmtTable {
        /// Path to the firmware ELF file
        elf: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Dump defmt location data from an ELF file
    DumpDefmtLocations {
        /// Path to the firmware ELF file
        elf: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Inspect and verify a .mayna archive
    Inspect {
        /// Path to the .mayna archive
        package: PathBuf,

        /// Output directory (default: a new temporary directory)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<(), mayna::error::Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create {
            elf,
            chip,
            espflash_args,
            partition_table,
            changelog,
            bootloader,
            min_firmware_version,
            publish_date,
            output,
        } => {
            let config = mayna::package::CreateConfig {
                elf_path: elf,
                chip,
                extra_args: espflash_args,
                partition_table_path: partition_table,
                changelog_path: changelog,
                bootloader_path: bootloader,
                min_firmware_version,
                publish_date,
                output_dir: output,
            };
            let pkg_path = mayna::package::create(config)?;
            println!("Created {}", pkg_path.display());
        }

        Commands::DumpMeta { elf } => {
            let meta = mayna::elf::extract_mayna_meta(&elf)?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }

        Commands::DumpFirmware {
            elf,
            chip,
            espflash_args,
            output,
        } => {
            mayna::elf::extract_firmware_binary(&elf, &output, chip, espflash_args.as_deref())?;
            println!("Wrote firmware binary to {}", output.display());
        }

        Commands::DumpDefmtTable { elf, output } => {
            mayna::elf::extract_defmt_table(&elf, &output)?;
            println!("Wrote defmt table to {}", output.display());
        }

        Commands::DumpDefmtLocations { elf, output } => {
            mayna::elf::extract_defmt_locations(&elf, &output)?;
            println!("Wrote defmt locations to {}", output.display());
        }

        Commands::Inspect { package, output } => {
            // If no output dir given, create a temp dir that persists after exit
            // (into_path() prevents cleanup on drop so the user can inspect it)
            let output_dir = match output {
                Some(dir) => dir,
                None => {
                    let tmp = tempfile::tempdir().map_err(|source| mayna::error::Error::Io {
                        path: std::path::PathBuf::from("tempdir"),
                        source,
                    })?;
                    tmp.keep()
                }
            };
            let manifest = mayna::package::extract(&package, &output_dir)?;
            println!(
                "Extracted {} v{} ({} components) to {}",
                manifest.device_type,
                manifest.firmware_version,
                manifest.components.len(),
                output_dir.display()
            );
        }
    }

    Ok(())
}
