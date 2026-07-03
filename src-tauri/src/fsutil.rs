use std::fs;
use std::path::{Component, Path};
use walkdir::WalkDir;

#[derive(Default)]
pub struct CleanStats {
    pub freed: u64,
    pub files: u64,
    pub errors: u64,
}

impl CleanStats {
    pub fn merge(&mut self, o: CleanStats) {
        self.freed += o.freed;
        self.files += o.files;
        self.errors += o.errors;
    }
}

/// 统计路径占用（不跟随符号链接/junction；不存在返回 0）
pub fn dir_size(path: &Path) -> u64 {
    match path.symlink_metadata() {
        Ok(m) if m.is_file() => m.len(),
        Ok(m) if m.is_dir() => WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum(),
        _ => 0,
    }
}

/// 删除前的安全校验：拒绝 `..`、过浅路径、以及关键保护目录本身。
/// 防止 %TEMP%/%USERPROFILE% 等环境变量被改成危险值（如指向配置根目录）时误删整个目录树。
fn is_safe_to_clean(path: &Path) -> bool {
    // 任何 .. 组件都拒绝：components() 会保留 ParentDir，会虚增计数从而绕过深度检查。
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    if path.components().count() < 4 {
        return false;
    }
    let norm = |p: &Path| {
        p.to_string_lossy()
            .trim_end_matches(|c| c == '\\' || c == '/')
            .to_ascii_lowercase()
    };
    let target = norm(path);
    // 关键保护目录：这些目录本身绝不允许被清空（只允许清其子目录）
    let mut protected: Vec<String> = [
        "USERPROFILE", "WINDIR", "SystemRoot", "LOCALAPPDATA", "APPDATA",
        "ProgramData", "ProgramFiles", "ProgramFiles(x86)", "PUBLIC",
    ]
    .iter()
    .filter_map(|k| std::env::var(k).ok())
    .map(|v| norm(Path::new(&v)))
    .collect();
    protected.extend(["c:\\users", "c:\\windows", "c:\\"].map(String::from));
    !protected.iter().any(|p| *p == target)
}

/// 删除路径：文件直接删；目录自底向上删内容（keep_root 保留目录本身）。
/// 不跟随符号链接/junction（只删链接本身）；删不掉的（占用中）跳过并计入 errors。
//
// ponytail: 基于路径字符串删除，存在 TOCTOU junction-swap 风险——本地攻击者若在提权清理
// C:\Windows\Temp 等世界可写目录时抢先把子目录换成 junction，可把删除重定向到目录外。
// 个人单用户机器上此威胁模型可忽略（需另有本地进程与你竞速）。要彻底封堵需改为基于句柄的
// 遍历（对每个目录以 FILE_FLAG_OPEN_REPARSE_POINT 打开、句柄相对删除，同 std::fs::remove_dir_all
// 在 CVE-2022-21658 后的做法）——若将来面向多用户/服务化运行再升级。
pub fn clean_path(path: &Path, keep_root: bool) -> CleanStats {
    let mut st = CleanStats::default();
    if !is_safe_to_clean(path) {
        st.errors += 1;
        return st;
    }
    let meta = match path.symlink_metadata() {
        Ok(m) => m,
        Err(_) => return st, // 不存在，直接跳过
    };
    if meta.is_file() {
        let len = meta.len();
        match remove_file_force(path) {
            Ok(_) => {
                st.freed += len;
                st.files += 1;
            }
            Err(_) => st.errors += 1,
        }
        return st;
    }
    if !meta.is_dir() {
        return st; // 目标本身是符号链接，不处理
    }

    let min_depth = usize::from(keep_root);
    for entry in WalkDir::new(path)
        .min_depth(min_depth)
        .contents_first(true)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let ft = entry.file_type();
        let p = entry.path();
        if ft.is_dir() {
            // 内容已先删完，空目录直接移除；没删干净的会失败，跳过即可
            fs::remove_dir(p).ok();
        } else if ft.is_symlink() {
            // junction / 符号链接：只删链接本身，不计大小
            if fs::remove_dir(p).or_else(|_| fs::remove_file(p)).is_ok() {
                st.files += 1;
            } else {
                st.errors += 1;
            }
        } else {
            let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
            match remove_file_force(p) {
                Ok(_) => {
                    st.freed += len;
                    st.files += 1;
                }
                Err(_) => st.errors += 1,
            }
        }
    }
    st
}

/// 删除文件；只读文件去掉只读属性后重试
fn remove_file_force(p: &Path) -> std::io::Result<()> {
    fs::remove_file(p).or_else(|e| {
        let meta = fs::symlink_metadata(p)?;
        let mut perm = meta.permissions();
        if perm.readonly() {
            perm.set_readonly(false);
            fs::set_permissions(p, perm)?;
            return fs::remove_file(p);
        }
        Err(e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("cclean_test_{}_{}", tag, std::process::id()))
    }

    #[test]
    fn clean_keeps_root_and_empties() {
        let root = temp_root("basic");
        let sub = root.join("a").join("b");
        fs::create_dir_all(&sub).unwrap();
        fs::write(root.join("f1.txt"), b"hello").unwrap();
        fs::write(sub.join("f2.txt"), b"world").unwrap();
        let ro = root.join("readonly.txt");
        fs::write(&ro, b"ro").unwrap();
        let mut perm = fs::metadata(&ro).unwrap().permissions();
        perm.set_readonly(true);
        fs::set_permissions(&ro, perm).unwrap();

        let st = clean_path(&root, true);
        assert_eq!(st.freed, 12);
        assert_eq!(st.files, 3);
        assert_eq!(st.errors, 0);
        assert!(root.exists());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        fs::remove_dir(&root).unwrap();
    }

    #[test]
    fn refuses_shallow_path() {
        // 不存在的浅路径：护栏必须在任何删除动作之前挡下
        let st = clean_path(Path::new("C:\\cclean_no_such_dir"), true);
        assert_eq!(st.freed, 0);
        assert!(st.errors > 0);
    }

    #[test]
    fn guard_rejects_dangerous_paths() {
        // .. 组件：即便原始计数够深也必须拒绝
        assert!(!is_safe_to_clean(Path::new(
            "C:\\Users\\A\\AppData\\Local\\Temp\\..\\..\\..\\.."
        )));
        // 关键保护目录本身
        assert!(!is_safe_to_clean(Path::new("C:\\Users")));
        assert!(!is_safe_to_clean(Path::new("C:\\Windows\\")));
        assert!(!is_safe_to_clean(Path::new("C:\\")));
        // 合法的缓存目标（足够深、无 ..）应放行
        assert!(is_safe_to_clean(Path::new("C:\\Windows\\Temp")));
        assert!(is_safe_to_clean(Path::new(
            "C:\\Users\\A\\AppData\\Local\\Temp"
        )));
    }

    #[test]
    fn missing_path_is_noop() {
        let st = clean_path(&temp_root("missing"), true);
        assert_eq!(st.freed, 0);
        assert_eq!(st.errors, 0);
    }
}
