use std::ptr;
use std::slice;

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

#[link(name = "user32")]
unsafe extern "system" {
    fn LockWorkStation() -> i32;
    fn OpenInputDesktop(dwFlags: u32, fInherit: i32, dwDesiredAccess: u32) -> usize;
    fn CloseDesktop(hDesktop: usize) -> i32;
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
        // Offset 16 (0x10), SessionFlags: 0 = WTS_SESSIONSTATE_LOCK (locked), 1 = WTS_SESSIONSTATE_UNLOCK (unlocked)
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
