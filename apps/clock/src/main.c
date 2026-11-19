#include "gtk/gtkcssprovider.h"
#include <libnotify/notification.h>
#include <stddef.h>
#include <stdio.h>
#include <wchar.h>
#define MINIAUDIO_IMPLEMENTATION
#include <glib-object.h>
#include <glib.h>
#include <gtk/gtk.h>
#include <libnotify/notify.h>
#include <miniaudio/miniaudio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>

GtkWidget *TMSTAT;
ma_engine ENGINE;
ma_sound SOUND;
GtkWidget *clock_label;
int sintd = 0;
int DEF_THEME;
int THEME;
char *T1 = "src/styles/sunshine.css";
char *T2 = "src/styles/dark.css";
char *T3 = "src/styles/light.css";
char *currtz;
const char *tzlist[] = {"Europe/Amsterdam",    "Europe/Athens",
                        "Europe/Belgrade",     "Europe/Berlin",
                        "Europe/Brussels",     "Europe/Bucharest",
                        "Europe/Budapest",     "Europe/Copenhagen",
                        "Europe/Dublin",       "Europe/Helsinki",
                        "Europe/Istanbul",     "Europe/Kyiv",
                        "Europe/Lisbon",       "Europe/London",
                        "Europe/Madrid",       "Europe/Moscow",
                        "Europe/Oslo",         "Europe/Paris",
                        "Europe/Prague",       "Europe/Riga",
                        "Europe/Rome",         "Europe/Sofia",
                        "Europe/Stockholm",    "Europe/Vienna",
                        "Europe/Vilnius",      "Europe/Warsaw",
                        "Europe/Zurich",

                        "Africa/Accra",        "Africa/Addis_Ababa",
                        "Africa/Cairo",        "Africa/Casablanca",
                        "Africa/Dakar",        "Africa/Johannesburg",
                        "Africa/Kampala",      "Africa/Lagos",
                        "Africa/Nairobi",      "Africa/Tunis",

                        "Asia/Baghdad",        "Asia/Bangkok",
                        "Asia/Beirut",         "Asia/Colombo",
                        "Asia/Dhaka",          "Asia/Dubai",
                        "Asia/Hong_Kong",      "Asia/Jakarta",
                        "Asia/Jerusalem",      "Asia/Karachi",
                        "Asia/Kathmandu",      "Asia/Kolkata",
                        "Asia/Kuala_Lumpur",   "Asia/Manila",
                        "Asia/Riyadh",         "Asia/Seoul",
                        "Asia/Shanghai",       "Asia/Singapore",
                        "Asia/Taipei",         "Asia/Tehran",
                        "Asia/Tokyo",

                        "Australia/Adelaide",  "Australia/Brisbane",
                        "Australia/Darwin",    "Australia/Melbourne",
                        "Australia/Perth",     "Australia/Sydney",

                        "America/Anchorage",   "America/Argentina/Buenos_Aires",
                        "America/Bogota",      "America/Chicago",
                        "America/Denver",      "America/Halifax",
                        "America/Havana",      "America/Lima",
                        "America/Los_Angeles", "America/Mexico_City",
                        "America/New_York",    "America/Phoenix",
                        "America/Santiago",    "America/Sao_Paulo",
                        "America/St_Johns",    "America/Toronto",
                        "America/Vancouver",

                        "Pacific/Auckland",    "Pacific/Fiji",
                        "Pacific/Guam",        "Pacific/Honolulu",
                        "Pacific/Pago_Pago"};
// int ctzs = 0;
GtkCssProvider *CSS;
GtkWidget *timeinput;
GtkWidget *sunshinetb;
GtkWidget *darktb;
GtkWidget *lightb;
GtkWidget *window;
GtkWidget *lcs;

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

void starttimer(GtkButton *button, gpointer data) {
  g_timeout_add_seconds(atoi(gtk_entry_get_text(GTK_ENTRY(timeinput))) * 60,
                        tdn, TMSTAT);
}

gchar *update() {
  time_t now = time(NULL);
  struct tm *t = localtime(&now);
  gchar buffer[50];
  strftime(buffer, sizeof(buffer), "%T", t);
  return g_strdup(buffer);
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
    break;
  case 2:
    g_timeout_add_seconds(15 * 60, tdn, TMSTAT);
    break;
  case 3:
    g_timeout_add_seconds(30 * 60, tdn, TMSTAT);
  case 4:
    g_timeout_add_seconds(60 * 60, tdn, TMSTAT);
  }
}

int show(char *css) {
  gtk_css_provider_load_from_data(CSS, css, -1, NULL);

  gtk_style_context_add_provider_for_screen(
      gdk_screen_get_default(), GTK_STYLE_PROVIDER(CSS),
      GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
  return 0;
}

void on_timezone_selected(GtkListBox *list_box, GtkListBoxRow *row,
                          gpointer data) {
  const gchar *tz =
      gtk_label_get_text(GTK_LABEL(gtk_bin_get_child(GTK_BIN(row))));
  setenv("TZ", tz, 1);
}

int load_theme(int theme) {
  printf("got called with %i\n", theme);
  switch (theme) {
  case 0: {
    FILE *cssf = fopen(T1, "r");
    if (cssf == NULL) {
      perror("Theme file not found, falling back to gtk theme.");
      return 1;
    }
    fseek(cssf, 0, SEEK_END);
    long size = ftell(cssf);
    rewind(cssf);
    char cssfb[size + 1];
    size_t n = fread(cssfb, 1, size, cssf);
    cssfb[n] = '\0';
    show(cssfb);
    fclose(cssf);
    printf("%s\n", cssfb);
    break;
  }
  case 1: {
    FILE *cssf = fopen(T2, "r");
    if (cssf == NULL) {
      perror("Theme file not found, falling back to gtk theme.");
      return 1;
    }
    fseek(cssf, 0, SEEK_END);
    long size = ftell(cssf);
    rewind(cssf);
    char cssfb[size + 1];
    size_t n = fread(cssfb, 1, size, cssf);
    cssfb[n] = '\0';
    show(cssfb);
    printf("%s\n", cssfb);
    break;
  }
  case 2: {
    FILE *cssf = fopen(T3, "r");
    if (cssf == NULL) {
      perror("Theme file not found, falling back to gtk theme.");
      return 1;
    }
    fseek(cssf, 0, SEEK_END);
    long size = ftell(cssf);
    rewind(cssf);
    char cssfb[size + 1];
    size_t n = fread(cssfb, 1, size, cssf);
    cssfb[n] = '\0';
    show(cssfb);
    printf("%s\n", cssfb);
    break;
  }
  }
  return 0;
}

void load_clock(int clockstate) {
  switch (clockstate) {
  case 0:
    gtk_widget_set_name(clock_label, "round-clock");
    break;
  case 1:
    gtk_widget_set_name(clock_label, "clock_label");
    break;
  }
  gtk_widget_show_all(window);
}

void settheme(GtkButton *button, gpointer data) {
  FILE *themef = fopen("include/deftheme", "w");
  if (gtk_toggle_button_get_active(GTK_TOGGLE_BUTTON(sunshinetb)) == TRUE) {
    fprintf(themef, "0");
    load_theme(0);
  } else if (gtk_toggle_button_get_active(GTK_TOGGLE_BUTTON(darktb)) == TRUE) {
    fprintf(themef, "1");
    load_theme(1);
  } else if (gtk_toggle_button_get_active(GTK_TOGGLE_BUTTON(lightb)) == TRUE) {
    fprintf(themef, "2");
    load_theme(2);
  }
  fclose(themef);
}

void legacyclockload(GtkSwitch *swic, gpointer data) {
  gboolean cstate = gtk_switch_get_active(GTK_SWITCH(lcs));
  FILE *clockf = fopen("include/clockstate", "w");
  if (cstate == FALSE) {
    load_clock(0);
    fputs("0", clockf);
  } else {
    load_clock(1);
    fputs("1", clockf);
  }
  fclose(clockf);
}

int main(int argc, char **argv) {
  FILE *clockfile = fopen("include/clockstate", "r");
  char cbuf[3];
  if (clockfile == NULL) {
    perror("failed to load default clock state, falling back.");
  }
  size_t nc = fread(cbuf, 1, sizeof(cbuf), clockfile);
  cbuf[nc] = '\0';
  int clockstate = atoi(cbuf);

  FILE *themefile = fopen("include/deftheme", "r");
  CSS = gtk_css_provider_new();
  if (themefile == NULL) {
    perror("failed to load default theme, falling back.");
    THEME = 0;
  }
  char tbuf[3];
  size_t n = fread(tbuf, 1, sizeof(tbuf) - 1, themefile);
  tbuf[n] = '\0';
  DEF_THEME = atoi(tbuf);
  switch (DEF_THEME) {
  case 0:
    THEME = DEF_THEME;
    break;
  case 1:
    THEME = DEF_THEME;
    break;
  case 2:
    THEME = DEF_THEME;
    break;
  }
  notify_init("Clock");
  gtk_init(&argc, &argv);
  load_theme(THEME);
  GtkWidget *tzl = gtk_list_box_new();
  GtkWidget *scroll = gtk_scrolled_window_new(NULL, NULL);
  gtk_widget_set_size_request(scroll, 250, 300);
  gtk_container_add(GTK_CONTAINER(scroll), tzl);
  gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(scroll), GTK_POLICY_NEVER,
                                 GTK_POLICY_AUTOMATIC);
  for (gsize i = 0; i < G_N_ELEMENTS(tzlist); i++) {
    GtkWidget *row = gtk_list_box_row_new();
    GtkWidget *label = gtk_label_new(tzlist[i]);

    gtk_widget_set_halign(label, GTK_ALIGN_START);
    gtk_container_add(GTK_CONTAINER(row), label);
    gtk_container_add(GTK_CONTAINER(tzl), row);
  }
  Mosaic_BTN fmin;
  fmin.btn = gtk_button_new_with_label("5mins");
  fmin.id = 0;
  Mosaic_BTN tmin;
  tmin.btn = gtk_button_new_with_label("10mins");
  tmin.id = 1;
  Mosaic_BTN fiftm;
  fiftm.btn = gtk_button_new_with_label("15mins");
  fiftm.id = 2;
  Mosaic_BTN thmin;
  thmin.btn = gtk_button_new_with_label("30mins");
  thmin.id = 3;
  Mosaic_BTN hour;
  hour.btn = gtk_button_new_with_label("1h");
  hour.id = 4;
  timeinput = gtk_entry_new();
  GtkWidget *themesetter = gtk_button_new_with_label("Set as default theme");
  sunshinetb = gtk_radio_button_new_with_label(NULL, "Sunshine");
  darktb = gtk_radio_button_new_with_label_from_widget(
      GTK_RADIO_BUTTON(sunshinetb), "Dark");
  lightb = gtk_radio_button_new_with_label_from_widget(GTK_RADIO_BUTTON(darktb),
                                                       "Light");
  window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
  GtkWidget *legacy = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
  GtkWidget *box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
  GtkWidget *startbtn = gtk_button_new_with_label("Start");
  GError *err = NULL;
  GdkPixbuf *icn = gdk_pixbuf_new_from_file("resources/img/clock.png", &err);
  GtkWidget *row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
  GtkWidget *separator = gtk_separator_new(GTK_ORIENTATION_VERTICAL);
  lcs = gtk_switch_new();
  gtk_box_pack_start(GTK_BOX(row), fmin.btn, FALSE, FALSE, 0);
  gtk_box_pack_start(GTK_BOX(row), tmin.btn, FALSE, FALSE, 0);
  gtk_box_pack_start(GTK_BOX(row), thmin.btn, FALSE, FALSE, 0);
  gtk_box_pack_start(GTK_BOX(row), hour.btn, FALSE, FALSE, 0);
  gtk_entry_set_placeholder_text(GTK_ENTRY(timeinput), "Minutes e.g 25");
  gtk_widget_set_halign(row, GTK_ALIGN_END);
  gtk_window_set_icon(GTK_WINDOW(window), icn);
  TMSTAT = gtk_label_new("");
  GtkWidget *settings = gtk_box_new(GTK_ORIENTATION_VERTICAL, 8);
  clock_label = gtk_label_new(update());
  gtk_widget_set_name(GTK_WIDGET(window), "mainwin");
  if (clockstate == 1) {
    gtk_switch_set_active(GTK_SWITCH(lcs), TRUE);
  } else {
    gtk_switch_set_active(GTK_SWITCH(lcs), FALSE);
  }
  load_clock(clockstate);
  fclose(clockfile);
  GtkWidget *radios = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
  gtk_container_add(GTK_CONTAINER(radios), sunshinetb);
  gtk_container_add(GTK_CONTAINER(radios), darktb);
  gtk_container_add(GTK_CONTAINER(radios), lightb);
  gtk_widget_set_valign(radios, GTK_ALIGN_CENTER);
  GtkWidget *startrow = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
  gtk_box_pack_start(GTK_BOX(settings), radios, FALSE, FALSE, 0);
  gtk_box_pack_start(GTK_BOX(settings), themesetter, FALSE, FALSE, 0);
  gtk_box_pack_start(GTK_BOX(startrow), startbtn, FALSE, FALSE, 0);
  gtk_widget_set_halign(startrow, GTK_ALIGN_END);
  gtk_widget_set_vexpand(box, TRUE);
  gtk_widget_set_valign(box, GTK_ALIGN_FILL);
  g_timeout_add(1000, update_clock, NULL);
  GtkWidget *main_row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
  gtk_box_pack_start(GTK_BOX(legacy), lcs, FALSE, FALSE, 0);
  gtk_widget_set_halign(lcs, GTK_ALIGN_START);
  gtk_widget_set_valign(lcs, GTK_ALIGN_START);
  gtk_widget_set_size_request(lcs, 120, 40);
  GtkWidget *lebal = gtk_label_new("Legacy clock");
  gtk_box_pack_start(GTK_BOX(legacy), lebal, FALSE, FALSE, 0);
  gtk_box_pack_start(GTK_BOX(settings), legacy, FALSE, FALSE, 0);
  gtk_widget_set_size_request(settings, 220, -1);
  gtk_widget_set_margin_start(settings, 20);
  gtk_widget_set_margin_end(settings, 20);
  gtk_widget_set_margin_top(settings, 20);
  gtk_box_pack_start(GTK_BOX(main_row), settings, FALSE, FALSE, 0);
  gtk_box_pack_start(GTK_BOX(main_row), separator, FALSE, FALSE, 0);
  gtk_box_pack_start(GTK_BOX(main_row), box, TRUE, TRUE, 0);
  gtk_container_add(GTK_CONTAINER(window), main_row);
  gtk_widget_set_halign(clock_label, GTK_ALIGN_CENTER);
  gtk_widget_set_valign(clock_label, GTK_ALIGN_CENTER);
  gtk_box_pack_end(GTK_BOX(box), row, FALSE, FALSE, 0);
  gtk_box_pack_end(GTK_BOX(box), startrow, FALSE, FALSE, 0);
  gtk_box_pack_end(GTK_BOX(box), timeinput, FALSE, FALSE, 0);
  gtk_box_pack_start(GTK_BOX(settings), scroll, FALSE, FALSE, 0);
  gtk_window_set_title(GTK_WINDOW(window), "Mosaic Clock");
  gtk_widget_set_name(scroll, "tzlistcss");
  gtk_widget_set_halign(fmin.btn, GTK_ALIGN_END);
  gtk_widget_set_valign(fmin.btn, GTK_ALIGN_START);
  gtk_widget_set_halign(fiftm.btn, GTK_ALIGN_END);
  gtk_widget_set_valign(fiftm.btn, GTK_ALIGN_START);
  gtk_widget_set_halign(tmin.btn, GTK_ALIGN_END);
  gtk_widget_set_valign(tmin.btn, GTK_ALIGN_START);
  gtk_widget_set_halign(thmin.btn, GTK_ALIGN_END);
  gtk_widget_set_valign(thmin.btn, GTK_ALIGN_START);
  gtk_widget_set_halign(hour.btn, GTK_ALIGN_END);
  gtk_widget_set_valign(hour.btn, GTK_ALIGN_START);
  gtk_widget_set_valign(startbtn, GTK_ALIGN_START);
  gtk_widget_set_halign(startbtn, GTK_ALIGN_END);
  gtk_widget_set_halign(sunshinetb, GTK_ALIGN_START);
  gtk_widget_set_valign(sunshinetb, GTK_ALIGN_START);
  gtk_widget_set_halign(darktb, GTK_ALIGN_START);
  gtk_widget_set_valign(darktb, GTK_ALIGN_START);
  gtk_widget_set_halign(lightb, GTK_ALIGN_START);
  gtk_widget_set_valign(lightb, GTK_ALIGN_START);
  gtk_widget_set_halign(timeinput, GTK_ALIGN_END);
  gtk_widget_set_halign(themesetter, GTK_ALIGN_END);
  gtk_widget_set_valign(themesetter, GTK_ALIGN_START);
  gtk_widget_set_halign(radios, GTK_ALIGN_START);
  gtk_widget_set_valign(radios, GTK_ALIGN_START);
  gtk_widget_set_size_request(timeinput, 240, -1);
  gtk_widget_set_size_request(sunshinetb, 100, 20);
  gtk_widget_set_size_request(darktb, 100, 20);
  gtk_widget_set_size_request(lightb, 100, 20);
  gtk_widget_set_size_request(fmin.btn, 240, 80);
  gtk_widget_set_size_request(startbtn, 240, 80);
  gtk_widget_set_size_request(tmin.btn, 240, 80);
  gtk_widget_set_size_request(thmin.btn, 240, 80);
  gtk_widget_set_size_request(hour.btn, 240, 80);
  gtk_widget_set_size_request(themesetter, 240, 80);
  gtk_widget_set_size_request(clock_label, 240, 100);
  gtk_widget_set_margin_end(startbtn, 20);
  gtk_widget_set_margin_end(tmin.btn, 8);
  gtk_widget_set_margin_end(sunshinetb, 20);
  gtk_widget_set_margin_end(darktb, 20);
  gtk_widget_set_margin_end(lightb, 20);
  gtk_widget_set_margin_end(fmin.btn, 20);
  gtk_widget_set_margin_end(thmin.btn, 20);
  gtk_widget_set_margin_end(hour.btn, 20);
  gtk_widget_set_margin_end(themesetter, 8);
  gtk_widget_set_margin_bottom(startbtn, 8);
  gtk_widget_set_margin_bottom(tmin.btn, 20);
  gtk_widget_set_margin_end(fmin.btn, 8);
  gtk_widget_set_margin_bottom(fmin.btn, 20);
  gtk_widget_set_margin_bottom(sunshinetb, 8);
  gtk_widget_set_margin_bottom(darktb, 8);
  gtk_widget_set_margin_bottom(lightb, 8);
  gtk_widget_set_margin_bottom(timeinput, 8);
  gtk_widget_set_margin_end(timeinput, 20);
  gtk_widget_set_margin_top(timeinput, 20);
  gtk_widget_set_margin_end(thmin.btn, 8);
  gtk_widget_set_margin_end(hour.btn, 20);
  gtk_widget_set_margin_start(radios, 20);
  gtk_box_pack_end(GTK_BOX(box), clock_label, FALSE, FALSE, 0);

  g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);
  g_signal_connect(fmin.btn, "clicked", G_CALLBACK(timer), &fmin);
  g_signal_connect(tmin.btn, "clicked", G_CALLBACK(timer), &tmin);
  g_signal_connect(thmin.btn, "clicked", G_CALLBACK(timer), &thmin);
  g_signal_connect(hour.btn, "clicked", G_CALLBACK(timer), &hour);
  g_signal_connect(themesetter, "clicked", G_CALLBACK(settheme), &themesetter);
  g_signal_connect(startbtn, "clicked", G_CALLBACK(starttimer), startbtn);
  g_signal_connect(lcs, "notify::active", G_CALLBACK(legacyclockload), &legacy);
  g_signal_connect(tzl, "row-activated", G_CALLBACK(on_timezone_selected),
                   NULL);
  ma_engine_init(NULL, &ENGINE);
  gtk_window_set_default_size(GTK_WINDOW(window), 1300, 650);

  gtk_widget_show_all(window);
  gtk_main();
}
