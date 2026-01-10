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

        // 创建项目
        println!(
            "{} Creating project '{}' with template '{}'...",
            style("🚀").cyan(),
            style(&project_name).bold(),
            style(&template_name).cyan()
        );

        // 使用 TemplateManager 创建项目（内部处理 hk.cargo.toml -> Cargo.toml ）
        TemplateManager::create_project(&template_name, &target_dir, &project_name)?;

        // 创建必要的额外目录
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
}
