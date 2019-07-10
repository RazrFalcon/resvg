#include "SvgViewNative.h"
#include <SkGraphics.h>
#include <SkSurface.h>
#include <SkCanvas.h>
#include <SkColor.h>
#include <SkRect.h>
#include <assert.h>
#include <gl/GrGLAssembleInterface.h>
#include <GrContext.h>
#include <GrContextOptions.h>

#define RESVG_SKIA_BACKEND

extern "C" {
#include "../../../capi/include/resvg.h"
}

class SvgViewNative::SvgViewNativeImpl
{
public:

	SvgViewNativeImpl() :
		tree_(nullptr)
	{
	}

	~SvgViewNativeImpl()
	{
		if (this->tree_) {
			resvg_tree_destroy(tree_);
		}
		imageSurface_.reset();
		if (grContext_) 
		{
			grContext_->unref();
		}
	}
	
	static bool Initialize()
	{		
		SkGraphics::Init();

		resvg_init_log();

		resvg_init_options(&opt_);
		opt_.font_family = "";
		opt_.languages = "";

		const GrGLInterface* grInterface = GrGLCreateNativeInterface();
		if (grInterface) {
			GrContextOptions grContextOptions;
			grContext_ = GrContext::Create(kOpenGL_GrBackend, (GrBackendContext)grInterface, grContextOptions);
		}

		SkSafeUnref(grInterface);
		grInterface = NULL;
	
		return true;
	}

	static void Terminate()
	{
		SkSafeSetNull(grContext_);
	}

	bool LoadFile(const char* filePath)
	{
		bool success = false;
		resvg_render_tree* tree;
		const auto err = resvg_parse_tree_from_file(filePath, &opt_, &tree);
		if (err == RESVG_OK) {

			if (this->tree_) {
				resvg_tree_destroy(tree_);
			}

			this->tree_ = tree;
			this->svgSize_ = resvg_get_image_size(tree);

			success = true;
		}

		return success;
	}

	unsigned int GetSvgWidth()
	{
		return svgSize_.width;
	}

	unsigned int GetSvgHeight()
	{
		return svgSize_.height;
	}

	unsigned int GetWidth()
	{
		return (unsigned int)bitmap_.width();
	}

	unsigned int GetHeight()
	{
		return (unsigned int)bitmap_.height();
	}

	unsigned int GetStride()
	{
		return (unsigned int)bitmap_.rowBytes();
	}

	bool DrawImage(double sx, double sy, double sw, double sh, uint32_t dw, uint32_t dh)
	{
		bool success = false;
		resvg_rect srcRect{ sx, sy, sw, sh };
		resvg_size imageSize = { dw, dh };

		// Create a render surface.
		if (createImageSurface(imageSize.width, imageSize.height)) {
			
			SkCanvas* imageCanvas = imageSurface_->getCanvas();
			resvg_skia_render_rect_to_canvas(tree_, &opt_, imageSize, &srcRect, imageCanvas);
			imageCanvas->flush();

			if (!grContext_) {
				success = true;
			}
			else {
				sk_sp<SkImage> image = imageSurface_->makeImageSnapshot();
				success = image->asLegacyBitmap(&bitmap_, SkImage::LegacyBitmapMode::kRO_LegacyBitmapMode);
			}

		}
		return success;
	}

	void* LockBitmap()
	{
		SkASSERT(bitmap_.width() * bitmap_.bytesPerPixel() == bitmap_.rowBytes());
		bitmap_.lockPixels();
		return bitmap_.getPixels();
	}

	void UnlockBitmap()
	{
		bitmap_.unlockPixels();
	}

	bool Export(const char* filePath, bool formatted)
	{
		return resvg_export_usvg(tree_, filePath, formatted);
	}

private:
	static resvg_options opt_;
	static GrContext* grContext_;	

	resvg_render_tree *tree_ = nullptr;
	resvg_size svgSize_;
	SkBitmap bitmap_;
	sk_sp<SkSurface> imageSurface_;
	
	bool createImageSurface(int width, int height)
	{
		if (width <= 0 || height <= 0) {
			return false;
		}

		bool success = true;
		imageSurface_.reset();

		if (!grContext_) {
			
			// Bitmap for Raster device mode.
			bitmap_.allocN32Pixels(0, 0);
			
			SkImageInfo info = bitmap_.info().makeWH(width, height);
			bitmap_.allocPixels(info);
			SkSurfaceProps surfaceProps(SkSurfaceProps::kLegacyFontHost_InitType);
			imageSurface_ = SkSurface::MakeRasterDirect(bitmap_.info(), bitmap_.getPixels(), bitmap_.rowBytes(), &surfaceProps);
		}
		else {
			
			// Increment the grContext reference count
			grContext_ = SkSafeRef(grContext_);

			const int colorDepth = 32;
			SkColorType colorType = colorDepth == 32 ? kRGBA_8888_SkColorType : kRGB_565_SkColorType;
			SkImageInfo info = SkImageInfo::Make(width, height, colorType, kPremul_SkAlphaType);
			imageSurface_ = SkSurface::MakeRenderTarget(grContext_, SkBudgeted::kNo, info);
		}
				
		return success;
	}

};

resvg_options SvgViewNative::SvgViewNativeImpl::opt_;
GrContext* SvgViewNative::SvgViewNativeImpl::grContext_ = nullptr;

SvgViewNative::SvgViewNative() :
	impl_(new SvgViewNativeImpl)
{
}

SvgViewNative::~SvgViewNative()
{
	delete impl_;
}

bool SvgViewNative::Initialize()
{
	return SvgViewNativeImpl::Initialize();
}

void SvgViewNative::Terminate()
{
	SvgViewNativeImpl::Terminate();
}

bool SvgViewNative::LoadFile(const char* filePath)
{
	return impl_->LoadFile(filePath);
}

unsigned int SvgViewNative::GetSvgWidth()
{
	return impl_->GetSvgWidth();
}

unsigned int SvgViewNative::GetSvgHeight()
{
	return impl_->GetSvgHeight();
}

unsigned int SvgViewNative::GetWidth()
{
	return impl_->GetWidth();
}

unsigned int SvgViewNative::GetHeight()
{
	return impl_->GetHeight();
}

unsigned int SvgViewNative::GetStride()
{
	return impl_->GetStride();
}

bool SvgViewNative::DrawImage(double sx, double sy, double sw, double sh, uint32_t dw, uint32_t dh)
{
	return impl_->DrawImage(sx, sy, sw, sh, dw, dh);
}

void* SvgViewNative::LockBitmap()
{
	return impl_->LockBitmap();
}

void SvgViewNative::UnlockBitmap()
{
	impl_->UnlockBitmap();
}

bool SvgViewNative::Export(const char* filePath, bool formatted)
{
	return impl_->Export(filePath, formatted);
}