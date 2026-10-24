use std::thread;
use std::time::Duration;

const DESKTOP_SWITCHDESKTOP: u32 = 0x0100;
const VK_SPACE: u8 = 0x20;
const VK_RETURN: u8 = 0x0D;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const KEYEVENTF_UNICODE: u32 = 0x0004;

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
    padding: [u8; 8], // align to largest union member (MOUSEINPUT/HARDWAREINPUT)
}

#[link(name = "user32")]
unsafe extern "system" {
    fn LockWorkStation() -> i32;
    fn OpenInputDesktop(dwFlags: u32, fInherit: i32, dwDesiredAccess: u32) -> usize;
    fn CloseDesktop(hDesktop: usize) -> i32;
    fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
    fn SendInput(cInputs: u32, pInputs: *const INPUT, cbSize: i32) -> u32;
}

pub fn is_workstation_locked() -> bool {
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

pub fn lock_workstation() -> bool {
    unsafe { LockWorkStation() != 0 }
}

pub fn dismiss_lock_curtain() {
    unsafe {
        // Press Space to slide up the lock screen wallpaper curtain
        keybd_event(VK_SPACE, 0, 0, 0);
        thread::sleep(Duration::from_millis(50));
        keybd_event(VK_SPACE, 0, KEYEVENTF_KEYUP, 0);
        thread::sleep(Duration::from_millis(100));

        // Also send Return in case it requires Enter
        keybd_event(VK_RETURN, 0, 0, 0);
        thread::sleep(Duration::from_millis(50));
        keybd_event(VK_RETURN, 0, KEYEVENTF_KEYUP, 0);
    }
}

pub fn inject_pin_keystrokes(pin: &str) {
    // 1. Dismiss lock curtain
    dismiss_lock_curtain();
    thread::sleep(Duration::from_millis(400));

    // 2. Type each digit of the Windows Hello PIN
    for ch in pin.chars() {
        let utf16 = ch as u16;
        let mut input_down = INPUT {
            type_: 1, // INPUT_KEYBOARD
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: utf16,
                dwFlags: KEYEVENTF_UNICODE,
                time: 0,
                dwExtraInfo: 0,
            },
            padding: [0; 8],
        };

        let mut input_up = INPUT {
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
            SendInput(1, &mut input_down, std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(25));
            SendInput(1, &mut input_up, std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(40));
        }
    }

    // 3. In case the PIN requires Enter (alphanumeric or custom policy)
    thread::sleep(Duration::from_millis(150));
    unsafe {
        keybd_event(VK_RETURN, 0, 0, 0);
        thread::sleep(Duration::from_millis(50));
        keybd_event(VK_RETURN, 0, KEYEVENTF_KEYUP, 0);
    }
}
