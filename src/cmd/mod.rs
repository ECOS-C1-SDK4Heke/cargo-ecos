pub mod build;
pub mod clean;
pub mod config;
pub mod flash;
pub mod init;
pub mod install;

pub trait Command {
    fn execute(&self) -> anyhow::Result<()>;
}

// 工具函数：查找项目根目录
pub fn find_project_root() -> anyhow::Result<std::path::PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if is_ecos_project(&cargo_toml)? {
                return Ok(current);
            }
        }

        // 到达根目录
        if !current.pop() {
            break;
        }
    }

    Err(anyhow::anyhow!(
        "Not an ECOS project directory.\n\
         Please run this command in a directory with an ECOS project Cargo.toml\n\
         or use 'cargo ecos init <name>' to create a new project."
    ))
}

// 检查是否是 ECOS 项目
pub fn is_ecos_project(cargo_toml_path: &std::path::Path) -> anyhow::Result<bool> {
    let content = std::fs::read_to_string(cargo_toml_path)?;
    let cargo_toml: toml::Value = toml::from_str(&content)?;

    if let Some(package) = cargo_toml.get("package") {
        if let Some(metadata) = package.get("metadata") {
            if let Some(ecos) = metadata.get("ecos") {
                if let Some(root) = ecos.get("ecos_project_root") {
                    return Ok(root.as_bool() == Some(true));
                }
            }
        }
    }

    Ok(false)
}

// 检查环境变量
pub fn check_sdk_home() -> anyhow::Result<String> {
    match std::env::var("ECOS_SDK_HOME") {
        Ok(path) => {
            let sdk_path = std::path::Path::new(&path);
            if !sdk_path.exists() {
                return Err(anyhow::anyhow!(
                    "ECOS_SDK_HOME directory '{}' does not exist.",
                    path
                ));
            }
            Ok(path)
        }
        Err(_) => Err(anyhow::anyhow!(
            "ECOS_SDK_HOME environment variable not set.\n\
             Please set it to your ECOS SDK installation directory.\n\
             Example: export ECOS_SDK_HOME=/path/to/embedded-sdk"
        )),
    }
}
