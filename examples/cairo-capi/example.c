#include <stdlib.h>

#include <gtk/gtk.h>

#include <resvg.h>

static resvg_render_tree *tree = NULL;
static resvg_options opt;

static gboolean
draw_cb(GtkWidget *widget, cairo_t *cr, gpointer data)
{
    GtkAllocation alloc;
    gtk_widget_get_allocation(widget, &alloc);

    if (tree) {
        resvg_size s = { alloc.width, alloc.height };
        resvg_cairo_render_to_canvas(tree, &opt, s, cr);
    }

    return FALSE;
}

static void
close_window(void)
{
    if (tree) {
        resvg_tree_destroy(tree);
    }
}

static void
parse_doc(const char *path)
{
    resvg_init_options(&opt);
    opt.path = path;
    opt.font_family = "Times New Roman";
    opt.languages = "en";

    int err = resvg_parse_tree_from_file(path, &opt, &tree);
    if (err != RESVG_OK) {
        printf("Error id: %i\n", err);
        abort();
    }
}

static void
activate(GtkApplication *app)
{
    GtkWidget *window;
    GtkWidget *frame;
    GtkWidget *drawing_area;

    window = gtk_application_window_new(app);
    gtk_window_set_title(GTK_WINDOW(window), "Drawing Area");

    g_signal_connect(window, "destroy", G_CALLBACK (close_window), NULL);

    gtk_container_set_border_width(GTK_CONTAINER(window), 8);

    frame = gtk_frame_new(NULL);
    gtk_frame_set_shadow_type(GTK_FRAME(frame), GTK_SHADOW_IN);
    gtk_container_add(GTK_CONTAINER(window), frame);

    drawing_area = gtk_drawing_area_new();
    gtk_widget_set_size_request(drawing_area, 400, 400);

    gtk_container_add(GTK_CONTAINER(frame), drawing_area);

    g_signal_connect(drawing_area, "draw", G_CALLBACK(draw_cb), NULL);

    gtk_widget_set_events(drawing_area, gtk_widget_get_events(drawing_area));

    gtk_widget_show_all(window);
}

static void
open(GApplication *app, GFile **files, gint n_files, const gchar *hint)
{
    gchar *path = g_file_get_path(files[0]);
    parse_doc(path);
    free(path);

    activate(app);
}

int
main(int argc, char **argv)
{
    resvg_init_log();

    GtkApplication *app;
    int status;

    app = gtk_application_new("org.gtk.example", G_APPLICATION_HANDLES_OPEN);
    g_signal_connect(app, "open", G_CALLBACK(open), NULL);
    status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);

    return status;
}
