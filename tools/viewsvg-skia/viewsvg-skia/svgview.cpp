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

	assert(grContext_);
	backendSurface_.reset();
	imageSurface_.reset();

	GrBackendRenderTargetDesc desc;
	desc.fWidth = width;
	desc.fHeight = height;

	// Only supporting 32 color depth at the moment.
	desc.fConfig = kRGBA_8888_GrPixelConfig;
	desc.fOrigin = kBottomLeft_GrSurfaceOrigin;
	desc.fSampleCnt = 0;
	desc.fStencilBits = 8;
	desc.fRenderTargetHandle = 0;	// Window back buffer

	SkSurfaceProps surfaceProps(SkSurfaceProps::kLegacyFontHost_InitType);
	backendSurface_ = SkSurface::MakeFromBackendRenderTarget(grContext_, desc, &surfaceProps);

	imageChanged_ = true;

	return true;
}

// This is experimental.  Fiddle with matImageObject, srcRect, and dstRect as needed.
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
	//resvg_rect dstRect{ 0.0, 0.0, (double)svgSize.width, (double)svgSize.height };
	resvg_rect dstRect{ 0.0, 0.0, (double)backendSurface_->width(), (double)backendSurface_->height() };

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
	//paint.setColor(0xFFFF0000);
	//paint.setFilterQuality(SkFilterQuality::kLow_SkFilterQuality);
	//backendCanvas->drawRect(SkRect::MakeXYWH(0, 0, 1, 1), paint);

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
	SkImageInfo info = SkImageInfo::Make(width, height, kRGBA_8888_SkColorType, kPremul_SkAlphaType);
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

resvg_size SvgView::getImageFitSize()
{
	resvg_size imageSize = resvg_get_image_size(tree_);

	SkScalar newWidth, newHeight;
	SkScalar viewAspectRatio = (SkScalar)imageSize.width / (SkScalar)imageSize.height;
	SkScalar canvasAspectRatio = (SkScalar) backendSurface_->width() / (SkScalar) backendSurface_->height();

	if (canvasAspectRatio > viewAspectRatio) {
		newWidth = (SkScalar)backendSurface_->height() * viewAspectRatio;
		newHeight = (SkScalar)backendSurface_->height();
	}
	else {
		newWidth = (SkScalar)backendSurface_->width();
		newHeight = (SkScalar)backendSurface_->width() / viewAspectRatio;
	}

	return { (uint32_t)newWidth, (uint32_t)newHeight };
}


void SvgView::drawImageToFit()
{
	if (imageChanged_) {

		resvg_size imageSize = getImageFitSize();
		createImageSurface(imageSize.width, imageSize.height);		
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

	// Center the image on integer coordinates (or else aliasing occurs).
	SkScalar x = floorf(((SkScalar)backendSurface_->width() * 0.5f) - ((SkScalar)image->width() * 0.5f));
	SkScalar y = floorf(((SkScalar)backendSurface_->height() * 0.5f) - ((SkScalar)image->height() * 0.5f));	
	backendCanvas->drawImage(image, x, y);
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
				drawImageToFit();
			}
			else {
				drawImageRect();
			}
		}

	}
}
