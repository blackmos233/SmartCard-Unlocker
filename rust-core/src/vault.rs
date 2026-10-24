use std::fs;
use std::path::PathBuf;
use std::ptr;

const CRYPTPROTECT_LOCAL_MACHINE: u32 = 0x00000004;

#[repr(C)]
#[allow(non_snake_case)]
struct DATA_BLOB {
    cbData: u32,
    pbData: *mut u8,
}

#[link(name = "crypt32")]
unsafe extern "system" {
    fn CryptProtectData(
        pDataIn: *const DATA_BLOB,
        szDataDescr: *const u16,
        pOptionalEntropy: *const DATA_BLOB,
        pvReserved: *const std::ffi::c_void,
        pPromptStruct: *const std::ffi::c_void,
        dwFlags: u32,
        pDataOut: *mut DATA_BLOB,
    ) -> i32;

    fn CryptUnprotectData(
        pDataIn: *const DATA_BLOB,
        ppszDataDescr: *mut *mut u16,
        pOptionalEntropy: *const DATA_BLOB,
        pvReserved: *const std::ffi::c_void,
        pPromptStruct: *const std::ffi::c_void,
        dwFlags: u32,
        pDataOut: *mut DATA_BLOB,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LocalFree(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

pub struct SecurityVault {
    system_vault: PathBuf,
    user_vault: PathBuf,
}

impl SecurityVault {
    pub fn new() -> Self {
        // System-wide location (accessible by Windows Service / SYSTEM and Administrators)
        let prog_data = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
        let sys_dir = PathBuf::from(prog_data).join("SmartCardUnlock");
        let _ = fs::create_dir_all(&sys_dir);
        let system_vault = sys_dir.join("vault.dat");

        // User-specific fallback location
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
        let user_dir = PathBuf::from(local_app_data).join("SmartCardUnlock");
        let _ = fs::create_dir_all(&user_dir);
        let user_vault = user_dir.join("rust_vault.dat");

        Self {
            system_vault,
            user_vault,
        }
    }

    pub fn is_enrolled(&self) -> bool {
        self.load_pin().is_some()
    }

    pub fn save_pin(&self, pin: &str) -> bool {
        let data = pin.as_bytes();
        let mut in_blob = DATA_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out_blob = DATA_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };

        // Use CRYPTPROTECT_LOCAL_MACHINE so both SYSTEM service and elevated admin can decrypt
        let res = unsafe {
            CryptProtectData(
                &mut in_blob,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_LOCAL_MACHINE,
                &mut out_blob,
            )
        };

        if res == 0 || out_blob.pbData.is_null() {
            return false;
        }

        let slice = unsafe {
            std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize)
        };

        let mut success = false;
        if fs::write(&self.system_vault, slice).is_ok() {
            success = true;
        }
        if fs::write(&self.user_vault, slice).is_ok() {
            success = true;
        }

        unsafe {
            LocalFree(out_blob.pbData as *mut std::ffi::c_void);
        }

        success
    }

    fn decrypt_file(&self, path: &PathBuf) -> Option<String> {
        let encrypted = fs::read(path).ok()?;
        if encrypted.is_empty() {
            return None;
        }
        let mut in_blob = DATA_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_ptr() as *mut u8,
        };
        let mut out_blob = DATA_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };

        let res = unsafe {
            CryptUnprotectData(
                &mut in_blob,
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                &mut out_blob,
            )
        };

        if res == 0 || out_blob.pbData.is_null() {
            return None;
        }

        let slice = unsafe {
            std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize)
        };
        let pin_str = String::from_utf8(slice.to_vec()).ok();

        unsafe {
            LocalFree(out_blob.pbData as *mut std::ffi::c_void);
        }

        pin_str
    }

    pub fn load_pin(&self) -> Option<String> {
        if let Some(pin) = self.decrypt_file(&self.system_vault) {
            return Some(pin);
        }
        self.decrypt_file(&self.user_vault)
    }

    pub fn clear(&self) -> bool {
        let _ = fs::remove_file(&self.system_vault);
        let _ = fs::remove_file(&self.user_vault);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_save_load_clear() {
        let vault = SecurityVault::new();
        assert!(vault.save_pin("654321"));
        assert!(vault.is_enrolled());
        let loaded = vault.load_pin();
        assert_eq!(loaded, Some("654321".to_string()));
        assert!(vault.clear());
        assert!(!vault.is_enrolled());
    }
}
