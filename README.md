<div align="center">

# 🧹 cclean — C 盘清理工具

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D6?logo=windows&logoColor=white)](#)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Release](https://img.shields.io/github/v/release/Cupcc/CCleaner?color=green)](https://github.com/Cupcc/CCleaner/releases)

轻量的 Windows C 盘清理桌面应用（Tauri v2，浅色界面、无边框自定义标题栏，纯静态 HTML 前端，无 Node 依赖，无控制台窗口）。单个 exe 约 8 MB。

</div>

原则：**启动即扫描；删除前弹窗确认；占用中的文件自动跳过；不跟随符号链接/junction；只清能安全清理的项。**

## 下载

前往 [Releases](https://github.com/Cupcc/CCleaner/releases) 下载 `cclean.exe`,双击即用（依赖 WebView2,Win11 自带）。建议右键「以管理员身份运行」以清理系统目录。

## 构建 / 运行

```
cargo build --release        # 产物 target\release\cclean.exe（推荐，无控制台）
cargo test                   # 核心逻辑单元测试（在 src-tauri 下）
```

依赖 WebView2（Win11 自带）。建议右键“以管理员身份运行”，否则部分系统目录无法清理。

## 结构

```
ui/index.html              前端（单文件，浅色 UI + 自定义标题栏，内联 CSS/JS）
src-tauri/
  src/main.rs              Tauri 命令：list_targets / scan_target / clean_target / is_admin
  src/targets.rs           清理项清单（数据驱动：路径模板 + %ENV% + * 通配）
  src/fsutil.rs            扫描与安全删除（护栏、不跟随链接、只读重试）
  src/win.rs               回收站 Shell API + 管理员检测
  capabilities/default.json 窗口最小化/关闭/拖动权限
  tauri.conf.json          无边框窗口（decorations:false）
```

## 界面

- 顶部：**C 盘容量条**（已用百分比 + 可用/总量，接近满时变橙/红），清理后自动刷新。
- 中部：可清理空间合计 + 扫描/清理按钮。
- 列表：常规清理项（图标+说明+大小+勾选），底部单独一组「系统深度清理（需管理员）」。
- **可展开**：点击每项左侧箭头展开，列出内部具体路径及各自大小，可逐条勾选/取消；
  父项支持全选/半选态。清理时只删被勾选的路径。
- **安全等级徽标**：每项标注「可安全清理 / 可重新下载 / 谨慎」，帮助判断是否删除。

## 清理项

- 常规 · 默认选中：回收站、临时文件、系统临时文件、浏览器缓存(Edge/Chrome/Firefox)、
  **应用缓存**(各类桌面/Electron 应用的 Cache/GPUCache/Code Cache、QQ 临时、Store 应用 INetCache)、
  缩略图缓存、字体缓存、着色器缓存(DirectX/NVIDIA)、崩溃转储、错误报告(WER)、传递优化缓存。
- 常规 · 默认不选：开发包缓存(npm/pip/NuGet/Yarn/cargo)、
  Windows 更新缓存（更新进行中清理可能出错）、Prefetch 预读、CBS/DISM 日志。
- 系统深度清理（需管理员，一键执行系统命令，释放量按磁盘可用空间增量计）：
  - **虚拟内存** — pagefile.sys/swapfile.sys 系统占用中无法直接删，显示大小并一键打开虚拟内存设置去调小/移盘。
  - **休眠文件** — `powercfg /h off` 删除 hiberfil.sys（仅在开启休眠时显示，可 `/h on` 恢复）。
  - **旧系统备份** — takeown 取得权限后删除 Windows.old（仅在存在时显示，删后不可回滚）。
  - **组件存储清理** — DISM `/StartComponentCleanup` 清理 WinSxS 中被取代的旧组件（较慢）。

## 安全设计

- 删除前 `is_safe_to_clean` 护栏：拒绝含 `..` 的路径、过浅路径、以及关键保护目录本身
  （%USERPROFILE% / %WINDIR% / C:\Users 等），防止环境变量被改坏时误删整个目录树。
- 不跟随符号链接 / junction（只删链接本身，不穿越）。
- 占用中的文件自动跳过并计数，不影响其余清理。
- 系统深度清理项默认不选、单独分组、需管理员；旧系统/休眠仅在实际存在时才显示。

## 免责声明

清理操作不可逆（回收站类除外），请在勾选前确认列表内容。本工具与 Piriform CCleaner® 无任何关联。

## License

[MIT](LICENSE) © 2026 Cupcc
