use crate::cmd::Command;
use anyhow::Result;
use clap::Args;
use console::style;
use humansize::{DECIMAL, format_size};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};

#[derive(Args)]
pub struct FlashCommand {
    /// Safe mode: only flash if .bin exists, don't auto build else
    #[arg(short, long)]
    safe: bool,

    /// Temporary flash path override
    #[arg(short, long, value_name = "PATH")]
    path: Option<String>,

    /// Use custom .bin file instead of default build output
    #[arg(short = 'f', long, value_name = "FILE")]
    file: Option<String>,

    /// Force rebuild before flashing (pass args to cargo ecos build)
    #[arg(short, long)]
    build: bool,

    /// Flash release build (implies --build -- --release)
    #[arg(short = 'r', long)]
    release: bool,

    /// Additional arguments to pass to cargo ecos build
    #[arg(last = true, allow_hyphen_values = true)]
    extra_build_args: Vec<String>,
}

impl Command for FlashCommand {
    fn execute(&self) -> Result<()> {
        println!("{} Flashing ECOS firmware...", style("⚡").cyan());

        // 找到项目根目录
        let project_root = crate::cmd::find_project_root()?;
        std::env::set_current_dir(&project_root)?;

        // 获取项目名称
        let project_name = extract_project_name(&project_root)?;

        // 确定要刷写的 .bin 文件路径
        let bin_path = if let Some(custom_file) = &self.file {
            // 使用自定义文件
            let path = PathBuf::from(custom_file);
            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "Custom .bin file not found: {}",
                    path.display()
                ));
            }
            println!("  Using custom file: {}", style(path.display()).dim());
            path
        } else {
            // 使用默认构建输出
            let default_bin = project_root
                .join("build")
                .join(format!("{}.bin", project_name));

            // 检查是否需要构建
            let should_build = match (self.build, self.release, default_bin.exists()) {
                // 明确要求构建（--build 或 --release）
                (true, _, _) | (_, true, _) => true,
                // 安全模式且文件存在
                (_, _, true) if self.safe => false,
                // 文件不存在且不是安全模式
                (_, _, false) if !self.safe => true,
                // 其他情况（文件存在且不是安全模式）
                _ => false,
            };

            if should_build {
                // 触发构建
                println!("  {} Building project...", style("🔨").cyan());
                self.trigger_build(&project_root)?;

                if !default_bin.exists() {
                    return Err(anyhow::anyhow!(
                        "Build output still not found after building: {}",
                        default_bin.display()
                    ));
                }
            } else if self.safe && !default_bin.exists() {
                // safe模式且文件不存在：报错
                return Err(anyhow::anyhow!(
                    "Build output not found: {}\nRun 'cargo ecos build' first or use --safe flag.",
                    default_bin.display()
                ));
            } else if default_bin.exists() {
                // 文件存在且不是safe模式，直接使用
                println!("  {} Using existing build output", style("✓").green());
            }

            default_bin
        };

        // 获取目标路径（从配置或参数）
        let target_path = self.get_target_path(&project_root)?;

        // 检查目标路径是否存在并可写
        self.check_target_path(&target_path)?;

        // 执行复制操作
        self.copy_bin_to_target(&bin_path, &target_path, &project_name)?;

        // 获取源文件的大小信息
        let src_metadata = fs::metadata(&bin_path)?;
        let src_size = src_metadata.len();
        let src_bits = src_size * 8;

        println!("✅ Firmware flashed successfully!");
        println!("  From: {}", style(bin_path.display()).dim());
        println!("  To:   {}", style(target_path.display()).dim());
        println!(
            "  Size: {} ({})",
            style(format_size(src_size, DECIMAL)).cyan(),
            style(format!("{} bits", src_bits)).dim()
        );

        Ok(())
    }
}

impl FlashCommand {
    /// 触发构建 - 调用 cargo ecos build
    fn trigger_build(&self, project_root: &Path) -> Result<()> {
        println!("  {} Building project...", style("🛠️").cyan());

        let mut build_cmd = StdCommand::new("cargo");
        build_cmd.args(["ecos", "build"]);

        if self.release {
            build_cmd.arg("--release");
        }

        for arg in &self.extra_build_args {
            build_cmd.arg(arg);
        }

        let status = build_cmd
            .current_dir(project_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Build failed"));
        }

        Ok(())
    }

    /// 获取目标路径
    fn get_target_path(&self, project_root: &Path) -> Result<PathBuf> {
        // 如果通过 --path 参数指定，使用它
        if let Some(path) = &self.path {
            let target = PathBuf::from(path);
            if !target.is_absolute() {
                return Err(anyhow::anyhow!(
                    "Flash path must be absolute: {}",
                    target.display()
                ));
            }
            return Ok(target);
        }

        // 否则从 Cargo.toml 读取配置
        let cargo_toml = project_root.join("Cargo.toml");
        let content = fs::read_to_string(&cargo_toml)?;

        // 解析 TOML 查找 flash 路径配置
        if let Some(flash_path) = Self::extract_flash_path_from_toml(&content) {
            if flash_path.is_empty()
                || flash_path.starts_with("default flash path")
                || flash_path.contains("not set")
                || flash_path.contains("TODO:")
            {
                return Err(anyhow::anyhow!(
                    "Flash path not configured.\n\
                     \nOptions:\n\
                     1. Run 'cargo ecos flash --path <path>' to specify target\n\
                     2. Reinitialize project with 'cargo ecos init --flash <path>'\n\
                     3. Manually edit Cargo.toml and add:\n\
                        [package.metadata.ecos]\n\
                        ecos_flash_cmd_to = \"your_path_here\""
                ));
            }
            Ok(PathBuf::from(flash_path))
        } else {
            Err(anyhow::anyhow!(
                "Flash configuration not found in Cargo.toml.\n\
                 \nOptions:\n\
                 1. Run 'cargo ecos flash --path <path>' to specify target\n\
                 2. Reinitialize project with 'cargo ecos init --flash <path>'\n\
                 3. Manually edit Cargo.toml and add:\n\
                    [package.metadata.ecos]\n\
                    ecos_flash_cmd_to = \"your_path_here\""
            ))
        }
    }

    /// 从 Cargo.toml 提取 flash 路径
    fn extract_flash_path_from_toml(content: &str) -> Option<String> {
        let toml_value: toml::Value = match toml::from_str(content) {
            Ok(value) => value,
            Err(_) => return None,
        };

        // 查找 [package.metadata.ecos].ecos_flash_cmd_to
        toml_value
            .get("package")?
            .get("metadata")?
            .get("ecos")?
            .get("ecos_flash_cmd_to")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// 检查目标路径
    fn check_target_path(&self, target_path: &Path) -> Result<()> {
        // 检查路径是否存在
        if !target_path.exists() {
            println!(
                "{} Flash target does not exist: {}",
                style("⚠️").yellow(),
                target_path.display()
            );

            // 如果是目录，尝试创建
            if target_path
                .to_string_lossy()
                .ends_with(std::path::MAIN_SEPARATOR)
                || target_path.to_string_lossy().ends_with('/')
                || target_path.to_string_lossy().ends_with('\\')
            {
                println!("  Creating directory: {}", target_path.display());
                fs::create_dir_all(target_path)?;
            } else {
                return Err(anyhow::anyhow!(
                    "Flash target path does not exist: {}",
                    target_path.display()
                ));
            }
        }

        // 检查是否可写
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(target_path)?;
            if metadata.permissions().mode() & 0o200 == 0 {
                println!(
                    "{} Flash target may not be writable: {}",
                    style("⚠️").yellow(),
                    target_path.display()
                );
            }
        }

        Ok(())
    }

    /// 复制 .bin 文件到目标位置
    fn copy_bin_to_target(
        &self,
        bin_path: &Path,
        target_path: &Path,
        project_name: &str,
    ) -> Result<()> {
        println!("  {} Copying firmware to target...", style("📋").cyan());

        let destination = if target_path.is_dir() {
            // 如果是目录，在目录内创建同名文件
            target_path.join(bin_path.file_name().unwrap_or_default())
        } else {
            // 如果是文件路径，直接使用
            target_path.to_path_buf()
        };

        // 确保目标目录存在
        if let Some(parent) = destination.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // 复制文件
        fs::copy(bin_path, &destination)?;

        println!(
            "  {} Copied {} to {}",
            style("✅").green(),
            style(project_name).bold(),
            style(destination.display()).dim()
        );

        // 如果是 USB 存储设备，尝试同步
        #[cfg(unix)]
        self.sync_filesystem_if_needed(&destination)?;

        Ok(())
    }

    #[cfg(unix)]
    fn sync_filesystem_if_needed(&self, destination: &Path) -> Result<()> {
        // 尝试判断是否是 removable 设备
        let _mount_point = destination
            .ancestors()
            .find(|path| path.exists() && *path != Path::new("/"))
            .unwrap_or(destination);

        // 运行 sync 命令确保数据写入
        let _ = StdCommand::new("sync").status();

        println!("  {} Filesystem synced", style("🔄").dim());

        Ok(())
    }

    #[cfg(not(unix))]
    fn sync_filesystem_if_needed(&self, _destination: &Path) -> Result<()> {
        Ok(())
    }
}

fn extract_project_name(project_root: &Path) -> Result<String> {
    let cargo_toml = project_root.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml)?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name =") {
            let parts: Vec<&str> = trimmed.split('=').collect();
            if parts.len() > 1 {
                let name = parts[1].trim().trim_matches('"').trim_matches('\'');
                return Ok(name.to_string());
            }
        }
    }

    Err(anyhow::anyhow!(
        "Could not extract project name from Cargo.toml"
    ))
}
