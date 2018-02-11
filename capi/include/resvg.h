/* This Source Code Form is subject to the terms of the Mozilla Public
   License, v. 2.0. If a copy of the MPL was not distributed with this
   file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#ifndef RESVG_H
#define RESVG_H

#ifdef RESVG_CAIRO_BACKEND
#include <cairo.h>
#endif


struct resvg_render_tree;
typedef struct resvg_render_tree resvg_render_tree;

typedef struct resvg_rect {
    double x;
    double y;
    double width;
    double height;
} resvg_rect;


void resvg_init_log();

/**
 * @brief Creates <b>resvg_render_tree</b> from file.
 *
 * .svg and .svgz files are supported.
 *
 * @param file_path UTF-8 file path. Will panic on NULL value.
 * @param dpi Target DPI. Impact units converting and text rendering.
 * @param error The error string if NULL is returned. Should be destroyed via resvg_error_msg_destroy.
 * @return Parsed render tree. NULL on error. Should be destroyed via resvg_rtree_destroy.
 */
resvg_render_tree *resvg_parse_rtree_from_file(const char *file_path,
                                               double dpi,
                                               char **error);

/**
 * @brief Creates <b>resvg_render_tree</b> from UTF-8 string.
 *
 * @param text UTF-8 string. Will panic on NULL value.
 * @param dpi Target DPI. Impact units converting and text rendering.
 * @param error The error string if NULL is returned. Should be destroyed via resvg_error_msg_destroy.
 * @return Parsed render tree. NULL on error. Should be destroyed via resvg_rtree_destroy.
 */
resvg_render_tree *resvg_parse_rtree_from_data(const char *text,
                                               double dpi,
                                               char **error);

void resvg_get_image_size(resvg_render_tree *rtree,
                          double *width,
                          double *height);

void resvg_rtree_destroy(resvg_render_tree *rtree);

void resvg_error_msg_destroy(char *msg);


#ifdef RESVG_CAIRO_BACKEND
void resvg_cairo_render_to_canvas(resvg_render_tree *rtree,
                                  resvg_rect view,
                                  cairo_t *cr);
#endif

#ifdef RESVG_QT_BACKEND
void resvg_qt_render_to_canvas(resvg_render_tree *rtree,
                               resvg_rect view,
                               void *painter);
#endif

#endif // RESVG_H
