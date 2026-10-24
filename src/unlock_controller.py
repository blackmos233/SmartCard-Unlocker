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


def is_workstation_locked() -> bool:
    """
    Check if Windows workstation is currently locked.
    Uses OpenInputDesktop which fails with ERROR_ACCESS_DENIED when locked.
    """
    h_desk = user32.OpenInputDesktop(0, False, DESKTOP_SWITCHDESKTOP)
    if not h_desk:
        # If open input desktop fails, the secure desktop / lock screen is active
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
        # Space press & release
        user32.keybd_event(VK_SPACE, 0, 0, 0)
        time.sleep(0.05)
        user32.keybd_event(VK_SPACE, 0, KEYEVENTF_KEYUP, 0)
        time.sleep(0.1)
    except Exception as e:
        logger.debug(f"Error dismissing lock curtain: {e}")


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
        self.on_screen_guard_request: Optional[Callable[[bool], None]] = None

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
        logger.info(f"Card authentication SUCCESS for user '{username}'. Initiating unlock...")

        # Request UI screen guard to dismiss
        if self.on_screen_guard_request:
            self.on_screen_guard_request(False)

        # If Windows native lock screen is active, dismiss curtain
        if is_workstation_locked():
            dismiss_lockscreen_curtain()

        if self.on_unlock_succeeded:
            self.on_unlock_succeeded(username)

    def _handle_card_removed(self, reader_name: str) -> None:
        """Invoked when smart card is removed."""
        logger.info(f"UnlockController: Card removed from {reader_name}")

        if not self.lock_on_removal:
            logger.info("Auto-lock on removal is disabled in settings.")
            return

        # Trigger Screen Guard overlay if available
        if self.on_screen_guard_request:
            self.on_screen_guard_request(True)

        # Also trigger native LockWorkStation
        lock_workstation()

    def manual_lock(self) -> None:
        """Trigger workstation lock manually."""
        if self.on_screen_guard_request:
            self.on_screen_guard_request(True)
        lock_workstation()

    def manual_unlock_with_pin(self, pin: str, card_token: str = "IC4442_DEFAULT_TOKEN") -> bool:
        """Emergency unlock with user PIN in case card is unavailable."""
        creds = self.vault.unlock_verify(pin=pin, card_token=card_token)
        if creds:
            if self.on_screen_guard_request:
                self.on_screen_guard_request(False)
            if self.on_unlock_succeeded:
                self.on_unlock_succeeded(creds[0])
            return True
        return False
