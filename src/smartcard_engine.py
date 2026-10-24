"""
SmartCard Engine Module
=======================
Handles low-level communication with RTS5169 / PC-SC Smart Card readers
and IC4442 (SLE4442) memory cards on Windows.
"""

import ctypes
from ctypes import wintypes
import threading
import time
import logging
from typing import Callable, Optional, Dict, List, Tuple

logger = logging.getLogger("SmartCardEngine")

# --- PC/SC Constants ---
SCARD_SCOPE_USER = 0
SCARD_SCOPE_TERMINAL = 1
SCARD_SCOPE_SYSTEM = 2

SCARD_SHARE_EXCLUSIVE = 1
SCARD_SHARE_SHARED = 2
SCARD_SHARE_DIRECT = 3

SCARD_PROTOCOL_UNDEFINED = 0
SCARD_PROTOCOL_T0 = 1
SCARD_PROTOCOL_T1 = 2
SCARD_PROTOCOL_RAW = 4

SCARD_LEAVE_CARD = 0
SCARD_RESET_CARD = 1
SCARD_UNPOWER_CARD = 2
SCARD_EJECT_CARD = 3

# Reader State Flags
SCARD_STATE_UNAWARE = 0x00000000
SCARD_STATE_IGNORE = 0x00000001
SCARD_STATE_CHANGED = 0x00000002
SCARD_STATE_UNKNOWN = 0x00000004
SCARD_STATE_UNAVAILABLE = 0x00000008
SCARD_STATE_EMPTY = 0x00000010
SCARD_STATE_PRESENT = 0x00000020
SCARD_STATE_ATRMATCH = 0x00000040
SCARD_STATE_EXCLUSIVE = 0x00000080
SCARD_STATE_INUSE = 0x00000100
SCARD_STATE_MUTE = 0x00000200
SCARD_STATE_UNPOWERED = 0x00000400

# Error Codes
SCARD_S_SUCCESS = 0x00000000
SCARD_E_CANCELLED = 0x80100002
SCARD_E_NO_READERS_AVAILABLE = 0x8010002E
SCARD_W_UNRESPONSIVE_CARD = 0x80100066
SCARD_W_UNPOWERED_CARD = 0x80100067
SCARD_W_RESET_CARD = 0x80100068
SCARD_W_REMOVED_CARD = 0x80100069

# Macro for control code: SCARD_CTL_CODE(code)
def SCARD_CTL_CODE(code: int) -> int:
    return (0x31 << 16) | (code << 2)

# Structures
class SCARD_READERSTATEW(ctypes.Structure):
    _fields_ = [
        ("szReader", wintypes.LPCWSTR),
        ("pvUserData", wintypes.LPVOID),
        ("dwCurrentState", wintypes.DWORD),
        ("dwEventState", wintypes.DWORD),
        ("cbAtr", wintypes.DWORD),
        ("rgbAtr", ctypes.c_byte * 36),
    ]

class SCARD_IO_REQUEST(ctypes.Structure):
    _fields_ = [
        ("dwProtocol", wintypes.DWORD),
        ("cbPciLength", wintypes.DWORD),
    ]


class SmartCardEngine:
    """
    Core Smart Card Engine for RTS5169 reader and IC4442 smartcard.
    Provides background card insertion/removal monitoring and card communication.
    """

    def __init__(self, target_reader_keyword: str = "EMV"):
        self.target_keyword = target_reader_keyword.lower()
        self.winscard = ctypes.windll.WinSCard
        self.h_context = wintypes.HANDLE()
        self._is_monitoring = False
        self._monitor_thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()

        # Callbacks
        self.on_card_inserted: Optional[Callable[[str, bytes, dict], None]] = None
        self.on_card_removed: Optional[Callable[[str], None]] = None
        self.on_reader_status_changed: Optional[Callable[[bool, str], None]] = None

        # State tracking
        self.current_reader: Optional[str] = None
        self.is_card_present: bool = False
        self.card_atr: bytes = b""
        self.last_event_counter: int = 0

        self._init_context()

    def _init_context(self) -> bool:
        """Establish PC/SC context."""
        try:
            res = self.winscard.SCardEstablishContext(
                SCARD_SCOPE_USER, None, None, ctypes.byref(self.h_context)
            )
            if res != SCARD_S_SUCCESS:
                logger.error(f"SCardEstablishContext failed: 0x{res & 0xFFFFFFFF:08X}")
                return False
            return True
        except Exception as e:
            logger.exception(f"Exception establishing SCARD context: {e}")
            return False

    def list_readers(self) -> List[str]:
        """Enumerate connected smart card readers."""
        if not self.h_context:
            if not self._init_context():
                return []

        cch = wintypes.DWORD()
        res = self.winscard.SCardListReadersW(self.h_context, None, None, ctypes.byref(cch))
        if res != SCARD_S_SUCCESS:
            return []

        buf = ctypes.create_unicode_buffer(cch.value)
        res = self.winscard.SCardListReadersW(self.h_context, None, buf, ctypes.byref(cch))
        if res != SCARD_S_SUCCESS:
            return []

        raw = buf[:cch.value]
        readers = [r for r in raw.split("\0") if r]
        return readers

    def find_target_reader(self) -> Optional[str]:
        """Find RTS5169 / EMV reader."""
        readers = self.list_readers()
        if not readers:
            return None

        # Look for target keyword or RTS / Realtek / EMV
        for r in readers:
            r_lower = r.lower()
            if self.target_keyword in r_lower or "rts" in r_lower or "realtek" in r_lower or "generic" in r_lower:
                return r

        # Fallback to the first reader if available
        return readers[0]

    def get_reader_state(self, reader_name: str) -> Tuple[bool, bytes, int]:
        """
        Query current card presence, ATR, and raw state flags for a reader.
        Returns: (is_present, atr_bytes, event_state)
        """
        rs = SCARD_READERSTATEW()
        rs.szReader = reader_name
        rs.dwCurrentState = SCARD_STATE_UNAWARE

        res = self.winscard.SCardGetStatusChangeW(self.h_context, 200, ctypes.byref(rs), 1)
        if res != SCARD_S_SUCCESS:
            return False, b"", 0

        event_state = rs.dwEventState
        is_present = bool(event_state & SCARD_STATE_PRESENT)
        atr = bytes(rs.rgbAtr[:rs.cbAtr]) if rs.cbAtr > 0 else b""
        return is_present, atr, event_state

    def probe_card_details(self, reader_name: str) -> dict:
        """
        Attempt to connect and inspect card specifics (ATR, type, memory if readable).
        """
        details = {
            "reader": reader_name,
            "connected": False,
            "atr": "",
            "card_type": "IC4442 / Synchronous Memory Card",
            "is_mute": True,
            "direct_mode_ok": False,
        }

        # Try DIRECT connect
        h_card = wintypes.HANDLE()
        active_proto = wintypes.DWORD()
        res_dir = self.winscard.SCardConnectW(
            self.h_context,
            reader_name,
            SCARD_SHARE_DIRECT,
            0,
            ctypes.byref(h_card),
            ctypes.byref(active_proto),
        )
        if res_dir == SCARD_S_SUCCESS:
            details["direct_mode_ok"] = True
            # Query presence attribute
            buf = (ctypes.c_byte * 256)()
            buflen = wintypes.DWORD(256)
            SCARD_ATTR_ICC_PRESENCE = 0x00090300
            if self.winscard.SCardGetAttrib(h_card, SCARD_ATTR_ICC_PRESENCE, buf, ctypes.byref(buflen)) == 0:
                details["icc_presence"] = int(buf[0])
            self.winscard.SCardDisconnect(h_card, SCARD_LEAVE_CARD)

        # Check status change
        is_pres, atr, state = self.get_reader_state(reader_name)
        details["is_present"] = is_pres
        if atr:
            details["atr"] = atr.hex(" ").upper()
            details["is_mute"] = False
        else:
            details["is_mute"] = bool(state & SCARD_STATE_MUTE)

        return details

    def start_monitoring(self, poll_interval: float = 0.5) -> None:
        """Start background card insertion and removal detection."""
        if self._is_monitoring:
            return

        self._is_monitoring = True
        self._stop_event.clear()
        self._monitor_thread = threading.Thread(
            target=self._monitor_loop, args=(poll_interval,), daemon=True
        )
        self._monitor_thread.start()
        logger.info("SmartCard monitoring started.")

    def stop_monitoring(self) -> None:
        """Stop background monitoring."""
        self._is_monitoring = False
        self._stop_event.set()
        if self._monitor_thread and self._monitor_thread.is_alive():
            self._monitor_thread.join(timeout=2.0)
        logger.info("SmartCard monitoring stopped.")

    def _monitor_loop(self, poll_interval: float) -> None:
        """Background thread monitoring card insertion & removal."""
        last_reader = None
        last_present = None

        while not self._stop_event.is_set():
            try:
                reader = self.find_target_reader()
                if not reader:
                    if last_reader is not None:
                        logger.warning("Smart card reader disconnected.")
                        self.current_reader = None
                        if self.on_reader_status_changed:
                            self.on_reader_status_changed(False, "")
                        last_reader = None
                    time.sleep(1.0)
                    continue

                if reader != last_reader:
                    logger.info(f"Connected to smart card reader: {reader}")
                    self.current_reader = reader
                    last_reader = reader
                    if self.on_reader_status_changed:
                        self.on_reader_status_changed(True, reader)

                # Check card presence
                is_present, atr, event_state = self.get_reader_state(reader)
                event_counter = (event_state >> 16) & 0xFFFF

                if last_present is None:
                    # Initial state
                    last_present = is_present
                    self.is_card_present = is_present
                    self.card_atr = atr
                    self.last_event_counter = event_counter
                    if is_present:
                        card_info = self.probe_card_details(reader)
                        if self.on_card_inserted:
                            self.on_card_inserted(reader, atr, card_info)
                elif is_present != last_present:
                    last_present = is_present
                    self.is_card_present = is_present
                    self.card_atr = atr
                    self.last_event_counter = event_counter

                    if is_present:
                        logger.info(f"Card inserted into {reader}!")
                        card_info = self.probe_card_details(reader)
                        if self.on_card_inserted:
                            self.on_card_inserted(reader, atr, card_info)
                    else:
                        logger.info(f"Card removed from {reader}!")
                        if self.on_card_removed:
                            self.on_card_removed(reader)

            except Exception as e:
                logger.debug(f"Error in card monitor loop: {e}")

            time.sleep(poll_interval)

    def close(self) -> None:
        """Release WinSCard context and cleanup."""
        self.stop_monitoring()
        if self.h_context:
            self.winscard.SCardReleaseContext(self.h_context)
            self.h_context = wintypes.HANDLE()
