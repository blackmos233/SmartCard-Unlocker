"""
RTS5169 / Realtek SLE4442 Low-Level Diagnostic Utility
======================================================
Interfaces with Realtek RsCRLib / low-level driver or WinSCard direct IOCTLs
to inspect and read/write raw IC4442 EEPROM zones.
"""

import os
import sys
import ctypes
from ctypes import wintypes
import logging

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

if sys.stdout.encoding.lower() != 'utf-8':
    try:
        sys.stdout.reconfigure(encoding='utf-8')
    except Exception:
        pass

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("Rts4442Tool")


class Rts4442Tool:
    """Diagnostic tool for IC4442 memory cards."""

    def __init__(self, reader_keyword: str = "EMV"):
        self.reader_keyword = reader_keyword
        self.winscard = ctypes.windll.WinSCard
        self.h_context = wintypes.HANDLE()
        self.winscard.SCardEstablishContext(0, None, None, ctypes.byref(self.h_context))

    def run_diagnostics(self):
        """Run full hardware check."""
        print("=" * 60)
        print("  RTS5169 / IC4442 读卡器与智能卡深度硬件诊断工具")
        print("=" * 60)

        # 1. PC/SC Readers
        cch = wintypes.DWORD()
        self.winscard.SCardListReadersW(self.h_context, None, None, ctypes.byref(cch))
        if cch.value == 0:
            print("[错误] 未发现任何 PC/SC 智能卡读卡器设备！")
            return

        buf = ctypes.create_unicode_buffer(cch.value)
        self.winscard.SCardListReadersW(self.h_context, None, buf, ctypes.byref(cch))
        readers = [r for r in buf[:cch.value].split("\0") if r]

        print(f"[OK] 发现已连接读卡器 ({len(readers)} 个):")
        for r in readers:
            print(f"     -> {r}")

        target = next((r for r in readers if self.reader_keyword.lower() in r.lower()), readers[0])
        print(f"\n[目标读卡器]: {target}")

        # 2. Query reader direct status
        SCARD_SHARE_DIRECT = 3
        h_card = wintypes.HANDLE()
        proto = wintypes.DWORD()
        res = self.winscard.SCardConnectW(
            self.h_context, target, SCARD_SHARE_DIRECT, 0, ctypes.byref(h_card), ctypes.byref(proto)
        )
        if res == 0:
            print("[OK] 成功以 DIRECT 模式建立与读卡器硬件控制器的底层连接！")
            # Query Vendor details
            for attr_id, name in [
                (0x00010100, "厂商名称 (Vendor Name)"),
                (0x00010101, "接口类型 (IFD Type)"),
                (0x00090300, "卡片存在指示 (ICC Presence)"),
            ]:
                abuf = (ctypes.c_byte * 256)()
                alen = wintypes.DWORD(256)
                if self.winscard.SCardGetAttrib(h_card, attr_id, abuf, ctypes.byref(alen)) == 0:
                    val = bytes(abuf[:alen.value])
                    try:
                        str_val = val.decode("utf-8", errors="ignore").strip()
                    except:
                        str_val = val.hex(" ")
                    print(f"     {name}: {str_val} (HEX: {val.hex()})")

            self.winscard.SCardDisconnect(h_card, 0)
        else:
            print(f"[警告] DIRECT 连接返回: 0x{res & 0xFFFFFFFF:08X}")

        # 3. Card state analysis
        from src.smartcard_engine import SCARD_READERSTATEW, SCARD_STATE_PRESENT, SCARD_STATE_MUTE
        rs = SCARD_READERSTATEW()
        rs.szReader = target
        rs.dwCurrentState = 0
        self.winscard.SCardGetStatusChangeW(self.h_context, 200, ctypes.byref(rs), 1)

        print("\n[卡片状态检测]:")
        is_pres = bool(rs.dwEventState & SCARD_STATE_PRESENT)
        is_mute = bool(rs.dwEventState & SCARD_STATE_MUTE)
        event_cnt = (rs.dwEventState >> 16) & 0xFFFF

        print(f"     物理插入状态: {'已插入 [PRESENT]' if is_pres else '未插卡 [EMPTY]'}")
        print(f"     插拔计数器: {event_cnt} 次")
        print(f"     ATR 状态: {'Card Mute (同步卡等待同步时钟或方向需核实)' if is_mute else 'ATR 正常'}")
        if rs.cbAtr > 0:
            atr = bytes(rs.rgbAtr[:rs.cbAtr])
            print(f"     ATR 字节: {atr.hex(' ').upper()}")
        else:
            print("     提示: IC4442 为同步存储卡 (2-wire)，标准通用 CCID 驱动下不返回异步 ATR。")

        print("=" * 60)

    def close(self):
        if self.h_context:
            self.winscard.SCardReleaseContext(self.h_context)


if __name__ == "__main__":
    tool = Rts4442Tool()
    tool.run_diagnostics()
    tool.close()
