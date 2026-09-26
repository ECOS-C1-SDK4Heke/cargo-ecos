<div align="center">

# Cargo-ECOS

仿 Vite 的 ECOS 嵌入式工程助手：创建、配置、构建、烧录，一条命令一个环节。

<p>
  <a href="https://crates.io/crates/cargo-ecos"><img alt="crates.io" src="https://img.shields.io/crates/v/cargo-ecos?style=flat-square&label=crates.io&labelColor=161B08&color=E33E26&logo=rust&logoColor=white"></a>
  <a href="https://crates.io/crates/cargo-ecos"><img alt="downloads" src="https://img.shields.io/crates/d/cargo-ecos?style=flat-square&labelColor=161B08&color=F74C00&logo=rust&logoColor=white"></a>
  <a href="https://docs.rs/cargo-ecos"><img alt="docs.rs" src="https://img.shields.io/badge/docs-docs.rs-CE422B?style=flat-square&labelColor=161B08&logo=docsdotrs&logoColor=white"></a>
  <a href="https://crates.io/crates/cargo-ecos"><img alt="license" src="https://img.shields.io/crates/l/cargo-ecos?style=flat-square&labelColor=161B08&color=B7410E"></a>
  <img alt="edition" src="https://img.shields.io/badge/rust-2024%20edition-161B08?style=flat-square&logo=rust&logoColor=white">
</p>

</div>

## 安装

```console
cargo install cargo-ecos
```

使用前先装好 ECOS SDK，并把 `ECOS_SDK_HOME` 指向其根目录。

## 快速开始

```console
cargo ecos init blinky        # 用模板创建工程
cd blinky
cargo ecos config             # Kconfig 菜单配置
cargo ecos build --release    # 构建固件
cargo ecos flash -r -- -s     # release 构建并烧录
```

## 命令一览

|        命令         |                                   作用                                    |
|:------------------:|:-----------------------------------------------------------------------:|
|  `cargo ecos init`   | 创建工程。`--template <名>` 选模板，`-f` 覆盖已有文件，`--flash <路径>` 预设烧录位置 |
| `cargo ecos config`  |  Kconfig 菜单配置。`--default --name <名>` 生成默认配置，模板名默认为 `c1`   |
|  `cargo ecos build`  |     构建固件。`-r` release 构建，`--no-mem-report` 跳过内存报告，`-s` 打印段信息     |
|  `cargo ecos flash`  | 烧录固件。`-s` 只烧录已有的 `.bin` 不自动构建，`-p <路径>` 临时改烧录位置，`-f <文件>` 用指定 `.bin`，`-b` 先构建再烧录，`-r` release 构建后烧录 |
|  `cargo ecos clean`  |                  清理构建产物。`-a` 连同配置与 include 目录一起清理                  |
| `cargo ecos version` |                                  显示版本                                  |

`--` 之后的参数原样传给底层的 `cargo build`。

## 模板

内置 `c1`（默认）、`c2`、`k1`、`l3` 四套，`cargo ecos init --template <名>` 选用；模板里的 `Cargo.toml` 写作 `hk.cargo.toml`，避免被 cargo 误认成项目源文件。

## 许可

[MIT](https://opensource.org/licenses/MIT) OR [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)
