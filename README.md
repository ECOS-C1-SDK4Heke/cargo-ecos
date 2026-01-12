# Cargo-ecos

仿Vite的项目模板创建+构建插件

```
# git clone 本仓库 && cd xxx && cargo install --path .
cargo install cargo-ecos

cargo ecos init [project_name] [-f] [--template <name>] [--flash <path>]
cargo ecos config [--default [name]]   # name 默认为 "c1"
cargo ecos build [-r <release>] [--no-mem-report] [-- args...]
cargo ecos flash [-s] [-p <path>] [-f <file>] [-b [-- args...]] [-r [-- args...]]
cargo ecos clean [-a]

cargo uninstall cargo-ecos
```

todo-list：见template目录
