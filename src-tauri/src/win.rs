//! Windows Shell API：回收站 + 管理员检测 + 磁盘容量 + 隐藏窗口执行命令

use std::os::windows::process::CommandExt;
use std::ptr::null_mut;
use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows_sys::Win32::UI::Shell::{
    IsUserAnAdmin, SHEmptyRecycleBinW, SHQueryRecycleBinW, SHERB_NOCONFIRMATION,
    SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHQUERYRBINFO,
};

const DRIVE: &str = "C:\\";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

pub fn is_admin() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

/// C 盘回收站 (字节数, 项目数)；查询失败返回 None
pub fn recycle_size() -> Option<(u64, u64)> {
    let root = wide(DRIVE);
    let mut info = SHQUERYRBINFO {
        cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
        i64Size: 0,
        i64NumItems: 0,
    };
    let hr = unsafe { SHQueryRecycleBinW(root.as_ptr(), &mut info) };
    (hr == 0).then_some((info.i64Size as u64, info.i64NumItems as u64))
}

/// 清空 C 盘回收站（无确认框/进度条/声音）
pub fn empty_recycle() -> bool {
    let root = wide(DRIVE);
    unsafe {
        SHEmptyRecycleBinW(
            null_mut(),
            root.as_ptr(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        ) == 0
    }
}

/// C 盘 (总容量, 可用) 字节；查询失败返回 None
pub fn disk_usage() -> Option<(u64, u64)> {
    let root = wide(DRIVE);
    let mut avail = 0u64;
    let mut total = 0u64;
    let ok = unsafe { GetDiskFreeSpaceExW(root.as_ptr(), &mut avail, &mut total, null_mut()) };
    (ok != 0).then_some((total, avail))
}

/// 当前 C 盘可用字节（失败返回 0）
pub fn free_bytes() -> u64 {
    disk_usage().map_or(0, |(_, free)| free)
}

/// 隐藏窗口执行一条命令并等待其结束（argv[0] 为程序名）；返回是否成功退出
pub fn run_hidden(argv: &[&str]) -> bool {
    let Some((cmd, args)) = argv.split_first() else {
        return false;
    };
    std::process::Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// 启动一个程序但不等待（用于打开系统设置对话框等长驻窗口）；返回是否成功启动
pub fn launch(argv: &[&str]) -> bool {
    let Some((cmd, args)) = argv.split_first() else {
        return false;
    };
    std::process::Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .is_ok()
}
