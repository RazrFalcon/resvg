How fonts were subsetted:

Twitter Color Emoji
1. Download: https://github.com/13rac1/twemoji-color-font/releases/download/v14.0.2/TwitterColorEmoji-SVGinOT-14.0.2.zip
2. Run `fonttools subset TwitterColorEmoji-SVGinOT.ttf --unicodes="U+1F601,U+1F980,U+1F3F3,U+FE0F,U+200D,U+1F308,U+1F600,U+1F603,U+1F90C,U+1F90F" --output-file=TwitterColorEmoji.subset.ttf`

Noto Color Emoji (CBDT)
1. Download: https://github.com/googlefonts/noto-emoji/blob/main/fonts/NotoColorEmoji.ttf
2. Run `fonttools subset NotoColorEmoji.ttf --unicodes="U+1F600" --output-file=NotoColorEmojiCBDT.subset.ttf`