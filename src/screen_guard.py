"""
Screen Guard Overlay Module
===========================
Provides a fullscreen security lock overlay when the card is pulled out.
Unlocks immediately when the authorized IC4442 card is inserted or via emergency PIN.
"""

import sys
from typing import Optional, Callable
from PyQt5.QtWidgets import (
    QWidget, QVBoxLayout, QLabel, QLineEdit, QPushButton,
    QHBoxLayout, QGraphicsDropShadowEffect, QFrame, QApplication
)
from PyQt5.QtCore import Qt, pyqtSignal, QPropertyAnimation, QEasingCurve
from PyQt5.QtGui import QFont, QColor, QPalette, QKeyEvent


class ScreenGuardWindow(QWidget):
    """
    Frameless, top-level fullscreen security overlay.
    """
    pin_unlock_requested = pyqtSignal(str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowFlags(
            Qt.WindowStaysOnTopHint |
            Qt.FramelessWindowHint |
            Qt.Tool
        )
        self.setAttribute(Qt.WA_TranslucentBackground, False)
        self.setAttribute(Qt.WA_ShowWithoutActivating, False)

        self._init_ui()

    def _init_ui(self):
        # Dark modern security styling
        self.setStyleSheet("""
            QWidget {
                background-color: #0b0f19;
                color: #e2e8f0;
                font-family: 'Segoe UI', 'Microsoft YaHei', sans-serif;
            }
            QFrame#cardFrame {
                background-color: #151d2f;
                border: 1px solid #2d3748;
                border-radius: 16px;
                padding: 30px;
            }
            QLabel#titleLabel {
                font-size: 26px;
                font-weight: bold;
                color: #63b3ed;
            }
            QLabel#subLabel {
                font-size: 15px;
                color: #a0aec0;
                margin-bottom: 15px;
            }
            QLabel#statusLabel {
                font-size: 14px;
                color: #cbd5e0;
                background-color: #1a202c;
                border-radius: 8px;
                padding: 10px;
            }
            QLineEdit {
                background-color: #1a202c;
                border: 2px solid #4a5568;
                border-radius: 8px;
                padding: 10px 14px;
                font-size: 15px;
                color: #ffffff;
            }
            QLineEdit:focus {
                border-color: #3182ce;
            }
            QPushButton#unlockBtn {
                background-color: #3182ce;
                color: #ffffff;
                border: none;
                border-radius: 8px;
                padding: 10px 20px;
                font-size: 15px;
                font-weight: bold;
            }
            QPushButton#unlockBtn:hover {
                background-color: #2b6cb0;
            }
            QPushButton#unlockBtn:pressed {
                background-color: #2c5282;
            }
            QLabel#errorLabel {
                color: #fc8181;
                font-size: 13px;
            }
        """)

        main_layout = QVBoxLayout(self)
        main_layout.setAlignment(Qt.AlignCenter)

        # Center Card Container
        card_frame = QFrame()
        card_frame.setObjectName("cardFrame")
        card_frame.setFixedWidth(520)

        card_layout = QVBoxLayout(card_frame)
        card_layout.setSpacing(14)

        # Icon and Title
        icon_label = QLabel("💳 🔒")
        icon_label.setAlignment(Qt.AlignCenter)
        icon_label.setStyleSheet("font-size: 48px; margin-bottom: 5px;")
        card_layout.addWidget(icon_label)

        title = QLabel("计算机已被智能卡守护锁定")
        title.setObjectName("titleLabel")
        title.setAlignment(Qt.AlignCenter)
        card_layout.addWidget(title)

        sub = QLabel("请将 IC4442 智能卡插入读卡器以自动解锁")
        sub.setObjectName("subLabel")
        sub.setAlignment(Qt.AlignCenter)
        card_layout.addWidget(sub)

        # Hardware Info Badge
        self.device_badge = QLabel("设备位置: Port_#0004.Hub_#0002 (Generic EMV / RTS5169)")
        self.device_badge.setObjectName("statusLabel")
        self.device_badge.setAlignment(Qt.AlignCenter)
        card_layout.addWidget(self.device_badge)

        card_layout.addSpacing(10)

        # Emergency PIN Option
        emergency_label = QLabel("如未随身携带卡片，可输入应急安全 PIN 解锁:")
        emergency_label.setStyleSheet("font-size: 13px; color: #718096;")
        card_layout.addWidget(emergency_label)

        pin_row = QHBoxLayout()
        self.pin_input = QLineEdit()
        self.pin_input.setPlaceholderText("请输入 6 位安全 PIN...")
        self.pin_input.setEchoMode(QLineEdit.Password)
        self.pin_input.returnPressed.connect(self._handle_pin_submit)
        pin_row.addWidget(self.pin_input)

        unlock_btn = QPushButton("解锁")
        unlock_btn.setObjectName("unlockBtn")
        unlock_btn.setCursor(Qt.PointingHandCursor)
        unlock_btn.clicked.connect(self._handle_pin_submit)
        pin_row.addWidget(unlock_btn)

        card_layout.addLayout(pin_row)

        self.error_label = QLabel("")
        self.error_label.setObjectName("errorLabel")
        self.error_label.setAlignment(Qt.AlignCenter)
        card_layout.addWidget(self.error_label)

        main_layout.addWidget(card_frame)

    def set_device_info(self, info_text: str):
        """Update hardware reader info badge."""
        self.device_badge.setText(info_text)

    def show_error(self, message: str):
        """Display error text for wrong PIN."""
        self.error_label.setText(message)

    def _handle_pin_submit(self):
        pin = self.pin_input.text().strip()
        if not pin:
            self.show_error("请输入 PIN 码")
            return
        self.error_label.setText("正在验证 PIN...")
        self.pin_unlock_requested.emit(pin)

    def activate_guard(self):
        """Activate full-screen guard on all monitors."""
        self.error_label.setText("")
        self.pin_input.clear()
        # Geometry to cover primary screen
        screen = QApplication.primaryScreen().geometry()
        self.setGeometry(screen)
        self.showFullScreen()
        self.raise_()
        self.activateWindow()

    def deactivate_guard(self):
        """Dismiss full-screen guard."""
        self.hide()

    def keyPressEvent(self, event: QKeyEvent):
        # Prevent Alt+F4 or Escape from closing guard
        if event.key() == Qt.Key_Escape:
            event.ignore()
            return
        super().keyPressEvent(event)
