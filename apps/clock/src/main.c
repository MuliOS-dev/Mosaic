#include <libnotify/notification.h>
#define MINIAUDIO_IMPLEMENTATION
#include <glib-object.h>
#include <glib.h>
#include <gtk/gtk.h>
#include <libnotify/notify.h>
#include <miniaudio/miniaudio.h>
#include <time.h>
#include <unistd.h>

GtkWidget *TMSTAT;
ma_engine ENGINE;
ma_sound SOUND;
GtkWidget *clock_label;
int sintd = 0;

typedef struct {
  GtkWidget *btn;
  int id;
} Mosaic_BTN;

void noti_helper(char title[], char body[], char icon[]) {
  NotifyNotification *n = notify_notification_new(title, body, NULL);
  GError *err = NULL;
  GdkPixbuf *icn = gdk_pixbuf_new_from_file(icon, &err);
  notify_notification_set_icon_from_pixbuf(n, icn);
  notify_notification_show(n, NULL);
  g_object_unref(n);
}

gboolean tdn(gpointer label) {
  gtk_label_set_text(label, "Timer Ended");
  if (sintd == 0) {
    ma_sound_init_from_file(&ENGINE, "resources/sound/timer_end.wav", 0, NULL,
                            NULL, &SOUND);
    ma_sound_start(&SOUND);
    sintd = 1;
  } else {
    ma_sound_start(&SOUND);
  }
  noti_helper("Timer Ended", "Your timer has ended", "resources/img/clock.png");
  return G_SOURCE_REMOVE;
}

gchar *update() {
  time_t now = time(NULL);
  gchar *timeval = ctime(&now);
  return timeval;
}

gboolean update_clock(gpointer data) {
  gtk_label_set_text(GTK_LABEL(clock_label), update());
  return G_SOURCE_CONTINUE;
}

void timer(GtkButton *button, gpointer data) {
  Mosaic_BTN *mbtn = data;
  switch (mbtn->id) {
  case 0:
    g_timeout_add_seconds(5 * 60, tdn, TMSTAT);
    break;
  case 1:
    g_timeout_add_seconds(10 * 60, tdn, TMSTAT);
  }
}

int main(int argc, char **argv) {

  // init
  notify_init("Clock");
  gtk_init(&argc, &argv);
  Mosaic_BTN fmin;
  fmin.btn = gtk_button_new_with_label("5mins");
  fmin.id = 0;
  Mosaic_BTN tmin;
  tmin.btn = gtk_button_new_with_label("10mins");
  tmin.id = 1;
  GtkWidget *window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
  GtkWidget *box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
  GError *err = NULL;
  GdkPixbuf *icn = gdk_pixbuf_new_from_file("resources/img/clock.png", &err);
  gtk_window_set_icon(GTK_WINDOW(window), icn);
  TMSTAT = gtk_label_new("");
  clock_label = gtk_label_new(update());
  g_timeout_add(1000, update_clock, NULL);
  gtk_container_add(GTK_CONTAINER(window), box);
  gtk_container_add(GTK_CONTAINER(box), fmin.btn);
  gtk_container_add(GTK_CONTAINER(box), tmin.btn);
  gtk_container_add(GTK_CONTAINER(box), clock_label);
  gtk_container_add(GTK_CONTAINER(box), TMSTAT);
  gtk_window_set_title(GTK_WINDOW(window), "Mosaic Clock");
  g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);
  g_signal_connect(fmin.btn, "clicked", G_CALLBACK(timer), &fmin);
  g_signal_connect(tmin.btn, "clicked", G_CALLBACK(timer), &fmin);
  ma_engine_init(NULL, &ENGINE);

  // render
  gtk_widget_show_all(window);
  gtk_main();
}
