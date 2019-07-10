#pragma once
#include <stdint.h>

class SvgViewNative
{
public:
	SvgViewNative();
	~SvgViewNative();

	static bool Initialize();
	static void Terminate();

	bool LoadFile(const char* filePath);
	bool DrawImage(double sx, double sy, double sw, double sh, uint32_t dw, uint32_t dh);

	unsigned int GetSvgWidth();
	unsigned int GetSvgHeight();

	unsigned int GetWidth();
	unsigned int GetHeight();
	unsigned int GetStride();

	void* LockBitmap();
	void UnlockBitmap();

	bool Export(const char* filePath, bool formatted);

private:
	class SvgViewNativeImpl;
	SvgViewNativeImpl* impl_;
};

