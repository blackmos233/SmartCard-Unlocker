"""
Windows Unlock Controller Module
================================
Manages workstation lock state, auto-locking on card removal,
and auto-unlocking on card insertion.
"""

import ctypes
from ctypes import wintypes
import time
import logging
import threading
from typing import Optional, Callable

from .security_vault import SecurityVault
from .smartcard_engine import SmartCardEngine

logger = logging.getLogger("UnlockController")

user32 = ctypes.windll.user32
wtsapi32 = ctypes.windll.wtsapi32

# Win32 Constants
DESKTOP_SWITCHDESKTOP = 0x0100
VK_SPACE = 0x20
VK_RETURN = 0x0D
KEYEVENTF_KEYUP = 0x0002
KEYEVENTF_UNICODE = 0x0004


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [
        ("wVk", wintypes.WORD),
        ("wScan", wintypes.WORD),
        ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.c_size_t),
    ]


class INPUT(ctypes.Structure):
    class _INPUT_UNION(ctypes.Union):
        _fields_ = [("ki", KEYBDINPUT)]

    _anonymous_ = ("_u",)
    _fields_ = [("type", wintypes.DWORD), ("_u", _INPUT_UNION)]


def is_workstation_locked() -> bool:
    """
    Check if Windows workstation is currently locked.
    Uses OpenInputDesktop which fails with ERROR_ACCESS_DENIED when locked.
    """
    h_desk = user32.OpenInputDesktop(0, False, DESKTOP_SWITCHDESKTOP)
    if not h_desk:
        return True
    user32.CloseDesktop(h_desk)
    return False


def lock_workstation() -> bool:
    """Lock the Windows workstation using Win32 API."""
    try:
        res = user32.LockWorkStation()
        logger.info(f"LockWorkStation called, result: {res}")
        return bool(res)
    except Exception as e:
        logger.exception(f"Error locking workstation: {e}")
        return False


def dismiss_lockscreen_curtain() -> None:
    """
    Send synthetic keypresses to wake monitor and dismiss Windows 10/11 lock screen slide curtain.
    """
    try:
        user32.keybd_event(VK_SPACE, 0, 0, 0)
        time.sleep(0.05)
        user32.keybd_event(VK_SPACE, 0, KEYEVENTF_KEYUP, 0)
        time.sleep(0.1)
        user32.keybd_event(VK_RETURN, 0, 0, 0)
        time.sleep(0.05)
        user32.keybd_event(VK_RETURN, 0, KEYEVENTF_KEYUP, 0)
    except Exception as e:
        logger.debug(f"Error dismissing lock curtain: {e}")


def inject_pin_keystrokes(pin: str) -> None:
    """
    Send Windows Hello PIN digits directly to the lock screen PIN input box.
    """
    dismiss_lockscreen_curtain()
    time.sleep(0.35)

    for ch in pin:
        utf16_val = ord(ch)
        down = INPUT(type=1)
        down.ki = KEYBDINPUT(wVk=0, wScan=utf16_val, dwFlags=KEYEVENTF_UNICODE, time=0, dwExtraInfo=0)
        up = INPUT(type=1)
        up.ki = KEYBDINPUT(wVk=0, wScan=utf16_val, dwFlags=KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, time=0, dwExtraInfo=0)

        user32.SendInput(1, ctypes.byref(down), ctypes.sizeof(INPUT))
        time.sleep(0.02)
        user32.SendInput(1, ctypes.byref(up), ctypes.sizeof(INPUT))
        time.sleep(0.04)

    time.sleep(0.15)
    user32.keybd_event(VK_RETURN, 0, 0, 0)
    time.sleep(0.05)
    user32.keybd_event(VK_RETURN, 0, KEYEVENTF_KEYUP, 0)


class UnlockController:
    """
    Coordinates SmartCard events with Windows lock/unlock actions and Security Vault.
    """

    def __init__(
        self,
        engine: SmartCardEngine,
        vault: SecurityVault,
        lock_on_removal: bool = True,
        unlock_on_insertion: bool = True,
    ):
        self.engine = engine
        self.vault = vault
        self.lock_on_removal = lock_on_removal
        self.unlock_on_insertion = unlock_on_insertion

        # Rate-limiting / anti brute-force
        self.failed_attempts = 0
        self.max_failed_attempts = 5
        self.lockout_until = 0.0

        # State & callbacks
        self.on_unlock_succeeded: Optional[Callable[[str], None]] = None
        self.on_unlock_failed: Optional[Callable[[str], None]] = None

        # Wire smartcard callbacks
        self.engine.on_card_inserted = self._handle_card_inserted
        self.engine.on_card_removed = self._handle_card_removed

    def _handle_card_inserted(self, reader_name: str, atr: bytes, card_info: dict) -> None:
        """Invoked when a smart card is inserted."""
        logger.info(f"UnlockController: Card inserted in {reader_name}")

        if not self.unlock_on_insertion:
            logger.info("Unlock on insertion is disabled in settings.")
            return

        # Check anti-brute-force lockout
        now = time.time()
        if now < self.lockout_until:
            rem = int(self.lockout_until - now)
            msg = f"Card unlock is temporarily locked out due to previous failed attempts. Please wait {rem}s."
            logger.warning(msg)
            if self.on_unlock_failed:
                self.on_unlock_failed(msg)
            return

        if not self.vault.is_enrolled():
            msg = "Card inserted, but no credentials enrolled in vault."
            logger.info(msg)
            if self.on_unlock_failed:
                self.on_unlock_failed(msg)
            return

        # Card token derivation:
        # Use card ATR or hardware instance identifier as primary token
        token_id = atr.hex().upper() if atr else "IC4442_DEFAULT_TOKEN"
        card_pin = "123456" # Default enrolled pin or user configured

        creds = self.vault.unlock_verify(
            pin=card_pin,
            card_token=token_id,
            current_reader=reader_name,
        )

        if not creds:
            # Fallback to default token if ATR was mute
            creds = self.vault.unlock_verify(
                pin=card_pin,
                card_token="IC4442_DEFAULT_TOKEN",
                current_reader=reader_name,
            )

        if not creds:
            self.failed_attempts += 1
            if self.failed_attempts >= self.max_failed_attempts:
                self.lockout_until = time.time() + 60.0
                logger.error("Too many failed card attempts! Locking out for 60s.")
            msg = "Card authentication failed: invalid card or credentials."
            logger.warning(msg)
            if self.on_unlock_failed:
                self.on_unlock_failed(msg)
            return

        # Authentication succeeded
        self.failed_attempts = 0
        username, password, domain = creds
        logger.info(f"Card authentication SUCCESS for user '{username}'.")

        # Inject PIN into Windows lock screen if locked
        if is_workstation_locked():
            logger.info("Workstation is locked. Injecting Windows Hello PIN to unlock...")
            # Use enrolled pin or password
            pin_to_inject = password if password else card_pin
            inject_pin_keystrokes(pin_to_inject)

        if self.on_unlock_succeeded:
            self.on_unlock_succeeded(username)

    def _handle_card_removed(self, reader_name: str) -> None:
        """Invoked when smart card is removed."""
        logger.info(f"UnlockController: Card removed from {reader_name}")

        if not self.lock_on_removal:
            logger.info("Auto-lock on removal is disabled in settings.")
            return

        # Trigger native Windows LockWorkStation immediately
        lock_workstation()

    def manual_lock(self) -> None:
        """Trigger workstation lock manually."""
        lock_workstation()

    def manual_unlock_with_pin(self, pin: str, card_token: str = "IC4442_DEFAULT_TOKEN") -> bool:
        """Emergency unlock with user PIN in case card is unavailable."""
        creds = self.vault.unlock_verify(pin=pin, card_token=card_token)
        if creds:
            if self.on_unlock_succeeded:
                self.on_unlock_succeeded(creds[0])
            return True
        return False
