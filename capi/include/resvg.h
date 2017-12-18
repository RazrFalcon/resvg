/* This Source Code Form is subject to the terms of the Mozilla Public
   License, v. 2.0. If a copy of the MPL was not distributed with this
   file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#ifndef RESVG_H
#define RESVG_H

#ifdef RESVG_CAIRO_BACKEND
#include <cairo.h>
#endif


struct resvg_document;
typedef struct resvg_document resvg_document;

void resvg_init_log();

/**
 * @brief Creates <b>resvg_document</b> from file.
 *
 * .svg and .svgz files are supported.
 *
 * @param file_path UTF-8 file path. Will panic on NULL value.
 * @param dpi Target DPI. Impact units converting and text rendering.
 * @param error The error string if NULL is returned. Should be destroyed via resvg_error_msg_destroy.
 * @return Parsed document. NULL on error. Should be destroyed via resvg_doc_destroy.
 */
resvg_document *resvg_parse_doc_from_file(const char *file_path,
                                          double dpi,
                                          char **error);

/**
 * @brief Creates <b>resvg_document</b> from UTF-8 string.
 *
 * @param text UTF-8 string. Will panic on NULL value.
 * @param dpi Target DPI. Impact units converting and text rendering.
 * @param error The error string if NULL is returned. Should be destroyed via resvg_error_msg_destroy.
 * @return Parsed document. NULL on error. Should be destroyed via resvg_doc_destroy.
 */
resvg_document *resvg_parse_doc_from_data(const char *text,
                                          double dpi,
                                          char **error);

void resvg_get_image_size(resvg_document *doc,
                          double *width,
                          double *height);

void resvg_doc_destroy(resvg_document *doc);

void resvg_error_msg_destroy(char *msg);


#ifdef RESVG_CAIRO_BACKEND
void resvg_cairo_render_to_canvas(cairo_t *cr,
                                  double x,
                                  double y,
                                  double width,
                                  double height,
                                  resvg_document *doc);
#endif

#ifdef RESVG_QT_BACKEND
void resvg_qt_render_to_canvas(void *painter,
                               double x,
                               double y,
                               double width,
                               double height,
                               resvg_document *doc);
#endif

#endif // RESVG_H
