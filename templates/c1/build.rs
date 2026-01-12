use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let sdk_home = env::var("ECOS_SDK_HOME").expect("ECOS_SDK_HOME not set");
    let sdk_path = PathBuf::from(&sdk_home);

    let (include_dirs, c_files) = scan_sdk_directories(&sdk_path);

    if !c_files.is_empty() {
        compile_c_files(&c_files, &include_dirs);
    }

    compile_startup_asm(&sdk_path);
    link_libraries(&sdk_path);

    println!("cargo:rerun-if-env-changed=ECOS_SDK_HOME");
}

fn scan_sdk_directories(sdk_path: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut include_dirs = vec![PathBuf::from("./include")];
    let mut c_files = Vec::new();

    let board_path = sdk_path.join("board/StarrySkyC1");
    if board_path.exists() {
        include_dirs.push(board_path.clone());
        scan_directory(&board_path, &mut include_dirs, &mut c_files);
    }

    for dir_name in &["components", "devices"] {
        let dir_path = sdk_path.join(dir_name);
        if dir_path.exists() {
            include_dirs.push(dir_path.clone());
            scan_directory(&dir_path, &mut include_dirs, &mut c_files);
        }
    }

    include_dirs.sort();
    include_dirs.dedup();

    (include_dirs, c_files)
}

fn scan_directory(dir: &Path, include_dirs: &mut Vec<PathBuf>, c_files: &mut Vec<PathBuf>) {
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current_dir) = stack.pop() {
        let entries = fs::read_dir(&current_dir).expect("Failed to read directory");
        let mut has_h = false;

        for entry in entries {
            let path = entry.expect("Failed to get directory entry").path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_str().expect("Invalid extension");
                    if ext_str.eq_ignore_ascii_case("h") {
                        has_h = true;
                    } else if ext_str.eq_ignore_ascii_case("c") {
                        c_files.push(path);
                    }
                }
            } else if path.is_dir() {
                stack.push(path);
            }
        }

        if has_h {
            include_dirs.push(current_dir);
        }
    }
}

fn compile_c_files(c_files: &[PathBuf], include_dirs: &[PathBuf]) {
    let mut build = cc::Build::new();

    for file in c_files {
        build.file(file);
    }

    build
        .flag("-mabi=ilp32")
        .flag("-march=rv32imac")
        .flag("-ffreestanding")
        .flag("-nostdlib")
        .opt_level(2)
        .warnings(false);

    for dir in include_dirs {
        if dir.exists() {
            build.include(dir);
        }
    }

    build.compile("ecos_user_sdk");
    println!("cargo:rustc-link-lib=static=ecos_user_sdk");
}

fn compile_startup_asm(sdk_path: &Path) {
    let start_s = sdk_path.join("board/StarrySkyC1/start.s");
    if !start_s.exists() {
        return;
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");

    let content = fs::read_to_string(&start_s).expect("Failed to read start.s");

    let modified_content = if content.contains(".global start") {
        content
    } else {
        format!(".global start\n\n{}", content)
    };

    let modified_start_s = PathBuf::from(&out_dir).join("start_global.s");
    fs::write(&modified_start_s, modified_content).expect("Failed to write modified start.s");

    let start_o = PathBuf::from(&out_dir).join("start.o");
    Command::new("riscv64-unknown-elf-as")
        .args(&[
            "-mabi=ilp32",
            "-march=rv32imac",
            "-o",
            start_o.to_str().expect("Invalid path"),
            modified_start_s.to_str().expect("Invalid path"),
        ])
        .status()
        .expect("Failed to assemble startup code");

    println!("cargo:rustc-link-arg={}", start_o.display());
}

fn link_libraries(sdk_path: &Path) {
    let sections_lds = sdk_path.join("board/StarrySkyC1/sections.lds");
    if sections_lds.exists() {
        let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
        let dest_lds = PathBuf::from(&out_dir).join("sections.lds");

        let content = fs::read_to_string(&sections_lds).expect("Failed to read linker script");

        fs::write(&dest_lds, content).expect("Failed to write modified linker script");

        println!("cargo:rustc-link-arg=-T{}", dest_lds.display());
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let start_o = PathBuf::from(&out_dir).join("start.o");

    if start_o.exists() {
        let ecos_user_sdk_a = PathBuf::from(&out_dir).join("libecos_user_sdk.a");
        if ecos_user_sdk_a.exists() {
            Command::new("riscv64-unknown-elf-ar")
                .args(&[
                    "q",
                    ecos_user_sdk_a.to_str().unwrap(),
                    start_o.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to add start.o to libecos_user_sdk.a");
        }

        let start_a = PathBuf::from(&out_dir).join("libstart.a");
        Command::new("riscv64-unknown-elf-ar")
            .args(&["rcs", start_a.to_str().unwrap(), start_o.to_str().unwrap()])
            .status()
            .expect("Failed to create libstart.a");

        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-lib=static=start");
    }

    println!("cargo:rustc-link-arg=-Wl,--gc-sections");
    println!("cargo:rustc-link-arg=-nostartfiles");
}
