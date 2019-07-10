#include "stdafx.h"
#include "svgview.h"
#include <SkSurface.h>
#include <SkCanvas.h>
#include <SkColor.h>
#include <SkRect.h>
#include <assert.h>
#include <gl/GrGLAssembleInterface.h>
#include <GrContext.h>
#include <GrContextOptions.h>

SvgView::SvgView() :
	tree_(nullptr),
	imageChanged_(false)
{
}

SvgView::~SvgView()
{
}

bool SvgView::init()
{
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

	return grContext_ != nullptr;
}

bool SvgView::loadFile(const char* filePath)
{
	bool success = false;
	resvg_render_tree* tree;
	const auto err = resvg_parse_tree_from_file(filePath, &opt_, &tree);
	if (err == RESVG_OK) {
	
		if (this->tree_) {
			resvg_tree_destroy(tree_);
		}
		
		this->tree_ = tree;
		
		imageChanged_ = true;
	
		success = true;
	}

	return success;
}


bool SvgView::resize(int width, int height)
{
	// Skia will fail if the surface has a zero dimension but it is apparently ok to continue
	//	using the existing surface when this happens.  So, simply return here.
	if (width <= 0 || height <= 0) {
		return false;
	}

	// TODO:  Add support support non-GL (raster mode) surfaces.
	assert(grContext_);
	backendSurface_.reset();
	imageSurface_.reset();

	GrBackendRenderTargetDesc desc;
	desc.fWidth = width;
	desc.fHeight = height;

	// Only supporting 32 color depth at the moment.
	const int colorDepth = 32;
	desc.fConfig = colorDepth == 32 ? kRGBA_8888_GrPixelConfig : kRGB_565_GrPixelConfig;
	desc.fOrigin = kBottomLeft_GrSurfaceOrigin;
	desc.fSampleCnt = 0;
	desc.fStencilBits = 8;
	desc.fRenderTargetHandle = 0;	// Window back buffer

	SkSurfaceProps surfaceProps(SkSurfaceProps::kLegacyFontHost_InitType);
	backendSurface_ = SkSurface::MakeFromBackendRenderTarget(grContext_, desc, &surfaceProps);

	imageChanged_ = true;

	return true;
}

void SvgView::drawImageRect()
{
	SkCanvas* backendCanvas = backendSurface_->getCanvas();

// Transform according whatever is on the the matrix stack of the image object when drawImage is called.
//***********************
	//SkMatrix matImageObject;
	//matImageObject.reset();
	//matImageObject.preTranslate(100, 100);
	//matImageObject.preRotate(45);
	//matImageObject.preScale(2, 1);
	//matImageObject.preSkew(1, 0);
	//backendCanvas->setMatrix(matImageObject);
//**********************

	resvg_size svgSize = resvg_get_image_size(tree_);
	resvg_rect srcRect{ 0.0, 0.0, (double)svgSize.width, (double)svgSize.height };
	resvg_rect dstRect{ 0.0, 0.0, (double)svgSize.width, (double)svgSize.height };

	// Get only the scale factors from the back canvas.
	SkSize scale;
	SkMatrix remainder;
	SkMatrix currentMatrix = backendCanvas->getTotalMatrix();
	currentMatrix.decomposeScale(&scale, &remainder);

	if (imageChanged_) {

		// Multiply the destination rect by the scale factors.
		resvg_size imageSize = {
			(uint32_t)((SkScalar)dstRect.width * scale.fWidth),
			(uint32_t)((SkScalar)dstRect.height * scale.fHeight)
		};

		// Create a render surface.
		createImageSurface(imageSize.width, imageSize.height);
		SkCanvas* imageCanvas = imageSurface_->getCanvas();

		imageCanvas->clear(SK_ColorTRANSPARENT);
		resvg_skia_render_rect_to_canvas(tree_, &opt_, imageSize, &srcRect, imageCanvas);
		imageCanvas->flush();
	}

	sk_sp<SkImage> image = imageSurface_->makeImageSnapshot();
	if (imageChanged_) {
		saveImage(image);
	}
	
	SkPaint paint;
	paint.setColor(0xFFFF0000);
	paint.setFilterQuality(SkFilterQuality::kLow_SkFilterQuality);
	backendCanvas->drawRect(SkRect::MakeXYWH(0, 0, 1, 1), paint);

	SkMatrix renderMatrix = currentMatrix;
	renderMatrix.preScale(SkScalarInvert(scale.fWidth), SkScalarInvert(scale.fHeight));
	backendCanvas->setMatrix(renderMatrix);	
	backendCanvas->drawImage(image, (SkScalar)dstRect.x, (SkScalar)dstRect.y, &paint);
	backendCanvas->flush();
	backendCanvas->setMatrix(currentMatrix);

	imageChanged_ = false;
}


void SvgView::createImageSurface(int width, int height)
{
	const int colorDepth = 32;
	SkColorType colorType = colorDepth == 32 ? kRGBA_8888_SkColorType : kRGB_565_SkColorType;
	SkImageInfo info = SkImageInfo::Make(width, height, colorType, kPremul_SkAlphaType);
	imageSurface_ = SkSurface::MakeRenderTarget(grContext_, SkBudgeted::kNo, info);
}

void SvgView::saveImage(sk_sp<SkImage> image)
{
	sk_sp<SkData> data(image->encode());
	if (data) {
#ifdef _DEBUG
		data->ref();
#endif
		SkFILEWStream stream("out.png");
		if (stream.write(data->data(), data->size())) {
			stream.flush();
		}
	}
}

void SvgView::drawImage()
{
	if (imageChanged_) {

		resvg_size imageSize{ 
			(uint32_t)backendSurface_->width(), 
			(uint32_t)backendSurface_->height() 
		};

		createImageSurface(backendSurface_->width(), backendSurface_->height());		
		SkCanvas* imageCanvas = imageSurface_->getCanvas();

		resvg_skia_render_to_canvas(tree_, &opt_, imageSize, imageCanvas);
		imageCanvas->flush();
	}

	sk_sp<SkImage> image = imageSurface_->makeImageSnapshot();
	if (imageChanged_) {
		saveImage(image);
	}

	SkCanvas* backendCanvas = backendSurface_->getCanvas();
	backendCanvas->resetMatrix();
	backendCanvas->drawImage(image, 0, 0);
	backendCanvas->flush();

	imageChanged_ = false;
}


void SvgView::render(bool fitWindow)
{	
	if (backendSurface_) {

		SkCanvas* backendCanvas = backendSurface_->getCanvas();		
		backendCanvas->clear(SkColorSetRGB(255, 255, 255));
		
		if (tree_) {	

			if (fitWindow) {
				drawImage();
			}
			else {
				drawImageRect();
			}
		}

	}
}
