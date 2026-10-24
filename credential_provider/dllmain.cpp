#include "common.h"
#include "guid.h"
#include "CClassFactory.h"
#include <shlwapi.h>

LONG g_cRefModule = 0;
HINSTANCE g_hInst = NULL;

static const WCHAR s_szCLSID[] = L"{7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}";
static const WCHAR s_szProviderName[] = L"SmartCardCredentialProvider";

BOOL WINAPI DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpReserved) {
    if (fdwReason == DLL_PROCESS_ATTACH) {
        g_hInst = hinstDLL;
        DisableThreadLibraryCalls(hinstDLL);
    }
    return TRUE;
}

STDAPI DllGetClassObject(REFCLSID rclsid, REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = NULL;

    if (!IsEqualCLSID(rclsid, CLSID_SmartCardCredentialProvider)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    CClassFactory* pFactory = new CClassFactory();
    if (!pFactory) {
        return E_OUTOFMEMORY;
    }

    HRESULT hr = pFactory->QueryInterface(riid, ppv);
    pFactory->Release();
    return hr;
}

STDAPI DllCanUnloadNow(void) {
    return (g_cRefModule == 0) ? S_OK : S_FALSE;
}

static HRESULT CreateRegKeyAndSetValue(HKEY hKeyRoot, LPCWSTR subKey, LPCWSTR valueName, LPCWSTR data) {
    HKEY hKey;
    LONG lRes = RegCreateKeyExW(hKeyRoot, subKey, 0, NULL, REG_OPTION_NON_VOLATILE, KEY_WRITE, NULL, &hKey, NULL);
    if (lRes != ERROR_SUCCESS) return HRESULT_FROM_WIN32(lRes);

    if (data) {
        lRes = RegSetValueExW(hKey, valueName, 0, REG_SZ, (const BYTE*)data, (DWORD)((wcslen(data) + 1) * sizeof(WCHAR)));
    }
    RegCloseKey(hKey);
    return HRESULT_FROM_WIN32(lRes);
}

STDAPI DllRegisterServer(void) {
    WCHAR szModule[MAX_PATH];
    if (GetModuleFileNameW(g_hInst, szModule, MAX_PATH) == 0) {
        return HRESULT_FROM_WIN32(GetLastError());
    }

    WCHAR szKey[MAX_PATH];

    // 1. HKLM\SOFTWARE\Classes\CLSID\{7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}
    swprintf_s(szKey, MAX_PATH, L"SOFTWARE\\Classes\\CLSID\\%ls", s_szCLSID);
    HRESULT hr = CreateRegKeyAndSetValue(HKEY_LOCAL_MACHINE, szKey, NULL, s_szProviderName);
    if (FAILED(hr)) return hr;

    // InprocServer32
    swprintf_s(szKey, MAX_PATH, L"SOFTWARE\\Classes\\CLSID\\%ls\\InprocServer32", s_szCLSID);
    hr = CreateRegKeyAndSetValue(HKEY_LOCAL_MACHINE, szKey, NULL, szModule);
    if (FAILED(hr)) return hr;
    hr = CreateRegKeyAndSetValue(HKEY_LOCAL_MACHINE, szKey, L"ThreadingModel", L"Apartment");
    if (FAILED(hr)) return hr;

    // 2. HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}
    swprintf_s(szKey, MAX_PATH, L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Credential Providers\\%ls", s_szCLSID);
    hr = CreateRegKeyAndSetValue(HKEY_LOCAL_MACHINE, szKey, NULL, s_szProviderName);
    if (FAILED(hr)) return hr;

    LogCredProv("DllRegisterServer: 成功向系统注册 SmartCardCredentialProvider！");
    return S_OK;
}

STDAPI DllUnregisterServer(void) {
    WCHAR szKey[MAX_PATH];

    // 1. Delete Credential Provider entry
    swprintf_s(szKey, MAX_PATH, L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Credential Providers\\%ls", s_szCLSID);
    RegDeleteTreeW(HKEY_LOCAL_MACHINE, szKey);

    // 2. Delete CLSID entry
    swprintf_s(szKey, MAX_PATH, L"SOFTWARE\\Classes\\CLSID\\%ls", s_szCLSID);
    RegDeleteTreeW(HKEY_LOCAL_MACHINE, szKey);

    LogCredProv("DllUnregisterServer: 成功从系统注销 SmartCardCredentialProvider。");
    return S_OK;
}
