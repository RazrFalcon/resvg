## Curves

Rendering a lot of curves:

```
test fill_cairo                                  ... bench:   7,892,305 ns/iter (+/- 128,850)
test fill_qt                                     ... bench:  26,203,977 ns/iter (+/- 149,269)
test fill_raqote                                 ... bench:   3,331,677 ns/iter (+/- 53,432)
test fill_skia                                   ... bench:   4,373,050 ns/iter (+/- 31,387)

test stroke_cairo                                ... bench:  20,969,930 ns/iter (+/- 97,578)
test stroke_qt                                   ... bench:   3,167,553 ns/iter (+/- 94,069)
test stroke_raqote                               ... bench:  18,353,069 ns/iter (+/- 273,128)
test stroke_skia                                 ... bench:   3,530,062 ns/iter (+/- 984,494)

test dashed_stroke_cairo                         ... bench:  19,065,295 ns/iter (+/- 52,734)
test dashed_stroke_qt                            ... bench:   2,816,995 ns/iter (+/- 67,351)
test dashed_stroke_raqote                        ... bench:  11,236,564 ns/iter (+/- 91,479)
test dashed_stroke_skia                          ... bench:   3,998,709 ns/iter (+/- 94,285)

test fill_and_stroke_cairo                       ... bench:  28,043,073 ns/iter (+/- 67,246)
test fill_and_stroke_qt                          ... bench:  27,980,339 ns/iter (+/- 127,339)
test fill_and_stroke_raqote                      ... bench:  20,794,529 ns/iter (+/- 117,458)
test fill_and_stroke_skia                        ... bench:   6,943,730 ns/iter (+/- 73,482)

test fill_and_stroke_with_gradient_cairo         ... bench:  32,411,361 ns/iter (+/- 78,008)
test fill_and_stroke_with_gradient_qt            ... bench:  30,810,458 ns/iter (+/- 539,121)
test fill_and_stroke_with_gradient_raqote        ... bench:  22,295,075 ns/iter (+/- 84,574)
test fill_and_stroke_with_gradient_skia          ... bench:   8,619,919 ns/iter (+/- 95,042)

test fill_and_stroke_with_pattern_cairo          ... bench:  31,293,526 ns/iter (+/- 91,663)
test fill_and_stroke_with_pattern_qt             ... bench:  30,536,466 ns/iter (+/- 288,623)
test fill_and_stroke_with_pattern_raqote         ... bench:  41,529,079 ns/iter (+/- 240,641)
test fill_and_stroke_with_pattern_skia           ... bench:  10,110,017 ns/iter (+/- 32,248)
```

Rendering a circle:

```
test fill_circle_cairo                           ... bench:      55,378 ns/iter (+/- 496)
test fill_circle_qt                              ... bench:      63,073 ns/iter (+/- 710)
test fill_circle_raqote                          ... bench:     164,150 ns/iter (+/- 2,687)
test fill_circle_skia                            ... bench:      47,249 ns/iter (+/- 919)

test stroke_circle_cairo                         ... bench:     127,016 ns/iter (+/- 791)
test stroke_circle_qt                            ... bench:      43,517 ns/iter (+/- 537)
test stroke_circle_raqote                        ... bench:     119,177 ns/iter (+/- 1,033)
test stroke_circle_skia                          ... bench:      38,453 ns/iter (+/- 203)

test fill_and_stroke_circle_with_gradient_cairo  ... bench:     853,685 ns/iter (+/- 1,592)
test fill_and_stroke_circle_with_gradient_qt     ... bench:     139,735 ns/iter (+/- 549)
test fill_and_stroke_circle_with_gradient_raqote ... bench:     406,325 ns/iter (+/- 503)
test fill_and_stroke_circle_with_gradient_skia   ... bench:     138,934 ns/iter (+/- 180)

test fill_and_stroke_circle_with_pattern_cairo   ... bench:     423,525 ns/iter (+/- 541)
test fill_and_stroke_circle_with_pattern_qt      ... bench:     149,905 ns/iter (+/- 425)
test fill_and_stroke_circle_with_pattern_raqote  ... bench:   2,371,549 ns/iter (+/- 14,568)
test fill_and_stroke_circle_with_pattern_skia    ... bench:     208,498 ns/iter (+/- 252)
```

## Filters

```
test blend_multiply_cairo        ... bench:   2,063,319 ns/iter (+/- 5,725)
test blend_multiply_qt           ... bench:   1,051,077 ns/iter (+/- 9,241)
test blend_multiply_raqote       ... bench:   3,484,172 ns/iter (+/- 10,110)
test blend_multiply_skia         ... bench:   1,296,631 ns/iter (+/- 4,545)

test box_blur_100px              ... bench:   1,043,684 ns/iter (+/- 1,063)
test iir_blur_100px              ... bench:   2,756,448 ns/iter (+/- 3,791)

test box_blur_500px              ... bench:  25,740,233 ns/iter (+/- 73,510)
test iir_blur_500px              ... bench:  68,137,775 ns/iter (+/- 42,083)

test color_matrix_cairo          ... bench:   2,162,332 ns/iter (+/- 1,248)
test color_matrix_qt             ... bench:   1,155,341 ns/iter (+/- 4,490)
test color_matrix_raqote         ... bench:   2,863,187 ns/iter (+/- 1,690)
test color_matrix_skia           ... bench:   1,493,913 ns/iter (+/- 1,352)

test composite_over_cairo        ... bench:   1,340,892 ns/iter (+/- 689)
test composite_over_qt           ... bench:     757,712 ns/iter (+/- 1,864)
test composite_over_raqote       ... bench:   3,198,051 ns/iter (+/- 1,654)
test composite_over_skia         ... bench:   1,067,709 ns/iter (+/- 523)

test composite_arithmetic_cairo  ... bench:   2,354,850 ns/iter (+/- 7,827)
test composite_arithmetic_qt     ... bench:   2,177,758 ns/iter (+/- 4,174)
test composite_arithmetic_raqote ... bench:   3,487,432 ns/iter (+/- 2,940)
test composite_arithmetic_skia   ... bench:   2,474,041 ns/iter (+/- 868)
```

## Layers

```
test element_with_opacity_cairo  ... bench:     125,664 ns/iter (+/- 226)
test element_with_opacity_qt     ... bench:      89,378 ns/iter (+/- 1,635)
test element_with_opacity_raqote ... bench:     445,484 ns/iter (+/- 1,263)
test element_with_opacity_skia   ... bench:     124,055 ns/iter (+/- 142)

test groups_with_opacity_cairo   ... bench:     172,716 ns/iter (+/- 432)
test groups_with_opacity_qt      ... bench:     173,847 ns/iter (+/- 652)
test groups_with_opacity_raqote  ... bench:     884,742 ns/iter (+/- 1,058)
test groups_with_opacity_skia    ... bench:     233,153 ns/iter (+/- 247)

test nested_clip_path_cairo      ... bench:     240,114 ns/iter (+/- 594)
test nested_clip_path_qt         ... bench:     471,939 ns/iter (+/- 2,506)
test nested_clip_path_raqote     ... bench:   2,042,673 ns/iter (+/- 8,539)
test nested_clip_path_skia       ... bench:     412,809 ns/iter (+/- 1,265)

test clip_path_cairo             ... bench:     162,257 ns/iter (+/- 236)
test clip_path_qt                ... bench:     265,143 ns/iter (+/- 16,374)
test clip_path_raqote            ... bench:   1,309,212 ns/iter (+/- 2,860)
test clip_path_skia              ... bench:     231,024 ns/iter (+/- 383)

test mask_cairo                  ... bench:     174,916 ns/iter (+/- 203)
test mask_qt                     ... bench:     336,101 ns/iter (+/- 2,356)
test mask_raqote                 ... bench:   1,326,855 ns/iter (+/- 2,678)
test mask_skia                   ... bench:     429,845 ns/iter (+/- 2,373)
```
