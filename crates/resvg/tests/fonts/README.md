How fonts were subsetted:

Twitter Color Emoji
1. Download: https://github.com/13rac1/twemoji-color-font/releases/download/v14.0.2/TwitterColorEmoji-SVGinOT-14.0.2.zip
2. Run `fonttools subset TwitterColorEmoji-SVGinOT.ttf --unicodes="U+1F601,U+1F980,U+1F3F3,U+FE0F,U+200D,U+1F308,U+1F600,U+1F603,U+1F90C,U+1F90F" --output-file=TwitterColorEmoji.subset.ttf`

Noto Color Emoji (CBDT)
1. Download: https://github.com/googlefonts/noto-emoji/blob/main/fonts/NotoColorEmoji.ttf
2. Run `fonttools subset NotoColorEmoji.ttf --unicodes="U+1F600" --output-file=NotoColorEmojiCBDT.subset.ttf`

Noto COLOR Emoji (COLRv1)
1. Download: https://fonts.google.com/noto/specimen/Noto+Color+Emoji
2. Run `fonttools subset NotoColorEmoji-Regular.ttf --unicodes="U+1F436,U+1F41D,U+1F313,U+1F313" --output-file=NotoColorEmojiCOLR.subset.ttf`
3. Run `fonttools ttx NotoColorEmojiCOLR.subset.ttf`
4. Go to the <name> section and rename all instances of "Noto Color Emoji" to "Noto Color Emoji COLR" (so that
we can distinguish them from CBDT in tests).
5. Run `fonttools ttx -f NotoColorEmojiCOLR.subset.ttx`