use std::fs;
use std::ptr;
use std::slice;
use std::thread;
use std::time::Duration;

pub const VK_BACK: u8 = 0x08;
pub const VK_RETURN: u8 = 0x0D;
pub const VK_SPACE: u8 = 0x20;
pub const KEYEVENTF_KEYUP: u32 = 0x0002;
pub const KEYEVENTF_UNICODE: u32 = 0x0004;

const DESKTOP_SWITCHDESKTOP: u32 = 0x0100;
const TOKEN_DUPLICATE: u32 = 0x0002;
const TOKEN_ASSIGN_PRIMARY: u32 = 0x0001;
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_ADJUST_PRIVILEGES: u32 = 0x0020;
const SE_PRIVILEGE_ENABLED: u32 = 0x00000002;
const MAXIMUM_ALLOWED: u32 = 0x02000000;
const SECURITY_IDENTIFICATION: i32 = 2;
const TOKEN_PRIMARY: i32 = 1;
const PROCESS_QUERY_INFORMATION: u32 = 0x0400;

#[repr(C)]
#[allow(non_snake_case)]
struct LUID {
    LowPart: u32,
    HighPart: i32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct LUID_AND_ATTRIBUTES {
    Luid: LUID,
    Attributes: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct TOKEN_PRIVILEGES {
    PrivilegeCount: u32,
    Privileges: [LUID_AND_ATTRIBUTES; 1],
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct WTS_PROCESS_INFOW {
    pub SessionId: u32,
    pub ProcessId: u32,
    pub pProcessName: *mut u16,
    pub pUserSid: usize,
}

#[repr(C)]
#[allow(non_snake_case)]
struct STARTUPINFOW {
    cb: u32,
    lpReserved: *mut u16,
    lpDesktop: *mut u16,
    lpTitle: *mut u16,
    dwX: u32,
    dwY: u32,
    dwXSize: u32,
    dwYSize: u32,
    dwXCountChars: u32,
    dwYCountChars: u32,
    dwFillAttribute: u32,
    dwFlags: u32,
    wShowWindow: u16,
    cbReserved2: u16,
    lpReserved2: *mut u8,
    hStdInput: usize,
    hStdOutput: usize,
    hStdError: usize,
}

#[repr(C)]
#[allow(non_snake_case)]
struct PROCESS_INFORMATION {
    hProcess: usize,
    hThread: usize,
    dwProcessId: u32,
    dwThreadId: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct KEYBDINPUT {
    wVk: u16,
    wScan: u16,
    dwFlags: u32,
    time: u32,
    dwExtraInfo: usize,
}

#[repr(C)]
struct INPUT {
    type_: u32,
    ki: KEYBDINPUT,
    padding: [u8; 8],
}

#[link(name = "user32")]
unsafe extern "system" {
    fn LockWorkStation() -> i32;
    fn OpenInputDesktop(dwFlags: u32, fInherit: i32, dwDesiredAccess: u32) -> usize;
    fn CloseDesktop(hDesktop: usize) -> i32;
    fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
    fn SendInput(cInputs: u32, pInputs: *const INPUT, cbSize: i32) -> u32;
}

#[link(name = "wtsapi32")]
unsafe extern "system" {
    fn WTSGetActiveConsoleSessionId() -> u32;
    fn WTSQuerySessionInformationW(
        hServer: usize,
        SessionId: u32,
        WTSInfoClass: u32,
        ppBuffer: *mut *mut u8,
        pBytesReturned: *mut u32,
    ) -> i32;
    fn WTSEnumerateProcessesW(
        hServer: usize,
        Reserved: u32,
        Version: u32,
        ppProcessInfo: *mut *mut WTS_PROCESS_INFOW,
        pCount: *mut u32,
    ) -> i32;
    fn WTSFreeMemory(pMemory: *mut std::ffi::c_void);
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> usize;
    fn CloseHandle(hObject: usize) -> i32;
    fn GetLastError() -> u32;
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> usize;
    fn LoadLibraryW(lpLibFileName: *const u16) -> usize;
    fn GetProcAddress(hModule: usize, lpProcName: *const i8) -> *mut std::ffi::c_void;
    fn FreeLibrary(hLibModule: usize) -> i32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(ProcessHandle: usize, DesiredAccess: u32, TokenHandle: *mut usize) -> i32;
    fn LookupPrivilegeValueW(lpSystemName: *const u16, lpName: *const u16, lpLuid: *mut LUID) -> i32;
    fn AdjustTokenPrivileges(
        TokenHandle: usize,
        DisableAllPrivileges: i32,
        NewState: *const TOKEN_PRIVILEGES,
        BufferLength: u32,
        PreviousState: *mut std::ffi::c_void,
        ReturnLength: *mut u32,
    ) -> i32;
    fn DuplicateTokenEx(
        hExistingToken: usize,
        dwDesiredAccess: u32,
        lpTokenAttributes: *mut std::ffi::c_void,
        ImpersonationLevel: i32,
        TokenType: i32,
        phNewToken: *mut usize,
    ) -> i32;
    fn CreateProcessAsUserW(
        hToken: usize,
        lpApplicationName: *const u16,
        lpCommandLine: *mut u16,
        lpProcessAttributes: *mut std::ffi::c_void,
        lpThreadAttributes: *mut std::ffi::c_void,
        bInheritHandles: i32,
        dwCreationFlags: u32,
        lpEnvironment: *mut std::ffi::c_void,
        lpCurrentDirectory: *const u16,
        lpStartupInfo: *const STARTUPINFOW,
        lpProcessInformation: *mut PROCESS_INFORMATION,
    ) -> i32;
    fn CreateProcessWithTokenW(
        hToken: usize,
        dwLogonFlags: u32,
        lpApplicationName: *const u16,
        lpCommandLine: *mut u16,
        dwCreationFlags: u32,
        lpEnvironment: *mut std::ffi::c_void,
        lpCurrentDirectory: *const u16,
        lpStartupInfo: *const STARTUPINFOW,
        lpProcessInformation: *mut PROCESS_INFORMATION,
    ) -> i32;
}

pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide_ptr(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(slice::from_raw_parts(ptr, len))
    }
}

pub fn enable_privilege(name: &str) -> bool {
    unsafe {
        let mut h_token = 0usize;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut h_token) == 0 {
            return false;
        }
        let mut luid = LUID { LowPart: 0, HighPart: 0 };
        let wname = to_wide(name);
        if LookupPrivilegeValueW(ptr::null(), wname.as_ptr(), &mut luid) == 0 {
            CloseHandle(h_token);
            return false;
        }
        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        let res = AdjustTokenPrivileges(h_token, 0, &tp, 0, ptr::null_mut(), ptr::null_mut());
        let err = GetLastError();
        CloseHandle(h_token);
        res != 0 && err == 0
    }
}

pub fn get_active_session_id() -> u32 {
    let sess = unsafe { WTSGetActiveConsoleSessionId() };
    if sess == 0xFFFFFFFF {
        1
    } else {
        sess
    }
}

pub fn is_session_locked(session_id: u32) -> bool {
    let mut buf: *mut u8 = ptr::null_mut();
    let mut bytes: u32 = 0;
    let res = unsafe { WTSQuerySessionInformationW(0, session_id, 25 /* WTSSessionInfoEx */, &mut buf, &mut bytes) };

    if res != 0 && !buf.is_null() && bytes >= 20 {
        // At offset 16 (0x10), SessionFlags: 0 = WTS_SESSIONSTATE_LOCK (locked), 1 = WTS_SESSIONSTATE_UNLOCK (unlocked)
        let flags = unsafe { *(buf.add(16) as *const i32) };
        unsafe { WTSFreeMemory(buf as *mut std::ffi::c_void); }
        return flags == 0;
    }

    // Fallback: check OpenInputDesktop from current thread
    unsafe {
        let h_desk = OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP);
        if h_desk == 0 {
            true
        } else {
            CloseDesktop(h_desk);
            false
        }
    }
}

type WTSLockWorkStationFn = unsafe extern "system" fn(usize, u32) -> i32;

pub fn lock_session(session_id: u32) -> bool {
    let mut wts_ok = false;
    unsafe {
        let mod_name = to_wide("wtsapi32.dll");
        let h_mod = LoadLibraryW(mod_name.as_ptr());
        if h_mod != 0 {
            let proc_name = std::ffi::CString::new("WTSLockWorkStation").unwrap();
            let proc = GetProcAddress(h_mod, proc_name.as_ptr());
            if !proc.is_null() {
                let func: WTSLockWorkStationFn = std::mem::transmute(proc);
                wts_ok = func(0, session_id) != 0;
            }
            FreeLibrary(h_mod);
        }
    }
    let u32_ok = unsafe { LockWorkStation() != 0 };
    wts_ok || u32_ok
}

pub fn find_winlogon_pid(session_id: u32) -> Option<u32> {
    let mut info_ptr: *mut WTS_PROCESS_INFOW = ptr::null_mut();
    let mut count: u32 = 0;

    let res = unsafe { WTSEnumerateProcessesW(0, 0, 1, &mut info_ptr, &mut count) };
    if res != 0 && !info_ptr.is_null() {
        let processes = unsafe { slice::from_raw_parts(info_ptr, count as usize) };
        let mut target_pid = None;
        for p in processes {
            let name = from_wide_ptr(p.pProcessName);
            if name.eq_ignore_ascii_case("winlogon.exe") && p.SessionId == session_id {
                target_pid = Some(p.ProcessId);
                break;
            }
        }
        unsafe { WTSFreeMemory(info_ptr as *mut std::ffi::c_void); }
        return target_pid;
    }
    None
}

pub fn spawn_injector_in_winlogon(session_id: u32, pin: &str) -> Result<(), String> {
    // Ensure privileges are enabled
    enable_privilege("SeDebugPrivilege");
    enable_privilege("SeImpersonatePrivilege");
    enable_privilege("SeAssignPrimaryTokenPrivilege");

    let winlogon_pid = match find_winlogon_pid(session_id) {
        Some(pid) => pid,
        None => return Err(format!("未在会话 {} 中找到 winlogon.exe 进程", session_id)),
    };

    let h_proc = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, 0, winlogon_pid) };
    if h_proc == 0 {
        return Err(format!("无法打开 winlogon.exe (错误代码: {})。请确认是否以管理员或系统服务权限运行！", unsafe { GetLastError() }));
    }

    let mut h_token = 0usize;
    if unsafe { OpenProcessToken(h_proc, TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY | TOKEN_QUERY, &mut h_token) } == 0 {
        let err = unsafe { GetLastError() };
        unsafe { CloseHandle(h_proc); }
        return Err(format!("无法获取 winlogon.exe 访问令牌 (错误代码: {})", err));
    }

    let mut h_dup_token = 0usize;
    let dup_res = unsafe {
        DuplicateTokenEx(
            h_token,
            MAXIMUM_ALLOWED,
            ptr::null_mut(),
            SECURITY_IDENTIFICATION,
            TOKEN_PRIMARY,
            &mut h_dup_token,
        )
    };

    unsafe {
        CloseHandle(h_token);
        CloseHandle(h_proc);
    }

    if dup_res == 0 || h_dup_token == 0 {
        return Err(format!("复制系统令牌失败 (错误代码: {})", unsafe { GetLastError() }));
    }

    let exe_path = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(e) => {
            unsafe { CloseHandle(h_dup_token); }
            return Err(format!("获取当前可执行文件路径失败: {}", e));
        }
    };

    let cmd = format!("\"{}\" inject-pin \"{}\"", exe_path, pin);
    let mut cmd_wide = to_wide(&cmd);
    let mut desktop_wide = to_wide("WinSta0\\Winlogon");

    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    si.lpDesktop = desktop_wide.as_mut_ptr();

    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    // Attempt 1: CreateProcessAsUserW
    let res = unsafe {
        CreateProcessAsUserW(
            h_dup_token,
            ptr::null(),
            cmd_wide.as_mut_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            0x00000010, // CREATE_NEW_CONSOLE
            ptr::null_mut(),
            ptr::null(),
            &si,
            &mut pi,
        )
    };

    if res != 0 {
        unsafe {
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
            CloseHandle(h_dup_token);
        }
        return Ok(());
    }

    let err1 = unsafe { GetLastError() };

    // Attempt 2: Fallback to CreateProcessWithTokenW (works for administrators with SeImpersonatePrivilege)
    let res2 = unsafe {
        CreateProcessWithTokenW(
            h_dup_token,
            0,
            ptr::null(),
            cmd_wide.as_mut_ptr(),
            0x00000010, // CREATE_NEW_CONSOLE
            ptr::null_mut(),
            ptr::null(),
            &si,
            &mut pi,
        )
    };

    unsafe { CloseHandle(h_dup_token); }

    if res2 != 0 {
        unsafe {
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
        }
        Ok(())
    } else {
        let err2 = unsafe { GetLastError() };
        Err(format!("无法在 Winlogon 桌面启动解锁注入进程 (CreateProcessAsUser 错误: {}, CreateProcessWithToken 错误: {})", err1, err2))
    }
}

pub fn inject_pin_on_current_desktop(pin: &str) {
    let prog_data = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    let log_path = std::path::PathBuf::from(prog_data).join("SmartCardUnlock").join("injector.log");
    let _ = fs::write(&log_path, format!("[Injector] 启动成功，正在唤醒锁屏并输入 PIN 码 (长度: {})... \n", pin.len()));

    // 1. Wake screen and dismiss lock curtain
    unsafe {
        keybd_event(VK_SPACE, 0, 0, 0);
        thread::sleep(Duration::from_millis(40));
        keybd_event(VK_SPACE, 0, KEYEVENTF_KEYUP, 0);
    }
    thread::sleep(Duration::from_millis(350));

    // 2. Clear any stale keystrokes in the PIN box
    for _ in 0..10 {
        unsafe {
            keybd_event(VK_BACK, 0, 0, 0);
            thread::sleep(Duration::from_millis(15));
            keybd_event(VK_BACK, 0, KEYEVENTF_KEYUP, 0);
            thread::sleep(Duration::from_millis(15));
        }
    }
    thread::sleep(Duration::from_millis(100));

    // 3. Type PIN digits via SendInput KEYEVENTF_UNICODE
    for ch in pin.chars() {
        let utf16 = ch as u16;
        let mut down = INPUT {
            type_: 1,
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: utf16,
                dwFlags: KEYEVENTF_UNICODE,
                time: 0,
                dwExtraInfo: 0,
            },
            padding: [0; 8],
        };
        let mut up = INPUT {
            type_: 1,
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: utf16,
                dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
            padding: [0; 8],
        };

        unsafe {
            SendInput(1, &mut down, std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(25));
            SendInput(1, &mut up, std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(35));
        }
    }

    // 4. Submit via Enter
    thread::sleep(Duration::from_millis(200));
    unsafe {
        keybd_event(VK_RETURN, 0, 0, 0);
        thread::sleep(Duration::from_millis(40));
        keybd_event(VK_RETURN, 0, KEYEVENTF_KEYUP, 0);
    }
    thread::sleep(Duration::from_millis(200));

    let _ = fs::write(&log_path, "[Injector] PIN 码注入完成！\n");
}
