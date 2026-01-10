use crate::cmd::Command;
use anyhow::Result;
use clap::Args;
use console::style;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};

#[derive(Args)]
pub struct ConfigCommand {
    /// Generate default configuration
    #[arg(long)]
    default: bool,

    /// Default configuration name (c1, c2, l3)
    #[arg(long, default_value = "c1")]
    name: String,
}

impl Command for ConfigCommand {
    fn execute(&self) -> Result<()> {
        // 找到项目根目录
        let project_root = crate::cmd::find_project_root()?;
        std::env::set_current_dir(&project_root)?;

        if self.default {
            self.generate_default_config(&project_root)?;
        } else {
            self.run_menuconfig(&project_root)?;
        }
        Ok(())
    }
}

impl ConfigCommand {
    fn run_menuconfig(&self, project_root: &Path) -> Result<()> {
        println!("{} Running menuconfig...", style("📋").cyan());

        // 检查 SDK
        let sdk_home = crate::cmd::check_sdk_home()?;
        let sdk_path = PathBuf::from(&sdk_home);

        // 确保目录存在
        std::fs::create_dir_all("configs")?;
        std::fs::create_dir_all("include/generated")?;
        std::fs::create_dir_all("include/config")?;

        // 如果没有 .config，创建默认
        let config_file = project_root.join("configs/.config");
        if !config_file.exists() {
            println!("  Creating default config...");
            self.create_default_config(project_root, &sdk_path)?;
        }

        // 检查/构建 Kconfig
        let kconfig_tools_dir = sdk_path.join("tools/kconfig/build");
        let mconf = kconfig_tools_dir.join("mconf");
        let conf = kconfig_tools_dir.join("conf");

        if !mconf.exists() || !conf.exists() {
            println!("  Building Kconfig tools...");
            self.build_kconfig_tools(&sdk_path)?;
        }

        // 运行 menuconfig
        let kconfig_file = sdk_path.join("tools/kconfig/Kconfig");
        println!("  Using Kconfig: {}", style(kconfig_file.display()).dim());

        let status = StdCommand::new(&mconf)
            .arg(&kconfig_file)
            .env("KCONFIG_CONFIG", &config_file)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("menuconfig failed"));
        }

        // 运行 syncconfig，直接输出到项目目录
        println!("{} Synchronizing configuration...", style("🔄").cyan());

        // 设置环境变量，让 Kconfig 输出到项目目录
        let status = StdCommand::new(&conf)
            .args(&["--syncconfig", kconfig_file.to_str().unwrap()])
            .env("KCONFIG_CONFIG", &config_file)
            .env("OUTPUT", project_root.join("include")) // 关键：指定输出目录
            .env("CONFIG_", "CONFIG_")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to sync config"));
        }

        // 清理不需要的中间文件
        self.cleanup_generated_files(project_root, &sdk_path)?;

        println!(
            "✅ Configuration saved to {}",
            style("configs/.config").cyan()
        );
        println!("✅ Generated headers in {}", style("include/").cyan());

        Ok(())
    }

    fn build_kconfig_tools(&self, sdk_path: &Path) -> Result<()> {
        let kconfig_dir = sdk_path.join("tools/kconfig");

        if !kconfig_dir.exists() {
            return Err(anyhow::anyhow!(
                "Kconfig directory not found: {}",
                kconfig_dir.display()
            ));
        }

        // 构建 kconfig（mconf 和 conf）
        let status = StdCommand::new("make")
            .current_dir(&kconfig_dir)
            .arg("mconf")
            .arg("conf")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to build Kconfig tools"));
        }

        // 构建 fixdep（如果需要）
        let fixdep_dir = sdk_path.join("tools/fixdep");
        if fixdep_dir.exists() {
            let _ = StdCommand::new("make")
                .current_dir(&fixdep_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }

        Ok(())
    }

    fn generate_default_config(&self, project_root: &Path) -> Result<()> {
        println!(
            "{} Generating default configuration '{}'...",
            style("⚙️").cyan(),
            style(&self.name).cyan()
        );

        let sdk_home = crate::cmd::check_sdk_home()?;
        let sdk_path = PathBuf::from(&sdk_home);

        // 确保目录存在
        std::fs::create_dir_all("configs")?;
        std::fs::create_dir_all("include/generated")?;
        std::fs::create_dir_all("include/config")?;

        self.create_default_config(project_root, &sdk_path)?;

        // 同步配置
        self.sync_config(project_root, &sdk_path)?;

        println!(
            "✅ Default configuration '{}' generated",
            style(&self.name).cyan()
        );

        Ok(())
    }

    fn create_default_config(&self, project_root: &Path, sdk_path: &Path) -> Result<()> {
        let config_file = project_root.join("configs/.config");

        // 从 SDK 复制默认配置
        let default_config = sdk_path.join(format!("configs/{}_defconfig", self.name));

        if default_config.exists() {
            std::fs::copy(&default_config, &config_file)?;
            println!(
                "  Copied default config from SDK: {}",
                default_config.display()
            );
        } else {
            // 创建最基本的配置
            let basic_config = format!(
                "# ECOS Configuration\n\
                 # Generated by cargo-ecos\n\
                 CONFIG_STARRYSKY_{}=y\n",
                self.name.to_uppercase()
            );
            std::fs::write(&config_file, basic_config)?;
            println!("  Created basic config");
        }

        Ok(())
    }

    fn sync_config(&self, project_root: &Path, sdk_path: &Path) -> Result<()> {
        // 检查 Kconfig 工具是否已构建
        let kconfig_tools_dir = sdk_path.join("tools/kconfig/build");
        let conf = kconfig_tools_dir.join("conf");

        if !conf.exists() {
            println!("  Building Kconfig tools...");
            self.build_kconfig_tools(sdk_path)?;
        }

        // 运行 syncconfig，直接输出到项目目录
        let kconfig_file = sdk_path.join("tools/kconfig/Kconfig");
        let config_file = project_root.join("configs/.config");

        let status = StdCommand::new(&conf)
            .args(&["--syncconfig", kconfig_file.to_str().unwrap()])
            .env("KCONFIG_CONFIG", &config_file)
            .env("OUTPUT", project_root.join("include")) // 关键：指定输出目录
            .env("CONFIG_", "CONFIG_")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to sync config"));
        }

        // 清理不需要的中间文件
        self.cleanup_generated_files(project_root, sdk_path)?;

        Ok(())
    }

    fn cleanup_generated_files(&self, project_root: &Path, sdk_path: &Path) -> Result<()> {
        // 检查 autoconf.h
        let autoconf_h = project_root.join("include/generated/autoconf.h");
        if !autoconf_h.exists() {
            // 如果 autoconf.h 不存在，检查是否有 auto.conf 并转换
            let auto_conf = project_root.join("include/config/auto.conf");
            if auto_conf.exists() {
                println!("  Converting auto.conf to autoconf.h...");
                self.convert_auto_conf_to_autoconf_h(&auto_conf, &autoconf_h)?;
            } else {
                println!("{} Warning: autoconf.h not generated", style("⚠️").yellow());
            }
        } else {
            println!(
                "  Generated: {}",
                style("include/generated/autoconf.h").dim()
            );
        }

        // 清理多余的的 configs/config 目录
        let project_config_dir = project_root.join("configs/config");
        if project_config_dir.exists() {
            println!("  Cleaning intermediate config files...");
            std::fs::remove_dir_all(&project_config_dir)?;
        }

        // 清理 SDK 生成的临时文件
        let sdk_dirs_to_clean = [
            sdk_path.join("include/generated"),
            sdk_path.join("include/config"),
            sdk_path.join("configs/config"),
            sdk_path.join("configs/generated"),
        ];

        for dir in &sdk_dirs_to_clean {
            if dir.exists() {
                let _ = std::fs::remove_dir_all(dir);
            }
        }

        // 清理 Kconfig 的临时文件
        let kconfig_temp_dirs = [
            sdk_path.join("tools/kconfig/build/.tmp"),
            sdk_path.join("tools/kconfig/build/.config.tmp"),
        ];

        for dir in &kconfig_temp_dirs {
            if dir.exists() {
                let _ = std::fs::remove_dir_all(dir);
            }
        }

        Ok(())
    }

    fn convert_auto_conf_to_autoconf_h(
        &self,
        auto_conf_path: &Path,
        autoconf_h_path: &Path,
    ) -> Result<()> {
        let content = match std::fs::read_to_string(auto_conf_path) {
            Ok(content) => content,
            Err(_) => return Ok(()),
        };

        let mut output = String::new();
        output.push_str("/* Automatically generated file; DO NOT EDIT. */\n");
        output.push_str("#ifndef __AUTOCONF_H__\n");
        output.push_str("#define __AUTOCONF_H__\n\n");

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("CONFIG_") {
                let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let name = parts[0].trim();
                    let value = parts[1].trim();

                    if value == "y" || value == "\"y\"" {
                        output.push_str(&format!("#define {} 1\n", name));
                    } else if value == "n" || value == "\"n\"" {
                        output.push_str(&format!("/* #undef {} */\n", name));
                    } else if value.starts_with('"') && value.ends_with('"') {
                        let str_value = &value[1..value.len() - 1];
                        output.push_str(&format!("#define {} \"{}\"\n", name, str_value));
                    } else {
                        output.push_str(&format!("#define {} {}\n", name, value));
                    }
                }
            }
        }

        output.push_str("\n#endif /* __AUTOCONF_H__ */\n");

        // 确保目录存在
        if let Some(parent) = autoconf_h_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(autoconf_h_path, output)?;
        Ok(())
    }
}
