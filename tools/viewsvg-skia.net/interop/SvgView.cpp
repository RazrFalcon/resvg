#include "SvgView.h"
#include <stdlib.h>
#include <string.h>
#include <msclr\marshal.h>

using namespace System;
using namespace System::Runtime::InteropServices;
using namespace msclr::interop;
using namespace System::Windows::Media;
using namespace System::Windows::Media::Imaging;

namespace Interop
{
	bool SvgView::Initialize()
	{
		return SvgViewNative::Initialize();
	}

	void SvgView::Terminate()
	{
		SvgViewNative::Terminate();
	}

	SvgView^ SvgView::LoadFile(String^ filePath)
	{
		SvgViewNative* native = new SvgViewNative();

		marshal_context ^ context = gcnew marshal_context();
		const char* szFilePath = context->marshal_as<const char*>(filePath);
		bool success = native->LoadFile(szFilePath);
		delete context;

		if (success) {
			return gcnew SvgView(native);
		}

		return nullptr;
	}

	BitmapSource^ SvgView::DrawImage(double sx, double sy, double sw, double sh, uint32_t dw, uint32_t dh)
	{
		if (native_->DrawImage(sx, sy, sw, sh, dw, dh)) {

			void* pixels = native_->LockBitmap();

			IntPtr ptr(pixels);
			int len = native_->GetHeight() * native_->GetStride();

			return BitmapSource::Create(
				native_->GetWidth(),
				native_->GetHeight(),
				96.0,
				96.0,
				PixelFormats::Bgra32,
				nullptr,
				ptr,
				len,
				native_->GetStride());
		}

		return nullptr;
	}

	bool SvgView::Export(System::String^ filePath, bool formatted)
	{
		marshal_context ^ context = gcnew marshal_context();
		const char* szFilePath = context->marshal_as<const char*>(filePath);
		bool success = native_->Export(szFilePath, formatted);		
		delete context;

		return success;
	}
	
}
