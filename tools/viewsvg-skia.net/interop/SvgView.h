#pragma once

#include "SvgViewNative.h"

namespace Interop {

	public ref class SvgView {
	public:		

		~SvgView() 
		{
			if (native_) {
				delete native_;
				native_ = nullptr;
			}
		}
		
		static bool Initialize();		
		static void Terminate();
		static SvgView^ LoadFile(System::String^ filePath);

		property unsigned int SvgWidth
		{
			unsigned int get()
			{
				return native_->GetSvgWidth();
			}
		}

		property unsigned int SvgHeight
		{
			unsigned int get()
			{
				return native_->GetSvgHeight();
			}
		}
		
		property unsigned int Width
		{
			unsigned int get()
			{
				return native_->GetWidth();
			}
		}

		property unsigned int Height
		{
			unsigned int get()
			{
				return native_->GetHeight();
			}
		}

		property unsigned int Stride
		{
			unsigned int get()
			{
				return native_->GetStride();
			}
		}
		
		System::Windows::Media::Imaging::BitmapSource^ DrawImage(
			double sx, double sy, double sw, double sh, uint32_t dw, uint32_t dh);

		bool Export(System::String^ filePath, bool formatted);

	protected:
		
		!SvgView()
		{
			if (native_) {
				delete native_;
				native_ = nullptr;
			}
		}	

	private:
		SvgView(SvgViewNative* native) :
			native_(native)
		{}

		SvgViewNative* native_;
	};
}
