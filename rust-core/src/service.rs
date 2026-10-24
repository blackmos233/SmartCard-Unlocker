use std::fs;
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use crate::lock::{get_active_session_id, is_session_locked, lock_session, to_wide};
use crate::scard::SmartCardEngine;
use crate::vault::SecurityVault;

const SERVICE_WIN32_OWN_PROCESS: u32 = 0x00000010;
const SERVICE_RUNNING: u32 = 0x00000004;
const SERVICE_STOP_PENDING: u32 = 0x00000003;
const SERVICE_STOPPED: u32 = 0x00000001;

const SERVICE_ACCEPT_STOP: u32 = 0x00000001;
const SERVICE_ACCEPT_SHUTDOWN: u32 = 0x00000004;
const SERVICE_ACCEPT_SESSIONCHANGE: u32 = 0x00000080;

const SERVICE_CONTROL_STOP: u32 = 0x00000001;
const SERVICE_CONTROL_SHUTDOWN: u32 = 0x00000005;

const SC_MANAGER_ALL_ACCESS: u32 = 0xF003F;
const SERVICE_ALL_ACCESS: u32 = 0xF01FF;
const SERVICE_AUTO_START: u32 = 0x00000002;
const SERVICE_ERROR_NORMAL: u32 = 0x00000001;
const SERVICE_CONFIG_DESCRIPTION: u32 = 1;

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
static STATUS_HANDLE: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
#[allow(non_snake_case)]
struct SERVICE_TABLE_ENTRYW {
    lpServiceName: *const u16,
    lpServiceProc: Option<unsafe extern "system" fn(u32, *mut *mut u16)>,
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct SERVICE_STATUS {
    pub dwServiceType: u32,
    pub dwCurrentState: u32,
    pub dwControlsAccepted: u32,
    pub dwWin32ExitCode: u32,
    pub dwServiceSpecificExitCode: u32,
    pub dwCheckPoint: u32,
    pub dwWaitHint: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct SERVICE_DESCRIPTIONW {
    lpDescription: *mut u16,
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct SERVICE_STATUS_PROCESS {
    pub dwServiceType: u32,
    pub dwCurrentState: u32,
    pub dwControlsAccepted: u32,
    pub dwWin32ExitCode: u32,
    pub dwServiceSpecificExitCode: u32,
    pub dwCheckPoint: u32,
    pub dwWaitHint: u32,
    pub dwProcessId: u32,
    pub dwServiceFlags: u32,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn StartServiceCtrlDispatcherW(lpServiceStartTable: *const SERVICE_TABLE_ENTRYW) -> i32;
    fn RegisterServiceCtrlHandlerExW(
        lpServiceName: *const u16,
        lpHandlerProc: Option<unsafe extern "system" fn(u32, u32, *mut std::ffi::c_void, *mut std::ffi::c_void) -> u32>,
        lpContext: *mut std::ffi::c_void,
    ) -> usize;
    fn SetServiceStatus(hServiceStatus: usize, lpServiceStatus: *const SERVICE_STATUS) -> i32;
    fn OpenSCManagerW(lpMachineName: *const u16, lpDatabaseName: *const u16, dwDesiredAccess: u32) -> usize;
    fn CreateServiceW(
        hSCManager: usize,
        lpServiceName: *const u16,
        lpDisplayName: *const u16,
        dwDesiredAccess: u32,
        dwServiceType: u32,
        dwStartType: u32,
        dwErrorControl: u32,
        lpBinaryPathName: *const u16,
        lpLoadOrderGroup: *const u16,
        lpdwTagId: *mut u32,
        lpDependencies: *const u16,
        lpServiceStartName: *const u16,
        lpPassword: *const u16,
    ) -> usize;
    fn OpenServiceW(hSCManager: usize, lpServiceName: *const u16, dwDesiredAccess: u32) -> usize;
    fn StartServiceW(hService: usize, dwNumServiceArgs: u32, lpServiceArgVectors: *mut *const u16) -> i32;
    fn ControlService(hService: usize, dwControl: u32, lpServiceStatus: *mut SERVICE_STATUS) -> i32;
    fn DeleteService(hService: usize) -> i32;
    fn CloseServiceHandle(hSCObject: usize) -> i32;
    fn ChangeServiceConfig2W(hService: usize, dwInfoLevel: u32, lpInfo: *mut std::ffi::c_void) -> i32;
    fn QueryServiceStatusEx(
        hService: usize,
        InfoLevel: u32,
        lpBuffer: *mut u8,
        cbBufSize: u32,
        pcbBytesNeeded: *mut u32,
    ) -> i32;
    fn RegOpenKeyExW(
        hKey: usize,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut usize,
    ) -> i32;
    fn RegCloseKey(hKey: usize) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(lpLibFileName: *const u16) -> usize;
    fn GetProcAddress(hModule: usize, lpProcName: *const i8) -> Option<unsafe extern "system" fn()>;
    fn FreeLibrary(hLibModule: usize) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> u32;
}

fn log_service(msg: &str) {
    let prog_data = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    let log_file = PathBuf::from(prog_data).join("SmartCardUnlock").join("service.log");
    let line = format!("[{}] {}\n", chrono_timestamp(), msg);
    let _ = fs::OpenOptions::new().create(true).append(true).open(log_file).and_then(|mut f| {
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

unsafe extern "system" fn service_ctrl_handler(
    control: u32,
    _event_type: u32,
    _event_data: *mut std::ffi::c_void,
    _context: *mut std::ffi::c_void,
) -> u32 {
    match control {
        SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN => {
            STOP_REQUESTED.store(true, Ordering::SeqCst);
            let status = SERVICE_STATUS {
                dwServiceType: SERVICE_WIN32_OWN_PROCESS,
                dwCurrentState: SERVICE_STOP_PENDING,
                dwControlsAccepted: 0,
                dwWin32ExitCode: 0,
                dwServiceSpecificExitCode: 0,
                dwCheckPoint: 1,
                dwWaitHint: 3000,
            };
            let h = STATUS_HANDLE.load(Ordering::SeqCst);
            if h != 0 {
                unsafe { SetServiceStatus(h, &status); }
            }
            0 // NO_ERROR
        }
        _ => 0,
    }
}

pub fn is_credential_provider_registered() -> bool {
    let key_path = to_wide(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}");
    let mut h_key: usize = 0;
    const HKEY_LOCAL_MACHINE: usize = 0x80000002;
    const KEY_READ: u32 = 0x20019;
    unsafe {
        let res = RegOpenKeyExW(HKEY_LOCAL_MACHINE, key_path.as_ptr(), 0, KEY_READ, &mut h_key);
        if res == 0 && h_key != 0 {
            RegCloseKey(h_key);
            true
        } else {
            false
        }
    }
}

pub fn register_credential_provider_dll() -> bool {
    let prog_data = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    let sys_dir = PathBuf::from(&prog_data).join("SmartCardUnlock");
    let _ = fs::create_dir_all(&sys_dir);
    let dll_dest = sys_dir.join("SmartCardCredentialProvider.dll");

    // Check workspace bin path
    let src_candidates = [
        PathBuf::from(r"D:\work\SmartCard\bin\SmartCardCredentialProvider.dll"),
        std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(&PathBuf::new())
            .join("SmartCardCredentialProvider.dll"),
    ];

    for src in &src_candidates {
        if src.exists() {
            let _ = fs::copy(src, &dll_dest);
            break;
        }
    }

    if !dll_dest.exists() {
        log_service(&format!("[错误] 找不到凭据提供程序 DLL: {:?}", dll_dest));
        return false;
    }

    let dll_wide = to_wide(dll_dest.to_str().unwrap_or_default());
    unsafe {
        let h_module = LoadLibraryW(dll_wide.as_ptr());
        if h_module != 0 {
            let proc_name = std::ffi::CString::new("DllRegisterServer").unwrap();
            let p_fn = GetProcAddress(h_module, proc_name.as_ptr());
            if let Some(reg_fn) = p_fn {
                let reg: extern "system" fn() -> i32 = std::mem::transmute(reg_fn);
                let hr = reg();
                log_service(&format!("SmartCardCredentialProvider 成功注册到系统！HRESULT = 0x{:08X}", hr));
                FreeLibrary(h_module);
                return hr == 0;
            }
            FreeLibrary(h_module);
        }
    }
    false
}

unsafe extern "system" fn service_main(_argc: u32, _argv: *mut *mut u16) {
    let name_wide = to_wide("SmartCardUnlock");
    let handle = unsafe {
        RegisterServiceCtrlHandlerExW(name_wide.as_ptr(), Some(service_ctrl_handler), ptr::null_mut())
    };

    if handle == 0 {
        return;
    }
    STATUS_HANDLE.store(handle, Ordering::SeqCst);

    let running_status = SERVICE_STATUS {
        dwServiceType: SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState: SERVICE_RUNNING,
        dwControlsAccepted: SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN | SERVICE_ACCEPT_SESSIONCHANGE,
        dwWin32ExitCode: 0,
        dwServiceSpecificExitCode: 0,
        dwCheckPoint: 0,
        dwWaitHint: 0,
    };
    unsafe { SetServiceStatus(handle, &running_status); }

    log_service("=== SmartCardUnlock 服务已启动 (SYSTEM 权限) ===");

    // 确保原生凭据提供程序向 Windows 注册
    register_credential_provider_dll();

    let engine = match SmartCardEngine::new() {
        Ok(e) => e,
        Err(err) => {
            log_service(&format!("[错误] 无法初始化 PC/SC 智能卡管理器 (错误码: 0x{:08X})", err));
            let stopped = SERVICE_STATUS {
                dwServiceType: SERVICE_WIN32_OWN_PROCESS,
                dwCurrentState: SERVICE_STOPPED,
                dwControlsAccepted: 0,
                dwWin32ExitCode: err as u32,
                dwServiceSpecificExitCode: 0,
                dwCheckPoint: 0,
                dwWaitHint: 0,
            };
            unsafe { SetServiceStatus(handle, &stopped); }
            return;
        }
    };

    let _vault = SecurityVault::new();
    let mut last_present: Option<bool> = None;

    while !STOP_REQUESTED.load(Ordering::SeqCst) {
        let readers = engine.list_readers();
        if readers.is_empty() {
            thread::sleep(Duration::from_millis(1000));
            continue;
        }

        let target_reader = readers
            .iter()
            .find(|r| {
                let lower = r.to_lowercase();
                lower.contains("emv") || lower.contains("rts") || lower.contains("realtek")
            })
            .unwrap_or(&readers[0]);

        let (present, _event_state) = engine.get_card_status(target_reader);

        match last_present {
            None => {
                last_present = Some(present);
                log_service(&format!("读卡器初始化: 当前卡片状态={}", if present { "已插入" } else { "未插卡" }));
            }
            Some(prev) => {
                if prev != present {
                    last_present = Some(present);
                    let session_id = get_active_session_id();

                    if present {
                        log_service(&format!("[事件] 检测到 IC4442 卡片插入！(目标会话: {})", session_id));
                        if is_session_locked(session_id) {
                            log_service("系统处于锁定状态，由系统原生 SmartCardCredentialProvider 处理 Windows 原生认证解锁。");
                        } else {
                            log_service("系统处于未锁定状态，无需执行解锁。");
                        }
                    } else {
                        log_service(&format!("[事件] 检测到 IC4442 卡片拔出！立即锁定会话 {}...", session_id));
                        let locked = lock_session(session_id);
                        log_service(&format!("锁屏调用结果: {}", if locked { "成功" } else { "触发失败" }));
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(200));
    }

    log_service("=== SmartCardUnlock 服务正在停止 ===");
    let stopped = SERVICE_STATUS {
        dwServiceType: SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState: SERVICE_STOPPED,
        dwControlsAccepted: 0,
        dwWin32ExitCode: 0,
        dwServiceSpecificExitCode: 0,
        dwCheckPoint: 0,
        dwWaitHint: 0,
    };
    unsafe { SetServiceStatus(handle, &stopped); }
}

pub fn run_service() {
    let name_wide = to_wide("SmartCardUnlock");
    let table = [
        SERVICE_TABLE_ENTRYW {
            lpServiceName: name_wide.as_ptr(),
            lpServiceProc: Some(service_main),
        },
        SERVICE_TABLE_ENTRYW {
            lpServiceName: ptr::null(),
            lpServiceProc: None,
        },
    ];

    unsafe {
        StartServiceCtrlDispatcherW(table.as_ptr());
    }
}

pub fn install_service() -> Result<(), String> {
    let exe_path = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(e) => return Err(format!("无法获取当前程序路径: {}", e)),
    };

    let bin_path = format!("\"{}\" service-run", exe_path);

    let scm = unsafe { OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_ALL_ACCESS) };
    if scm == 0 {
        return Err(format!("无法连接 Windows 服务控制管理器 (错误代码: {})。请务必右键『以管理员身份运行』！", unsafe { GetLastError() }));
    }

    let svc_name = to_wide("SmartCardUnlock");
    let disp_name = to_wide("SmartCard RTS5169 Auto Unlock Service");
    let bin_path_w = to_wide(&bin_path);

    let svc = unsafe {
        CreateServiceW(
            scm,
            svc_name.as_ptr(),
            disp_name.as_ptr(),
            SERVICE_ALL_ACCESS,
            SERVICE_WIN32_OWN_PROCESS,
            SERVICE_AUTO_START,
            SERVICE_ERROR_NORMAL,
            bin_path_w.as_ptr(),
            ptr::null(),
            ptr::null_mut(),
            ptr::null(),
            ptr::null(), // LocalSystem
            ptr::null(),
        )
    };

    if svc == 0 {
        let err = unsafe { GetLastError() };
        unsafe { CloseServiceHandle(scm); }
        if err == 1073 {
            return Err("服务已存在，若要更新请先执行卸载服务 (uninstall-service)".to_string());
        }
        return Err(format!("创建服务失败 (错误代码: {})", err));
    }

    let desc = to_wide("基于 RTS5169 读卡器和 IC4442 卡片的 Windows 原生插拔卡自动锁屏与 Hello PIN 自动解锁服务。");
    let mut sd = SERVICE_DESCRIPTIONW {
        lpDescription: desc.as_ptr() as *mut u16,
    };
    unsafe {
        ChangeServiceConfig2W(svc, SERVICE_CONFIG_DESCRIPTION, &mut sd as *mut _ as *mut std::ffi::c_void);
        StartServiceW(svc, 0, ptr::null_mut());
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);
    }

    Ok(())
}

pub fn uninstall_service() -> Result<(), String> {
    let scm = unsafe { OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_ALL_ACCESS) };
    if scm == 0 {
        return Err(format!("无法连接 Windows 服务控制管理器 (错误代码: {})。请务必以管理员身份运行！", unsafe { GetLastError() }));
    }

    let svc_name = to_wide("SmartCardUnlock");
    let svc = unsafe { OpenServiceW(scm, svc_name.as_ptr(), SERVICE_ALL_ACCESS) };
    if svc == 0 {
        unsafe { CloseServiceHandle(scm); }
        return Err("服务未安装或已被卸载。".to_string());
    }

    let mut status: SERVICE_STATUS = unsafe { std::mem::zeroed() };
    unsafe {
        ControlService(svc, SERVICE_CONTROL_STOP, &mut status);
        thread::sleep(Duration::from_millis(500));
        let res = DeleteService(svc);
        let err = GetLastError();
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);
        if res == 0 {
            return Err(format!("删除服务失败 (错误代码: {})", err));
        }
    }

    Ok(())
}

pub fn start_service() -> Result<(), String> {
    let scm = unsafe { OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_ALL_ACCESS) };
    if scm == 0 {
        return Err(format!("无法连接服务管理器 (错误码: {})", unsafe { GetLastError() }));
    }

    let svc_name = to_wide("SmartCardUnlock");
    let svc = unsafe { OpenServiceW(scm, svc_name.as_ptr(), SERVICE_ALL_ACCESS) };
    if svc == 0 {
        unsafe { CloseServiceHandle(scm); }
        return Err("服务不存在，请先执行安装服务。".to_string());
    }

    let res = unsafe { StartServiceW(svc, 0, ptr::null_mut()) };
    let err = unsafe { GetLastError() };
    unsafe {
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);
    }

    if res != 0 || err == 1056 { // 1056 = ERROR_SERVICE_ALREADY_RUNNING
        Ok(())
    } else {
        Err(format!("启动服务失败 (错误码: {})", err))
    }
}

pub fn stop_service() -> Result<(), String> {
    let scm = unsafe { OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_ALL_ACCESS) };
    if scm == 0 {
        return Err(format!("无法连接服务管理器 (错误码: {})", unsafe { GetLastError() }));
    }

    let svc_name = to_wide("SmartCardUnlock");
    let svc = unsafe { OpenServiceW(scm, svc_name.as_ptr(), SERVICE_ALL_ACCESS) };
    if svc == 0 {
        unsafe { CloseServiceHandle(scm); }
        return Err("服务不存在。".to_string());
    }

    let mut status: SERVICE_STATUS = unsafe { std::mem::zeroed() };
    let res = unsafe { ControlService(svc, SERVICE_CONTROL_STOP, &mut status) };
    let err = unsafe { GetLastError() };
    unsafe {
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);
    }

    if res != 0 || err == 1062 { // 1062 = ERROR_SERVICE_NOT_ACTIVE
        Ok(())
    } else {
        Err(format!("停止服务失败 (错误码: {})", err))
    }
}

pub fn is_service_installed_and_running() -> (bool, bool) {
    let scm = unsafe { OpenSCManagerW(ptr::null(), ptr::null(), 0x0001 /* SC_MANAGER_CONNECT */) };
    if scm == 0 {
        return (false, false);
    }

    let svc_name = to_wide("SmartCardUnlock");
    let svc = unsafe { OpenServiceW(scm, svc_name.as_ptr(), 0x0004 /* SERVICE_QUERY_STATUS */) };
    if svc == 0 {
        unsafe { CloseServiceHandle(scm); }
        return (false, false);
    }

    let mut status_buf = [0u8; std::mem::size_of::<SERVICE_STATUS_PROCESS>()];
    let mut bytes_needed = 0u32;
    let res = unsafe {
        QueryServiceStatusEx(
            svc,
            0, // SC_STATUS_PROCESS_INFO
            status_buf.as_mut_ptr(),
            status_buf.len() as u32,
            &mut bytes_needed,
        )
    };

    unsafe {
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);
    }

    if res != 0 {
        let status = unsafe { &*(status_buf.as_ptr() as *const SERVICE_STATUS_PROCESS) };
        (true, status.dwCurrentState == SERVICE_RUNNING)
    } else {
        (true, false)
    }
}
