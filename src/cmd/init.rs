use crate::cmd::Command;
use crate::templates::TemplateManager;
use anyhow::Result;
use clap::Args;
use console::style;
use dialoguer::{Confirm, Input, Select};
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct InitCommand {
    /// Project directory path (use "." for current directory)
    /// Example: cargo ecos init my-project
    ///          cargo ecos init ./my-project
    ///          cargo ecos init path/to/project
    project_path: Option<String>,

    /// Template name (c1, c2, l3)
    #[arg(long)]
    template: Option<String>,

    /// Force overwrite existing files
    #[arg(short, long)]
    force: bool,
}

impl Command for InitCommand {
    fn execute(&self) -> Result<()> {
        // 获取项目目录和名称
        let (target_dir, project_name) = self.get_project_info()?;

        // 获取可用模板
        let available_templates = TemplateManager::list_templates();
        if available_templates.is_empty() {
            return Err(anyhow::anyhow!(
                "No templates available. Please reinstall cargo-ecos."
            ));
        }

        // 获取可用模板名称
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

        // 创建项目
        println!(
            "{} Creating project '{}' with template '{}'...",
            style("🚀").cyan(),
            style(&project_name).bold(),
            style(&template_name).cyan()
        );

        TemplateManager::create_project(&template_name, &target_dir, &project_name)?;

        // 创建必要的目录
        self.create_extra_directories(&target_dir)?;

        println!(
            "✅ {} project initialized successfully!",
            style("ECOS").green()
        );
        println!(
            "📁 Project created at: {}",
            style(target_dir.display()).cyan()
        );
        println!("🎯 Target platform: {}", style(&template_name).cyan());

        Ok(())
    }
}

impl InitCommand {
    /// 获取项目目录和名称
    fn get_project_info(&self) -> Result<(PathBuf, String)> {
        match &self.project_path {
            Some(path) if path == "." => {
                // 在当前目录初始化
                let current_dir = std::env::current_dir()?;

                // 获取项目名称：使用当前目录的名称
                let project_name = current_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "ecos-project".to_string());

                Ok((current_dir, project_name))
            }
            Some(path) => {
                let path = PathBuf::from(path);

                // 如果是相对路径且以 ./ 开头，移除它
                let path = if path.starts_with("./") {
                    path.strip_prefix("./").unwrap().to_path_buf()
                } else {
                    path
                };

                // 检查是否没有父目录或父目录是空：init xxx == init ./xxx
                let has_parent = path.parent().map(|p| p != Path::new("")).unwrap_or(false);

                if has_parent {
                    // 检查父目录是否存在
                    if let Some(parent) = path.parent() {
                        if !parent.exists() {
                            if self.force {
                                println!(
                                    "  Creating parent directory: {}",
                                    style(parent.display()).dim()
                                );
                                std::fs::create_dir_all(parent)?;
                            } else {
                                return Err(anyhow::anyhow!(
                                    "Parent directory '{}' does not exist.\n\
                                     Use -f flag to create it automatically.",
                                    parent.display()
                                ));
                            }
                        }
                    }
                }

                // 如果是相对路径，转换为绝对路径
                let target_dir = if path.is_absolute() {
                    path
                } else {
                    std::env::current_dir()?.join(path)
                };

                // 项目名称：使用最后一级目录名
                let project_name = target_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| {
                        // 如果没有文件名，使用默认名
                        "ecos-project".to_string()
                    });

                Ok((target_dir, project_name))
            }
            None => {
                // 交互式输入项目路径
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

    /// 显示相对路径（相对于当前工作目录）
    fn display_relative_path(&self, path: &Path) -> String {
        match std::env::current_dir() {
            Ok(current_dir) => match path.strip_prefix(&current_dir) {
                Ok(relative) => {
                    if relative == Path::new("") {
                        ".".to_string()
                    } else {
                        format!("./{}", relative.display())
                    }
                }
                Err(_) => path.display().to_string(),
            },
            Err(_) => path.display().to_string(),
        }
    }

    /// 检查目录状态
    fn check_directory_status(&self, target_dir: &Path) -> Result<()> {
        let relative_path = self.display_relative_path(target_dir);

        // 如果目录不存在，创建它
        if !target_dir.exists() {
            println!(
                "  Creating project directory: {}",
                style(&relative_path).dim()
            );
            std::fs::create_dir_all(target_dir)?;
            return Ok(());
        }

        // 目录存在，检查是否为空（忽略 .git 目录）
        if self.is_directory_non_empty(target_dir) {
            if self.force {
                println!("  Directory is not empty, force overwriting...");
            } else {
                println!("{} Directory is not empty:", style("⚠️").yellow());
                println!("  {}", style(&relative_path).dim());

                let proceed = Confirm::new()
                    .with_prompt("Creating project will overwrite existing files. Continue?")
                    .default(false)
                    .interact()?;

                if !proceed {
                    println!("{} Operation cancelled", style("❌").red());
                    return Err(anyhow::anyhow!("Operation cancelled by user"));
                }
            }
        } else {
            println!(
                "  Using existing directory: {}",
                style(&relative_path).dim()
            );
        }

        Ok(())
    }

    /// 检查目录是否非空
    fn is_directory_non_empty(&self, dir: &Path) -> bool {
        match std::fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let file_name = path.file_name().unwrap_or_default();

                        // 忽略 .git 目录
                        if file_name != ".git" {
                            return true;
                        }
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    /// 创建额外的目录
    fn create_extra_directories(&self, target_dir: &Path) -> Result<()> {
        let dirs = ["configs", "include", "build"];

        for dir in &dirs {
            let dir_path = target_dir.join(dir);
            if !dir_path.exists() {
                std::fs::create_dir_all(&dir_path)?;
                println!("  Created directory: {}", style(dir_path.display()).dim());
            }
        }

        Ok(())
    }
}
