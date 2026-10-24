use std::ffi::c_void;
use std::ptr;

pub const SCARD_SCOPE_USER: u32 = 0;
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
        let mut h_context = 0usize;
        let res = unsafe {
            SCardEstablishContext(SCARD_SCOPE_USER, ptr::null(), ptr::null(), &mut h_context)
        };
        if res == SCARD_S_SUCCESS {
            Ok(Self { h_context })
        } else {
            Err(res)
        }
    }

    pub fn list_readers(&self) -> Vec<String> {
        let mut cch: u32 = 0;
        let res = unsafe {
            SCardListReadersW(self.h_context, ptr::null(), ptr::null_mut(), &mut cch)
        };
        if res != SCARD_S_SUCCESS || cch == 0 {
            return Vec::new();
        }

        let mut buf = vec![0u16; cch as usize];
        let res = unsafe {
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

    pub fn get_card_status(&self, reader: &str) -> (bool, u32) {
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
            SCardGetStatusChangeW(self.h_context, 200, &mut rs, 1)
        };

        if res == SCARD_S_SUCCESS {
            let present = (rs.dwEventState & SCARD_STATE_PRESENT) != 0;
            (present, rs.dwEventState)
        } else {
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
