#include "common.h"
#include "thumbnailprovider.h"

#include "gdiplus.h"

#include <QtWinExtras/QtWin>

using namespace Gdiplus;

CThumbnailProvider::CThumbnailProvider()
{
    DllAddRef();
    m_cRef = 1;
}

CThumbnailProvider::~CThumbnailProvider()
{
    if (m_pSite) {
        m_pSite->Release();
        m_pSite = nullptr;
    }

    DllRelease();
}

STDMETHODIMP CThumbnailProvider::QueryInterface(REFIID riid, void** ppvObject)
{
    static const QITAB qit[] =
    {
        QITABENT(CThumbnailProvider, IInitializeWithStream),
        QITABENT(CThumbnailProvider, IThumbnailProvider),
        QITABENT(CThumbnailProvider, IObjectWithSite),
        {0},
    };
    return QISearch(this, qit, riid, ppvObject);
}

STDMETHODIMP_(ULONG) CThumbnailProvider::AddRef()
{
    LONG cRef = InterlockedIncrement(&m_cRef);
    return (ULONG)cRef;
}

STDMETHODIMP_(ULONG) CThumbnailProvider::Release()
{
    LONG cRef = InterlockedDecrement(&m_cRef);
    if (cRef == 0) {
        delete this;
    }

    return (ULONG)cRef;
}

STDMETHODIMP CThumbnailProvider::Initialize(IStream *pstm, DWORD grfMode)
{
    Q_UNUSED(grfMode)

    STATSTG stat;
    if (pstm->Stat(&stat, STATFLAG_DEFAULT) != S_OK) {
        return S_FALSE;
    }

    char *data = new char[stat.cbSize.QuadPart];

    ULONG len;
    if (pstm->Read(data, stat.cbSize.QuadPart, &len) != S_OK) {
        return S_FALSE;
    }

    // TODO: find a way to get the current file path,
    // which will allow relative images resolving.

    m_renderer.load(QByteArray(data, stat.cbSize.QuadPart));

    return S_OK;
}

STDMETHODIMP CThumbnailProvider::GetThumbnail(UINT cx, HBITMAP *phbmp, WTS_ALPHATYPE *pdwAlpha)
{
    *phbmp = nullptr;
    *pdwAlpha = WTSAT_ARGB;

    if (!m_renderer.isValid()) {
        return E_NOTIMPL;
    }

    QSize size = m_renderer.defaultSize();

    int width, height;
    if (size.width() == size.height()){
        width = cx;
        height = cx;
    } else if (size.width() > size.height()){
        width = cx;
        height = size.height() * ((double)cx / (double)size.width());
    } else {
        width = size.width() * ((double)cx / (double)size.height());
        height = cx;
    }

    QImage img(width, height, QImage::Format_ARGB32);
    img.fill(Qt::transparent);

    QPainter painter(&img);
    m_renderer.render(&painter);
    painter.end();

    *phbmp = QtWin::toHBITMAP(QPixmap::fromImage(img), QtWin::HBitmapAlpha);

    if (*phbmp != nullptr) {
        return NOERROR;
    }

    return E_NOTIMPL;
}

STDMETHODIMP CThumbnailProvider::GetSite(REFIID riid, void** ppvSite)
{
    if (m_pSite) {
        return m_pSite->QueryInterface(riid, ppvSite);
    }

    return E_NOINTERFACE;
}

STDMETHODIMP CThumbnailProvider::SetSite(IUnknown* pUnkSite)
{
    if (m_pSite) {
        m_pSite->Release();
        m_pSite = nullptr;
    }

    m_pSite = pUnkSite;

    if (m_pSite) {
        m_pSite->AddRef();
    }

    return S_OK;
}

STDAPI CThumbnailProvider_CreateInstance(REFIID riid, void** ppvObject)
{
    *ppvObject = nullptr;

    CThumbnailProvider* ptp = new CThumbnailProvider();
    if (!ptp) {
        return E_OUTOFMEMORY;
    }

    HRESULT hr = ptp->QueryInterface(riid, ppvObject);
    ptp->Release();
    return hr;
}
