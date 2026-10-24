#include "CSmartCardProvider.h"
#include "guid.h"
#include <process.h>

static const CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR s_rgFieldDescriptors[] = {
    { SFI_TILEIMAGE, CPFT_TILE_IMAGE,    const_cast<LPWSTR>(L"") },
    { SFI_LABEL,     CPFT_LARGE_TEXT,     const_cast<LPWSTR>(L"智能卡") },
    { SFI_STATUS,    CPFT_SMALL_TEXT,     const_cast<LPWSTR>(L"状态") },
    { SFI_SUBMIT,    CPFT_SUBMIT_BUTTON,  const_cast<LPWSTR>(L"解锁") },
};

static unsigned __stdcall CardThreadProc(void* param) {
    CSmartCardProvider* pThis = static_cast<CSmartCardProvider*>(param);
    if (pThis) {
        pThis->MonitorCardLoop();
    }
    return 0;
}

CSmartCardProvider::CSmartCardProvider() :
    m_cRef(1),
    m_cpus(CPUS_INVALID),
    m_pEvents(NULL),
    m_upAdviseContext(0),
    m_pCredential(NULL),
    m_hThread(NULL),
    m_hStopEvent(NULL),
    m_bAutoLogon(false)
{
    m_pCredential = new CSmartCardCredential();
    m_hStopEvent = CreateEvent(NULL, TRUE, FALSE, NULL);
    DllAddRef();
}

CSmartCardProvider::~CSmartCardProvider() {
    UnAdvise();

    if (m_hStopEvent) {
        CloseHandle(m_hStopEvent);
        m_hStopEvent = NULL;
    }
    if (m_pCredential) {
        m_pCredential->Release();
        m_pCredential = NULL;
    }
    DllRelease();
}

HRESULT CSmartCardProvider::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = NULL;

    if (riid == IID_IUnknown || riid == IID_ICredentialProvider) {
        *ppv = static_cast<ICredentialProvider*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

ULONG CSmartCardProvider::AddRef() {
    return InterlockedIncrement(&m_cRef);
}

ULONG CSmartCardProvider::Release() {
    LONG c = InterlockedDecrement(&m_cRef);
    if (c == 0) {
        delete this;
    }
    return c;
}

HRESULT CSmartCardProvider::SetUsageScenario(CREDENTIAL_PROVIDER_USAGE_SCENARIO cpus, DWORD dwFlags) {
    m_cpus = cpus;
    if (m_pCredential) {
        m_pCredential->Release();
    }
    m_pCredential = new CSmartCardCredential(cpus);

    char logBuf[128];
    sprintf_s(logBuf, "SetUsageScenario: %d, Flags: 0x%08X", (int)cpus, (UINT)dwFlags);
    LogCredProv(logBuf);

    // Support both workstation unlock and logon
    switch (cpus) {
    case CPUS_LOGON:
    case CPUS_UNLOCK_WORKSTATION:
        return S_OK;
    default:
        return E_NOTIMPL;
    }
}

HRESULT CSmartCardProvider::SetSerialization(const CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION* pcpcs) {
    return S_OK;
}

HRESULT CSmartCardProvider::Advise(ICredentialProviderEvents* pcpe, UINT_PTR upAdviseContext) {
    LogCredProv("Advise: 注册事件监听，启动智能卡状态监控线程...");
    if (m_pEvents) {
        m_pEvents->Release();
    }
    m_pEvents = pcpe;
    m_upAdviseContext = upAdviseContext;
    if (m_pEvents) {
        m_pEvents->AddRef();
    }

    if (m_hStopEvent) {
        ResetEvent(m_hStopEvent);
    }

    if (!m_hThread) {
        m_hThread = (HANDLE)_beginthreadex(NULL, 0, CardThreadProc, this, 0, NULL);
    }

    return S_OK;
}

HRESULT CSmartCardProvider::UnAdvise() {
    LogCredProv("UnAdvise: 停止智能卡监听线程...");
    if (m_hStopEvent) {
        SetEvent(m_hStopEvent);
    }
    if (m_hThread) {
        WaitForSingleObject(m_hThread, 1500);
        CloseHandle(m_hThread);
        m_hThread = NULL;
    }
    if (m_pEvents) {
        m_pEvents->Release();
        m_pEvents = NULL;
    }
    return S_OK;
}

HRESULT CSmartCardProvider::GetFieldDescriptorCount(DWORD* pdwCount) {
    if (!pdwCount) return E_POINTER;
    *pdwCount = SFI_NUM_FIELDS;
    return S_OK;
}

HRESULT CSmartCardProvider::GetFieldDescriptorAt(DWORD dwIndex, CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR** ppcpfd) {
    if (!ppcpfd || dwIndex >= SFI_NUM_FIELDS) return E_INVALIDARG;

    CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR* pcpfd = (CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR*)CoTaskMemAlloc(sizeof(CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR));
    if (!pcpfd) return E_OUTOFMEMORY;

    *pcpfd = s_rgFieldDescriptors[dwIndex];
    HRESULT hr = CoTaskMemAllocString(s_rgFieldDescriptors[dwIndex].pszLabel, &pcpfd->pszLabel);
    if (FAILED(hr)) {
        CoTaskMemFree(pcpfd);
        return hr;
    }

    *ppcpfd = pcpfd;
    return S_OK;
}

HRESULT CSmartCardProvider::GetCredentialCount(DWORD* pdwCount, DWORD* pdwDefault, BOOL* pbAutoLogonWithDefault) {
    if (!pdwCount || !pdwDefault || !pbAutoLogonWithDefault) return E_POINTER;

    *pdwCount = 1;
    *pdwDefault = 0;
    *pbAutoLogonWithDefault = m_bAutoLogon ? TRUE : FALSE;

    // Reset auto-logon trigger to prevent infinite spinning loops on authentication failure
    m_bAutoLogon = false;

    char logBuf[128];
    sprintf_s(logBuf, "GetCredentialCount: Count=1, AutoLogon=%d", *pbAutoLogonWithDefault ? 1 : 0);
    LogCredProv(logBuf);

    return S_OK;
}

HRESULT CSmartCardProvider::GetCredentialAt(DWORD dwIndex, ICredentialProviderCredential** ppcpc) {
    if (!ppcpc || dwIndex != 0 || !m_pCredential) return E_INVALIDARG;
    m_pCredential->AddRef();
    *ppcpc = m_pCredential;
    return S_OK;
}

void CSmartCardProvider::MonitorCardLoop() {
    SCARDCONTEXT hContext = 0;
    LONG lRes = SCardEstablishContext(SCARD_SCOPE_SYSTEM, NULL, NULL, &hContext);
    if (lRes != SCARD_S_SUCCESS) {
        lRes = SCardEstablishContext(SCARD_SCOPE_USER, NULL, NULL, &hContext);
    }
    if (lRes != SCARD_S_SUCCESS) {
        LogCredProv("智能卡监控: SCardEstablishContext 失败");
        return;
    }

    LogCredProv("智能卡监控线程运行中 (PC/SC WinSCard)");

    WCHAR targetReader[256] = {0};
    DWORD cchReaders = SCARD_AUTOALLOCATE;
    LPWSTR pmszReaders = NULL;

    bool lastPresent = false;
    bool initialCheck = true;

    while (WaitForSingleObject(m_hStopEvent, 200) == WAIT_TIMEOUT) {
        // Enumerate readers if not found
        if (targetReader[0] == 0) {
            lRes = SCardListReadersW(hContext, NULL, (LPWSTR)&pmszReaders, &cchReaders);
            if (lRes == SCARD_S_SUCCESS && pmszReaders) {
                LPCWSTR p = pmszReaders;
                while (*p) {
                    std::wstring rname(p);
                    if (rname.find(L"EMV") != std::wstring::npos ||
                        rname.find(L"RTS") != std::wstring::npos ||
                        rname.find(L"Realtek") != std::wstring::npos ||
                        targetReader[0] == 0) {
                        wcscpy_s(targetReader, p);
                    }
                    p += wcslen(p) + 1;
                }
                SCardFreeMemory(hContext, pmszReaders);
                pmszReaders = NULL;
                cchReaders = SCARD_AUTOALLOCATE;

                if (targetReader[0] != 0) {
                    char buf[300];
                    sprintf_s(buf, "找到智能卡读卡器: %ls", targetReader);
                    LogCredProv(buf);
                }
            }
            if (targetReader[0] == 0) {
                continue;
            }
        }

        // Query reader status
        SCARD_READERSTATEW state;
        ZeroMemory(&state, sizeof(state));
        state.szReader = targetReader;
        state.dwCurrentState = SCARD_STATE_UNAWARE;

        lRes = SCardGetStatusChangeW(hContext, 100, &state, 1);
        if (lRes == SCARD_S_SUCCESS) {
            bool isPresent = (state.dwEventState & SCARD_STATE_PRESENT) != 0;

            if (initialCheck) {
                initialCheck = false;
                lastPresent = isPresent;
                if (m_pCredential) {
                    m_pCredential->SetCardInserted(isPresent);
                }
            } else if (isPresent != lastPresent) {
                lastPresent = isPresent;
                LogCredProv(isPresent ? "【硬件事件】检测到 IC4442 卡片插入！" : "【硬件事件】检测到 IC4442 卡片拔出。");

                if (m_pCredential) {
                    m_pCredential->SetCardInserted(isPresent);
                }

                if (isPresent) {
                    // Card inserted -> Trigger Auto Unlock
                    m_bAutoLogon = true;
                    if (m_pCredential) {
                        m_pCredential->TriggerAutoLogon();
                    }

                    if (m_pEvents) {
                        LogCredProv("向 LogonUI 派发 CredentialsChanged 事件，触发系统自动解锁...");
                        m_pEvents->CredentialsChanged(m_upAdviseContext);
                    }
                }
            }
        } else if (lRes == SCARD_E_UNKNOWN_READER || lRes == (LONG)SCARD_E_NO_READERS_AVAILABLE) {
            targetReader[0] = 0; // reset reader
        }
    }

    SCardReleaseContext(hContext);
    LogCredProv("智能卡监控线程安全退出。");
}
