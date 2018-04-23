/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

/**
 * @file resvg.h
 *
 * resvg C-API
 */

#ifndef RESVG_H
#define RESVG_H

#include <stdbool.h>
#include <stdint.h>

#ifdef RESVG_CAIRO_BACKEND
#include <cairo.h>
#endif


/**
 * @brief An opaque pointer to the global library handle.
 *
 * Must be invoked before any other \b resvg code.
 *
 * Currently, handles \b QGuiApplication object which must be created
 * in order to draw text. If you don't plan to draw text - it's better to skip
 * the initialization.
 *
 * If you are using this library from an existing Qt application you can skip it.
 *
 * Does nothing when only \b cairo backend is enabled/used.
 *
 * \b Note: \b QGuiApplication initialization is pretty slow (up to 100ms).
 */
typedef struct resvg_handle resvg_handle;

/**
 * @brief An opaque pointer to the rendering tree.
 */
typedef struct resvg_render_tree resvg_render_tree;

/**
 * @brief An RGB color representation.
 */
typedef struct resvg_color {
    uint8_t r; /**< Red component. */
    uint8_t g; /**< Green component. */
    uint8_t b; /**< Blue component. */
} resvg_color;

/**
 * @brief A "fit to" type.
 *
 * All types produce proportional scaling.
 */
typedef enum resvg_fit_to_type {
    RESVG_FIT_TO_ORIGINAL, /**< Use an original image size. */
    RESVG_FIT_TO_WIDTH, /**< Fit an image to a specified width. */
    RESVG_FIT_TO_HEIGHT, /**< Fit an image to a specified height. */
    RESVG_FIT_TO_ZOOM, /**< Zoom an image using scaling factor */
} resvg_fit_to_type;

/**
 * @brief A "fit to" property.
 */
typedef struct resvg_fit_to {
    resvg_fit_to_type type; /**< Fit type. */
    float value; /**< Fit to value. Must be > 0. */
} resvg_fit_to;

/**
 * @brief Rendering options.
 */
typedef struct resvg_options {
    /// SVG image path. Used to resolve relative image paths.
    const char *path;
    /// Output DPI. Default: 96.
    double dpi;
    /// Fits the image using specified options.
    /// Default: \b RESVG_FIT_TO_ORIGINAL.
    resvg_fit_to fit_to;
    /// Draw background. Default: false.
    bool draw_background;
    /// Background color.
    resvg_color background;
    /// Keep named groups. If set to \b true, all non-empty
    /// groups with \b id attribute will not be removed.
    bool keep_named_groups;
} resvg_options;

/**
 * @brief A rectangle representation.
 */
typedef struct resvg_rect {
    double x; /**< X position. */
    double y; /**< Y position. */
    double width; /**< Width. */
    double height; /**< Height. */
} resvg_rect;

/**
 * @brief A size representation.
 */
typedef struct resvg_size {
    uint32_t width; /**< Width. */
    uint32_t height; /**< Height. */
} resvg_size;

/**
 * @brief A 2D transform representation.
 */
typedef struct resvg_transform {
    double a; /**< \b a value */
    double b; /**< \b b value */
    double c; /**< \b c value */
    double d; /**< \b d value */
    double e; /**< \b e value */
    double f; /**< \b f value */
} resvg_transform;

/**
 * @brief Initializes the library.
 *
 * See #resvg_handle for details.
 *
 * @return Library handle.
 */
resvg_handle* resvg_init();

/**
 * @brief Destroys the #resvg_handle.
 *
 * @param handle Library handle.
 */
void resvg_destroy(resvg_handle *handle);

/**
 * @brief Initializes the library log.
 *
 * Use it if you want to see any warnings.
 *
 * All warnings will be printed to the \b stderr.
 */
void resvg_init_log();

/**
 * @brief Initializes the #resvg_options structure.
 */
void resvg_init_options(resvg_options *opt)
{
    opt->path = NULL;
    opt->dpi = 96;
    opt->fit_to.type = RESVG_FIT_TO_ORIGINAL;
    opt->fit_to.value = 0;
    opt->draw_background = false;
    opt->background.r = 0;
    opt->background.g = 0;
    opt->background.b = 0;
    opt->keep_named_groups = false;
}

/**
 * @brief Creates #resvg_render_tree from file.
 *
 * .svg and .svgz files are supported.
 *
 * @param file_path UTF-8 file path.
 * @param opt Rendering options.
 * @param error The error string if NULL was returned. Should be destroyed via #resvg_error_msg_destroy.
 * @return Parsed render tree. NULL on error. Should be destroyed via #resvg_rtree_destroy.
 */
resvg_render_tree *resvg_parse_rtree_from_file(const char *file_path,
                                               const resvg_options *opt,
                                               char **error);

/**
 * @brief Creates #resvg_render_tree from UTF-8 string.
 *
 * @param text UTF-8 string.
 * @param opt Rendering options.
 * @param error The error string if NULL was returned. Should be destroyed via #resvg_error_msg_destroy.
 * @return Parsed render tree. NULL on error. Should be destroyed via #resvg_rtree_destroy.
 */
resvg_render_tree *resvg_parse_rtree_from_data(const char *text,
                                               const resvg_options *opt,
                                               char **error);

/**
 * @brief Returns an image size.
 *
 * @param rtree Render tree.
 * @return Image size.
 */
resvg_size resvg_get_image_size(const resvg_render_tree *rtree);

/**
 * @brief Returns an image viewbox.
 *
 * @param rtree Render tree.
 * @return Image viewbox.
 */
resvg_rect resvg_get_image_viewbox(const resvg_render_tree *rtree);

/**
 * @brief Returns \b true if a node with such an ID exists.
 *
 * @param rtree Render tree.
 * @param id Node's ID. UTF-8 string.
 * @return \b true if a node exists. \b false if a node doesn't exist or ID isn't a UTF-8 string.
 */
bool resvg_node_exists(const resvg_render_tree *rtree,
                       const char *id);

/**
 * @brief Returns node's transform by ID.
 *
 * @param rtree Render tree.
 * @param id Node's ID. UTF-8 string.
 * @param ts Node's transform.
 * @return \b false if a node with such an ID does not exist or ID isn't a UTF-8 string.
 */
bool resvg_get_node_transform(const resvg_render_tree *rtree,
                              const char *id,
                              resvg_transform *ts);

/**
 * @brief Destroys the #resvg_render_tree.
 *
 * @param rtree Render tree.
 */
void resvg_rtree_destroy(resvg_render_tree *rtree);

/**
 * @brief Destroys the error message.
 *
 * @param msg Error message.
 */
void resvg_error_msg_destroy(char *msg);


#ifdef RESVG_CAIRO_BACKEND
/**
 * @brief Returns node's bounding box by ID.
 *
 * @param rtree Render tree.
 * @param opt Rendering options.
 * @param id Node's ID.
 * @param bbox Node's bounding box.
 * @return \b false if a node with such an ID does not exist
 * @return \b false if ID isn't a UTF-8 string.
 * @return \b false if ID is an empty string
 */
bool resvg_cairo_get_node_bbox(const resvg_render_tree *rtree,
                               const resvg_options *opt,
                               const char *id,
                               resvg_rect *bbox);

/**
 * @brief Renders the #resvg_render_tree to file.
 *
 * @param rtree Render tree.
 * @param opt Rendering options.
 * @param file_path File path.
 * @return \b false if \b file_path isn't an UTF-8 string.
 * @return \b false on "Out of memory".
 * @return \b false on file write error.
 */
bool resvg_cairo_render_to_image(const resvg_render_tree *rtree,
                                 const resvg_options *opt,
                                 const char *file_path);

/**
 * @brief Renders the #resvg_render_tree to canvas.
 *
 * @param rtree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param cr Canvas.
 */
void resvg_cairo_render_to_canvas(const resvg_render_tree *rtree,
                                  const resvg_options *opt,
                                  resvg_size size,
                                  cairo_t *cr);

/**
 * @brief Renders a Node by ID to canvas.
 *
 * Does nothing on error.
 *
 * @param rtree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param id Node's ID.
 * @param cr Canvas.
 */
void resvg_cairo_render_to_canvas_by_id(const resvg_render_tree *rtree,
                                        const resvg_options *opt,
                                        resvg_size size,
                                        const char *id,
                                        cairo_t *cr);
#endif // RESVG_CAIRO_BACKEND

#ifdef RESVG_QT_BACKEND
/**
 * @brief Returns node's bounding box by ID.
 *
 * @param rtree Render tree.
 * @param opt Rendering options.
 * @param id Node's ID.
 * @param bbox Node's bounding box.
 * @return \b false if a node with such an ID does not exist,
 *         ID is an empty string or ID isn't a UTF-8 string.
 */
bool resvg_qt_get_node_bbox(const resvg_render_tree *rtree,
                            const resvg_options *opt,
                            const char *id,
                            resvg_rect *bbox);

/**
 * @brief Renders the #resvg_render_tree to file.
 *
 * @param rtree Render tree.
 * @param opt Rendering options.
 * @param file_path File path.
 * @return \b false if \b file_path isn't an UTF-8 string.
 * @return \b false on "Out of memory".
 * @return \b false on file write error.
 */
bool resvg_qt_render_to_image(const resvg_render_tree *rtree,
                              const resvg_options *opt,
                              const char *file_path);

/**
 * @brief Renders the #resvg_render_tree to canvas.
 *
 * @param rtree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param painter Canvas.
 */
void resvg_qt_render_to_canvas(const resvg_render_tree *rtree,
                               const resvg_options *opt,
                               resvg_size size,
                               void *painter);

/**
 * @brief Renders a Node by ID to canvas.
 *
 * Does nothing on error.
 *
 * @param rtree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param id Node's ID.
 * @param painter Canvas.
 */
void resvg_qt_render_to_canvas_by_id(const resvg_render_tree *rtree,
                                     const resvg_options *opt,
                                     resvg_size size,
                                     const char *id,
                                     void *painter);
#endif // RESVG_QT_BACKEND

#endif // RESVG_H
