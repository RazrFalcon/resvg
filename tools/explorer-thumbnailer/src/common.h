#pragma once

#include <ShlObj.h>
#include <Shlwapi.h>
#include <strsafe.h>
#include <thumbcache.h>
#include <Windows.h>

STDAPI_(ULONG) DllAddRef();
STDAPI_(ULONG) DllRelease();
STDAPI_(HINSTANCE) DllInstance();

#define szCLSID_SvgThumbnailProvider L"{EF399C53-03F4-489E-98BF-69E00F695ECD}"
DEFINE_GUID(CLSID_SvgThumbnailProvider,
0xef399c53, 0x3f4, 0x489e, 0x98, 0xbf, 0x69, 0xe0, 0xf, 0x69, 0x5e, 0xcd);
