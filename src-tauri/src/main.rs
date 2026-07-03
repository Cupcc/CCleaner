#![windows_subsystem = "windows"] // 不弹控制台黑框

mod fsutil;
mod targets;
mod win;

use serde::Serialize;
use std::path::PathBuf;
use targets::{Action, Target, TARGETS};

#[derive(Serialize)]
struct TargetInfo {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    default_on: bool,
    advanced: bool,
    /// true 表示不删除、点击后打开系统设置（虚拟内存）；不计入“可清理空间”合计
    opens_settings: bool,
    /// 是否可展开逐条勾选（Paths 类型）
    expandable: bool,
    /// 安全等级：safe / redownload / caution，用于前端提示徽标
    level: &'static str,
}

/// 展开后的单条路径及其大小（供前端展开勾选）
#[derive(Serialize)]
struct Entry {
    path: String,
    size: u64,
}

#[derive(Serialize)]
struct ScanResult {
    total: u64,
    entries: Vec<Entry>,
}

#[derive(Serialize)]
struct CleanResult {
    freed: u64,
    files: u64,
    errors: u64,
}

#[derive(Serialize)]
struct DiskInfo {
    total: u64,
    free: u64,
}

fn level_of(id: &str) -> &'static str {
    match id {
        "pkg-cache" => "redownload",
        "update-cache" | "pagefile" | "hibernate" | "windows-old" | "winsxs" => "caution",
        _ => "safe",
    }
}

#[tauri::command]
fn list_targets() -> Vec<TargetInfo> {
    TARGETS
        .iter()
        .filter(|t| t.is_relevant())
        .map(|t| TargetInfo {
            id: t.id,
            name: t.name,
            desc: t.desc,
            default_on: t.default_on,
            advanced: t.advanced,
            opens_settings: matches!(t.action, Action::OpenSettings { .. }),
            expandable: matches!(t.action, Action::Paths(_)),
            level: level_of(t.id),
        })
        .collect()
}

#[tauri::command]
fn is_admin() -> bool {
    win::is_admin()
}

#[tauri::command]
fn disk_info() -> DiskInfo {
    let (total, free) = win::disk_usage().unwrap_or((0, 0));
    DiskInfo { total, free }
}

#[tauri::command]
async fn scan_target(id: String) -> Result<ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || scan_blocking(&id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn clean_target(id: String, paths: Option<Vec<String>>) -> Result<CleanResult, String> {
    tauri::async_runtime::spawn_blocking(move || clean_blocking(&id, paths.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

fn find(id: &str) -> Result<&'static Target, String> {
    TARGETS
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("未知清理项: {id}"))
}

fn size_of_paths(paths: &[&str]) -> u64 {
    paths
        .iter()
        .flat_map(|p| targets::expand(p))
        .map(|p| fsutil::dir_size(&p))
        .sum()
}

fn scan_blocking(id: &str) -> Result<ScanResult, String> {
    let t = find(id)?;
    Ok(match &t.action {
        Action::Recycle => ScanResult {
            total: win::recycle_size().map_or(0, |(bytes, _)| bytes),
            entries: vec![],
        },
        Action::Paths(ps) => {
            // 展开成具体路径，逐条给出大小（供前端展开勾选），只保留有内容的
            let mut entries: Vec<Entry> = ps
                .iter()
                .flat_map(|p| targets::expand(p))
                .filter_map(|p| {
                    let size = fsutil::dir_size(&p);
                    (size > 0).then(|| Entry { path: p.display().to_string(), size })
                })
                .collect();
            entries.sort_by(|a, b| b.size.cmp(&a.size));
            ScanResult { total: entries.iter().map(|e| e.size).sum(), entries }
        }
        Action::System { size_paths, .. } | Action::OpenSettings { size_paths, .. } => {
            ScanResult { total: size_of_paths(size_paths), entries: vec![] }
        }
    })
}

fn clean_blocking(id: &str, paths: Option<&[String]>) -> Result<CleanResult, String> {
    let t = find(id)?;
    match &t.action {
        Action::Recycle => Ok(match win::recycle_size() {
            Some((bytes, items)) if items > 0 => {
                if win::empty_recycle() {
                    CleanResult { freed: bytes, files: items, errors: 0 }
                } else {
                    CleanResult { freed: 0, files: 0, errors: 1 }
                }
            }
            _ => CleanResult { freed: 0, files: 0, errors: 0 },
        }),
        Action::Paths(ps) => {
            // 全部展开路径；若前端传了子集，仅清理其中属于本项的路径（防越权）
            let all: Vec<PathBuf> = ps.iter().flat_map(|p| targets::expand(p)).collect();
            let selected: Vec<PathBuf> = match paths {
                Some(list) => all
                    .into_iter()
                    .filter(|p| list.iter().any(|s| *s == p.display().to_string()))
                    .collect(),
                None => all,
            };
            let mut st = fsutil::CleanStats::default();
            for p in selected {
                st.merge(fsutil::clean_path(&p, true));
            }
            Ok(CleanResult { freed: st.freed, files: st.files, errors: st.errors })
        }
        Action::System { size_paths, commands, .. } => {
            if !win::is_admin() {
                return Err("需要管理员权限，请以管理员身份运行".into());
            }
            // 释放量 = 清理前后 C 盘可用空间差（对系统命令统一、准确）
            let before = win::free_bytes();
            for cmd in *commands {
                win::run_hidden(cmd);
            }
            let after = win::free_bytes();
            // 成功判据：目标路径已消失（DISM 无 size_paths 时视为成功）
            let leftover = size_paths
                .iter()
                .flat_map(|p| targets::expand(p))
                .any(|p| p.exists());
            Ok(CleanResult {
                freed: after.saturating_sub(before),
                files: 0,
                errors: u64::from(leftover),
            })
        }
        Action::OpenSettings { exec, .. } => {
            // 打开系统设置由用户手动调整，不删除任何东西
            let ok = win::launch(exec);
            Ok(CleanResult { freed: 0, files: 0, errors: u64::from(!ok) })
        }
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_targets,
            is_admin,
            disk_info,
            scan_target,
            clean_target
        ])
        .run(tauri::generate_context!())
        .expect("tauri 启动失败");
}
