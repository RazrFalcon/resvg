#define INITGUID
#include "common.h"
#include "classfactory.h"

STDAPI CThumbnailProvider_CreateInstance(REFIID riid, void** ppvObject);

CClassFactory::CClassFactory()
{
    DllAddRef();
}

CClassFactory::~CClassFactory()
{
    DllRelease();
}

STDMETHODIMP CClassFactory::QueryInterface(REFIID riid, void** ppvObject)
{
    static const QITAB qit[] =
    {
        QITABENT(CClassFactory, IClassFactory),
        {0},
    };

    return QISearch(this, qit, riid, ppvObject);
}

STDMETHODIMP_(ULONG) CClassFactory::AddRef()
{
    LONG cRef = InterlockedIncrement(&m_cRef);
    return (ULONG)cRef;
}

STDMETHODIMP_(ULONG) CClassFactory::Release()
{
    LONG cRef = InterlockedDecrement(&m_cRef);
    if (0 == cRef) {
        delete this;
    }

    return (ULONG)cRef;
}

STDMETHODIMP CClassFactory::CreateInstance(IUnknown* punkOuter, REFIID riid, void** ppvObject)
{
    if (punkOuter) {
        return CLASS_E_NOAGGREGATION;
    }

    return CThumbnailProvider_CreateInstance(riid, ppvObject);
}

STDMETHODIMP CClassFactory::LockServer(BOOL fLock)
{
    (void)fLock;
    return E_NOTIMPL;
}

STDAPI DllGetClassObject(REFCLSID rclsid, REFIID riid, void **ppv)
{
    if (!ppv) {
        return E_INVALIDARG;
    }

    if (!IsEqualCLSID(CLSID_SvgThumbnailProvider, rclsid)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    CClassFactory *pcf;
    HRESULT hr;

    pcf = new CClassFactory();
    if (!pcf) {
        return E_OUTOFMEMORY;
    }

    hr = pcf->QueryInterface(riid, ppv);
    pcf->Release();

    return hr;
}
