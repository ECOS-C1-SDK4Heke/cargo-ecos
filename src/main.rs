// src/main.rs
mod cmd;
mod templates;

use clap::{Parser, Subcommand};
#[allow(unused)]
use cmd::install::{InstallCommand, UninstallCommand};
use cmd::{
    Command, build::BuildCommand, clean::CleanCommand, config::ConfigCommand, flash::FlashCommand,
    init::InitCommand,
};

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    #[command(subcommand)]
    Ecos(EcosCommands),
}

#[derive(Subcommand)]
enum EcosCommands {
    /// Initialize a new ECOS project
    Init(InitCommand),

    /// Configure project using Kconfig menu
    Config(ConfigCommand),

    /// Build ECOS firmware
    Build(BuildCommand),

    /// Flash firmware to target device
    Flash(FlashCommand),

    /// Clean all build artifacts
    Clean(CleanCommand),

    /// Install templates to system (dev
    #[cfg_attr(not(feature = "install"), doc = "")]
    #[cfg_attr(not(feature = "install"), command(hide = true))]
    #[cfg(feature = "install")]
    Install(InstallCommand),

    /// Uninstall templates from system
    #[cfg_attr(not(feature = "install"), doc = "")]
    #[cfg_attr(not(feature = "install"), command(hide = true))]
    #[cfg(feature = "install")]
    Uninstall(UninstallCommand),
}

fn main() -> anyhow::Result<()> {
    let CargoCli::Ecos(cmd) = CargoCli::parse();

    match cmd {
        EcosCommands::Init(cmd) => cmd.execute(),
        EcosCommands::Config(cmd) => cmd.execute(),
        EcosCommands::Build(cmd) => cmd.execute(),
        EcosCommands::Clean(cmd) => cmd.execute(),
        EcosCommands::Flash(cmd) => cmd.execute(),
        #[cfg(feature = "install")]
        EcosCommands::Install(cmd) => cmd.execute(),
        #[cfg(feature = "install")]
        EcosCommands::Uninstall(cmd) => cmd.execute(),
    }
}
