<div align="center">

# 🧹 cclean — C 盘清理工具

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D6?logo=windows&logoColor=white)](#)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Release](https://img.shields.io/github/v/release/Cupcc/CCleaner?color=green)](https://github.com/Cupcc/CCleaner/releases)

**单文件 8 MB · 秒开秒扫 · 完全离线 · 免费开源** —— 给 C 盘快满了的你。

</div>

## cclean简介

- 🪶 **体积极小** — 单个 exe 仅 **8 MB**,免安装、绿色便携,下载即用,删掉不留痕。市面清理软件动辄一两百 MB 安装包,还要常驻后台。
- ⚡ **快** — 实测冷启动 **0.2 秒**出窗口,打开即自动扫描,不用点任何按钮就能看到能腾出多少空间;自身进程内存仅 **~35 MB**（界面复用系统自带 WebView2,不打包 Chromium）。
- 🔌 **完全离线** — 全部代码**零网络请求**:不联网、无遥测、无自动更新、不上传任何数据。源码就在这里,可自行审计、自行编译。
- 🆓 **真免费** — MIT 开源。没有广告、没有弹窗、没有「深度清理请开通会员」。
- 🛡️ **不乱删** — 删除前弹窗确认;护栏校验拒绝可疑路径,%USERPROFILE% / %WINDIR% 等关键目录碰不到;占用中的文件自动跳过;不跟随符号链接。
- 🎯 **AI提示协助选择删除** — 近 20 类清理项,每项可**展开到具体路径逐条勾选**,并标注安全等级徽标（可安全清理 / 可重新下载 / 谨慎）,不做一键全删的莽夫。
- 🧹 **清得干净** — 回收站、临时文件、Edge/Chrome/Firefox 浏览器缓存、Electron/QQ/Store 应用缓存、缩略图/字体/着色器缓存、崩溃转储、开发包缓存(npm/pip/cargo…),直到休眠文件、Windows.old、WinSxS 组件存储等系统级大头。

> 实测数据来自 Win11 桌面机,不同机器略有差异。

## 下载

前往 [Releases](https://github.com/Cupcc/CCleaner/releases) 下载 `cclean.exe`,双击即用（依赖 WebView2,Win11 自带）。建议右键「以管理员身份运行」,否则部分系统目录无法清理。

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
