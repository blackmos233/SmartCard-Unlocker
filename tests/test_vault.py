"""
Unit tests for Security Vault
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from src.security_vault import SecurityVault

def test_vault():
    test_vault_path = os.path.join(os.path.dirname(__file__), "test_vault.dat")
    vault = SecurityVault(vault_path=test_vault_path)

    # Clean previous test file if exists
    if os.path.exists(test_vault_path):
        os.remove(test_vault_path)

    print("[TEST] Checking is_enrolled on clean state...")
    assert not vault.is_enrolled(), "Vault should not be enrolled initially!"

    print("[TEST] Enrolling test credentials...")
    success = vault.enroll(
        username="TestUser",
        password="MySecretPassword123!",
        pin="654321",
        card_token="CARD_TOKEN_TEST_ABC",
        reader_info="Port_#0004.Hub_#0002",
        domain="WORKGROUP"
    )
    assert success, "Enrollment failed!"
    assert vault.is_enrolled(), "Vault should now be enrolled!"

    print("[TEST] Checking metadata...")
    meta = vault.get_enrolled_metadata()
    assert meta["username"] == "TestUser", f"Wrong username: {meta}"
    assert meta["reader_bound"] == "Port_#0004.Hub_#0002", f"Wrong reader: {meta}"

    print("[TEST] Verifying unlock with correct PIN and token...")
    creds = vault.unlock_verify(
        pin="654321",
        card_token="CARD_TOKEN_TEST_ABC",
        current_reader="Port_#0004.Hub_#0002"
    )
    assert creds is not None, "Decryption should have succeeded!"
    u, p, d = creds
    assert u == "TestUser"
    assert p == "MySecretPassword123!"
    assert d == "WORKGROUP"
    print(f"[TEST] Successfully decrypted credentials for {u}!")

    print("[TEST] Testing with WRONG PIN (should fail)...")
    wrong_creds = vault.unlock_verify(
        pin="000000",
        card_token="CARD_TOKEN_TEST_ABC"
    )
    assert wrong_creds is None, "Decryption with wrong PIN MUST fail!"

    print("[TEST] Testing with WRONG Card Token (should fail)...")
    wrong_token = vault.unlock_verify(
        pin="654321",
        card_token="WRONG_TOKEN"
    )
    assert wrong_token is None, "Decryption with wrong Token MUST fail!"

    print("[TEST] Clearing vault...")
    vault.clear_vault()
    assert not vault.is_enrolled(), "Vault should be cleared!"
    assert not os.path.exists(test_vault_path), "Vault file should be deleted!"

    print("[TEST] Security Vault tests all PASSED!")

if __name__ == "__main__":
    test_vault()
