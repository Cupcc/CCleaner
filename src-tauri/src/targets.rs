use std::path::PathBuf;

/// 每个清理项的执行方式——全部可一键执行，没有“只提示不做”的项
pub enum Action {
    /// 删除这些路径下的内容
    Paths(&'static [&'static str]),
    /// 清空 C 盘回收站（走 Shell API）
    Recycle,
    /// 系统级操作：估算大小用 size_paths，执行 commands（隐藏窗口、需管理员），
    /// 释放量由清理前后磁盘可用空间差得出。always_show=true 则始终显示。
    System {
        size_paths: &'static [&'static str],
        commands: &'static [&'static [&'static str]],
        always_show: bool,
    },
    /// 无法直接删除、需在系统设置里调整的项（如虚拟内存）：显示大小，点击打开设置程序。
    OpenSettings {
        size_paths: &'static [&'static str],
        exec: &'static [&'static str],
    },
}

pub struct Target {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub action: Action,
    /// 是否默认勾选
    pub default_on: bool,
    /// 系统深度清理项（UI 里单独分组、需管理员）
    pub advanced: bool,
}

impl Target {
    /// 当前机器上是否显示该项（如休眠文件仅在开启休眠时显示）
    pub fn is_relevant(&self) -> bool {
        match &self.action {
            Action::System { size_paths, always_show, .. } => {
                *always_show || size_paths.iter().flat_map(|p| expand(p)).any(|p| p.exists())
            }
            Action::OpenSettings { size_paths, .. } => {
                size_paths.iter().flat_map(|p| expand(p)).any(|p| p.exists())
            }
            _ => true,
        }
    }
}

pub static TARGETS: &[Target] = &[
    // ── 常规清理 ─────────────────────────────
    Target {
        id: "recycle",
        name: "回收站",
        desc: "清空 C 盘回收站",
        action: Action::Recycle,
        default_on: true,
        advanced: false,
    },
    Target {
        id: "user-temp",
        name: "临时文件",
        desc: "当前用户临时目录（%TEMP%）",
        action: Action::Paths(&["%TEMP%"]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "win-temp",
        name: "系统临时文件",
        desc: "C:\\Windows\\Temp",
        action: Action::Paths(&["%WINDIR%\\Temp"]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "browser-cache",
        name: "浏览器缓存",
        desc: "Edge / Chrome / Firefox 缓存（不动 Cookie、密码、历史）",
        action: Action::Paths(&[
            "%LOCALAPPDATA%\\Microsoft\\Edge\\User Data\\*\\Cache",
            "%LOCALAPPDATA%\\Microsoft\\Edge\\User Data\\*\\Code Cache",
            "%LOCALAPPDATA%\\Microsoft\\Edge\\User Data\\*\\GPUCache",
            "%LOCALAPPDATA%\\Google\\Chrome\\User Data\\*\\Cache",
            "%LOCALAPPDATA%\\Google\\Chrome\\User Data\\*\\Code Cache",
            "%LOCALAPPDATA%\\Google\\Chrome\\User Data\\*\\GPUCache",
            "%LOCALAPPDATA%\\Mozilla\\Firefox\\Profiles\\*\\cache2",
        ]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "app-cache",
        name: "应用缓存",
        desc: "各类应用的 Cache / GPUCache（微信/QQ/Electron 等桌面应用，可重建）",
        action: Action::Paths(&[
            "%APPDATA%\\*\\Cache",
            "%APPDATA%\\*\\Code Cache",
            "%APPDATA%\\*\\GPUCache",
            "%LOCALAPPDATA%\\*\\Cache",
            "%LOCALAPPDATA%\\*\\Code Cache",
            "%LOCALAPPDATA%\\*\\GPUCache",
            "%APPDATA%\\Tencent\\QQ\\*\\FileRecvTemp",
            "%LOCALAPPDATA%\\Packages\\*\\AC\\INetCache",
        ]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "thumb-cache",
        name: "缩略图缓存",
        desc: "资源管理器缩略图/图标缓存，自动重建",
        action: Action::Paths(&[
            "%LOCALAPPDATA%\\Microsoft\\Windows\\Explorer\\thumbcache_*.db",
            "%LOCALAPPDATA%\\Microsoft\\Windows\\Explorer\\iconcache_*.db",
        ]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "font-cache",
        name: "字体缓存",
        desc: "字体缓存，自动重建",
        action: Action::Paths(&[
            "%WINDIR%\\ServiceProfiles\\LocalService\\AppData\\Local\\FontCache",
        ]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "dx-cache",
        name: "着色器缓存",
        desc: "DirectX / NVIDIA 着色器缓存，自动重建",
        action: Action::Paths(&[
            "%LOCALAPPDATA%\\D3DSCache",
            "%LOCALAPPDATA%\\NVIDIA\\DXCache",
            "%LOCALAPPDATA%\\NVIDIA\\GLCache",
        ]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "crash-dumps",
        name: "崩溃转储",
        desc: "蓝屏/程序崩溃 dump（常有数 GB）",
        action: Action::Paths(&[
            "%LOCALAPPDATA%\\CrashDumps",
            "%WINDIR%\\Minidump",
            "%WINDIR%\\LiveKernelReports",
            "%WINDIR%\\MEMORY.DMP",
        ]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "wer",
        name: "错误报告",
        desc: "Windows 错误报告队列/存档（WER）",
        action: Action::Paths(&[
            "C:\\ProgramData\\Microsoft\\Windows\\WER\\ReportQueue",
            "C:\\ProgramData\\Microsoft\\Windows\\WER\\ReportArchive",
            "C:\\ProgramData\\Microsoft\\Windows\\WER\\Temp",
            "%LOCALAPPDATA%\\Microsoft\\Windows\\WER\\ReportQueue",
            "%LOCALAPPDATA%\\Microsoft\\Windows\\WER\\ReportArchive",
        ]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "do-cache",
        name: "传递优化缓存",
        desc: "Windows 更新 P2P 分发缓存",
        action: Action::Paths(&[
            "%WINDIR%\\ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache",
        ]),
        default_on: true,
        advanced: false,
    },
    Target {
        id: "pkg-cache",
        name: "开发包缓存",
        desc: "npm / pip / NuGet / Yarn / cargo 下载缓存（可重新下载）",
        action: Action::Paths(&[
            "%LOCALAPPDATA%\\npm-cache",
            "%APPDATA%\\npm-cache",
            "%LOCALAPPDATA%\\pip\\cache",
            "%LOCALAPPDATA%\\NuGet\\v3-cache",
            "%LOCALAPPDATA%\\Yarn\\Cache",
            "%LOCALAPPDATA%\\Yarn\\Berry\\cache",
            "%USERPROFILE%\\.cargo\\registry\\cache",
            "%USERPROFILE%\\.cargo\\registry\\src",
            "%USERPROFILE%\\.nuget\\packages",
        ]),
        default_on: false,
        advanced: false,
    },
    Target {
        id: "update-cache",
        name: "Windows 更新缓存",
        desc: "已下载的更新包（更新进行中清理可能出错，默认不选）",
        action: Action::Paths(&["%WINDIR%\\SoftwareDistribution\\Download"]),
        default_on: false,
        advanced: false,
    },
    Target {
        id: "prefetch",
        name: "预读缓存",
        desc: "Prefetch，清后首次启动应用略慢",
        action: Action::Paths(&["%WINDIR%\\Prefetch"]),
        default_on: false,
        advanced: false,
    },
    Target {
        id: "sys-logs",
        name: "系统组件日志",
        desc: "CBS / DISM / 更新日志，排障时有用",
        action: Action::Paths(&[
            "%WINDIR%\\Logs\\CBS",
            "%WINDIR%\\Logs\\DISM",
            "%WINDIR%\\Logs\\WindowsUpdate",
        ]),
        default_on: false,
        advanced: false,
    },
    // ── 系统深度清理（需管理员，一键执行系统命令）────────
    Target {
        id: "pagefile",
        name: "虚拟内存",
        desc: "pagefile.sys / swapfile.sys 由系统占用中，无法直接删除。点击打开系统设置，可调小或移到其他盘",
        action: Action::OpenSettings {
            size_paths: &["C:\\pagefile.sys", "C:\\swapfile.sys"],
            exec: &["SystemPropertiesPerformance.exe"],
        },
        default_on: false,
        advanced: true,
    },
    Target {
        id: "hibernate",
        name: "休眠文件",
        desc: "关闭休眠并删除 hiberfil.sys（可回收数 GB，同时关闭快速启动，可用 powercfg /h on 恢复）",
        action: Action::System {
            size_paths: &["C:\\hiberfil.sys"],
            commands: &[&["powercfg", "/h", "off"]],
            always_show: false,
        },
        default_on: false,
        advanced: true,
    },
    Target {
        id: "windows-old",
        name: "旧系统备份",
        desc: "删除 Windows.old（升级残留，删后无法回滚到旧系统）",
        action: Action::System {
            size_paths: &["C:\\Windows.old", "C:\\$Windows.~BT", "C:\\$Windows.~WS"],
            commands: &[
                &["takeown", "/f", "C:\\Windows.old", "/r", "/d", "y"],
                &["icacls", "C:\\Windows.old", "/grant", "*S-1-5-32-544:F", "/t", "/c"],
                &["cmd", "/c", "rd", "/s", "/q", "C:\\Windows.old"],
                &["cmd", "/c", "rd", "/s", "/q", "C:\\$Windows.~BT"],
                &["cmd", "/c", "rd", "/s", "/q", "C:\\$Windows.~WS"],
            ],
            always_show: false,
        },
        default_on: false,
        advanced: true,
    },
    Target {
        id: "winsxs",
        name: "组件存储清理",
        desc: "DISM 清理 WinSxS 中被取代的旧组件（较慢，可能数分钟）",
        action: Action::System {
            size_paths: &[],
            commands: &[&["Dism.exe", "/Online", "/Cleanup-Image", "/StartComponentCleanup"]],
            always_show: true,
        },
        default_on: false,
        advanced: true,
    },
];

/// 展开路径模板：%ENV% 环境变量 + 单个组件里的一个 * 通配。
/// 环境变量不存在或通配目录不可读时返回空（该路径跳过）。
pub fn expand(template: &str) -> Vec<PathBuf> {
    let mut s = String::new();
    let mut rest = template;
    while let Some(i) = rest.find('%') {
        s.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) => {
                match std::env::var(&after[..j]) {
                    Ok(v) => s.push_str(&v),
                    Err(_) => return vec![],
                }
                rest = &after[j + 1..];
            }
            None => {
                s.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    s.push_str(rest);

    if !s.contains('*') {
        return vec![PathBuf::from(s)];
    }
    // 展开含 * 的组件（只支持一个通配组件）
    let comps: Vec<String> = PathBuf::from(&s)
        .iter()
        .map(|c| c.to_string_lossy().into_owned())
        .collect();
    let star = comps.iter().position(|c| c.contains('*')).unwrap();
    let base: PathBuf = comps[..star].iter().collect();
    let Ok(rd) = std::fs::read_dir(&base) else {
        return vec![];
    };
    rd.flatten()
        .filter(|e| glob_match(&e.file_name().to_string_lossy(), &comps[star]))
        .map(|e| {
            let mut p = base.join(e.file_name());
            for c in &comps[star + 1..] {
                p.push(c);
            }
            p
        })
        .collect()
}

/// 只支持一个 * 的文件名匹配，不区分大小写（Windows 文件系统语义）
fn glob_match(name: &str, pat: &str) -> bool {
    match pat.split_once('*') {
        None => name.eq_ignore_ascii_case(pat),
        Some((pre, suf)) => {
            let n = name.to_ascii_lowercase();
            n.len() >= pre.len() + suf.len()
                && n.starts_with(&pre.to_ascii_lowercase())
                && n.ends_with(&suf.to_ascii_lowercase())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob() {
        assert!(glob_match("thumbcache_32.db", "thumbcache_*.db"));
        assert!(glob_match("ThumbCache_1024.DB", "thumbcache_*.db"));
        assert!(!glob_match("iconcache.db", "thumbcache_*.db"));
        assert!(glob_match("Default", "*"));
        assert!(glob_match("Cache", "Cache"));
    }

    #[test]
    fn expand_env() {
        let p = expand("%WINDIR%\\Temp");
        assert_eq!(p.len(), 1);
        assert!(p[0].ends_with("Temp"));
        assert!(p[0].exists());
    }

    #[test]
    fn expand_missing_env_skips() {
        assert!(expand("%NO_SUCH_VAR_CCLEAN%\\x").is_empty());
    }

    #[test]
    fn winsxs_always_shows_hibernate_conditional() {
        let winsxs = TARGETS.iter().find(|t| t.id == "winsxs").unwrap();
        assert!(winsxs.is_relevant());
        // 常规项始终显示
        assert!(TARGETS.iter().find(|t| t.id == "user-temp").unwrap().is_relevant());
    }
}
