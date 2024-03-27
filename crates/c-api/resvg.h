/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

/**
 * @file svgr.h
 *
 * svgr C API
 */

#ifndef RESVG_H
#define RESVG_H

#include <stdbool.h>
#include <stdint.h>

#define RESVG_MAJOR_VERSION 0
#define RESVG_MINOR_VERSION 40
#define RESVG_PATCH_VERSION 0
#define RESVG_VERSION "0.40.0"

/**
 * @brief List of possible errors.
 */
typedef enum {
    /**
     * Everything is ok.
     */
    RESVG_OK = 0,
    /**
     * Only UTF-8 content are supported.
     */
    RESVG_ERROR_NOT_AN_UTF8_STR,
    /**
     * Failed to open the provided file.
     */
    RESVG_ERROR_FILE_OPEN_FAILED,
    /**
     * Compressed SVG must use the GZip algorithm.
     */
    RESVG_ERROR_MALFORMED_GZIP,
    /**
     * We do not allow SVG with more than 1_000_000 elements for security reasons.
     */
    RESVG_ERROR_ELEMENTS_LIMIT_REACHED,
    /**
     * SVG doesn't have a valid size.
     *
     * Occurs when width and/or height are <= 0.
     *
     * Also occurs if width, height and viewBox are not set.
     */
    RESVG_ERROR_INVALID_SIZE,
    /**
     * Failed to parse an SVG data.
     */
    RESVG_ERROR_PARSING_FAILED,
} svgr_error;

/**
 * @brief A image rendering method.
 */
typedef enum {
    RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY,
    RESVG_IMAGE_RENDERING_OPTIMIZE_SPEED,
} svgr_image_rendering;

/**
 * @brief A shape rendering method.
 */
typedef enum {
    RESVG_SHAPE_RENDERING_OPTIMIZE_SPEED,
    RESVG_SHAPE_RENDERING_CRISP_EDGES,
    RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION,
} svgr_shape_rendering;

/**
 * @brief A text rendering method.
 */
typedef enum {
    RESVG_TEXT_RENDERING_OPTIMIZE_SPEED,
    RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY,
    RESVG_TEXT_RENDERING_GEOMETRIC_PRECISION,
} svgr_text_rendering;

/**
 * @brief An SVG to #svgr_render_tree conversion options.
 *
 * Also, contains a fonts database used during text to path conversion.
 * The database is empty by default.
 */
typedef struct svgr_options svgr_options;

/**
 * @brief An opaque pointer to the rendering tree.
 */
typedef struct svgr_render_tree svgr_render_tree;

/**
 * @brief A 2D transform representation.
 */
typedef struct {
    float a;
    float b;
    float c;
    float d;
    float e;
    float f;
} svgr_transform;

/**
 * @brief A size representation.
 */
typedef struct {
    float width;
    float height;
} svgr_size;

/**
 * @brief A rectangle representation.
 */
typedef struct {
    float x;
    float y;
    float width;
    float height;
} svgr_rect;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * @brief Creates an identity transform.
 */
svgr_transform svgr_transform_identity(void);

/**
 * @brief Initializes the library log.
 *
 * Use it if you want to see any warnings.
 *
 * Must be called only once.
 *
 * All warnings will be printed to the `stderr`.
 */
void svgr_init_log(void);

/**
 * @brief Creates a new #svgr_options object.
 *
 * Should be destroyed via #svgr_options_destroy.
 */
svgr_options *svgr_options_create(void);

/**
 * @brief Sets a directory that will be used during relative paths resolving.
 *
 * Expected to be the same as the directory that contains the SVG file,
 * but can be set to any.
 *
 * Must be UTF-8. Can be set to NULL.
 *
 * Default: NULL
 */
void svgr_options_set_resources_dir(svgr_options *opt, const char *path);

/**
 * @brief Sets the target DPI.
 *
 * Impact units conversion.
 *
 * Default: 96
 */
void svgr_options_set_dpi(svgr_options *opt, float dpi);

/**
 * @brief Sets the default font family.
 *
 * Will be used when no `font-family` attribute is set in the SVG.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Default: Times New Roman
 */
void svgr_options_set_font_family(svgr_options *opt, const char *family);

/**
 * @brief Sets the default font size.
 *
 * Will be used when no `font-size` attribute is set in the SVG.
 *
 * Default: 12
 */
void svgr_options_set_font_size(svgr_options *opt, float size);

/**
 * @brief Sets the `serif` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Times New Roman
 */
void svgr_options_set_serif_family(svgr_options *opt, const char *family);

/**
 * @brief Sets the `sans-serif` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Arial
 */
void svgr_options_set_sans_serif_family(svgr_options *opt, const char *family);

/**
 * @brief Sets the `cursive` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Comic Sans MS
 */
void svgr_options_set_cursive_family(svgr_options *opt, const char *family);

/**
 * @brief Sets the `fantasy` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Papyrus on macOS, Impact on other OS'es
 */
void svgr_options_set_fantasy_family(svgr_options *opt, const char *family);

/**
 * @brief Sets the `monospace` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Courier New
 */
void svgr_options_set_monospace_family(svgr_options *opt, const char *family);

/**
 * @brief Sets a comma-separated list of languages.
 *
 * Will be used to resolve a `systemLanguage` conditional attribute.
 *
 * Example: en,en-US.
 *
 * Must be UTF-8. Can be NULL.
 *
 * Default: en
 */
void svgr_options_set_languages(svgr_options *opt, const char *languages);

/**
 * @brief Sets the default shape rendering method.
 *
 * Will be used when an SVG element's `shape-rendering` property is set to `auto`.
 *
 * Default: `RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION`
 */
void svgr_options_set_shape_rendering_mode(svgr_options *opt, svgr_shape_rendering mode);

/**
 * @brief Sets the default text rendering method.
 *
 * Will be used when an SVG element's `text-rendering` property is set to `auto`.
 *
 * Default: `RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY`
 */
void svgr_options_set_text_rendering_mode(svgr_options *opt, svgr_text_rendering mode);

/**
 * @brief Sets the default image rendering method.
 *
 * Will be used when an SVG element's `image-rendering` property is set to `auto`.
 *
 * Default: `RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY`
 */
void svgr_options_set_image_rendering_mode(svgr_options *opt, svgr_image_rendering mode);

/**
 * @brief Loads a font data into the internal fonts database.
 *
 * Prints a warning into the log when the data is not a valid TrueType font.
 *
 * Has no effect when the `text` feature is not enabled.
 */
void svgr_options_load_font_data(svgr_options *opt, const char *data, uintptr_t len);

/**
 * @brief Loads a font file into the internal fonts database.
 *
 * Prints a warning into the log when the data is not a valid TrueType font.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * @return #svgr_error with RESVG_OK, RESVG_ERROR_NOT_AN_UTF8_STR or RESVG_ERROR_FILE_OPEN_FAILED
 */
int32_t svgr_options_load_font_file(svgr_options *opt, const char *file_path);

/**
 * @brief Loads system fonts into the internal fonts database.
 *
 * This method is very IO intensive.
 *
 * This method should be executed only once per #svgr_options.
 *
 * The system scanning is not perfect, so some fonts may be omitted.
 * Please send a bug report in this case.
 *
 * Prints warnings into the log.
 *
 * Has no effect when the `text` feature is not enabled.
 */
void svgr_options_load_system_fonts(svgr_options *opt);

/**
 * @brief Destroys the #svgr_options.
 */
void svgr_options_destroy(svgr_options *opt);

/**
 * @brief Creates #svgr_render_tree from file.
 *
 * .svg and .svgz files are supported.
 *
 * See #svgr_is_image_empty for details.
 *
 * @param file_path UTF-8 file path.
 * @param opt Rendering options. Must not be NULL.
 * @param tree Parsed render tree. Should be destroyed via #svgr_tree_destroy.
 * @return #svgr_error
 */
int32_t svgr_parse_tree_from_file(const char *file_path,
                                   const svgr_options *opt,
                                   svgr_render_tree **tree);

/**
 * @brief Creates #svgr_render_tree from data.
 *
 * See #svgr_is_image_empty for details.
 *
 * @param data SVG data. Can contain SVG string or gzip compressed data. Must not be NULL.
 * @param len Data length.
 * @param opt Rendering options. Must not be NULL.
 * @param tree Parsed render tree. Should be destroyed via #svgr_tree_destroy.
 * @return #svgr_error
 */
int32_t svgr_parse_tree_from_data(const char *data,
                                   uintptr_t len,
                                   const svgr_options *opt,
                                   svgr_render_tree **tree);

/**
 * @brief Checks that tree has any nodes.
 *
 * @param tree Render tree.
 * @return Returns `true` if tree has no nodes.
 */
bool svgr_is_image_empty(const svgr_render_tree *tree);

/**
 * @brief Returns an image size.
 *
 * The size of a canvas that required to render this SVG.
 *
 * The `width` and `height` attributes in SVG.
 *
 * @param tree Render tree.
 * @return Image size.
 */
svgr_size svgr_get_image_size(const svgr_render_tree *tree);

/**
 * @brief Returns an image viewbox.
 *
 * The `viewBox` attribute in SVG.
 *
 * @param tree Render tree.
 * @return Image viewbox.
 */
svgr_rect svgr_get_image_viewbox(const svgr_render_tree *tree);

/**
 * @brief Returns an image bounding box.
 *
 * Can be smaller or bigger than a `viewbox`.
 *
 * @param tree Render tree.
 * @param bbox Image's bounding box.
 * @return `false` if an image has no elements.
 */
bool svgr_get_image_bbox(const svgr_render_tree *tree, svgr_rect *bbox);

/**
 * @brief Returns `true` if a renderable node with such an ID exists.
 *
 * @param tree Render tree.
 * @param id Node's ID. UTF-8 string. Must not be NULL.
 * @return `true` if a node exists.
 * @return `false` if a node doesn't exist or ID isn't a UTF-8 string.
 * @return `false` if a node exists, but not renderable.
 */
bool svgr_node_exists(const svgr_render_tree *tree, const char *id);

/**
 * @brief Returns node's transform by ID.
 *
 * @param tree Render tree.
 * @param id Node's ID. UTF-8 string. Must not be NULL.
 * @param transform Node's transform.
 * @return `true` if a node exists.
 * @return `false` if a node doesn't exist or ID isn't a UTF-8 string.
 * @return `false` if a node exists, but not renderable.
 */
bool svgr_get_node_transform(const svgr_render_tree *tree,
                              const char *id,
                              svgr_transform *transform);

/**
 * @brief Returns node's bounding box in canvas coordinates by ID.
 *
 * @param tree Render tree.
 * @param id Node's ID. Must not be NULL.
 * @param bbox Node's bounding box.
 * @return `false` if a node with such an ID does not exist
 * @return `false` if ID isn't a UTF-8 string.
 * @return `false` if ID is an empty string
 */
bool svgr_get_node_bbox(const svgr_render_tree *tree, const char *id, svgr_rect *bbox);

/**
 * @brief Returns node's bounding box, including stroke, in canvas coordinates by ID.
 *
 * @param tree Render tree.
 * @param id Node's ID. Must not be NULL.
 * @param bbox Node's bounding box.
 * @return `false` if a node with such an ID does not exist
 * @return `false` if ID isn't a UTF-8 string.
 * @return `false` if ID is an empty string
 */
bool svgr_get_node_stroke_bbox(const svgr_render_tree *tree, const char *id, svgr_rect *bbox);

/**
 * @brief Destroys the #svgr_render_tree.
 */
void svgr_tree_destroy(svgr_render_tree *tree);

/**
 * @brief Renders the #svgr_render_tree onto the pixmap.
 *
 * @param tree A render tree.
 * @param transform A root SVG transform. Can be used to position SVG inside the `pixmap`.
 * @param width Pixmap width.
 * @param height Pixmap height.
 * @param pixmap Pixmap data. Should have width*height*4 size and contain
 *               premultiplied RGBA8888 pixels.
 */
void svgr_render(const svgr_render_tree *tree,
                  svgr_transform transform,
                  uint32_t width,
                  uint32_t height,
                  char *pixmap);

/**
 * @brief Renders a Node by ID onto the image.
 *
 * @param tree A render tree.
 * @param id Node's ID. Must not be NULL.
 * @param transform A root SVG transform. Can be used to position SVG inside the `pixmap`.
 * @param width Pixmap width.
 * @param height Pixmap height.
 * @param pixmap Pixmap data. Should have width*height*4 size and contain
 *               premultiplied RGBA8888 pixels.
 * @return `false` when `id` is not a non-empty UTF-8 string.
 * @return `false` when the selected `id` is not present.
 * @return `false` when an element has a zero bbox.
 */
bool svgr_render_node(const svgr_render_tree *tree,
                       const char *id,
                       svgr_transform transform,
                       uint32_t width,
                       uint32_t height,
                       char *pixmap);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* RESVG_H */
