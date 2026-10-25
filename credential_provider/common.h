#pragma once
#ifndef WINVER
#define WINVER 0x0A00
#endif
#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0A00
#endif
#ifndef NTDDI_VERSION
#define NTDDI_VERSION 0x0A000002
#endif

#include <windows.h>
#include <credentialprovider.h>
#include <ntsecapi.h>
#include <winscard.h>
#include <wtsapi32.h>
#include <wincrypt.h>
#include <string>
#include <vector>
#include <fstream>
#include <sstream>

extern LONG g_cRefModule;

inline void DllAddRef() {
    InterlockedIncrement(&g_cRefModule);
}

inline void DllRelease() {
    InterlockedDecrement(&g_cRefModule);
}

inline void LogCredProv(const std::string& msg) {
    char progData[MAX_PATH] = {0};
    if (!GetEnvironmentVariableA("ProgramData", progData, MAX_PATH)) {
        strcpy_s(progData, "C:\\ProgramData");
    }
    std::string path = std::string(progData) + "\\SmartCardUnlock\\credprov.log";

    SYSTEMTIME st;
    GetLocalTime(&st);
    char timeBuf[64];
    sprintf_s(timeBuf, "[%04d-%02d-%02d %02d:%02d:%02d.%03d] ",
              st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, st.wMilliseconds);

    std::ofstream ofs(path.c_str(), std::ios::app);
    if (ofs.is_open()) {
        ofs << timeBuf << msg << "\n";
    }
}

inline HRESULT CoTaskMemAllocString(PCWSTR pszSource, LPWSTR* ppszDest) {
    if (!pszSource || !ppszDest) return E_INVALIDARG;
    size_t cch = wcslen(pszSource) + 1;
    *ppszDest = (LPWSTR)CoTaskMemAlloc(cch * sizeof(WCHAR));
    if (!*ppszDest) return E_OUTOFMEMORY;
    wcscpy_s(*ppszDest, cch, pszSource);
    return S_OK;
}

inline bool IsHardwareCardPresent() {
    SCARDCONTEXT hCtx = 0;
    LONG lRes = SCardEstablishContext(SCARD_SCOPE_SYSTEM, NULL, NULL, &hCtx);
    if (lRes != SCARD_S_SUCCESS) {
        lRes = SCardEstablishContext(SCARD_SCOPE_USER, NULL, NULL, &hCtx);
    }
    if (lRes != SCARD_S_SUCCESS || hCtx == 0) {
        return false;
    }

    DWORD cchReaders = SCARD_AUTOALLOCATE;
    LPWSTR pmszReaders = NULL;
    lRes = SCardListReadersW(hCtx, NULL, (LPWSTR)&pmszReaders, &cchReaders);
    if (lRes != SCARD_S_SUCCESS || !pmszReaders) {
        SCardReleaseContext(hCtx);
        return false;
    }

    WCHAR targetReader[256] = { 0 };
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
    SCardFreeMemory(hCtx, pmszReaders);

    if (targetReader[0] == 0) {
        SCardReleaseContext(hCtx);
        return false;
    }

    SCARD_READERSTATEW state;
    ZeroMemory(&state, sizeof(state));
    state.szReader = targetReader;
    state.dwCurrentState = SCARD_STATE_UNAWARE;

    lRes = SCardGetStatusChangeW(hCtx, 50, &state, 1);
    bool present = false;
    if (lRes == SCARD_S_SUCCESS) {
        present = (state.dwEventState & SCARD_STATE_PRESENT) != 0;
    }

    SCardReleaseContext(hCtx);
    return present;
}

struct StoredCredentials {
    std::wstring username;
    std::wstring domain;
    std::wstring password;
    bool isValid;
};

inline StoredCredentials ReadVaultCredentials() {
    StoredCredentials creds;
    creds.isValid = false;
    creds.domain = L".";

    char progData[MAX_PATH] = {0};
    if (!GetEnvironmentVariableA("ProgramData", progData, MAX_PATH)) {
        strcpy_s(progData, "C:\\ProgramData");
    }
    std::string path = std::string(progData) + "\\SmartCardUnlock\\vault.dat";

    std::ifstream ifs(path.c_str(), std::ios::binary);
    if (!ifs.is_open()) {
        LogCredProv("无法打开凭据库文件: " + path);
        return creds;
    }

    std::vector<BYTE> encrypted((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());
    ifs.close();

    if (encrypted.empty()) {
        LogCredProv("凭据库文件为空");
        return creds;
    }

    DATA_BLOB inBlob;
    inBlob.cbData = (DWORD)encrypted.size();
    inBlob.pbData = encrypted.data();

    DATA_BLOB outBlob = { 0, NULL };

    if (!CryptUnprotectData(&inBlob, NULL, NULL, NULL, NULL, 0, &outBlob)) {
        DWORD err = GetLastError();
        char buf[64];
        sprintf_s(buf, "CryptUnprotectData 失败 (错误码: %lu)", err);
        LogCredProv(buf);
        return creds;
    }

    std::string secret((char*)outBlob.pbData, outBlob.cbData);
    LocalFree(outBlob.pbData);

    // Parse payload: username\npassword or single PIN/password
    size_t newlinePos = secret.find('\n');
    if (newlinePos != std::string::npos) {
        std::string u = secret.substr(0, newlinePos);
        std::string p = secret.substr(newlinePos + 1);
        while (!u.empty() && (u.back() == '\r' || u.back() == ' ')) u.pop_back();
        while (!p.empty() && (p.back() == '\r' || p.back() == '\n' || p.back() == ' ')) p.pop_back();

        int cchU = MultiByteToWideChar(CP_UTF8, 0, u.c_str(), -1, NULL, 0);
        std::vector<WCHAR> wU(cchU);
        MultiByteToWideChar(CP_UTF8, 0, u.c_str(), -1, wU.data(), cchU);
        creds.username = wU.data();

        int cchP = MultiByteToWideChar(CP_UTF8, 0, p.c_str(), -1, NULL, 0);
        std::vector<WCHAR> wP(cchP);
        MultiByteToWideChar(CP_UTF8, 0, p.c_str(), -1, wP.data(), cchP);
        creds.password = wP.data();
    } else {
        // Single secret (PIN or password): determine username from active console session
        while (!secret.empty() && (secret.back() == '\r' || secret.back() == '\n' || secret.back() == ' ')) secret.pop_back();
        int cchP = MultiByteToWideChar(CP_UTF8, 0, secret.c_str(), -1, NULL, 0);
        std::vector<WCHAR> wP(cchP);
        MultiByteToWideChar(CP_UTF8, 0, secret.c_str(), -1, wP.data(), cchP);
        creds.password = wP.data();

        // Query active console username
        DWORD sessId = WTSGetActiveConsoleSessionId();
        LPWSTR pUser = NULL;
        DWORD bytesRet = 0;
        if (WTSQuerySessionInformationW(WTS_CURRENT_SERVER_HANDLE, sessId, WTSUserName, &pUser, &bytesRet) && pUser) {
            creds.username = pUser;
            WTSFreeMemory(pUser);
        } else {
            creds.username = L"black"; // Fallback to current detected user
        }
    }

    if (!creds.username.empty() && !creds.password.empty()) {
        creds.isValid = true;
    }

    return creds;
}
