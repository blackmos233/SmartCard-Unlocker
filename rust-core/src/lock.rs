use std::fs;
use std::ptr;
use std::slice;
use std::thread;
use std::time::Duration;

pub const VK_BACK: u8 = 0x08;
pub const VK_RETURN: u8 = 0x0D;
pub const VK_SPACE: u8 = 0x20;
pub const KEYEVENTF_KEYUP: u32 = 0x0002;

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
    fn mouse_event(dwFlags: u32, dx: u32, dy: u32, dwData: u32, dwExtraInfo: usize);
    fn SendInput(cInputs: u32, pInputs: *const INPUT, cbSize: i32) -> u32;
    fn MapVirtualKeyW(uCode: u32, uMapType: u32) -> u32;
    fn GetThreadDesktop(dwThreadId: u32) -> usize;
    fn GetUserObjectInformationW(
        hObj: usize,
        nIndex: i32,
        pvInfo: *mut std::ffi::c_void,
        nLength: u32,
        lpnLengthNeeded: *mut u32,
    ) -> i32;
    fn OpenDesktopW(lpszDesktop: *const u16, dwFlags: u32, fInherit: i32, dwDesiredAccess: u32) -> usize;
    fn SetThreadDesktop(hDesktop: usize) -> i32;
}

#[link(name = "wtsapi32")]
unsafe extern "system" {
    fn WTSGetActiveConsoleSessionId() -> u32;
    fn WTSQueryUserToken(SessionId: u32, phToken: *mut usize) -> i32;
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
    fn GetCurrentThreadId() -> u32;
    fn CloseHandle(hObject: usize) -> i32;
    fn GetLastError() -> u32;
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> usize;
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

fn spawn_process_in_session(token: usize, cmd: &str, desktop: &str) -> bool {
    let mut cmd_w = to_wide(cmd);
    let mut desk_w = to_wide(desktop);
    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    si.lpDesktop = desk_w.as_mut_ptr();
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let res = unsafe {
        CreateProcessAsUserW(
            token,
            ptr::null(),
            cmd_w.as_mut_ptr(),
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
        }
        return true;
    }

    // Fallback: CreateProcessWithTokenW
    let res2 = unsafe {
        CreateProcessWithTokenW(
            token,
            0,
            ptr::null(),
            cmd_w.as_mut_ptr(),
            0x00000010,
            ptr::null_mut(),
            ptr::null(),
            &si,
            &mut pi,
        )
    };

    if res2 != 0 {
        unsafe {
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
        }
        true
    } else {
        false
    }
}

pub fn lock_session(session_id: u32) -> bool {
    enable_privilege("SeDebugPrivilege");
    enable_privilege("SeImpersonatePrivilege");
    enable_privilege("SeAssignPrimaryTokenPrivilege");

    // 1. If we are already running in the target session, LockWorkStation works directly
    let cur_session = unsafe { WTSGetActiveConsoleSessionId() };
    if cur_session == session_id {
        if unsafe { LockWorkStation() != 0 } {
            return true;
        }
    }

    // 2. From Session 0 service: Query user token and spawn lock process into Session 1
    let mut h_user_token = 0usize;
    if unsafe { WTSQueryUserToken(session_id, &mut h_user_token) } != 0 {
        let ok = spawn_process_in_session(h_user_token, "rundll32.exe user32.dll,LockWorkStation", "WinSta0\\Default");
        unsafe { CloseHandle(h_user_token); }
        if ok {
            return true;
        }
    }

    // 3. Fallback: Duplicate winlogon.exe token in target session and spawn lock
    if let Some(winlogon_pid) = find_winlogon_pid(session_id) {
        unsafe {
            let h_proc = OpenProcess(PROCESS_QUERY_INFORMATION, 0, winlogon_pid);
            if h_proc != 0 {
                let mut h_token = 0usize;
                if OpenProcessToken(h_proc, TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY | TOKEN_QUERY, &mut h_token) != 0 {
                    let mut h_dup = 0usize;
                    if DuplicateTokenEx(h_token, MAXIMUM_ALLOWED, ptr::null_mut(), SECURITY_IDENTIFICATION, TOKEN_PRIMARY, &mut h_dup) != 0 {
                        let ok = spawn_process_in_session(h_dup, "rundll32.exe user32.dll,LockWorkStation", "WinSta0\\Default");
                        CloseHandle(h_dup);
                        CloseHandle(h_token);
                        CloseHandle(h_proc);
                        if ok {
                            return true;
                        }
                    }
                    CloseHandle(h_token);
                }
                CloseHandle(h_proc);
            }
        }
    }

    unsafe { LockWorkStation() != 0 }
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

#[allow(dead_code)]
pub fn spawn_injector_in_winlogon(session_id: u32, pin: &str) -> Result<(), String> {
    enable_privilege("SeDebugPrivilege");
    enable_privilege("SeImpersonatePrivilege");
    enable_privilege("SeAssignPrimaryTokenPrivilege");

    let winlogon_pid = match find_winlogon_pid(session_id) {
        Some(pid) => pid,
        None => return Err(format!("未在会话 {} 中找到 winlogon.exe 进程", session_id)),
    };

    let h_proc = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, 0, winlogon_pid) };
    if h_proc == 0 {
        return Err(format!("无法打开 winlogon.exe (错误代码: {})。请确认权限！", unsafe { GetLastError() }));
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
    let ok = spawn_process_in_session(h_dup_token, &cmd, "WinSta0\\Winlogon");
    unsafe { CloseHandle(h_dup_token); }

    if ok {
        Ok(())
    } else {
        Err(format!("启动 Winlogon 注入进程失败 (错误代码: {})", unsafe { GetLastError() }))
    }
}

fn log_injector(msg: &str) {
    let prog_data = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    let log_path = std::path::PathBuf::from(prog_data).join("SmartCardUnlock").join("injector.log");
    let line = format!("[{}] {}\n", chrono_timestamp(), msg);
    let _ = fs::OpenOptions::new().create(true).append(true).open(log_path).and_then(|mut f| {
        use std::io::Write;
        f.write_all(line.as_bytes())
    });
}

fn chrono_timestamp() -> String {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct SYSTEMTIME {
        wYear: u16,
        wMonth: u16,
        wDayOfWeek: u16,
        wDay: u16,
        wHour: u16,
        wMinute: u16,
        wSecond: u16,
        wMilliseconds: u16,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
    }
    let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut st); }
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}", st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, st.wMilliseconds)
}

pub fn inject_pin_on_current_desktop(pin: &str) {
    let cur_thread = unsafe { GetCurrentThreadId() };
    let h_desk = unsafe { GetThreadDesktop(cur_thread) };
    let mut desk_name = [0u16; 256];
    let mut needed = 0u32;
    unsafe {
        GetUserObjectInformationW(h_desk, 2 /* UOI_NAME */, desk_name.as_mut_ptr() as *mut _, 512, &mut needed);
    }
    let dname = String::from_utf16_lossy(&desk_name[..(needed as usize / 2).saturating_sub(1)]);
    log_injector(&format!("Injector 启动: ThreadId={}, Desktop={}, PIN长度={}", cur_thread, dname, pin.len()));

    // Attach to Winlogon if not already attached
    let winlogon_w = to_wide("Winlogon");
    let h_winlogon = unsafe { OpenDesktopW(winlogon_w.as_ptr(), 0, 0, MAXIMUM_ALLOWED) };
    if h_winlogon != 0 {
        let sres = unsafe { SetThreadDesktop(h_winlogon) };
        log_injector(&format!("SetThreadDesktop to Winlogon: res={}", sres));
        unsafe { CloseDesktop(h_winlogon); }
    }

    // 1. Wake display by moving mouse slightly
    unsafe {
        mouse_event(0x0001 /* MOUSEEVENTF_MOVE */, 2, 2, 0, 0);
        thread::sleep(Duration::from_millis(50));
        mouse_event(0x0001 /* MOUSEEVENTF_MOVE */, 65534, 65534, 0, 0);
        thread::sleep(Duration::from_millis(50));
    }

    // 2. Slide up lock curtain
    unsafe {
        keybd_event(VK_SPACE, 0, 0, 0);
        thread::sleep(Duration::from_millis(50));
        keybd_event(VK_SPACE, 0, KEYEVENTF_KEYUP, 0);
    }

    // CRUCIAL: Allow Windows 10/11 lock screen slide animation to finish and focus PIN box!
    thread::sleep(Duration::from_millis(700));

    // 3. Clear previous characters in PIN box (Ctrl+A then Backspace)
    unsafe {
        keybd_event(0x11 /* VK_CONTROL */, 0, 0, 0);
        thread::sleep(Duration::from_millis(30));
        keybd_event(0x41 /* VK_A */, 0, 0, 0);
        thread::sleep(Duration::from_millis(30));
        keybd_event(0x41, 0, KEYEVENTF_KEYUP, 0);
        thread::sleep(Duration::from_millis(30));
        keybd_event(0x11, 0, KEYEVENTF_KEYUP, 0);
        thread::sleep(Duration::from_millis(50));

        keybd_event(VK_BACK, 0, 0, 0);
        thread::sleep(Duration::from_millis(30));
        keybd_event(VK_BACK, 0, KEYEVENTF_KEYUP, 0);
        thread::sleep(Duration::from_millis(100));
    }

    // 4. Send each PIN digit using REAL Virtual-Key and Scan Code
    for (idx, ch) in pin.chars().enumerate() {
        let vk: u16 = if ch.is_ascii_digit() {
            (ch as u8) as u16 // '0' is 0x30, '1' is 0x31 ...
        } else if ch.is_ascii_alphabetic() {
            (ch.to_ascii_uppercase() as u8) as u16
        } else {
            ch as u16
        };
        let scan = unsafe { MapVirtualKeyW(vk as u32, 0) } as u16;

        let mut down = INPUT {
            type_: 1,
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            },
            padding: [0; 8],
        };
        let mut up = INPUT {
            type_: 1,
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
            padding: [0; 8],
        };

        let r1 = unsafe { SendInput(1, &mut down, std::mem::size_of::<INPUT>() as i32) };
        thread::sleep(Duration::from_millis(35));
        let r2 = unsafe { SendInput(1, &mut up, std::mem::size_of::<INPUT>() as i32) };
        thread::sleep(Duration::from_millis(50));

        if r1 == 0 || r2 == 0 {
            log_injector(&format!("按键 [{}] 输入失败 (错误: {})", idx, unsafe { GetLastError() }));
        }
    }

    // 5. Submit via Enter
    thread::sleep(Duration::from_millis(250));
    let enter_scan = unsafe { MapVirtualKeyW(VK_RETURN as u32, 0) } as u16;
    let mut enter_down = INPUT {
        type_: 1,
        ki: KEYBDINPUT {
            wVk: VK_RETURN as u16,
            wScan: enter_scan,
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        },
        padding: [0; 8],
    };
    let mut enter_up = INPUT {
        type_: 1,
        ki: KEYBDINPUT {
            wVk: VK_RETURN as u16,
            wScan: enter_scan,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        },
        padding: [0; 8],
    };
    unsafe {
        SendInput(1, &mut enter_down, std::mem::size_of::<INPUT>() as i32);
        thread::sleep(Duration::from_millis(40));
        SendInput(1, &mut enter_up, std::mem::size_of::<INPUT>() as i32);
    }
    thread::sleep(Duration::from_millis(200));

    log_injector("PIN 码完整输入流程执行完毕！");
}
