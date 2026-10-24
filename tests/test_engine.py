"""
Unit test for SmartCard Engine
"""
import sys
import os
import time

# Add src to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from src.smartcard_engine import SmartCardEngine

def test_engine():
    print("[TEST] Initializing SmartCardEngine...")
    engine = SmartCardEngine(target_reader_keyword="EMV")
    readers = engine.list_readers()
    print(f"[TEST] Readers found: {readers}")
    assert len(readers) > 0, "No smartcard readers found!"

    target = engine.find_target_reader()
    print(f"[TEST] Target reader: {target}")
    assert target is not None, "Target reader not detected!"

    is_present, atr, state = engine.get_reader_state(target)
    print(f"[TEST] Reader state: present={is_present}, atr={atr.hex()}, state=0x{state:08X}")

    details = engine.probe_card_details(target)
    print(f"[TEST] Card details: {details}")

    # Test callback mechanism
    events = []
    def on_inserted(r, atr, info):
        print(f"[CALLBACK] Card Inserted: reader={r}, info={info}")
        events.append("inserted")

    def on_removed(r):
        print(f"[CALLBACK] Card Removed: reader={r}")
        events.append("removed")

    engine.on_card_inserted = on_inserted
    engine.on_card_removed = on_removed

    print("[TEST] Starting background monitor for 2 seconds...")
    engine.start_monitoring(poll_interval=0.2)
    time.sleep(1.5)
    engine.stop_monitoring()
    engine.close()

    print("[TEST] Engine test passed successfully!")

if __name__ == "__main__":
    test_engine()
