"""
Windows SmartCard Unlock Tool - Main Entry Point
================================================
Hardware Target: Realtek RTS5169 (Generic EMV) Smartcard Reader & IC4442 Card
Device Location: Port_#0004.Hub_#0002
"""

import sys
import os
import ctypes
from ctypes import wintypes
import logging

from PyQt5.QtWidgets import QApplication
from PyQt5.QtCore import Qt

from src.smartcard_engine import SmartCardEngine
from src.security_vault import SecurityVault
from src.unlock_controller import UnlockController
from src.gui_app import MainWindow

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] [%(name)s] %(message)s",
    handlers=[
        logging.StreamHandler(sys.stdout),
    ],
)
logger = logging.getLogger("SmartCardApp")

# Windows Single-Instance Mutex
MUTEX_NAME = "Global\\SmartCardUnlockTool_SingleInstanceMutex"


def acquire_single_instance_mutex():
    """Ensure only one instance runs simultaneously."""
    kernel32 = ctypes.windll.kernel32
    h_mutex = kernel32.CreateMutexW(None, False, MUTEX_NAME)
    ERROR_ALREADY_EXISTS = 183
    if kernel32.GetLastError() == ERROR_ALREADY_EXISTS:
        return None
    return h_mutex


def main():
    # Enable high-DPI scaling
    QApplication.setAttribute(Qt.AA_EnableHighDpiScaling, True)
    QApplication.setAttribute(Qt.AA_UseHighDpiPixmaps, True)

    app = QApplication(sys.argv)
    app.setQuitOnLastWindowClosed(False)

    # Check single instance
    h_mutex = acquire_single_instance_mutex()
    if not h_mutex:
        logger.warning("Another instance of SmartCard Unlock Tool is already running.")
        sys.exit(0)

    logger.info("Initializing SmartCard Engine (Target: Generic EMV / RTS5169)...")
    engine = SmartCardEngine(target_reader_keyword="EMV")

    logger.info("Initializing Cryptographic Security Vault...")
    vault = SecurityVault()

    logger.info("Initializing Windows Unlock Controller...")
    controller = UnlockController(
        engine=engine,
        vault=vault,
        lock_on_removal=True,
        unlock_on_insertion=True,
    )

    logger.info("Starting background card monitoring...")
    engine.start_monitoring(poll_interval=0.5)

    window = MainWindow(engine=engine, vault=vault, controller=controller)

    # Check if started with --silent (e.g. at Windows startup)
    if "--silent" in sys.argv:
        logger.info("Started in silent mode (minimized to system tray).")
        window.hide()
    else:
        window.show()

    exit_code = app.exec_()

    # Clean up
    engine.stop_monitoring()
    engine.close()
    if h_mutex:
        ctypes.windll.kernel32.CloseHandle(h_mutex)

    sys.exit(exit_code)


if __name__ == "__main__":
    main()
