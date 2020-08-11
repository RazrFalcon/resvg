#pragma once

#include <ResvgQt.h>

#include <thumbcache.h>

class CThumbnailProvider : public IThumbnailProvider, IObjectWithSite, IInitializeWithStream
{
public:
    CThumbnailProvider();

    // IUnknown methods
    STDMETHOD(QueryInterface)(REFIID, void**);
    STDMETHOD_(ULONG, AddRef)();
    STDMETHOD_(ULONG, Release)();

    // IInitializeWithStream methods
    STDMETHOD(Initialize)(IStream*, DWORD);

    // IThumbnailProvider methods
    STDMETHOD(GetThumbnail)(UINT, HBITMAP*, WTS_ALPHATYPE*);

    // IObjectWithSite methods
    STDMETHOD(GetSite)(REFIID, void**);
    STDMETHOD(SetSite)(IUnknown*);

private:
    ~CThumbnailProvider();

private:
    LONG m_cRef;
    IUnknown* m_pSite = nullptr;
    ResvgRenderer m_renderer;
};
