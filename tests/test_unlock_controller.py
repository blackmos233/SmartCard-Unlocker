"""
Unit test for Unlock Controller
"""
import sys
import os

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from src.smartcard_engine import SmartCardEngine
from src.security_vault import SecurityVault
from src.unlock_controller import UnlockController, is_workstation_locked

def test_unlock_controller():
    print("[TEST] Testing is_workstation_locked()...")
    locked = is_workstation_locked()
    print(f"[TEST] Current lock status: {locked}")

    test_vault_path = os.path.join(os.path.dirname(__file__), "test_ctrl_vault.dat")
    if os.path.exists(test_vault_path):
        os.remove(test_vault_path)

    vault = SecurityVault(vault_path=test_vault_path)
    vault.enroll(
        username="AdminBlack",
        password="ValidPassword123!",
        pin="123456",
        card_token="IC4442_DEFAULT_TOKEN",
        reader_info="Generic EMV Smartcard Reader 0"
    )

    engine = SmartCardEngine(target_reader_keyword="EMV")
    controller = UnlockController(
        engine=engine,
        vault=vault,
        lock_on_removal=False, # Don't actually lock test user's desktop during test!
        unlock_on_insertion=True
    )

    unlocked_users = []
    screen_guard_states = []

    controller.on_unlock_succeeded = lambda u: unlocked_users.append(u)
    controller.on_screen_guard_request = lambda s: screen_guard_states.append(s)

    print("[TEST] Simulating card insertion...")
    controller._handle_card_inserted("Generic EMV Smartcard Reader 0", b"", {})
    assert len(unlocked_users) == 1, f"Expected 1 unlock event, got {len(unlocked_users)}"
    assert unlocked_users[0] == "AdminBlack", f"Unexpected unlocked user: {unlocked_users[0]}"
    print(f"[TEST] Card inserted simulation succeeded: unlocked {unlocked_users[0]}")

    print("[TEST] Simulating card removal...")
    controller._handle_card_removed("Generic EMV Smartcard Reader 0")
    print("[TEST] Card removed simulation finished.")

    # Cleanup test vault
    vault.clear_vault()
    engine.close()

    print("[TEST] Unlock Controller test PASSED successfully!")

if __name__ == "__main__":
    test_unlock_controller()
