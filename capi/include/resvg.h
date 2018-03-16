/* This Source Code Form is subject to the terms of the Mozilla Public
   License, v. 2.0. If a copy of the MPL was not distributed with this
   file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#ifndef RESVG_H
#define RESVG_H

#include <stdbool.h>

#ifdef RESVG_CAIRO_BACKEND
#include <cairo.h>
#endif


typedef struct resvg_handle resvg_handle;
typedef struct resvg_render_tree resvg_render_tree;

typedef struct resvg_color {
    unsigned char r;
    unsigned char g;
    unsigned char b;
} resvg_color;

typedef enum resvg_fit_to_type {
    RESVG_FIT_TO_ORIGINAL,
    RESVG_FIT_TO_WIDTH,
    RESVG_FIT_TO_HEIGHT,
    RESVG_FIT_TO_ZOOM,
} resvg_fit_to_type;

typedef struct resvg_fit_to {
    resvg_fit_to_type type;
    float value;
} resvg_fit_to;

typedef struct resvg_options {
    const char *path;
    double dpi;
    resvg_fit_to fit_to;
    bool draw_background;
    resvg_color background;
    bool keep_named_groups;
} resvg_options;

typedef struct resvg_rect {
    double x;
    double y;
    double width;
    double height;
} resvg_rect;

typedef struct resvg_size {
    unsigned int width;
    unsigned int height;
} resvg_size;

typedef struct resvg_transform {
    double a;
    double b;
    double c;
    double d;
    double e;
    double f;
} resvg_transform;


resvg_handle* resvg_init();
void resvg_destroy(resvg_handle *handle);

void resvg_init_log();

void resvg_init_options(resvg_options *opt)
{
    opt->path = NULL;
    opt->dpi = 96;
    opt->fit_to.type = RESVG_FIT_TO_ORIGINAL;
    opt->fit_to.value = 0;
    opt->draw_background = false;
    opt->keep_named_groups = false;
}

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
                                               const resvg_options *opt,
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
                                               const resvg_options *opt,
                                               char **error);

void resvg_get_image_size(const resvg_render_tree *rtree,
                          double *width,
                          double *height);

bool resvg_node_exists(const resvg_render_tree *rtree,
                       const char *id);

bool resvg_get_node_transform(const resvg_render_tree *rtree,
                              const char *id,
                              resvg_transform *ts);

void resvg_rtree_destroy(resvg_render_tree *rtree);

void resvg_error_msg_destroy(char *msg);


#ifdef RESVG_CAIRO_BACKEND
bool resvg_cairo_get_node_bbox(const resvg_render_tree *rtree,
                               const resvg_options *opt,
                               const char *id,
                               resvg_rect *bbox);

bool resvg_cairo_render_to_image(const resvg_render_tree *rtree,
                                 const resvg_options *opt,
                                 const char *file_path);

void resvg_cairo_render_to_canvas(const resvg_render_tree *rtree,
                                  const resvg_options *opt,
                                  resvg_size size,
                                  cairo_t *cr);

void resvg_cairo_render_to_canvas_by_id(const resvg_render_tree *rtree,
                                        const resvg_options *opt,
                                        resvg_size size,
                                        const char *id,
                                        void *painter);
#endif

#ifdef RESVG_QT_BACKEND
bool resvg_qt_get_node_bbox(const resvg_render_tree *rtree,
                            const resvg_options *opt,
                            const char *id,
                            resvg_rect *bbox);

bool resvg_qt_render_to_image(const resvg_render_tree *rtree,
                              const resvg_options *opt,
                              const char *file_path);

void resvg_qt_render_to_canvas(const resvg_render_tree *rtree,
                               const resvg_options *opt,
                               resvg_size size,
                               void *painter);

void resvg_qt_render_to_canvas_by_id(const resvg_render_tree *rtree,
                                     const resvg_options *opt,
                                     resvg_size size,
                                     const char *id,
                                     void *painter);
#endif

#endif // RESVG_H
