"""
System Tray Icon Module
=======================
Provides background system tray integration and status notifications.
"""

from PyQt5.QtWidgets import QSystemTrayIcon, QMenu, QAction, QApplication, QStyle
from PyQt5.QtGui import QIcon, QPixmap, QPainter, QColor, QFont
from PyQt5.QtCore import pyqtSignal, QObject


def create_smartcard_pixmap(is_active: bool = True, card_present: bool = False) -> QPixmap:
    """Dynamically generate a smartcard tray icon."""
    pix = QPixmap(32, 32)
    pix.fill(QColor(0, 0, 0, 0))

    painter = QPainter(pix)
    painter.setRenderHint(QPainter.Antialiasing)

    # Card body
    card_color = QColor("#2b6cb0") if is_active else QColor("#4a5568")
    painter.setBrush(card_color)
    painter.setPen(QColor("#1a365d"))
    painter.drawRoundedRect(3, 7, 26, 18, 3, 3)

    # Chip gold contacts
    painter.setBrush(QColor("#ecc94b"))
    painter.setPen(QColor("#b7791f"))
    painter.drawRect(7, 12, 7, 8)

    # Status indicator dot
    dot_color = QColor("#48bb78") if card_present else (QColor("#ecc94b") if is_active else QColor("#f56565"))
    painter.setBrush(dot_color)
    painter.setPen(QColor("#ffffff"))
    painter.drawEllipse(21, 16, 8, 8)

    painter.end()
    return pix


class SmartCardTrayIcon(QSystemTrayIcon):
    """System Tray Icon for Smart Card Unlock Tool."""

    show_window_requested = pyqtSignal()
    lock_requested = pyqtSignal()
    toggle_guard_requested = pyqtSignal(bool)
    exit_requested = pyqtSignal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.is_monitoring = True
        self.is_card_present = False

        self._update_icon()
        self._init_menu()
        self.activated.connect(self._on_activated)

    def _update_icon(self):
        pix = create_smartcard_pixmap(self.is_monitoring, self.is_card_present)
        self.setIcon(QIcon(pix))
        status = "卡片已插入" if self.is_card_present else "未插卡"
        self.setToolTip(f"Windows 插卡解锁工具 - 守护中 ({status})")

    def _init_menu(self):
        menu = QMenu()

        self.action_show = QAction("打开主面板", self)
        self.action_show.triggered.connect(self.show_window_requested.emit)
        menu.addAction(self.action_show)

        menu.addSeparator()

        self.action_lock = QAction("立即锁定系统", self)
        self.action_lock.triggered.connect(self.lock_requested.emit)
        menu.addAction(self.action_lock)

        self.action_toggle = QAction("暂停守护", self)
        self.action_toggle.triggered.connect(self._handle_toggle)
        menu.addAction(self.action_toggle)

        menu.addSeparator()

        action_exit = QAction("退出程序", self)
        action_exit.triggered.connect(self.exit_requested.emit)
        menu.addAction(action_exit)

        self.setContextMenu(menu)

    def _on_activated(self, reason):
        if reason in (QSystemTrayIcon.Trigger, QSystemTrayIcon.DoubleClick):
            self.show_window_requested.emit()

    def _handle_toggle(self):
        self.is_monitoring = not self.is_monitoring
        self.action_toggle.setText("恢复守护" if not self.is_monitoring else "暂停守护")
        self.toggle_guard_requested.emit(self.is_monitoring)
        self._update_icon()

    def update_card_status(self, present: bool):
        """Update card presence indicator."""
        self.is_card_present = present
        self._update_icon()

    def notify(self, title: str, message: str, is_info: bool = True):
        """Show tray balloon notification."""
        icon = QSystemTrayIcon.Information if is_info else QSystemTrayIcon.Warning
        self.showMessage(title, message, icon, 3000)
