use std::ffi::c_void;
use std::ptr;

pub const SCARD_SCOPE_USER: u32 = 0;
pub const SCARD_SCOPE_SYSTEM: u32 = 2;
pub const SCARD_S_SUCCESS: i32 = 0;
pub const SCARD_STATE_UNAWARE: u32 = 0x00000000;
#[allow(dead_code)]
pub const SCARD_STATE_EMPTY: u32 = 0x00000010;
pub const SCARD_STATE_PRESENT: u32 = 0x00000020;
#[allow(dead_code)]
pub const SCARD_STATE_MUTE: u32 = 0x00000200;

#[repr(C)]
#[allow(non_snake_case)]
pub struct SCARD_READERSTATEW {
    pub szReader: *const u16,
    pub pvUserData: *mut c_void,
    pub dwCurrentState: u32,
    pub dwEventState: u32,
    pub cbAtr: u32,
    pub rgbAtr: [u8; 36],
}

#[link(name = "winscard")]
unsafe extern "system" {
    fn SCardEstablishContext(
        dwScope: u32,
        pvReserved1: *const c_void,
        pvReserved2: *const c_void,
        phContext: *mut usize,
    ) -> i32;

    fn SCardReleaseContext(hContext: usize) -> i32;

    fn SCardIsValidContext(hContext: usize) -> i32;

    fn SCardListReadersW(
        hContext: usize,
        mszGroups: *const u16,
        mszReaders: *mut u16,
        pcchReaders: *mut u32,
    ) -> i32;

    fn SCardGetStatusChangeW(
        hContext: usize,
        dwTimeout: u32,
        rgReaderStates: *mut SCARD_READERSTATEW,
        cReaders: u32,
    ) -> i32;
}

pub struct SmartCardEngine {
    h_context: usize,
}

impl SmartCardEngine {
    pub fn new() -> Result<Self, i32> {
        let mut engine = Self { h_context: 0 };
        engine.refresh_context();
        Ok(engine)
    }

    pub fn refresh_context(&mut self) -> bool {
        if self.h_context != 0 {
            unsafe {
                SCardReleaseContext(self.h_context);
            }
            self.h_context = 0;
        }
        let mut h = 0usize;
        let mut res = unsafe {
            SCardEstablishContext(SCARD_SCOPE_SYSTEM, ptr::null(), ptr::null(), &mut h)
        };
        if res != SCARD_S_SUCCESS {
            res = unsafe {
                SCardEstablishContext(SCARD_SCOPE_USER, ptr::null(), ptr::null(), &mut h)
            };
        }
        if res == SCARD_S_SUCCESS && h != 0 {
            self.h_context = h;
            true
        } else {
            false
        }
    }

    pub fn is_context_valid(&self) -> bool {
        if self.h_context == 0 {
            return false;
        }
        unsafe { SCardIsValidContext(self.h_context) == SCARD_S_SUCCESS }
    }

    pub fn list_readers(&mut self) -> Vec<String> {
        if !self.is_context_valid() {
            self.refresh_context();
        }

        if self.h_context == 0 {
            return Vec::new();
        }

        let mut cch: u32 = 0;
        let mut res = unsafe {
            SCardListReadersW(self.h_context, ptr::null(), ptr::null_mut(), &mut cch)
        };

        // If list failed or no readers available, refresh context and retry to capture hotplug!
        if res != SCARD_S_SUCCESS || cch == 0 {
            self.refresh_context();
            if self.h_context == 0 {
                return Vec::new();
            }
            cch = 0;
            res = unsafe {
                SCardListReadersW(self.h_context, ptr::null(), ptr::null_mut(), &mut cch)
            };
            if res != SCARD_S_SUCCESS || cch == 0 {
                return Vec::new();
            }
        }

        let mut buf = vec![0u16; cch as usize];
        res = unsafe {
            SCardListReadersW(self.h_context, ptr::null(), buf.as_mut_ptr(), &mut cch)
        };
        if res != SCARD_S_SUCCESS {
            return Vec::new();
        }

        // Parse null-separated wide strings
        let mut readers = Vec::new();
        let mut start = 0;
        for i in 0..buf.len() {
            if buf[i] == 0 {
                if i > start {
                    let s = String::from_utf16_lossy(&buf[start..i]);
                    if !s.is_empty() {
                        readers.push(s);
                    }
                }
                start = i + 1;
            }
        }
        readers
    }

    pub fn get_card_status(&mut self, reader: &str) -> (bool, u32) {
        if !self.is_context_valid() {
            self.refresh_context();
        }

        if self.h_context == 0 {
            return (false, 0);
        }

        let wide: Vec<u16> = reader.encode_utf16().chain(std::iter::once(0)).collect();
        let mut rs = SCARD_READERSTATEW {
            szReader: wide.as_ptr(),
            pvUserData: ptr::null_mut(),
            dwCurrentState: SCARD_STATE_UNAWARE,
            dwEventState: 0,
            cbAtr: 0,
            rgbAtr: [0; 36],
        };

        let res = unsafe {
            SCardGetStatusChangeW(self.h_context, 100, &mut rs, 1)
        };

        if res == SCARD_S_SUCCESS {
            let present = (rs.dwEventState & SCARD_STATE_PRESENT) != 0;
            (present, rs.dwEventState)
        } else {
            // Status query failed (e.g. reader physically removed) -> refresh context
            self.refresh_context();
            (false, 0)
        }
    }
}

impl Drop for SmartCardEngine {
    fn drop(&mut self) {
        if self.h_context != 0 {
            unsafe {
                SCardReleaseContext(self.h_context);
            }
        }
    }
}
