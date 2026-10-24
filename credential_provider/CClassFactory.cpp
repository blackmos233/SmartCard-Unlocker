#include "CClassFactory.h"
#include "CSmartCardProvider.h"

CClassFactory::CClassFactory() : m_cRef(1) {
    DllAddRef();
}

CClassFactory::~CClassFactory() {
    DllRelease();
}

HRESULT CClassFactory::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = NULL;

    if (riid == IID_IUnknown || riid == IID_IClassFactory) {
        *ppv = static_cast<IClassFactory*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

ULONG CClassFactory::AddRef() {
    return InterlockedIncrement(&m_cRef);
}

ULONG CClassFactory::Release() {
    LONG c = InterlockedDecrement(&m_cRef);
    if (c == 0) {
        delete this;
    }
    return c;
}

HRESULT CClassFactory::CreateInstance(IUnknown* pUnkOuter, REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = NULL;

    if (pUnkOuter != NULL) {
        return CLASS_E_NOAGGREGATION;
    }

    CSmartCardProvider* pProvider = new CSmartCardProvider();
    if (!pProvider) {
        return E_OUTOFMEMORY;
    }

    HRESULT hr = pProvider->QueryInterface(riid, ppv);
    pProvider->Release();
    return hr;
}

HRESULT CClassFactory::LockServer(BOOL fLock) {
    if (fLock) {
        DllAddRef();
    } else {
        DllRelease();
    }
    return S_OK;
}
