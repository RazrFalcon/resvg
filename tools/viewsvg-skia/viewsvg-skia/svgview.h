#pragma once
#include <SkSurface.h>
#include <GrContext.h>

#define RESVG_SKIA_BACKEND

extern "C" {
#include "../../../capi/include/resvg.h"
}

class SvgView
{
public:

	typedef void(*OnRenderedCallback)(const resvg_size& size, const void* pixels, const void* param);

	SvgView();
	~SvgView();

	bool init();
	bool loadFile(const char* filePath);
	bool resize(int width, int height);
	void render(bool fitWindow);

private:
	
	resvg_options opt_;
	resvg_render_tree *tree_ = nullptr;
	GrContext* grContext_;
	SkBitmap bitmap_;
	
	sk_sp<SkSurface> backendSurface_;
	sk_sp<SkSurface> imageSurface_;
	bool imageChanged_;

	resvg_size getImageFitSize();
	void createImageSurface(int width, int height);
	void drawImageToFit();
	void drawImageRect();
	void saveImage(sk_sp<SkImage> image);

};

