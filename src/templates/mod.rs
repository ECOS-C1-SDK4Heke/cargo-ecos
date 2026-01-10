use anyhow::Result;
use console::style;
use include_dir::{Dir, include_dir};
use std::path::Path;

static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");

#[derive(Debug)]
pub struct TemplateManager;

impl TemplateManager {
    pub fn list_templates() -> Vec<String> {
        TEMPLATES_DIR
            .dirs()
            .filter_map(|dir| {
                dir.path()
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn template_exists(name: &str) -> bool {
        TEMPLATES_DIR.get_dir(name).is_some()
    }

    pub fn get_template<'a>(name: &'a str) -> Result<&'a Dir<'a>> {
        let static_name: &'static str = Box::leak(name.to_string().into_boxed_str());
        TEMPLATES_DIR.get_dir(static_name).ok_or_else(|| {
            anyhow::anyhow!(
                "Template '{}' not found.\nAvailable templates: {}",
                name,
                Self::list_templates().join(", ")
            )
        })
    }

    pub fn create_project(
        template_name: &str,
        project_dir: &Path,
        project_name: &str,
    ) -> Result<()> {
        let template = Self::get_template(template_name)?;

        println!("{} Creating project structure...", style("📁").cyan());

        Self::create_directory_structure(template, project_dir, "")?;

        Self::process_template_files(template, project_dir, "", project_name)?;

        Ok(())
    }

    fn create_directory_structure<'a>(
        template: &'a Dir<'a>,
        base_dir: &Path,
        relative_path: &str,
    ) -> Result<()> {
        for subdir in template.dirs() {
            let dir_name = subdir.path().file_name().unwrap().to_string_lossy();
            let new_relative = if relative_path.is_empty() {
                dir_name.to_string()
            } else {
                format!("{}/{}", relative_path, dir_name)
            };

            let target_dir = base_dir.join(&new_relative);
            std::fs::create_dir_all(&target_dir)?;

            Self::create_directory_structure(subdir, base_dir, &new_relative)?;
        }

        Ok(())
    }

    fn process_template_files<'a>(
        template: &'a Dir<'a>,
        base_dir: &Path,
        relative_path: &str,
        project_name: &str,
    ) -> Result<()> {
        for file in template.files() {
            let file_name = file.path().file_name().unwrap().to_string_lossy();
            let target_path = if relative_path.is_empty() {
                base_dir.join(&*file_name)
            } else {
                base_dir.join(relative_path).join(&*file_name)
            };

            let content = std::str::from_utf8(file.contents())
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in template file: {}", e))?;

            let processed_content = Self::process_template_content(content, project_name);
            std::fs::write(&target_path, processed_content)?;

            println!("  📄 Created: {}", style(target_path.display()).dim());
        }

        for subdir in template.dirs() {
            let dir_name = subdir.path().file_name().unwrap().to_string_lossy();
            let new_relative = if relative_path.is_empty() {
                dir_name.to_string()
            } else {
                format!("{}/{}", relative_path, dir_name)
            };

            Self::process_template_files(subdir, base_dir, &new_relative, project_name)?;
        }

        Ok(())
    }

    fn process_template_content(content: &str, project_name: &str) -> String {
        content.replace("{{project_name}}", project_name)
    }

    pub fn install_templates_to_system() -> Result<()> {
        println!(
            "{} Templates are embedded in the binary.",
            style("ℹ️").cyan()
        );
        println!("  No need to install them separately.");
        Ok(())
    }

    pub fn uninstall_templates_from_system() -> Result<()> {
        if let Some(home_dir) = dirs::home_dir() {
            let old_template_dir = home_dir.join(".cargo-ecos").join("templates");
            if old_template_dir.exists() {
                println!("{} Removing old templates...", style("🗑️").cyan());
                let _ = std::fs::remove_dir_all(old_template_dir);
            }
        }
        Ok(())
    }
}
