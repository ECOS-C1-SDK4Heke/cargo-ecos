use crate::cmd::Command;
use anyhow::Result;
use clap::Args;
use console::style;
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};

#[derive(Args)]
pub struct CleanCommand {
    /// Clean all artifacts including configs and include directories
    #[arg(short = 'a', long)]
    all: bool,
}

impl Command for CleanCommand {
    fn execute(&self) -> Result<()> {
        let project_root = crate::cmd::find_project_root()?;
        std::env::set_current_dir(&project_root)?;

        if self.all {
            println!(
                "{} Cleaning ALL ECOS project artifacts...",
                style("🧹").cyan()
            );
        } else {
            println!(
                "{} Cleaning ECOS project build artifacts...",
                style("🧹").cyan()
            );
        }

        println!("  🗑️  Running cargo clean...");
        let status = StdCommand::new("cargo")
            .arg("clean")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            println!("{} Cargo clean failed", style("⚠️").yellow());
        }

        if Path::new("build").exists() {
            println!("  🗑️  Removing build directory...");
            let _ = std::fs::remove_dir_all("build");
        }

        if self.all {
            println!("  🗑️  Removing configs and include directories...");

            let configs_to_clean = [
                "configs/.config",
                "configs/.config.old",
                "configs/config",
                "configs/generated",
            ];

            for config in &configs_to_clean {
                if Path::new(config).exists() {
                    println!("    Removing {}...", config);
                    if Path::new(config).is_dir() {
                        let _ = std::fs::remove_dir_all(config);
                    } else {
                        let _ = std::fs::remove_file(config);
                    }
                }
            }

            if Path::new("include").exists() {
                println!("    Removing include directory...");
                let _ = std::fs::remove_dir_all("include");
            }
        }

        println!("✅ Clean completed!");
        Ok(())
    }
}
