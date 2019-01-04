#define INITGUID
#include "common.h"

#include <QGuiApplication>


HINSTANCE g_hinstDll = nullptr;
LONG g_cRef = 0;


typedef struct _REGKEY_DELETEKEY
{
    HKEY hKey;
    LPCWSTR lpszSubKey;
} REGKEY_DELETEKEY;


typedef struct _REGKEY_SUBKEY_AND_VALUE
{
    HKEY hKey;
    LPCWSTR lpszSubKey;
    LPCWSTR lpszValue;
    DWORD dwType;
    DWORD_PTR dwData;
} REGKEY_SUBKEY_AND_VALUE;

STDAPI CreateRegistryKeys(REGKEY_SUBKEY_AND_VALUE* aKeys, ULONG cKeys);
STDAPI DeleteRegistryKeys(REGKEY_DELETEKEY* aKeys, ULONG cKeys);

BOOL APIENTRY DllMain(HINSTANCE hinstDll, DWORD dwReason, LPVOID pvReserved)
{
    Q_UNUSED(pvReserved)

    if (dwReason == DLL_PROCESS_ATTACH) {
        g_hinstDll = hinstDll;

        // TODO: This is ultra bad, but I have no idea how to fix this.
        // The problem is that Qt have to load platform plugins,
        // but the current directory is C:/WINDOWS/system32
        // and not the install one.
        //
        // At the moment, the installer will disallow the install directory change.
        QCoreApplication::addLibraryPath("C:/Program Files/reSVG Explorer Extension");

        int argc = 1;
        new QGuiApplication(argc, nullptr);
    }

    return TRUE;
}

STDAPI_(HINSTANCE) DllInstance()
{
    return g_hinstDll;
}

STDAPI DllCanUnloadNow()
{
    return g_cRef ? S_FALSE : S_OK;
}

STDAPI_(ULONG) DllAddRef()
{
    LONG cRef = InterlockedIncrement(&g_cRef);
    return cRef;
}

STDAPI_(ULONG) DllRelease()
{
    LONG cRef = InterlockedDecrement(&g_cRef);
    if (0 > cRef) {
        cRef = 0;
    }

    return cRef;
}

STDAPI DllRegisterServer()
{
    WCHAR szModule[MAX_PATH];

    ZeroMemory(szModule, sizeof(szModule));
    GetModuleFileName(g_hinstDll, szModule, ARRAYSIZE(szModule));

    REGKEY_SUBKEY_AND_VALUE keys[] = {
        {HKEY_CLASSES_ROOT, L"CLSID\\" szCLSID_SvgThumbnailProvider, nullptr, REG_SZ, (DWORD_PTR)L"SVG Thumbnail Provider"},
        {HKEY_CLASSES_ROOT, L"CLSID\\" szCLSID_SvgThumbnailProvider L"\\InprocServer32", nullptr, REG_SZ, (DWORD_PTR)szModule},
        {HKEY_CLASSES_ROOT, L"CLSID\\" szCLSID_SvgThumbnailProvider L"\\InprocServer32", L"ThreadingModel", REG_SZ, (DWORD_PTR)L"Apartment"},
        {HKEY_CLASSES_ROOT, L".SVG\\shellex\\{E357FCCD-A995-4576-B01F-234630154E96}", nullptr, REG_SZ, (DWORD_PTR)szCLSID_SvgThumbnailProvider}
    };

    return CreateRegistryKeys(keys, ARRAYSIZE(keys));
}

STDAPI DllUnregisterServer()
{
    REGKEY_DELETEKEY keys[] = {{HKEY_CLASSES_ROOT, L"CLSID\\" szCLSID_SvgThumbnailProvider}};
    return DeleteRegistryKeys(keys, ARRAYSIZE(keys));
}

STDAPI CreateRegistryKey(REGKEY_SUBKEY_AND_VALUE* pKey)
{
    size_t cbData = 0;
    LPVOID pvData = nullptr;
    HRESULT hr = S_OK;

    switch(pKey->dwType)
    {
    case REG_DWORD:
        pvData = (LPVOID)(LPDWORD)&pKey->dwData;
        cbData = sizeof(DWORD);
        break;

    case REG_SZ:
    case REG_EXPAND_SZ:
        hr = StringCbLength((LPCWSTR)pKey->dwData, STRSAFE_MAX_CCH, &cbData);
        if (SUCCEEDED(hr)) {
            pvData = (LPVOID)(LPCWSTR)pKey->dwData;
            cbData += sizeof(WCHAR);
        }
        break;

    default:
        hr = E_INVALIDARG;
    }

    if (SUCCEEDED(hr)) {
        LSTATUS status = SHSetValue(pKey->hKey, pKey->lpszSubKey, pKey->lpszValue,
                                    pKey->dwType, pvData, (DWORD)cbData);
        if (status != NOERROR) {
            hr = HRESULT_FROM_WIN32(status);
        }
    }

    return hr;
}

STDAPI CreateRegistryKeys(REGKEY_SUBKEY_AND_VALUE* aKeys, ULONG cKeys)
{
    HRESULT hr = S_OK;

    for (ULONG iKey = 0; iKey < cKeys; iKey++) {
        HRESULT hrTemp = CreateRegistryKey(&aKeys[iKey]);
        if (FAILED(hrTemp)) {
            hr = hrTemp;
        }
    }

    return hr;
}

STDAPI DeleteRegistryKeys(REGKEY_DELETEKEY* aKeys, ULONG cKeys)
{
    HRESULT hr = S_OK;
    LSTATUS status;

    for (ULONG iKey = 0; iKey < cKeys; iKey++) {
        status = RegDeleteTree(aKeys[iKey].hKey, aKeys[iKey].lpszSubKey);
        if (status != NOERROR) {
            hr = HRESULT_FROM_WIN32(status);
        }
    }

    return hr;
}
