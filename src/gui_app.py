"""
Main GUI Application Module
===========================
Modern PyQt5 Dashboard for RTS5169 / IC4442 SmartCard Windows Unlock Tool.
"""

import os
import sys
import time
import winreg
from typing import Optional

from PyQt5.QtWidgets import (
    QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QLabel,
    QPushButton, QTabWidget, QLineEdit, QCheckBox, QFrame,
    QGridLayout, QMessageBox, QTextEdit, QGroupBox, QFileDialog
)
from PyQt5.QtCore import Qt, QTimer, pyqtSignal, QSize
from PyQt5.QtGui import QFont, QIcon, QColor, QPalette

from .smartcard_engine import SmartCardEngine
from .security_vault import SecurityVault
from .unlock_controller import UnlockController, is_workstation_locked
from .screen_guard import ScreenGuardWindow
from .tray_icon import SmartCardTrayIcon


REG_RUN_KEY = r"Software\Microsoft\Windows\CurrentVersion\Run"
APP_RUN_NAME = "SmartCardUnlockTool"


def is_autostart_enabled() -> bool:
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, REG_RUN_KEY, 0, winreg.KEY_READ) as key:
            val, _ = winreg.QueryValueEx(key, APP_RUN_NAME)
            return bool(val)
    except OSError:
        return False


def set_autostart_enabled(enable: bool) -> bool:
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, REG_RUN_KEY, 0, winreg.KEY_SET_VALUE) as key:
            if enable:
                python_exe = sys.executable
                script_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "main.py"))
                cmd = f'"{python_exe}" "{script_path}" --silent'
                winreg.SetValueEx(key, APP_RUN_NAME, 0, winreg.REG_SZ, cmd)
            else:
                try:
                    winreg.DeleteValue(key, APP_RUN_NAME)
                except FileNotFoundError:
                    pass
        return True
    except Exception as e:
        print(f"Error updating autostart: {e}")
        return False


class MainWindow(QMainWindow):
    """Main Application Window."""

    def __init__(self, engine: SmartCardEngine, vault: SecurityVault, controller: UnlockController):
        super().__init__()
        self.engine = engine
        self.vault = vault
        self.controller = controller

        self.setWindowTitle("Windows 插卡解锁工具 (RTS5169 + IC4442)")
        self.setFixedSize(760, 560)

        # Components
        self.screen_guard = ScreenGuardWindow()
        self.screen_guard.pin_unlock_requested.connect(self._handle_pin_unlock)

        self.tray = SmartCardTrayIcon(self)
        self.tray.show_window_requested.connect(self._show_window)
        self.tray.lock_requested.connect(self._manual_lock)
        self.tray.toggle_guard_requested.connect(self._toggle_guard)
        self.tray.exit_requested.connect(self._exit_app)
        self.tray.show()

        # Connect Controller to Screen Guard & Tray
        self.controller.on_screen_guard_request = self._handle_guard_request
        self.controller.on_unlock_succeeded = self._handle_unlock_succeeded
        self.controller.on_unlock_failed = self._handle_unlock_failed

        self.screen_guard_enabled = True

        self._init_ui()
        self._init_timer()
        self._refresh_status()

    def _init_ui(self):
        # Modern Dark-themed stylesheet
        self.setStyleSheet("""
            QMainWindow, QWidget {
                background-color: #0f172a;
                color: #e2e8f0;
                font-family: 'Segoe UI', 'Microsoft YaHei', sans-serif;
            }
            QTabWidget::pane {
                border: 1px solid #334155;
                border-radius: 10px;
                background-color: #1e293b;
                top: -1px;
            }
            QTabBar::tab {
                background-color: #0f172a;
                color: #94a3b8;
                padding: 10px 22px;
                margin-right: 4px;
                border-top-left-radius: 8px;
                border-top-right-radius: 8px;
                font-size: 14px;
                font-weight: 500;
            }
            QTabBar::tab:selected {
                background-color: #1e293b;
                color: #38bdf8;
                border: 1px solid #334155;
                border-bottom: none;
            }
            QGroupBox {
                border: 1px solid #334155;
                border-radius: 8px;
                margin-top: 14px;
                font-size: 14px;
                font-weight: bold;
                color: #38bdf8;
                padding-top: 12px;
            }
            QGroupBox::title {
                subcontrol-origin: margin;
                left: 12px;
                padding: 0 6px;
            }
            QLineEdit {
                background-color: #0f172a;
                border: 1px solid #475569;
                border-radius: 6px;
                padding: 8px 12px;
                color: #ffffff;
                font-size: 14px;
            }
            QLineEdit:focus {
                border-color: #38bdf8;
            }
            QPushButton {
                background-color: #2563eb;
                color: #ffffff;
                border: none;
                border-radius: 6px;
                padding: 8px 16px;
                font-size: 14px;
                font-weight: 500;
            }
            QPushButton:hover {
                background-color: #1d4ed8;
            }
            QPushButton:pressed {
                background-color: #1e40af;
            }
            QPushButton#dangerBtn {
                background-color: #dc2626;
            }
            QPushButton#dangerBtn:hover {
                background-color: #b91c1c;
            }
            QPushButton#secondaryBtn {
                background-color: #334155;
            }
            QPushButton#secondaryBtn:hover {
                background-color: #475569;
            }
            QCheckBox {
                font-size: 14px;
                spacing: 8px;
            }
            QCheckBox::indicator {
                width: 18px;
                height: 18px;
                border-radius: 4px;
                border: 1px solid #475569;
                background-color: #0f172a;
            }
            QCheckBox::indicator:checked {
                background-color: #2563eb;
                border-color: #38bdf8;
            }
        """)

        central = QWidget()
        self.setCentralWidget(central)
        root_layout = QVBoxLayout(central)
        root_layout.setContentsMargins(16, 16, 16, 16)
        root_layout.setSpacing(12)

        # Header bar
        header = QHBoxLayout()
        title_box = QVBoxLayout()
        app_title = QLabel("Windows 插卡解锁工具")
        app_title.setStyleSheet("font-size: 20px; font-weight: bold; color: #38bdf8;")
        app_sub = QLabel("硬件设备: Realtek RTS5169 (Generic EMV) · IC4442 智能卡 · 位置: Port_#0004.Hub_#0002")
        app_sub.setStyleSheet("font-size: 12px; color: #94a3b8;")
        title_box.addWidget(app_title)
        title_box.addWidget(app_sub)
        header.addLayout(title_box)

        header.addStretch()

        self.header_status_badge = QLabel("● 守护运行中")
        self.header_status_badge.setStyleSheet("color: #4ade80; font-weight: bold; font-size: 13px; background-color: #1e293b; padding: 6px 12px; border-radius: 12px;")
        header.addWidget(self.header_status_badge)

        root_layout.addLayout(header)

        # Tab Widget
        tabs = QTabWidget()
        tabs.addTab(self._create_dashboard_tab(), "运行状态")
        tabs.addTab(self._create_enrollment_tab(), "卡片绑定与凭据")
        tabs.addTab(self._create_settings_tab(), "安全策略")
        tabs.addTab(self._create_diagnostics_tab(), "读卡器诊断")
        root_layout.addWidget(tabs)

    def _create_dashboard_tab(self) -> QWidget:
        widget = QWidget()
        layout = QVBoxLayout(widget)
        layout.setContentsMargins(18, 18, 18, 18)
        layout.setSpacing(16)

        # Big Status Card
        status_box = QFrame()
        status_box.setStyleSheet("background-color: #0f172a; border: 1px solid #334155; border-radius: 12px; padding: 16px;")
        status_layout = QHBoxLayout(status_box)

        self.card_icon_label = QLabel("💳")
        self.card_icon_label.setStyleSheet("font-size: 52px; margin-right: 12px;")
        status_layout.addWidget(self.card_icon_label)

        info_vbox = QVBoxLayout()
        self.card_status_title = QLabel("正在检测智能卡...")
        self.card_status_title.setStyleSheet("font-size: 18px; font-weight: bold; color: #ffffff;")
        self.card_status_detail = QLabel("读卡器: 检测中...")
        self.card_status_detail.setStyleSheet("font-size: 13px; color: #94a3b8; margin-top: 4px;")
        info_vbox.addWidget(self.card_status_title)
        info_vbox.addWidget(self.card_status_detail)
        status_layout.addLayout(info_vbox)

        status_layout.addStretch()

        self.card_badge = QLabel("检测中")
        self.card_badge.setStyleSheet("background-color: #334155; color: #ffffff; padding: 8px 16px; border-radius: 8px; font-weight: bold;")
        status_layout.addWidget(self.card_badge)

        layout.addWidget(status_box)

        # Grid Details
        grid = QGridLayout()
        grid.setSpacing(12)

        self.lbl_reader_name = QLabel("Generic EMV Smartcard Reader 0")
        self.lbl_card_type = QLabel("IC4442 (SLE4442) 接触式存储卡")
        self.lbl_enrolled_user = QLabel("未绑定")
        self.lbl_session_state = QLabel("活动 (正常)")

        row_data = [
            ("读卡器名称:", self.lbl_reader_name),
            ("卡片协议与类型:", self.lbl_card_type),
            ("已绑定的 Windows 账户:", self.lbl_enrolled_user),
            ("当前会话锁定状态:", self.lbl_session_state),
        ]

        for i, (k, v) in enumerate(row_data):
            lbl_k = QLabel(k)
            lbl_k.setStyleSheet("color: #94a3b8; font-weight: 500;")
            v.setStyleSheet("color: #ffffff; font-weight: bold;")
            grid.addWidget(lbl_k, i, 0)
            grid.addWidget(v, i, 1)

        layout.addLayout(grid)
        layout.addStretch()

        # Action Buttons
        btn_row = QHBoxLayout()
        btn_lock = QPushButton("🔒 立即测试锁屏")
        btn_lock.clicked.connect(self._manual_lock)
        btn_row.addWidget(btn_lock)

        btn_refresh = QPushButton("🔄 刷新设备状态")
        btn_refresh.setObjectName("secondaryBtn")
        btn_refresh.clicked.connect(self._refresh_status)
        btn_row.addWidget(btn_refresh)

        btn_row.addStretch()
        layout.addLayout(btn_row)

        return widget

    def _create_enrollment_tab(self) -> QWidget:
        widget = QWidget()
        layout = QVBoxLayout(widget)
        layout.setContentsMargins(18, 18, 18, 18)
        layout.setSpacing(14)

        desc = QLabel("将当前插入的 IC4442 卡片与 Windows 登录凭据进行安全绑定。\n凭据将使用 Windows DPAPI 和 AES-256-GCM 进行双层硬件绑定加密。")
        desc.setStyleSheet("color: #94a3b8; font-size: 13px; line-height: 1.4;")
        layout.addWidget(desc)

        group = QGroupBox("Windows 凭据绑定")
        form = QGridLayout(group)
        form.setSpacing(12)

        form.addWidget(QLabel("Windows 用户名:"), 0, 0)
        self.input_username = QLineEdit(os.environ.get("USERNAME", ""))
        form.addWidget(self.input_username, 0, 1)

        form.addWidget(QLabel("Windows 登录密码:"), 1, 0)
        self.input_password = QLineEdit()
        self.input_password.setEchoMode(QLineEdit.Password)
        self.input_password.setPlaceholderText("请输入你的 Windows 账户登录密码...")
        form.addWidget(self.input_password, 1, 1)

        form.addWidget(QLabel("自定义安全 PIN (可选):"), 2, 0)
        self.input_pin = QLineEdit("123456")
        self.input_pin.setPlaceholderText("4-8位数字，作为应急及二次验证...")
        form.addWidget(self.input_pin, 2, 1)

        layout.addWidget(group)

        # Action Buttons
        btn_box = QHBoxLayout()
        btn_enroll = QPushButton("🔐 一键绑定并加密写入安全金库")
        btn_enroll.clicked.connect(self._handle_enroll)
        btn_box.addWidget(btn_enroll)

        btn_clear = QPushButton("🗑 清除现有绑定")
        btn_clear.setObjectName("dangerBtn")
        btn_clear.clicked.connect(self._handle_clear_vault)
        btn_box.addWidget(btn_clear)

        btn_box.addStretch()
        layout.addLayout(btn_box)

        layout.addStretch()
        return widget

    def _create_settings_tab(self) -> QWidget:
        widget = QWidget()
        layout = QVBoxLayout(widget)
        layout.setContentsMargins(18, 18, 18, 18)
        layout.setSpacing(16)

        grp_policy = QGroupBox("智能卡插拔响应策略")
        vbox_policy = QVBoxLayout(grp_policy)
        vbox_policy.setSpacing(12)

        self.chk_lock_on_remove = QCheckBox("拔卡自动锁屏 (拔出 IC4442 卡片时立即锁定系统)")
        self.chk_lock_on_remove.setChecked(True)
        self.chk_lock_on_remove.stateChanged.connect(self._update_settings)
        vbox_policy.addWidget(self.chk_lock_on_remove)

        self.chk_unlock_on_insert = QCheckBox("插卡自动解锁 (插入 IC4442 卡片并验证通过时自动解锁)")
        self.chk_unlock_on_insert.setChecked(True)
        self.chk_unlock_on_insert.stateChanged.connect(self._update_settings)
        vbox_policy.addWidget(self.chk_unlock_on_insert)

        self.chk_screen_guard = QCheckBox("启用全屏安全屏保锁 (Screen Guard 防旁路防护)")
        self.chk_screen_guard.setChecked(True)
        self.chk_screen_guard.stateChanged.connect(self._update_settings)
        vbox_policy.addWidget(self.chk_screen_guard)

        layout.addWidget(grp_policy)

        grp_system = QGroupBox("系统自启动设置")
        vbox_sys = QVBoxLayout(grp_system)

        self.chk_autostart = QCheckBox("开机自动启动并在后台静默守护")
        self.chk_autostart.setChecked(is_autostart_enabled())
        self.chk_autostart.stateChanged.connect(self._toggle_autostart)
        vbox_sys.addWidget(self.chk_autostart)

        layout.addWidget(grp_system)
        layout.addStretch()
        return widget

    def _create_diagnostics_tab(self) -> QWidget:
        widget = QWidget()
        layout = QVBoxLayout(widget)
        layout.setContentsMargins(18, 18, 18, 18)
        layout.setSpacing(12)

        top_row = QHBoxLayout()
        top_row.addWidget(QLabel("智能卡实时事件与驱动交互日志:"))
        top_row.addStretch()

        btn_test_probe = QPushButton("🔍 深度探测卡片属性")
        btn_test_probe.setObjectName("secondaryBtn")
        btn_test_probe.clicked.connect(self._deep_probe)
        top_row.addWidget(btn_test_probe)

        btn_clear_log = QPushButton("清空日志")
        btn_clear_log.setObjectName("secondaryBtn")
        btn_clear_log.clicked.connect(lambda: self.log_text.clear())
        top_row.addWidget(btn_clear_log)

        layout.addLayout(top_row)

        self.log_text = QTextEdit()
        self.log_text.setReadOnly(True)
        self.log_text.setStyleSheet("background-color: #0b0f19; color: #a5f3fc; font-family: Consolas, monospace; font-size: 13px; border: 1px solid #334155; border-radius: 8px;")
        layout.addWidget(self.log_text)

        return widget

    def _init_timer(self):
        """Periodic status refresh timer (every 1 second)."""
        self.timer = QTimer(self)
        self.timer.timeout.connect(self._refresh_status)
        self.timer.start(1000)

    def log(self, text: str):
        timestamp = time.strftime("%H:%M:%S")
        self.log_text.append(f"[{timestamp}] {text}")

    def _refresh_status(self):
        reader = self.engine.current_reader or self.engine.find_target_reader()
        if not reader:
            self.card_status_title.setText("未检测到读卡器")
            self.card_status_detail.setText("请检查 RTS5169 读卡器是否插在 Port_#0004.Hub_#0002")
            self.card_badge.setText("无读卡器")
            self.card_badge.setStyleSheet("background-color: #ef4444; color: #ffffff; padding: 6px 14px; border-radius: 6px; font-weight: bold;")
            self.card_icon_label.setText("⚠️")
            self.lbl_reader_name.setText("未连接")
            self.tray.update_card_status(False)
            return

        self.lbl_reader_name.setText(reader)
        is_pres, atr, state = self.engine.get_reader_state(reader)
        self.tray.update_card_status(is_pres)

        # Session lock state
        locked = is_workstation_locked()
        if locked:
            self.lbl_session_state.setText("已锁定 (Secure Desktop)")
            self.lbl_session_state.setStyleSheet("color: #f59e0b; font-weight: bold;")
        else:
            self.lbl_session_state.setText("活动 (正常)")
            self.lbl_session_state.setStyleSheet("color: #4ade80; font-weight: bold;")

        # Card status
        if is_pres:
            self.card_status_title.setText("IC4442 卡片已插入")
            self.card_status_detail.setText(f"读卡器: {reader} · 状态值: 0x{state:08X}")
            self.card_badge.setText("已插卡")
            self.card_badge.setStyleSheet("background-color: #22c55e; color: #ffffff; padding: 6px 14px; border-radius: 6px; font-weight: bold;")
            self.card_icon_label.setText("💳")
        else:
            self.card_status_title.setText("未检测到卡片")
            self.card_status_detail.setText(f"读卡器就绪，等待插入 IC4442 智能卡...")
            self.card_badge.setText("等待插卡")
            self.card_badge.setStyleSheet("background-color: #f59e0b; color: #ffffff; padding: 6px 14px; border-radius: 6px; font-weight: bold;")
            self.card_icon_label.setText("📭")

        # Enrollment metadata
        meta = self.vault.get_enrolled_metadata()
        if meta:
            self.lbl_enrolled_user.setText(f"{meta['username']} (已安全绑定)")
            self.lbl_enrolled_user.setStyleSheet("color: #38bdf8; font-weight: bold;")
        else:
            self.lbl_enrolled_user.setText("未绑定凭据")
            self.lbl_enrolled_user.setStyleSheet("color: #94a3b8; font-weight: bold;")

    def _deep_probe(self):
        reader = self.engine.find_target_reader()
        if not reader:
            self.log("未检测到读卡器，无法执行探测。")
            return
        self.log(f"开始探测读卡器: {reader}")
        details = self.engine.probe_card_details(reader)
        for k, v in details.items():
            self.log(f"  {k}: {v}")
        self.log("探测完成。")

    def _handle_enroll(self):
        username = self.input_username.text().strip()
        password = self.input_password.text()
        pin = self.input_pin.text().strip() or "123456"

        if not username:
            QMessageBox.warning(self, "提示", "请输入 Windows 用户名！")
            return
        if not password:
            QMessageBox.warning(self, "提示", "请输入 Windows 登录密码！")
            return

        reader = self.engine.current_reader or "Generic EMV Smartcard Reader 0"
        is_pres, atr, _ = self.engine.get_reader_state(reader)
        if not is_pres:
            res = QMessageBox.question(
                self,
                "提示",
                "读卡器中当前未检测到卡片，是否仍要预绑定当前凭据？",
                QMessageBox.Yes | QMessageBox.No
            )
            if res != QMessageBox.Yes:
                return

        token_id = atr.hex().upper() if atr else "IC4442_DEFAULT_TOKEN"

        success = self.vault.enroll(
            username=username,
            password=password,
            pin=pin,
            card_token=token_id,
            reader_info=reader
        )

        if success:
            self.input_password.clear()
            self._refresh_status()
            self.log(f"已成功绑定用户 '{username}' 的登录凭据到当前卡片！")
            QMessageBox.information(self, "成功", f"卡片已成功绑定到 Windows 用户 '{username}'！\n密码已通过 DPAPI + AES-256-GCM 双重加密保存。")
        else:
            QMessageBox.critical(self, "失败", "绑定凭据失败，请查看日志！")

    def _handle_clear_vault(self):
        if not self.vault.is_enrolled():
            QMessageBox.information(self, "提示", "当前没有绑定的凭据。")
            return

        res = QMessageBox.question(
            self,
            "确认清除",
            "确定要清除已绑定的 Windows 解锁凭据吗？清除后插卡将无法解锁。",
            QMessageBox.Yes | QMessageBox.No
        )
        if res == QMessageBox.Yes:
            self.vault.clear_vault()
            self._refresh_status()
            self.log("已清空安全金库中的凭据。")
            QMessageBox.information(self, "提示", "已成功清除绑定。")

    def _update_settings(self):
        self.controller.lock_on_removal = self.chk_lock_on_remove.isChecked()
        self.controller.unlock_on_insertion = self.chk_unlock_on_insert.isChecked()
        self.screen_guard_enabled = self.chk_screen_guard.isChecked()
        self.log(f"策略更新: 拔卡锁屏={self.controller.lock_on_removal}, 插卡解锁={self.controller.unlock_on_insertion}, 安全屏保={self.screen_guard_enabled}")

    def _toggle_autostart(self):
        enable = self.chk_autostart.isChecked()
        set_autostart_enabled(enable)
        self.log(f"开机自启动设置为: {enable}")

    def _manual_lock(self):
        self.log("手动触发系统锁定...")
        self.controller.manual_lock()

    def _toggle_guard(self, active: bool):
        if active:
            self.header_status_badge.setText("● 守护运行中")
            self.header_status_badge.setStyleSheet("color: #4ade80; font-weight: bold; font-size: 13px; background-color: #1e293b; padding: 6px 12px; border-radius: 12px;")
            self.engine.start_monitoring()
            self.log("已恢复智能卡守护监听。")
        else:
            self.header_status_badge.setText("○ 守护已暂停")
            self.header_status_badge.setStyleSheet("color: #94a3b8; font-weight: bold; font-size: 13px; background-color: #1e293b; padding: 6px 12px; border-radius: 12px;")
            self.engine.stop_monitoring()
            self.log("已暂停智能卡守护监听。")

    def _handle_guard_request(self, activate: bool):
        if activate and self.screen_guard_enabled:
            self.screen_guard.activate_guard()
        else:
            self.screen_guard.deactivate_guard()

    def _handle_pin_unlock(self, pin: str):
        success = self.controller.manual_unlock_with_pin(pin)
        if success:
            self.log("应急 PIN 解锁成功！")
            self.screen_guard.deactivate_guard()
            self.tray.notify("解锁成功", "已通过应急 PIN 解锁系统。")
        else:
            self.log("应急 PIN 解锁失败：PIN 码错误。")
            self.screen_guard.show_error("PIN 码错误，验证失败！")

    def _handle_unlock_succeeded(self, username: str):
        self.log(f"插卡解锁成功！已识别用户: {username}")
        self.tray.notify("智能卡解锁", f"已识别 IC4442 卡片，欢迎回来 {username}！")

    def _handle_unlock_failed(self, reason: str):
        self.log(f"插卡解锁未通过: {reason}")
        self.tray.notify("智能卡验证未通过", reason, is_info=False)

    def _show_window(self):
        self.showNormal()
        self.raise_()
        self.activateWindow()

    def _exit_app(self):
        self.engine.stop_monitoring()
        self.engine.close()
        self.tray.hide()
        sys.exit(0)

    def closeEvent(self, event):
        """Minimize to tray on window close."""
        event.ignore()
        self.hide()
        self.tray.notify("后台运行中", "Windows 插卡解锁工具已最小化至系统托盘继续守护。")
