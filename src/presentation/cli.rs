//! Clap CLI wiring into application use cases.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

use crate::application::{ConvertUseCase, FixOptions, FixUseCase, InspectOptions, InspectUseCase};
use crate::domain::constants::DEFAULT_TESS_TOL_MM;
use crate::domain::profiles::{list_profiles, load_profile};
use crate::infrastructure::{FileMeshRepository, SilentProgress, StderrProgress};

#[derive(Debug, Parser)]
#[command(
    name = "k1fix",
    version,
    about = "Fix 3D models for Creality K1 printers"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Inspect a model and print diagnostics
    Inspect {
        file: PathBuf,
        #[arg(long, default_value = "k1")]
        profile: String,
        #[arg(long)]
        json: Option<PathBuf>,
        #[arg(long, default_value_t = DEFAULT_TESS_TOL_MM)]
        tess_tol: f64,
        #[arg(long, default_value_t = 3.0)]
        margin: f64,
    },
    /// Repair and/or auto-fit a model
    Fix {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value = "k1")]
        profile: String,
        #[arg(long)]
        no_autofit: bool,
        #[arg(long)]
        no_repair: bool,
        #[arg(long)]
        scale_to_fit: bool,
        /// Prefer tall orientations (contact-first). Default is flat/low-Z.
        #[arg(long)]
        prefer_tall: bool,
        #[arg(long, default_value_t = 3.0)]
        margin: f64,
        #[arg(long, default_value_t = true)]
        drop_specks: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: Option<PathBuf>,
        #[arg(long, default_value_t = DEFAULT_TESS_TOL_MM)]
        tess_tol: f64,
    },
    /// Convert between mesh formats
    Convert {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = DEFAULT_TESS_TOL_MM)]
        tess_tol: f64,
    },
    /// Printer profile commands
    Profiles {
        #[command(subcommand)]
        command: ProfileCommands,
    },
}

#[derive(Debug, Subcommand)]
enum ProfileCommands {
    /// List embedded printer profiles
    List,
}

/// Entry point used by `main`.
pub fn run() -> ExitCode {
    match run_inner() {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("{} {err:#}", "error:".red().bold());
            ExitCode::from(3)
        }
    }
}

fn run_inner() -> Result<u8> {
    let cli = Cli::parse();
    let repo = FileMeshRepository::default();
    let progress = StderrProgress;

    match cli.command {
        Commands::Inspect {
            file,
            profile,
            json,
            tess_tol,
            margin,
        } => {
            let profile = load_profile(&profile).context("load profile")?;
            let report = InspectUseCase::new(&repo, &SilentProgress).execute(
                &file,
                &profile,
                &InspectOptions { margin, tess_tol },
            )?;
            print!("{}", report.render_human());
            if let Some(path) = json {
                std::fs::write(&path, report.to_json()?).context("write json")?;
            }
            Ok(report.exit_code as u8)
        }
        Commands::Fix {
            input,
            output,
            profile,
            no_autofit,
            no_repair,
            scale_to_fit,
            prefer_tall,
            margin,
            drop_specks,
            dry_run,
            json,
            tess_tol,
        } => {
            let profile = load_profile(&profile).context("load profile")?;
            let report = FixUseCase::new(&repo, &repo, &progress).execute(
                &input,
                &output,
                &profile,
                &FixOptions {
                    repair: !no_repair,
                    autofit: !no_autofit,
                    scale_to_fit,
                    prefer_flat: !prefer_tall,
                    margin,
                    drop_specks,
                    dry_run,
                    tess_tol,
                },
            )?;
            if !dry_run && report.exit_code != 2 {
                eprintln!("{} {}", "wrote".green().bold(), output.display());
            } else if dry_run {
                eprintln!("{}", "dry-run: no output written".dimmed());
            } else if report
                .autofit
                .as_ref()
                .is_some_and(|s| s.contains("split") || s.contains("scale-to-fit"))
            {
                eprintln!("{}", "does not fit".yellow().bold());
            } else if !dry_run {
                eprintln!("{} {}", "wrote".green().bold(), output.display());
            }
            print!("{}", report.render_human());
            if let Some(path) = json {
                std::fs::write(&path, report.to_json()?)?;
            }
            Ok(report.exit_code as u8)
        }
        Commands::Convert {
            input,
            output,
            tess_tol,
        } => {
            ConvertUseCase::new(&repo, &repo, &SilentProgress)
                .execute(&input, &output, tess_tol)?;
            eprintln!(
                "{} {} → {}",
                "converted".green().bold(),
                input.display(),
                output.display()
            );
            Ok(0)
        }
        Commands::Profiles {
            command: ProfileCommands::List,
        } => {
            for p in list_profiles() {
                println!(
                    "{} — {} ({:.0}×{:.0}×{:.0} mm)",
                    p.id, p.name, p.bed_x_mm, p.bed_y_mm, p.build_z_mm
                );
            }
            Ok(0)
        }
    }
}
