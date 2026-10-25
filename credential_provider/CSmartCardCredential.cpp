#include "CSmartCardCredential.h"
#include "guid.h"
#include <ntsecapi.h>

CSmartCardCredential::CSmartCardCredential(CREDENTIAL_PROVIDER_USAGE_SCENARIO cpus) :
    m_cRef(1),
    m_cpus(cpus),
    m_pEvents(NULL),
    m_bCardInserted(false),
    m_bAutoLogon(false),
    m_statusText(L"请插入卡片进行解锁")
{
    // 初始化时探测物理智能卡当前真实在位状态
    m_bCardInserted = IsHardwareCardPresent();
    if (m_bCardInserted) {
        m_statusText = L"卡片已插入，按下回车或提交箭头以解锁";
    }
    DllAddRef();
}

CSmartCardCredential::~CSmartCardCredential() {
    if (m_pEvents) {
        m_pEvents->Release();
        m_pEvents = NULL;
    }
    DllRelease();
}

HRESULT CSmartCardCredential::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = NULL;

    if (riid == IID_IUnknown || riid == IID_ICredentialProviderCredential) {
        *ppv = static_cast<ICredentialProviderCredential*>(this);
        AddRef();
        return S_OK;
    } else if (riid == IID_ICredentialProviderCredential2) {
        *ppv = static_cast<ICredentialProviderCredential2*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

ULONG CSmartCardCredential::AddRef() {
    return InterlockedIncrement(&m_cRef);
}

ULONG CSmartCardCredential::Release() {
    LONG c = InterlockedDecrement(&m_cRef);
    if (c == 0) {
        delete this;
    }
    return c;
}

HRESULT CSmartCardCredential::Advise(ICredentialProviderCredentialEvents* pcpce) {
    if (m_pEvents) {
        m_pEvents->Release();
    }
    m_pEvents = pcpce;
    if (m_pEvents) {
        m_pEvents->AddRef();
    }
    return S_OK;
}

HRESULT CSmartCardCredential::UnAdvise() {
    if (m_pEvents) {
        m_pEvents->Release();
        m_pEvents = NULL;
    }
    return S_OK;
}

HRESULT CSmartCardCredential::SetSelected(BOOL* pbAutoLogon) {
    if (pbAutoLogon) {
        *pbAutoLogon = (m_bAutoLogon && m_bCardInserted) ? TRUE : FALSE;
        m_bAutoLogon = false; // Reset to avoid re-triggering loop
    }
    return S_OK;
}

HRESULT CSmartCardCredential::SetDeselected() {
    return S_OK;
}

HRESULT CSmartCardCredential::GetFieldState(DWORD dwFieldID, CREDENTIAL_PROVIDER_FIELD_STATE* pcpfs, CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE* pcpfis) {
    if (!pcpfs || !pcpfis) return E_POINTER;

    switch (dwFieldID) {
    case SFI_TILEIMAGE:
    case SFI_LABEL:
    case SFI_STATUS:
        *pcpfs = CPFS_DISPLAY_IN_SELECTED_TILE;
        *pcpfis = CPFIS_NONE;
        return S_OK;
    case SFI_SUBMIT:
        *pcpfs = m_bCardInserted ? CPFS_DISPLAY_IN_SELECTED_TILE : CPFS_HIDDEN;
        *pcpfis = CPFIS_NONE;
        return S_OK;
    default:
        *pcpfs = CPFS_HIDDEN;
        *pcpfis = CPFIS_NONE;
        return S_OK;
    }
}

HRESULT CSmartCardCredential::GetStringValue(DWORD dwFieldID, LPWSTR* ppsz) {
    if (!ppsz) return E_POINTER;
    switch (dwFieldID) {
    case SFI_LABEL:
        return CoTaskMemAllocString(L"RTS5169 智能卡解锁 (IC4442)", ppsz);
    case SFI_STATUS:
        return CoTaskMemAllocString(m_statusText.c_str(), ppsz);
    default:
        *ppsz = NULL;
        return S_OK;
    }
}

HRESULT CSmartCardCredential::GetBitmapValue(DWORD dwFieldID, HBITMAP* phbmp) {
    if (!phbmp) return E_POINTER;
    *phbmp = NULL; // System will provide default tile glyph if NULL
    return S_OK;
}

HRESULT CSmartCardCredential::GetCheckboxValue(DWORD dwFieldID, BOOL* pbChecked, LPWSTR* ppszLabel) {
    return E_NOTIMPL;
}

HRESULT CSmartCardCredential::GetSubmitButtonValue(DWORD dwFieldID, DWORD* pdwAdjacentTo) {
    if (!pdwAdjacentTo) return E_POINTER;
    if (dwFieldID == SFI_SUBMIT) {
        if (!m_bCardInserted) {
            return E_NOTIMPL;
        }
        *pdwAdjacentTo = SFI_STATUS;
        return S_OK;
    }
    return E_NOTIMPL;
}

HRESULT CSmartCardCredential::GetComboBoxValueCount(DWORD dwFieldID, DWORD* pcItems, DWORD* pdwSelectedItem) {
    return E_NOTIMPL;
}

HRESULT CSmartCardCredential::GetComboBoxValueAt(DWORD dwFieldID, DWORD dwItem, LPWSTR* ppszItem) {
    return E_NOTIMPL;
}

HRESULT CSmartCardCredential::SetStringValue(DWORD dwFieldID, LPCWSTR psz) {
    return E_NOTIMPL;
}

HRESULT CSmartCardCredential::SetCheckboxValue(DWORD dwFieldID, BOOL bChecked) {
    return E_NOTIMPL;
}

HRESULT CSmartCardCredential::SetComboBoxSelectedValue(DWORD dwFieldID, DWORD dwSelectedItem) {
    return E_NOTIMPL;
}

HRESULT CSmartCardCredential::CommandLinkClicked(DWORD dwFieldID) {
    return E_NOTIMPL;
}

HRESULT CSmartCardCredential::GetSerialization(
    CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE* pcpgsr,
    CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION* pcpcs,
    LPWSTR* ppszOptionalStatusText,
    CREDENTIAL_PROVIDER_STATUS_ICON* pcpsiOptionalStatusIcon)
{
    if (!pcpgsr || !pcpcs || !ppszOptionalStatusText || !pcpsiOptionalStatusIcon) {
        return E_POINTER;
    }

    *pcpgsr = CPGSR_NO_CREDENTIAL_FINISHED;
    *ppszOptionalStatusText = NULL;
    *pcpsiOptionalStatusIcon = CPSI_NONE;
    ZeroMemory(pcpcs, sizeof(*pcpcs));

    LogCredProv("GetSerialization 调用触发，正在进行智能卡校验...");

    bool hwPresent = IsHardwareCardPresent();
    if (!m_bCardInserted || !hwPresent) {
        m_bCardInserted = false;
        m_bAutoLogon = false;
        m_statusText = L"请插入卡片进行解锁";
        LogCredProv("卡片未插入");

        CoTaskMemAllocString(L"未检测到卡，请先插入卡片", ppszOptionalStatusText);
        *pcpsiOptionalStatusIcon = CPSI_ERROR;
        *pcpgsr = CPGSR_NO_CREDENTIAL_FINISHED;

        if (m_pEvents) {
            m_pEvents->SetFieldString(this, SFI_STATUS, m_statusText.c_str());
            m_pEvents->SetFieldState(this, SFI_SUBMIT, CPFS_HIDDEN);
        }
        return S_OK;
    }

    LogCredProv("智能卡已确认，正在读取凭据库...");

    StoredCredentials creds = ReadVaultCredentials();
    if (!creds.isValid) {
        LogCredProv("错误: 凭据库未配置或凭据无效！");
        CoTaskMemAllocString(L"智能卡未绑定有效凭据，请先在系统中配置！", ppszOptionalStatusText);
        *pcpsiOptionalStatusIcon = CPSI_ERROR;
        return S_OK;
    }

    // Connect to LSA
    HANDLE hLsa = NULL;
    NTSTATUS status = LsaConnectUntrusted(&hLsa);
    if (status != 0 || !hLsa) {
        LogCredProv("错误: LsaConnectUntrusted 失败！");
        return E_FAIL;
    }

    // Lookup Negotiate package (handles Kerberos / NTLM / CloudAP)
    const char* pkgNameStr = "Negotiate";
    LSA_STRING pkgName;
    pkgName.Buffer = (PCHAR)pkgNameStr;
    pkgName.Length = (USHORT)strlen(pkgNameStr);
    pkgName.MaximumLength = pkgName.Length;

    ULONG authPackage = 0;
    status = LsaLookupAuthenticationPackage(hLsa, &pkgName, &authPackage);
    if (status != 0) {
        // Fallback to MSV1_0
        const char* msv10Str = "MICROSOFT_AUTHENTICATION_PACKAGE_V1_0";
        pkgName.Buffer = (PCHAR)msv10Str;
        pkgName.Length = (USHORT)strlen(msv10Str);
        pkgName.MaximumLength = pkgName.Length;
        status = LsaLookupAuthenticationPackage(hLsa, &pkgName, &authPackage);
    }
    LsaDeregisterLogonProcess(hLsa);

    if (status != 0) {
        LogCredProv("错误: LsaLookupAuthenticationPackage 失败！");
        return E_FAIL;
    }

    // Calculate buffer layout
    size_t cbDomain = creds.domain.length() * sizeof(WCHAR);
    size_t cbUser = creds.username.length() * sizeof(WCHAR);
    size_t cbPassword = creds.password.length() * sizeof(WCHAR);

    BYTE* pBuffer = NULL;
    DWORD cbTotal = 0;

    if (m_cpus == CPUS_UNLOCK_WORKSTATION) {
        // Workstation Unlock scenario requires KERB_INTERACTIVE_UNLOCK_LOGON
        DWORD headerSize = sizeof(KERB_INTERACTIVE_UNLOCK_LOGON); // 64 bytes
        DWORD offsetDomain = headerSize;
        DWORD offsetUser = offsetDomain + (DWORD)(cbDomain + sizeof(WCHAR));
        DWORD offsetPassword = offsetUser + (DWORD)(cbUser + sizeof(WCHAR));
        cbTotal = offsetPassword + (DWORD)(cbPassword + sizeof(WCHAR));

        pBuffer = (BYTE*)CoTaskMemAlloc(cbTotal);
        if (!pBuffer) return E_OUTOFMEMORY;
        ZeroMemory(pBuffer, cbTotal);

        PKERB_INTERACTIVE_UNLOCK_LOGON pUnlock = (PKERB_INTERACTIVE_UNLOCK_LOGON)pBuffer;
        pUnlock->Logon.MessageType = KerbWorkstationUnlockLogon; // 7

        // String buffers in serialization are relative byte offsets from buffer start
        pUnlock->Logon.LogonDomainName.Length = (USHORT)cbDomain;
        pUnlock->Logon.LogonDomainName.MaximumLength = (USHORT)(cbDomain + sizeof(WCHAR));
        pUnlock->Logon.LogonDomainName.Buffer = (PWSTR)(INT_PTR)offsetDomain;
        memcpy(pBuffer + offsetDomain, creds.domain.c_str(), cbDomain);

        pUnlock->Logon.UserName.Length = (USHORT)cbUser;
        pUnlock->Logon.UserName.MaximumLength = (USHORT)(cbUser + sizeof(WCHAR));
        pUnlock->Logon.UserName.Buffer = (PWSTR)(INT_PTR)offsetUser;
        memcpy(pBuffer + offsetUser, creds.username.c_str(), cbUser);

        pUnlock->Logon.Password.Length = (USHORT)cbPassword;
        pUnlock->Logon.Password.MaximumLength = (USHORT)(cbPassword + sizeof(WCHAR));
        pUnlock->Logon.Password.Buffer = (PWSTR)(INT_PTR)offsetPassword;
        memcpy(pBuffer + offsetPassword, creds.password.c_str(), cbPassword);

        pUnlock->LogonId.LowPart = 0;
        pUnlock->LogonId.HighPart = 0;
    } else {
        // Standard interactive logon scenario requires KERB_INTERACTIVE_LOGON
        DWORD headerSize = 64; // Align to 64 bytes
        DWORD offsetDomain = headerSize;
        DWORD offsetUser = offsetDomain + (DWORD)(cbDomain + sizeof(WCHAR));
        DWORD offsetPassword = offsetUser + (DWORD)(cbUser + sizeof(WCHAR));
        cbTotal = offsetPassword + (DWORD)(cbPassword + sizeof(WCHAR));

        pBuffer = (BYTE*)CoTaskMemAlloc(cbTotal);
        if (!pBuffer) return E_OUTOFMEMORY;
        ZeroMemory(pBuffer, cbTotal);

        PKERB_INTERACTIVE_LOGON pLogon = (PKERB_INTERACTIVE_LOGON)pBuffer;
        pLogon->MessageType = KerbInteractiveLogon; // 2

        pLogon->LogonDomainName.Length = (USHORT)cbDomain;
        pLogon->LogonDomainName.MaximumLength = (USHORT)(cbDomain + sizeof(WCHAR));
        pLogon->LogonDomainName.Buffer = (PWSTR)(INT_PTR)offsetDomain;
        memcpy(pBuffer + offsetDomain, creds.domain.c_str(), cbDomain);

        pLogon->UserName.Length = (USHORT)cbUser;
        pLogon->UserName.MaximumLength = (USHORT)(cbUser + sizeof(WCHAR));
        pLogon->UserName.Buffer = (PWSTR)(INT_PTR)offsetUser;
        memcpy(pBuffer + offsetUser, creds.username.c_str(), cbUser);

        pLogon->Password.Length = (USHORT)cbPassword;
        pLogon->Password.MaximumLength = (USHORT)(cbPassword + sizeof(WCHAR));
        pLogon->Password.Buffer = (PWSTR)(INT_PTR)offsetPassword;
        memcpy(pBuffer + offsetPassword, creds.password.c_str(), cbPassword);
    }

    pcpcs->clsidCredentialProvider = CLSID_SmartCardCredentialProvider;
    pcpcs->ulAuthenticationPackage = authPackage;
    pcpcs->cbSerialization = cbTotal;
    pcpcs->rgbSerialization = pBuffer;

    *pcpgsr = CPGSR_RETURN_CREDENTIAL_FINISHED;

    char logBuf[128];
    sprintf_s(logBuf, "GetSerialization 成功构建凭据！Package=%lu, Size=%lu, User=%ls, Scenario=%d",
              authPackage, cbTotal, creds.username.c_str(), (int)m_cpus);
    LogCredProv(logBuf);

    return S_OK;
}

HRESULT CSmartCardCredential::ReportResult(
    NTSTATUS ntsStatus,
    NTSTATUS ntsSubstatus,
    LPWSTR* ppszOptionalStatusText,
    CREDENTIAL_PROVIDER_STATUS_ICON* pcpsiOptionalStatusIcon)
{
    char logBuf[128];
    sprintf_s(logBuf, "ReportResult 认证结果返回: Status=0x%08X, SubStatus=0x%08X", (UINT)ntsStatus, (UINT)ntsSubstatus);
    LogCredProv(logBuf);

    m_bAutoLogon = false;

    if (ntsStatus == 0) {
        LogCredProv("认证成功，已解锁");
    } else {
        LogCredProv("认证失败，请检查凭据是否与 Windows 账户匹配");
        CoTaskMemAllocString(L"智能卡凭据认证失败，请重试或检查绑定设置", ppszOptionalStatusText);
        if (pcpsiOptionalStatusIcon) {
            *pcpsiOptionalStatusIcon = CPSI_ERROR;
        }
    }
    return S_OK;
}

HRESULT CSmartCardCredential::GetUserSid(LPWSTR* sid) {
    if (!sid) return E_POINTER;
    *sid = NULL;
    return E_NOTIMPL; // Returning E_NOTIMPL makes it available across all users on lock screen
}

void CSmartCardCredential::SetCardInserted(bool inserted) {
    m_bCardInserted = inserted;
    if (inserted) {
        m_statusText = L"卡片已插入，按下回车或提交箭头以解锁";
    } else {
        m_statusText = L"请插入卡片进行解锁";
    }

    if (m_pEvents) {
        m_pEvents->SetFieldString(this, SFI_STATUS, m_statusText.c_str());
        m_pEvents->SetFieldState(this, SFI_SUBMIT, inserted ? CPFS_DISPLAY_IN_SELECTED_TILE : CPFS_HIDDEN);
        m_pEvents->SetFieldState(this, SFI_STATUS, CPFS_DISPLAY_IN_SELECTED_TILE);
    }
}

void CSmartCardCredential::TriggerAutoLogon() {
    m_bAutoLogon = true;
    m_bCardInserted = true;
    m_statusText = L"智能卡认证中...";

    if (m_pEvents) {
        m_pEvents->SetFieldString(this, SFI_STATUS, m_statusText.c_str());
        m_pEvents->SetFieldState(this, SFI_SUBMIT, CPFS_DISPLAY_IN_SELECTED_TILE);
        m_pEvents->SetFieldState(this, SFI_STATUS, CPFS_DISPLAY_IN_SELECTED_TILE);
    }
}
