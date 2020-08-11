#pragma once

#include <Windows.h>

class CClassFactory : public IClassFactory
{
public:
    CClassFactory();

    // IUnknown methods
    STDMETHOD(QueryInterface)(REFIID, void**);
    STDMETHOD_(ULONG, AddRef)();
    STDMETHOD_(ULONG, Release)();

    // IClassFactory methods
    STDMETHOD(CreateInstance)(IUnknown*, REFIID, void**);
    STDMETHOD(LockServer)(BOOL);

private:
    ~CClassFactory();

private:
    LONG m_cRef = 1;
};
