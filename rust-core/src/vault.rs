use std::fs;
use std::path::PathBuf;
use std::ptr;

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
    vault_file: PathBuf,
}

impl SecurityVault {
    pub fn new() -> Self {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(local_app_data).join("SmartCardUnlock");
        let _ = fs::create_dir_all(&dir);
        let vault_file = dir.join("rust_vault.dat");
        Self { vault_file }
    }

    pub fn is_enrolled(&self) -> bool {
        self.vault_file.exists() && fs::metadata(&self.vault_file).map(|m| m.len() > 0).unwrap_or(false)
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

        let res = unsafe {
            CryptProtectData(
                &mut in_blob,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                &mut out_blob,
            )
        };

        if res == 0 || out_blob.pbData.is_null() {
            return false;
        }

        let slice = unsafe {
            std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize)
        };

        let write_ok = fs::write(&self.vault_file, slice).is_ok();

        unsafe {
            LocalFree(out_blob.pbData as *mut std::ffi::c_void);
        }

        write_ok
    }

    pub fn load_pin(&self) -> Option<String> {
        let encrypted = fs::read(&self.vault_file).ok()?;
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

    pub fn clear(&self) -> bool {
        if self.vault_file.exists() {
            fs::remove_file(&self.vault_file).is_ok()
        } else {
            true
        }
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
