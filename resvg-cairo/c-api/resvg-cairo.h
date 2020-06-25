/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

/**
 * @file resvg-cairo.h
 *
 * resvg C API for the cairo backend
 */

#ifndef RESVG_CAIRO_H
#define RESVG_CAIRO_H

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>
#include <cairo.h>
#include <resvg.h>

#define RESVG_CAIRO_MAJOR_VERSION 0
#define RESVG_CAIRO_MINOR_VERSION 10
#define RESVG_CAIRO_PATCH_VERSION 0
#define RESVG_CAIRO_VERSION "0.10.0"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Renders the #resvg_render_tree onto the canvas.
 *
 * \b Warning: the canvas must not have a transform.
 *
 * @param tree Render tree.
 * @param size Canvas size.
 * @param cr Canvas.
 */
void resvg_cairo_render_to_canvas(const resvg_render_tree *tree,
                                  resvg_size size,
                                  cairo_t *cr);

/**
 * @brief Renders a Node by ID onto the canvas.
 *
 * \b Warning: the canvas must not have a transform.
 *
 * Does nothing on error.
 *
 * @param tree Render tree.
 * @param size Canvas size.
 * @param id Node's ID.
 * @param cr Canvas.
 */
void resvg_cairo_render_to_canvas_by_id(const resvg_render_tree *tree,
                                        resvg_size size,
                                        const char *id,
                                        cairo_t *cr);

#ifdef __cplusplus
}
#endif

#endif /* RESVG_CAIRO_H */
