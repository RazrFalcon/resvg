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
#include <stddef.h>

#ifdef RESVG_CAIRO_BACKEND
#include <cairo.h>
#endif


#define RESVG_MAJOR_VERSION 0
#define RESVG_MINOR_VERSION 8
#define RESVG_PATCH_VERSION 0
#define RESVG_VERSION "0.8.0"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief An opaque pointer to the rendering tree.
 */
typedef struct resvg_render_tree resvg_render_tree;

/**
 * @brief List of possible errors.
 */
typedef enum resvg_error {
    /** Everything is ok. */
    RESVG_OK = 0,
    /** Only UTF-8 content are supported. */
    RESVG_ERROR_NOT_AN_UTF8_STR,
    /** Failed to open the provided file. */
    RESVG_ERROR_FILE_OPEN_FAILED,
    /** Failed to write to the provided file. */
    RESVG_ERROR_FILE_WRITE_FAILED,
    /** Only \b svg and \b svgz suffixes are supported. */
    RESVG_ERROR_INVALID_FILE_SUFFIX,
    /** Compressed SVG must use the GZip algorithm. */
    RESVG_ERROR_MALFORMED_GZIP,
    /**
     * SVG doesn't have a valid size.
     *
     * Occurs when width and/or height are <= 0.
     *
     * Also occurs if width, height and viewBox are not set.
     * This is against the SVG spec, but an automatic size detection is not supported yet.
     */
    RESVG_ERROR_INVALID_SIZE,
    /** Failed to parse an SVG data. */
    RESVG_ERROR_PARSING_FAILED,
    /** Failed to allocate an image. */
    RESVG_ERROR_NO_CANVAS,
} resvg_error;

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
 * @brief A shape rendering method.
 */
typedef enum resvg_shape_rendering {
    RESVG_SHAPE_RENDERING_OPTIMIZE_SPEED,
    RESVG_SHAPE_RENDERING_CRISP_EDGES,
    RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION,
} resvg_shape_rendering;

/**
 * @brief A text rendering method.
 */
typedef enum resvg_text_rendering {
    RESVG_TEXT_RENDERING_OPTIMIZE_SPEED,
    RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY,
    RESVG_TEXT_RENDERING_GEOMETRIC_PRECISION,
} resvg_text_rendering;

/**
 * @brief An image rendering method.
 */
typedef enum resvg_image_rendering {
    RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY,
    RESVG_IMAGE_RENDERING_OPTIMIZE_SPEED,
} resvg_image_rendering;

/**
 * @brief Rendering options.
 */
typedef struct resvg_options {
    /** SVG image path. Used to resolve relative image paths.
     *
     * Default: NULL
     */
    const char *path;

    /** Output DPI.
     *
     * Default: 96.
     */
    double dpi;

    /** Default font family.
     *
     * Must be set before passing to rendering functions.
     *
     * Default: NULL.
     */
    const char *font_family;

    /** Default font size.
     *
     * Default: 12.
     */
    double font_size;

    /**
     * Sets a comma-separated list of languages that will be used
     * during the 'systemLanguage' attribute resolving.
     * Examples: 'en-US', 'en-US, ru-RU', 'en, ru'
     *
     * Must be set before passing to rendering functions.
     *
     * Default: NULL.
     */
    const char *languages;

    /**
     * Specifies the default shape rendering method.
     *
     * Will be used when an SVG element's \b shape-rendering property is set to \b auto.
     *
     * Default: \b RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION.
     */
    resvg_shape_rendering shape_rendering;

    /**
     * Specifies the default text rendering method.
     *
     * Will be used when an SVG element's \b text-rendering property is set to \b auto.
     *
     * Default: \b RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY.
     */
    resvg_text_rendering text_rendering;

    /**
     * Specifies the default image rendering method.
     *
     * Will be used when an SVG element's \b image-rendering property is set to \b auto.
     *
     * Default: \b RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY.
     */
    resvg_image_rendering image_rendering;

    /**
     * Fits the image using specified options.
     *
     * Default: \b RESVG_FIT_TO_ORIGINAL.
     */
    resvg_fit_to fit_to;

    /** Draw background.
     *
     * Default: false.
     */
    bool draw_background;

    /** Background color. */
    resvg_color background;

    /**
     * Keep named groups. If set to \b true, all non-empty
     * groups with \b id attribute will not be removed.
     *
     * Default: false
     */
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
 * @brief Initializes the library log.
 *
 * Use it if you want to see any warnings.
 *
 * Must be called only once.
 *
 * All warnings will be printed to the \b stderr.
 */
void resvg_init_log();

/**
 * @brief Initializes the #resvg_options structure.
 */
void resvg_init_options(resvg_options *opt);

/**
 * @brief Creates #resvg_render_tree from file.
 *
 * .svg and .svgz files are supported.
 *
 * See #resvg_is_image_empty for details.
 *
 * @param file_path UTF-8 file path.
 * @param opt Rendering options.
 * @param tree Parsed render tree. Should be destroyed via #resvg_tree_destroy.
 * @return #resvg_error
 */
int resvg_parse_tree_from_file(const char *file_path,
                               const resvg_options *opt,
                               resvg_render_tree **tree);

/**
 * @brief Creates #resvg_render_tree from data.
 *
 * See #resvg_is_image_empty for details.
 *
 * @param data SVG data. Can contain SVG string or gzip compressed data.
 * @param len Data length.
 * @param opt Rendering options.
 * @param tree Parsed render tree. Should be destroyed via #resvg_tree_destroy.
 * @return #resvg_error
 */
int resvg_parse_tree_from_data(const char *data,
                               const size_t len,
                               const resvg_options *opt,
                               resvg_render_tree **tree);

/**
 * @brief Checks that tree has any nodes.
 *
 * @param tree Render tree.
 * @return Returns \b true if tree has any nodes.
 */
bool resvg_is_image_empty(const resvg_render_tree *tree);

/**
 * @brief Returns an image size.
 *
 * The size of a canvas that required to render this SVG.
 *
 * @param tree Render tree.
 * @return Image size.
 */
resvg_size resvg_get_image_size(const resvg_render_tree *tree);

/**
 * @brief Returns an image viewbox.
 *
 * @param tree Render tree.
 * @return Image viewbox.
 */
resvg_rect resvg_get_image_viewbox(const resvg_render_tree *tree);

/**
 * @brief Returns an image bounding box.
 *
 * Can be smaller or bigger than a \b viewbox.
 *
 * @param tree Render tree.
 * @param bbox Image's bounding box.
 * @return \b false if an image has no elements.
 */
bool resvg_get_image_bbox(const resvg_render_tree *tree,
                          resvg_rect *bbox);

/**
 * @brief Returns \b true if a renderable node with such an ID exists.
 *
 * @param tree Render tree.
 * @param id Node's ID. UTF-8 string.
 * @return \b true if a node exists.
 * @return \b false if a node doesn't exist or ID isn't a UTF-8 string.
 * @return \b false if a node exists, but not renderable.
 */
bool resvg_node_exists(const resvg_render_tree *tree,
                       const char *id);

/**
 * @brief Returns node's transform by ID.
 *
 * @param tree Render tree.
 * @param id Node's ID. UTF-8 string.
 * @param ts Node's transform.
 * @return \b true if a node exists.
 * @return \b false if a node doesn't exist or ID isn't a UTF-8 string.
 * @return \b false if a node exists, but not renderable.
 */
bool resvg_get_node_transform(const resvg_render_tree *tree,
                              const char *id,
                              resvg_transform *ts);

/**
 * @brief Returns node's bounding box by ID.
 *
 * @param tree Render tree.
 * @param id Node's ID.
 * @param bbox Node's bounding box.
 * @return \b false if a node with such an ID does not exist
 * @return \b false if ID isn't a UTF-8 string.
 * @return \b false if ID is an empty string
 */
bool resvg_get_node_bbox(const resvg_render_tree *tree,
                         const char *id,
                         resvg_rect *bbox);

/**
 * @brief Destroys the #resvg_render_tree.
 *
 * @param tree Render tree.
 */
void resvg_tree_destroy(resvg_render_tree *tree);

#ifdef RESVG_CAIRO_BACKEND
/**
 * @brief Renders the #resvg_render_tree to file.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param file_path File path.
 * @return #resvg_error
 */
int resvg_cairo_render_to_image(const resvg_render_tree *tree,
                                const resvg_options *opt,
                                const char *file_path);

/**
 * @brief Renders the #resvg_render_tree to canvas.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param cr Canvas.
 */
void resvg_cairo_render_to_canvas(const resvg_render_tree *tree,
                                  const resvg_options *opt,
                                  resvg_size size,
                                  cairo_t *cr);

/**
 * @brief Renders a Node by ID to canvas.
 *
 * Does nothing on error.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param id Node's ID.
 * @param cr Canvas.
 */
void resvg_cairo_render_to_canvas_by_id(const resvg_render_tree *tree,
                                        const resvg_options *opt,
                                        resvg_size size,
                                        const char *id,
                                        cairo_t *cr);
#endif /* RESVG_CAIRO_BACKEND */

#ifdef RESVG_QT_BACKEND
/**
 * @brief Renders the #resvg_render_tree to file.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param file_path File path.
 * @return #resvg_error
 */
int resvg_qt_render_to_image(const resvg_render_tree *tree,
                             const resvg_options *opt,
                             const char *file_path);

/**
 * @brief Renders the #resvg_render_tree to canvas.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param painter Canvas.
 */
void resvg_qt_render_to_canvas(const resvg_render_tree *tree,
                               const resvg_options *opt,
                               resvg_size size,
                               void *painter);

/**
 * @brief Renders a Node by ID to canvas.
 *
 * Does nothing on error.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param id Node's ID.
 * @param painter Canvas.
 */
void resvg_qt_render_to_canvas_by_id(const resvg_render_tree *tree,
                                     const resvg_options *opt,
                                     resvg_size size,
                                     const char *id,
                                     void *painter);
#endif /* RESVG_QT_BACKEND */

#ifdef RESVG_RAQOTE_BACKEND
/**
 * @brief Renders the #resvg_render_tree to file.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param file_path File path.
 * @return #resvg_error
 */
int resvg_raqote_render_to_image(const resvg_render_tree *tree,
                                 const resvg_options *opt,
                                 const char *file_path);

/**
 * Raqote backend doesn't have render_to_canvas and render_to_canvas_by_id
 * methods since it's a Rust library.
 */

#endif /* RESVG_RAQOTE_BACKEND */

#ifdef RESVG_SKIA_BACKEND

/**
 * @brief Renders the #resvg_render_tree to file.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param file_path File path.
 * @return #resvg_error
 */
int resvg_skia_render_to_image(const resvg_render_tree *tree,
                               const resvg_options *opt,
                               const char *file_path);

/**
 * @brief Renders the #resvg_render_tree to canvas.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param canvas Skia Canvas.
 */
void resvg_skia_render_to_canvas(const resvg_render_tree *tree,
                                 const resvg_options *opt,
                                 resvg_size size,
                                 void *canvas);

/**
 * @brief Renders a Node by ID to canvas.
 *
 * Does nothing on error.
 *
 * @param tree Render tree.
 * @param opt Rendering options.
 * @param size Canvas size.
 * @param id Node's ID.
 * @param canvas Skia Canvas.
 */
void resvg_skia_render_to_canvas_by_id(const resvg_render_tree *tree,
                                       const resvg_options *opt,
                                       resvg_size size,
                                       const char *id,
                                       void *canvas);
#endif /* RESVG_SKIA_BACKEND */

#ifdef __cplusplus
}
#endif

#endif /* RESVG_H */
