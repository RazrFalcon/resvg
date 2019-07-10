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
	}
	
	static bool Initialize()
	{		
		SkGraphics::Init();

		resvg_init_log();

		resvg_init_options(&opt_);
		opt_.font_family = "";
		opt_.languages = "";
		
		return true;
	}

	static void Terminate()
	{
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
		return (unsigned int)imageSurface_->width();
	}

	unsigned int GetHeight()
	{
		return (unsigned int)imageSurface_->height();
	}

	unsigned int GetStride()
	{		
		SkPixmap pixmap;
		if (imageSurface_->peekPixels(&pixmap)) {
			return (int)pixmap.rowBytes();
		}
		return 0;
	}

	const void* GetPixels()
	{
		SkPixmap pixmap;
		if (imageSurface_->peekPixels(&pixmap)) {
			return pixmap.addr();
		}
		return nullptr;
	}

	bool DrawImage(double sx, double sy, double sw, double sh, uint32_t dw, uint32_t dh)
	{
		bool success = false;
		resvg_size imageSize = { dw, dh };

		// Create a render surface.
		if (createImageSurface(imageSize.width, imageSize.height)) {			

			resvg_rect srcRect{ sx, sy, sw, sh };
			SkCanvas* imageCanvas = imageSurface_->getCanvas();
			resvg_skia_render_rect_to_canvas(tree_, &opt_, imageSize, &srcRect, imageCanvas);

			imageCanvas->flush();
			success = true;			
		}
		return success;
	}

	bool Export(const char* filePath, bool formatted)
	{
		return resvg_export_usvg(tree_, filePath, formatted);
	}

private:
	static resvg_options opt_;

	resvg_render_tree *tree_ = nullptr;
	resvg_size svgSize_;
	sk_sp<SkSurface> imageSurface_;
	
	bool createImageSurface(int width, int height)
	{
		if (width <= 0 || height <= 0) {
			return false;
		}

		SkImageInfo info = SkImageInfo::Make(width, height, kN32_SkColorType, kPremul_SkAlphaType);
		imageSurface_ = SkSurface::MakeRaster(info);

		return imageSurface_ != nullptr;
	}

};

resvg_options SvgViewNative::SvgViewNativeImpl::opt_;

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

const void* SvgViewNative::GetPixels()
{
	return impl_->GetPixels();
}

bool SvgViewNative::Export(const char* filePath, bool formatted)
{
	return impl_->Export(filePath, formatted);
}