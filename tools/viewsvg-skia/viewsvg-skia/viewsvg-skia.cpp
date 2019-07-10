// viewsvg-skia.cpp : Defines the entry point for the application.
//

#include "stdafx.h"
#include "viewsvg-skia.h"
#include <Commdlg.h>
#include <stdint.h>
#include "svgview.h"
#include <gl/GL.h>
#include "gl/glcorearb.h"
#include "gl/wglext.h"

#define MAX_LOADSTRING 100

// Global Variables:
HINSTANCE hInst;                                // current instance
WCHAR szTitle[MAX_LOADSTRING];                  // The title bar text
WCHAR szWindowClass[MAX_LOADSTRING];            // the main window class name
HWND hWnd_;										// Main window handle
HDC	hdc_;										// Main window display context.
HGLRC glContext_;								// GL render context.
SvgView svgView_;
bool fitToWindow_;
BOOL quit_;

// Forward declarations of functions included in this code module:
ATOM                MyRegisterClass(HINSTANCE hInstance);
BOOL                InitInstance(HINSTANCE, int);
LRESULT CALLBACK    WndProc(HWND, UINT, WPARAM, LPARAM);
INT_PTR CALLBACK    About(HWND, UINT, WPARAM, LPARAM);
int					CreateGLContext(int swapInterval);
void				ResizeViewToClient(HWND hWnd);
bool				GetFitToWindow(HWND hWnd);

int APIENTRY wWinMain(_In_ HINSTANCE hInstance,
                     _In_opt_ HINSTANCE hPrevInstance,
                     _In_ LPWSTR    lpCmdLine,
                     _In_ int       nCmdShow)
{
    UNREFERENCED_PARAMETER(hPrevInstance);
    UNREFERENCED_PARAMETER(lpCmdLine);

    // Initialize global strings
    LoadStringW(hInstance, IDS_APP_TITLE, szTitle, MAX_LOADSTRING);
    LoadStringW(hInstance, IDC_VIEWSVGSKIA, szWindowClass, MAX_LOADSTRING);
    MyRegisterClass(hInstance);

    // Perform application initialization:
    if (!InitInstance (hInstance, nCmdShow)) {
        return FALSE;
    }

	hdc_ = ::GetDC(hWnd_);
	glContext_ = NULL;
	CreateGLContext(1);

	if (!svgView_.init()) {
		return FALSE;
	}

	svgView_.loadFile("..\\..\\..\\capi\\skiatests\\drawing-7-viewbox.svg");
	ResizeViewToClient(hWnd_);

	fitToWindow_ = GetFitToWindow(hWnd_);

	ShowWindow(hWnd_, nCmdShow);
	UpdateWindow(hWnd_);

    HACCEL hAccelTable = LoadAccelerators(hInstance, MAKEINTRESOURCE(IDC_VIEWSVGSKIA));

    MSG msg;

    // Main message loop:
	while (true) {

		if (::PeekMessage(&msg, NULL, 0, 0, PM_REMOVE)) {

			if (msg.message == WM_QUIT) {
				break;
			}

			if (!TranslateAccelerator(msg.hwnd, hAccelTable, &msg))
			{
				::TranslateMessage(&msg);
				::DispatchMessage(&msg);
			}
		}


		svgView_.render(fitToWindow_);
	
		::SwapBuffers(hdc_);
	}

	if (glContext_) {
		wglDeleteContext(glContext_);
	}

	if (hWnd_ && hdc_) {
		::ReleaseDC(hWnd_, hdc_);
	}

    return (int) msg.wParam;
}

//
//  FUNCTION: MyRegisterClass()
//
//  PURPOSE: Registers the window class.
//
ATOM MyRegisterClass(HINSTANCE hInstance)
{
    WNDCLASSEXW wcex;

    wcex.cbSize = sizeof(WNDCLASSEX);

    wcex.style          = CS_HREDRAW | CS_VREDRAW;
    wcex.lpfnWndProc    = WndProc;
    wcex.cbClsExtra     = 0;
    wcex.cbWndExtra     = 0;
    wcex.hInstance      = hInstance;
    wcex.hIcon          = LoadIcon(hInstance, MAKEINTRESOURCE(IDI_VIEWSVGSKIA));
    wcex.hCursor        = LoadCursor(nullptr, IDC_ARROW);
    wcex.hbrBackground  = (HBRUSH)(COLOR_WINDOW+1);
    wcex.lpszMenuName   = MAKEINTRESOURCEW(IDC_VIEWSVGSKIA);
    wcex.lpszClassName  = szWindowClass;
    wcex.hIconSm        = LoadIcon(wcex.hInstance, MAKEINTRESOURCE(IDI_SMALL));

    return RegisterClassExW(&wcex);
}

//
//   FUNCTION: InitInstance(HINSTANCE, int)
//
//   PURPOSE: Saves instance handle and creates main window
//
//   COMMENTS:
//
//        In this function, we save the instance handle in a global variable and
//        create and display the main program window.
//
BOOL InitInstance(HINSTANCE hInstance, int nCmdShow)
{
   hInst = hInstance; // Store instance handle in our global variable

   hWnd_ = CreateWindowW(szWindowClass, szTitle, WS_OVERLAPPEDWINDOW,
      CW_USEDEFAULT, 0, CW_USEDEFAULT, 0, nullptr, nullptr, hInstance, nullptr);

   if (!hWnd_)
   {
      return FALSE;
   }
 
   return TRUE;
}


static int IsExtensionSupported(const char * extname)
{
	PFNGLGETSTRINGIPROC glGetStringi = (PFNGLGETSTRINGIPROC)wglGetProcAddress("glGetStringi");
	if (!glGetStringi) {
		//LOG(WARN, "glGetStringi not supported");
	}
	else {
		GLint numExtensions;
		GLint i;
		glGetIntegerv(GL_NUM_EXTENSIONS, &numExtensions);

		for (i = 0; i < numExtensions; i++) {
			const GLubyte * e = glGetStringi(GL_EXTENSIONS, i);
			if (!strcmp((const char *)e, extname)) {
				return 0;
			}
		}
	}
	return -1;
}

int SetSwapInterval(int swapInterval)
{
	int retVal = -1;
	if (IsExtensionSupported("WGL_EXT_swap_control") == 0) {
		PFNWGLSWAPINTERVALEXTPROC wglSwapIntervalEXT = (PFNWGLSWAPINTERVALEXTPROC)wglGetProcAddress("wglSwapIntervalEXT");
		if (wglSwapIntervalEXT && wglSwapIntervalEXT(swapInterval)) { 
			retVal = 0;
		}
	}
	return retVal;
}


int CreateGLContext(int swapInterval)
{
	if (glContext_) {
		// GL Already intitialized
		return -1;
	}

	if (!hdc_) {
		// Graphics not initialized
		return -1;
	}

	PIXELFORMATDESCRIPTOR pfd;
	::ZeroMemory(&pfd, sizeof(pfd));

	pfd.nSize = sizeof(pfd);
	pfd.nVersion = 1;
	pfd.dwFlags = PFD_DRAW_TO_WINDOW |
		PFD_SUPPORT_OPENGL |
		PFD_GENERIC_ACCELERATED |
		PFD_DOUBLEBUFFER;
	pfd.iPixelType = PFD_TYPE_RGBA;

	pfd.cColorBits = 24;
	pfd.cRedBits = 8;
	pfd.cBlueBits = 8;
	pfd.cGreenBits = 8;
	pfd.cDepthBits = 32;

	int iPixelFormat = ::ChoosePixelFormat(hdc_, &pfd);
	::SetPixelFormat(hdc_, iPixelFormat, &pfd);

	glContext_ = wglCreateContext(hdc_);

	if (!wglMakeCurrent(hdc_, glContext_)) {
		return -1;
	}

	return SetSwapInterval(swapInterval);
}


bool GetSVGFilePath(char* szFilePath, DWORD size)
{
	OPENFILENAMEA ofn;
	
	// open a file name
	ZeroMemory(&ofn, sizeof(ofn));
	ofn.lStructSize = sizeof(ofn);
	ofn.hwndOwner = NULL;
	ofn.lpstrFile = szFilePath;
	ofn.lpstrFile[0] = '\0';
	ofn.nMaxFile = size;
	ofn.lpstrFilter = "All\0*.*\0SVG\0*.svg\0";
	ofn.nFilterIndex = 2;
	ofn.lpstrFileTitle = NULL;
	ofn.nMaxFileTitle = 0;
	ofn.lpstrInitialDir = NULL;
	ofn.Flags = OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST;
	
	return GetOpenFileNameA(&ofn);
}

void ResizeViewToClient(HWND hWnd)
{
	RECT rect;
	GetClientRect(hWnd, &rect);
	svgView_.resize(rect.right - rect.left, rect.bottom - rect.top);
}

bool GetFitToWindow(HWND hWnd)
{
	HMENU hMenu = GetMenu(hWnd);
	UINT state = GetMenuState(hMenu, IDM_VIEW_FITWINDOW, MF_BYCOMMAND);
	return state & MF_CHECKED;
}

//
//  FUNCTION: WndProc(HWND, UINT, WPARAM, LPARAM)
//
//  PURPOSE: Processes messages for the main window.
//
//  WM_COMMAND  - process the application menu
//  WM_PAINT    - Paint the main window
//  WM_DESTROY  - post a quit message and return
//
//
LRESULT CALLBACK WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam)
{
	switch (message)
	{
	case WM_COMMAND:
	{
		int wmId = LOWORD(wParam);
		// Parse the menu selections:
		switch (wmId)
		{
		case IDM_ABOUT:
			DialogBox(hInst, MAKEINTRESOURCE(IDD_ABOUTBOX), hWnd, About);
			break;
		case IDM_FILE_OPEN:
			char szFilePath[MAX_PATH];
			if (GetSVGFilePath(szFilePath, MAX_PATH)) {
				svgView_.loadFile(szFilePath);
				ResizeViewToClient(hWnd);
			}
			break;
		case IDM_VIEW_FITWINDOW:
			{				
				fitToWindow_ = !GetFitToWindow(hWnd);
				CheckMenuItem(GetMenu(hWnd), IDM_VIEW_FITWINDOW, fitToWindow_ ? MF_CHECKED : MF_UNCHECKED);
				ResizeViewToClient(hWnd);
			}
			break;
		case IDM_EXIT:
			DestroyWindow(hWnd);
			break;
		default:
			return DefWindowProc(hWnd, message, wParam, lParam);
		}
	}
	break;

	case WM_SIZE:
		ResizeViewToClient(hWnd);
		break;

	case WM_ERASEBKGND:
		// Windows sends a WM_ERASEBKGND message when the background needs to be erased.
		// Tell Windows that you handled the message by returning a non - zero number(TRUE).
		// On MFC, override OnEraseBkgnd and just return TRUE.
		return TRUE;

    case WM_DESTROY:
        PostQuitMessage(0);
        break;
    default:
        return DefWindowProc(hWnd, message, wParam, lParam);
    }
    return 0;
}

// Message handler for about box.
INT_PTR CALLBACK About(HWND hDlg, UINT message, WPARAM wParam, LPARAM lParam)
{
    UNREFERENCED_PARAMETER(lParam);
    switch (message)
    {
    case WM_INITDIALOG:
        return (INT_PTR)TRUE;

    case WM_COMMAND:
        if (LOWORD(wParam) == IDOK || LOWORD(wParam) == IDCANCEL)
        {
            EndDialog(hDlg, LOWORD(wParam));
            return (INT_PTR)TRUE;
        }
        break;
    }
    return (INT_PTR)FALSE;
}
