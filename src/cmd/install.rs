use crate::cmd::Command;
use crate::templates::TemplateManager;
use anyhow::Result;
use clap::Args;
use console::style;

#[derive(Args)]
pub struct InstallCommand;

impl Command for InstallCommand {
    fn execute(&self) -> Result<()> {
        println!("{} Installing cargo-ecos templates...", style("📦").cyan());

        TemplateManager::install_templates_to_system()?;

        println!("✅ Templates installed successfully!");
        println!("Location: ~/.cargo-ecos/templates/");

        Ok(())
    }
}

use dialoguer::Confirm;

#[derive(Args)]
pub struct UninstallCommand {
    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    yes: bool,
}

impl Command for UninstallCommand {
    fn execute(&self) -> Result<()> {
        if !self.yes {
            let confirm = Confirm::new()
                .with_prompt("Are you sure you want to uninstall cargo-ecos templates?")
                .default(false)
                .interact()?;

            if !confirm {
                println!("{} Uninstall cancelled", style("❌").red());
                return Ok(());
            }
        }

        println!(
            "{} Uninstalling cargo-ecos templates...",
            style("🗑️").cyan()
        );

        TemplateManager::uninstall_templates_from_system()?;

        println!("✅ Templates uninstalled successfully!");

        Ok(())
    }
}
