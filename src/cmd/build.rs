use crate::cmd::Command;
use anyhow::Result;
use clap::Args;
use console::style;
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};

#[derive(Args)]
pub struct BuildCommand {
    /// Build in release mode
    #[arg(long, short)]
    release: bool,

    /// Skip memory report generation
    #[arg(long)]
    no_mem_report: bool,

    /// Additional arguments to pass to cargo build
    #[arg(last = true, num_args = 0.., allow_hyphen_values = true)]
    args: Vec<String>,
}

impl Command for BuildCommand {
    fn execute(&self) -> Result<()> {
        // 找到项目根目录
        let project_root = crate::cmd::find_project_root()?;
        std::env::set_current_dir(&project_root)?;

        println!("{} Building ECOS firmware...", style("🔨").cyan());

        // 检查 autoconf.h 是否存在
        let autoconf_h = project_root.join("include/generated/autoconf.h");
        if !autoconf_h.exists() {
            println!(
                "{} {}",
                style("❌").red(),
                style("include/generated/autoconf.h not found").bold()
            );
            return Err(anyhow::anyhow!(
                "Configuration not found. Run 'cargo ecos config' first."
            ));
        }

        // 检查环境
        check_environment()?;
        let sdk_home = crate::cmd::check_sdk_home()?;

        let mut cargo_cmd = StdCommand::new("cargo");
        cargo_cmd.arg("build");

        if self.release {
            cargo_cmd.arg("--release");
            println!("  Mode: {}", style("release").bold());
        } else {
            println!("  Mode: {}", style("debug").bold());
        }

        for arg in &self.args {
            cargo_cmd.arg(arg);
        }

        let status = cargo_cmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Cargo build failed"));
        }

        self.run_postbuild(&project_root)?;

        if !self.no_mem_report {
            self.generate_memory_report(&project_root, &sdk_home)?;
        }

        println!("✅ {} Build completed successfully!", style("ECOS").green());

        Ok(())
    }
}

impl BuildCommand {
    fn run_postbuild(&self, project_root: &Path) -> Result<()> {
        println!("{} Running post-build steps...", style("🛠️").cyan());

        let profile = if self.release { "release" } else { "debug" };

        // 读取项目名称
        let project_name = extract_project_name(project_root)?;

        // ELF 文件路径
        let elf = project_root.join(format!(
            "target/riscv32im-unknown-none-elf/{}/{}",
            profile, project_name
        ));
        if !elf.exists() {
            return Err(anyhow::anyhow!("ELF file not found: {}", elf.display()));
        }

        let out_dir = project_root.join("build");
        std::fs::create_dir_all(&out_dir)?;

        // 清理旧文件
        let _ = std::fs::remove_file(out_dir.join(format!("{}.bin", project_name)));
        let _ = std::fs::remove_file(out_dir.join(format!("{}.hex", project_name)));
        let _ = std::fs::remove_file(out_dir.join(format!("{}.txt", project_name)));

        // objcopy 生成 bin 文件
        println!("  📦 Generating binary file...");
        let status = StdCommand::new("riscv64-unknown-elf-objcopy")
            .args(&[
                "-O",
                "binary",
                elf.to_str().unwrap(),
                out_dir
                    .join(format!("{}.bin", project_name))
                    .to_str()
                    .unwrap(),
            ])
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to generate binary file"));
        }

        // objcopy 生成 hex 文件
        println!("  🔢 Generating hex file...");
        let status = StdCommand::new("riscv64-unknown-elf-objcopy")
            .args(&[
                "-O",
                "verilog",
                elf.to_str().unwrap(),
                out_dir
                    .join(format!("{}.hex", project_name))
                    .to_str()
                    .unwrap(),
            ])
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to generate hex file"));
        }

        // 修复 hex 文件地址
        let hex_path = out_dir.join(format!("{}.hex", project_name));
        let hex_content = std::fs::read_to_string(&hex_path)?;
        let fixed_hex = hex_content.replace("@30000000", "@00000000");
        std::fs::write(&hex_path, fixed_hex)?;

        // objdump 生成反汇编
        println!("  📝 Generating disassembly...");
        let output = StdCommand::new("riscv64-unknown-elf-objdump")
            .args(&["-d", elf.to_str().unwrap()])
            .output()?;

        std::fs::write(out_dir.join(format!("{}.txt", project_name)), output.stdout)?;

        println!("{} Post-build steps completed", style("✅").green());
        Ok(())
    }

    fn generate_memory_report(&self, project_root: &Path, sdk_home: &str) -> Result<()> {
        println!("{} Generating memory usage report...", style("📊").cyan());

        let profile = if self.release { "release" } else { "debug" };
        let project_name = extract_project_name(project_root)?;
        let elf_path = project_root.join(format!(
            "target/riscv32im-unknown-none-elf/{}/{}",
            profile, project_name
        ));

        if !elf_path.exists() {
            println!(
                "{} ELF file not found, skipping memory report",
                style("⚠️").yellow()
            );
            return Ok(());
        }

        // 检查 mem_report.mk 是否存在
        let sdk_path = Path::new(sdk_home);
        let mem_report_mk = sdk_path.join("tools/scripts/mem_report.mk");

        if mem_report_mk.exists() {
            // 创建一个临时的 Makefile 来调用 mem_report
            let temp_makefile = project_root.join(".temp_makefile.mk");
            let makefile_content = format!(
                "CROSS=riscv64-unknown-elf-\n\
                include {}\n\n\
                .PHONY: report\n\
                report:\n\t$(call show_mem_usage,{})\n",
                mem_report_mk.display(),
                elf_path.display()
            );

            std::fs::write(&temp_makefile, makefile_content)?;

            let status = StdCommand::new("make")
                .current_dir(project_root)
                .arg("-f")
                .arg(&temp_makefile)
                .arg("report")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()?;

            // 清理临时文件
            let _ = std::fs::remove_file(&temp_makefile);

            if !status.success() {
                println!("{} Memory report generation failed", style("⚠️").yellow());
            }
        } else {
            println!("{} mem_report.mk not found in SDK", style("⚠️").yellow());
            println!("  Expected at: {}", mem_report_mk.display());
        }

        Ok(())
    }
}

fn extract_project_name(project_root: &Path) -> Result<String> {
    let cargo_toml = project_root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml)?;

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

fn check_environment() -> Result<()> {
    // 检查 RISC-V 工具链
    for tool in &[
        "riscv64-unknown-elf-gcc",
        "riscv64-unknown-elf-objcopy",
        "riscv64-unknown-elf-objdump",
    ] {
        let status = StdCommand::new("which")
            .arg(tool)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Tool '{}' not found in PATH.\n\
                 Please install RISC-V toolchain.",
                tool
            ));
        }
    }

    Ok(())
}
