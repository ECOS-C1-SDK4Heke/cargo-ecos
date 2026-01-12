use crate::cmd::Command;
use crate::templates::TemplateManager;
use anyhow::Result;
use clap::Args;
use console::style;
use dialoguer::{Confirm, Input, Select};
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct InitCommand {
    /// Project directory path
    #[arg(value_name = "PATH")]
    project_path: Option<String>,

    /// Template name
    #[arg(long)]
    template: Option<String>,

    /// Force overwrite existing files
    #[arg(short, long)]
    force: bool,

    /// Where will be copy/flash to (e.g., /mnt/e or E:\\)
    #[arg(long)]
    flash: Option<String>,
}

impl Command for InitCommand {
    fn execute(&self) -> Result<()> {
        // 获取项目目录和名称
        let (target_dir, project_name) = self.get_project_info()?;

        // 基于 hk.cargo.toml 检测可用模板
        let available_templates = TemplateManager::list_templates();
        if available_templates.is_empty() {
            return Err(anyhow::anyhow!(
                "No templates available. Please reinstall cargo-ecos."
            ));
        }

        // 获取或选择模板名称
        let template_name = if let Some(template) = &self.template {
            if !available_templates.contains(template) {
                return Err(anyhow::anyhow!(
                    "Template '{}' not found.\nAvailable templates: {}",
                    template,
                    available_templates.join(", ")
                ));
            }
            template.clone()
        } else {
            let selection = Select::new()
                .with_prompt("Select target platform")
                .items(&available_templates)
                .default(0)
                .interact()?;
            available_templates[selection].clone()
        };

        // 检查目录状态
        self.check_directory_status(&target_dir)?;

        // 获取 flash 设备路径（在选择了模板之后）
        let flash_path = if let Some(path) = &self.flash {
            // 如果通过命令行指定了，就使用它
            path.clone()
        } else {
            // 交互式询问 flash 路径，允许为空
            let default_flash = if cfg!(windows) {
                "E:\\".to_string()
            } else {
                "/mnt/e".to_string()
            };

            let input = Input::<String>::new()
                .with_prompt(format!(
                    "Flash device path (press Enter to skip, e.g. {})",
                    default_flash
                ))
                .allow_empty(true)
                .validate_with(|input: &String| {
                    if input.is_empty() {
                        // 允许为空，表示不配置默认路径
                        Ok(())
                    } else {
                        // 检查路径是否有效
                        let path = Path::new(input);
                        if path.is_absolute() {
                            Ok(())
                        } else {
                            Err("Please enter an absolute path or leave empty")
                        }
                    }
                })
                .interact()?;

            input
        };

        // 创建项目
        println!(
            "{} Creating project '{}' with template '{}'...",
            style("🚀").cyan(),
            style(&project_name).bold(),
            style(&template_name).cyan()
        );

        // 使用 TemplateManager 创建项目（内部处理 hk.cargo.toml -> Cargo.toml ）
        TemplateManager::create_project(&template_name, &target_dir, &project_name, &flash_path)?;

        // 创建必要的额外目录
        self.create_extra_directories(&target_dir)?;

        // 尝试初始化 Git 仓库
        let git_initialized = match self.init_empty_git_folder(&target_dir, &project_name) {
            Ok(_) => true,
            Err(e) => {
                println!("  {}: {}", style("Git skipped").yellow().bold(), e);
                false
            }
        };

        println!(
            "✅ {} project initialized successfully!",
            style("ECOS").green()
        );
        println!(
            "📁 Project created at: {}",
            style(target_dir.display()).cyan()
        );
        println!("🎯 Target platform: {}", style(&template_name).cyan());

        if !flash_path.is_empty() {
            println!("⚡ Flash path: {}", style(&flash_path).cyan());
            println!(
                "{} Use 'cargo ecos flash' to copy firmware to this path",
                style("💡").dim()
            );
        } else {
            println!("{} Flash path not configured", style("⚠️").yellow());
            println!(
                "  {} Use 'cargo ecos flash --path <path>' to specify target when flashing",
                style("💡").dim()
            );
        }

        if git_initialized {
            println!(
                "\n📦 {} Git repository initialized.",
                style("Next steps:").bold().cyan()
            );
            println!("  {}", style("To connect to a remote repository:").dim());
            println!(
                "  {}",
                style("> git remote add origin git@<your remote repository>.git").dim()
            );
            println!("  {}", style("To rename the default branch:").dim());
            println!("  {}", style("> git branch -M main").dim());
            println!("  {}", style("To push your changes:").dim());
            println!("  {}", style("> git push -u origin main").dim());
            println!("  {}", style("To make further changes:").dim());
            println!("  {}", style("> git add .").dim());
            println!(
                "  {}",
                style("> git commit -a -m \"<type>: description\"").dim()
            );
            println!("  {}", style("> git push").dim());
        }

        Ok(())
    }
}

impl InitCommand {
    /// 获取项目目录和名称
    fn get_project_info(&self) -> Result<(PathBuf, String)> {
        match &self.project_path {
            // 在当前目录初始化
            Some(path) if path == "." => {
                let current_dir = std::env::current_dir()?;
                let project_name = current_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "ecos-project".to_string());
                Ok((current_dir, project_name))
            }

            // 指定路径初始化
            Some(path) => {
                let mut path = PathBuf::from(path);

                // 规范化 ./ 开头的路径
                if path.starts_with("./") {
                    path = path.strip_prefix("./")?.to_path_buf();
                }

                let has_parent = path.parent().map(|p| p != Path::new("")).unwrap_or(false);

                // 检查父目录是否存在
                if has_parent {
                    if let Some(parent) = path.parent() {
                        if !parent.exists() {
                            if self.force {
                                std::fs::create_dir_all(parent)?;
                            } else {
                                return Err(anyhow::anyhow!(
                                    "Parent directory '{}' does not exist.\nUse -f flag to create it automatically.",
                                    parent.display()
                                ));
                            }
                        }
                    }
                }

                // 转换为绝对路径
                let target_dir = if path.is_absolute() {
                    path
                } else {
                    std::env::current_dir()?.join(path)
                };

                // 从目录名获取项目名称
                let project_name = target_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "ecos-project".to_string());

                Ok((target_dir, project_name))
            }

            // 交互式输入
            None => {
                let path: String = Input::new()
                    .with_prompt("Project directory path")
                    .default("my-ecos-project".to_string())
                    .interact()?;

                let path = PathBuf::from(path);
                let target_dir = if path.is_absolute() {
                    path
                } else {
                    std::env::current_dir()?.join(path)
                };

                let project_name = target_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "ecos-project".to_string());

                Ok((target_dir, project_name))
            }
        }
    }

    /// 检查目录状态
    fn check_directory_status(&self, target_dir: &Path) -> Result<()> {
        // 目录不存在则创建
        if !target_dir.exists() {
            std::fs::create_dir_all(target_dir)?;
            return Ok(());
        }

        if self.is_directory_non_empty(target_dir) {
            if self.force {
                // 强制模式直接覆盖
            } else {
                let proceed = Confirm::new()
                    .with_prompt("Directory is not empty. Overwrite existing files?")
                    .default(false)
                    .interact()?;

                if !proceed {
                    return Err(anyhow::anyhow!("Operation cancelled by user"));
                }
            }
        }

        Ok(())
    }

    /// 检测目录是否非空
    fn is_directory_non_empty(&self, dir: &Path) -> bool {
        std::fs::read_dir(dir)
            .map(|mut entries| {
                entries.any(|entry| {
                    entry
                        .ok()
                        .and_then(|e| e.file_name().into_string().ok())
                        .map(|name| name != ".git")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }

    /// 创建额外的必要目录
    fn create_extra_directories(&self, target_dir: &Path) -> Result<()> {
        for dir in &["configs", "include", "build"] {
            let dir_path = target_dir.join(dir);
            if !dir_path.exists() {
                std::fs::create_dir_all(&dir_path)?;
                println!("  Created directory: {}", style(dir_path.display()).dim());
            }
        }
        Ok(())
    }

    /// 初始化空的 .git 项目
    fn init_empty_git_folder(&self, target_dir: &Path, project_name: &str) -> Result<()> {
        use anyhow::Context;

        // 检查git是否可用
        let git_check = std::process::Command::new("git").arg("--version").output();

        if git_check.is_err() {
            return Err(anyhow::anyhow!("Git is not installed or not found in PATH"));
        }

        // 检查是否已经存在.git目录
        let git_dir = target_dir.join(".git");
        if git_dir.exists() {
            return Err(anyhow::anyhow!(
                "Git repository already exists at {}",
                target_dir.display()
            ));
        }

        println!("  {}", style("Initializing Git repository...").dim());

        // 初始化git仓库
        let init_result = std::process::Command::new("git")
            .arg("init")
            .arg("--quiet")
            .current_dir(target_dir)
            .status()
            .with_context(|| format!("Failed to run git init in {}", target_dir.display()))?;

        if !init_result.success() {
            return Err(anyhow::anyhow!("Git initialization failed"));
        }

        println!("    {}", style("✓ Git repository initialized").green());

        // 添加所有文件
        let add_result = std::process::Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(target_dir)
            .status();

        if let Ok(status) = add_result {
            if status.success() {
                println!("    {}", style("✓ Added all files to staging").green());
            }
        }

        // 创建初始提交
        let commit_message = format!(
            "Initialized: Project [{}] at {}",
            project_name,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        let commit_result = std::process::Command::new("git")
            .arg("commit")
            .arg("-a")
            .arg("-m")
            .arg(&commit_message)
            .arg("--quiet")
            .current_dir(target_dir)
            .status();

        match commit_result {
            Ok(status) if status.success() => {
                println!(
                    "    {}",
                    style(format!("✓ Initial commit: {}", commit_message)).green()
                );
            }
            Ok(_) => {
                println!(
                    "    {}",
                    style("⚠ Initial commit failed (no changes or other issue)").yellow()
                );
            }
            Err(_) => {
                println!(
                    "    {}",
                    style("⚠ Could not create initial commit").yellow()
                );
            }
        }

        Ok(())
    }
}
