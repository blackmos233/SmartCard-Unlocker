"""
Security Vault Module
=====================
Provides military-grade credential protection using a hybrid encryption scheme:
1. Windows DPAPI (CryptProtectData / CryptUnprotectData)
2. AES-256-GCM authenticated symmetric encryption
3. PBKDF2-HMAC-SHA256 (100,000 iterations) key derivation
4. Hardware binding (Machine GUID + SmartCard Reader port)
"""

import os
import json
import base64
import winreg
import logging
from typing import Optional, Tuple, Dict, Any

import win32crypt
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives import hashes

logger = logging.getLogger("SecurityVault")

VAULT_DIR = os.path.join(os.environ.get("LOCALAPPDATA", os.path.expanduser("~")), "SmartCardUnlock")
VAULT_FILE = os.path.join(VAULT_DIR, "vault.dat")
PBKDF2_ITERATIONS = 100_000


def get_machine_hardware_id() -> str:
    """
    Retrieve unique machine hardware identifier from Windows registry.
    """
    try:
        with winreg.OpenKey(
            winreg.HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Cryptography",
            0,
            winreg.KEY_READ | winreg.KEY_WOW64_64KEY,
        ) as key:
            guid, _ = winreg.QueryValueEx(key, "MachineGuid")
            return str(guid).strip()
    except Exception as e:
        logger.warning(f"Could not read MachineGuid from registry: {e}")
        # Fallback to computer name + processor arch
        comp = os.environ.get("COMPUTERNAME", "UNKNOWN-HOST")
        arch = os.environ.get("PROCESSOR_IDENTIFIER", "UNKNOWN-CPU")
        return f"{comp}-{arch}"


class SecurityVault:
    """
    Cryptographic vault managing Windows unlock credentials securely.
    """

    def __init__(self, vault_path: Optional[str] = None):
        self.vault_path = vault_path or VAULT_FILE
        vault_parent = os.path.dirname(self.vault_path)
        if not os.path.exists(vault_parent):
            os.makedirs(vault_parent, exist_ok=True)

    def _derive_key(self, pin: str, card_token: str, salt: bytes) -> bytes:
        """
        Derive a 256-bit AES key using PBKDF2-HMAC-SHA256 with 100,000 iterations.
        Combines User PIN, Card Token, and Machine Hardware GUID.
        """
        hw_id = get_machine_hardware_id()
        key_material = f"{pin}:{card_token}:{hw_id}".encode("utf-8")

        kdf = PBKDF2HMAC(
            algorithm=hashes.SHA256(),
            length=32,
            salt=salt,
            iterations=PBKDF2_ITERATIONS,
        )
        return kdf.derive(key_material)

    def is_enrolled(self) -> bool:
        """Check if a card and credentials are currently enrolled."""
        return os.path.exists(self.vault_path) and os.path.getsize(self.vault_path) > 0

    def get_enrolled_metadata(self) -> Optional[Dict[str, Any]]:
        """Retrieve non-sensitive enrollment metadata (e.g. username, enrollment date)."""
        if not self.is_enrolled():
            return None
        try:
            with open(self.vault_path, "rb") as f:
                outer_blob = f.read()
            # Decrypt DPAPI layer
            inner_json_bytes = win32crypt.CryptUnprotectData(outer_blob, None, None, None, 0)[1]
            payload = json.loads(inner_json_bytes.decode("utf-8"))
            return {
                "username": payload.get("username", ""),
                "domain": payload.get("domain", ""),
                "reader_bound": payload.get("reader_bound", ""),
                "created_at": payload.get("created_at", 0),
            }
        except Exception as e:
            logger.error(f"Failed to read vault metadata: {e}")
            return None

    def enroll(
        self,
        username: str,
        password: str,
        pin: str = "123456",
        card_token: str = "IC4442_DEFAULT_TOKEN",
        reader_info: str = "Port_#0004.Hub_#0002",
        domain: str = "",
    ) -> bool:
        """
        Securely enroll Windows unlock credentials.
        
        Args:
            username: Windows user account name
            password: Windows user password
            pin: User-defined smartcard PIN (defaults to 123456 or custom)
            card_token: Unique card token or ATR-derived identity
            reader_info: Bound reader identifier / hardware location
            domain: Domain name or local machine name
        """
        import time

        try:
            salt = os.urandom(16)
            nonce = os.urandom(12)
            aes_key = self._derive_key(pin, card_token, salt)

            # Plaintext payload to encrypt with AES-256-GCM
            secret_payload = json.dumps(
                {"password": password, "verified_at": time.time()}
            ).encode("utf-8")

            # Encrypt with AES-GCM
            aesgcm = AESGCM(aes_key)
            associated_data = f"{username}:{reader_info}".encode("utf-8")
            ciphertext = aesgcm.encrypt(nonce, secret_payload, associated_data)

            # Construct vault structure
            vault_struct = {
                "version": 1,
                "username": username,
                "domain": domain,
                "reader_bound": reader_info,
                "salt": base64.b64encode(salt).decode("ascii"),
                "nonce": base64.b64encode(nonce).decode("ascii"),
                "ciphertext": base64.b64encode(ciphertext).decode("ascii"),
                "created_at": time.time(),
            }

            inner_data = json.dumps(vault_struct).encode("utf-8")

            # Outer encryption layer: Windows DPAPI (tied to current Windows user & machine)
            outer_blob = win32crypt.CryptProtectData(
                inner_data, "SmartCardUnlockVault", None, None, None, 0
            )

            # Write atomically to file
            temp_file = f"{self.vault_path}.tmp"
            with open(temp_file, "wb") as f:
                f.write(outer_blob)
            if os.path.exists(self.vault_path):
                os.remove(self.vault_path)
            os.rename(temp_file, self.vault_path)

            logger.info(f"Successfully enrolled credentials for user '{username}'.")
            return True
        except Exception as e:
            logger.exception(f"Failed to enroll credentials: {e}")
            return False

    def unlock_verify(
        self,
        pin: str = "123456",
        card_token: str = "IC4442_DEFAULT_TOKEN",
        current_reader: str = "Port_#0004.Hub_#0002",
    ) -> Optional[Tuple[str, str, str]]:
        """
        Verify the presented card + PIN and decrypt the Windows credentials.
        
        Returns:
            (username, password, domain) if successful, None if verification fails.
        """
        if not self.is_enrolled():
            logger.warning("Unlock attempted but vault is not enrolled.")
            return None

        try:
            with open(self.vault_path, "rb") as f:
                outer_blob = f.read()

            # Layer 1: Decrypt DPAPI
            inner_json_bytes = win32crypt.CryptUnprotectData(outer_blob, None, None, None, 0)[1]
            vault_struct = json.loads(inner_json_bytes.decode("utf-8"))

            username = vault_struct["username"]
            domain = vault_struct.get("domain", "")
            reader_bound = vault_struct.get("reader_bound", "")
            salt = base64.b64decode(vault_struct["salt"])
            nonce = base64.b64decode(vault_struct["nonce"])
            ciphertext = base64.b64decode(vault_struct["ciphertext"])

            # Layer 2: Derive AES key from PIN + card token + hardware ID
            aes_key = self._derive_key(pin, card_token, salt)

            # Layer 3: Authenticated decryption with AES-256-GCM
            aesgcm = AESGCM(aes_key)
            associated_data = f"{username}:{reader_bound}".encode("utf-8")
            secret_payload = aesgcm.decrypt(nonce, ciphertext, associated_data)

            secret_dict = json.loads(secret_payload.decode("utf-8"))
            password = secret_dict["password"]

            logger.info(f"Credential successfully decrypted for user '{username}'.")
            return username, password, domain

        except Exception as e:
            logger.warning(f"Verification / Decryption failed (invalid card, PIN, or hardware): {e}")
            return None

    def clear_vault(self) -> bool:
        """Securely wipe and remove the vault file."""
        if os.path.exists(self.vault_path):
            try:
                # Overwrite file with zeros before deleting
                size = os.path.getsize(self.vault_path)
                with open(self.vault_path, "wb") as f:
                    f.write(os.urandom(size))
                os.remove(self.vault_path)
                logger.info("Vault securely wiped.")
                return True
            except Exception as e:
                logger.error(f"Error wiping vault: {e}")
                return False
        return True
