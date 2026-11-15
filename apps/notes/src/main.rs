use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Timeout;
use leptos::*;
use serde::{Deserialize, Serialize};

const NOTES_KEY: &str = "mosaic.notes";
const SETTINGS_KEY: &str = "mosaic.settings";

// ───────────────────────── Data ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Note {
    id: String,
    title: String,
    body: String,
    tag: String,
    color: String,
    updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Settings {
    theme: String,
    font: String,
    density: String,
    blur: String,
    title_scale: String,
    radius: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "light".into(),
            font: "sans".into(),
            density: "comfortable".into(),
            blur: "frosted".into(),
            title_scale: "standard".into(),
            radius: "24px".into(),
        }
    }
}

fn pick(t: &str, l: &'static str, d: &'static str, s: &'static str) -> &'static str {
    match t {
        "dark" => d,
        "sunshine" => s,
        _ => l,
    }
}

struct Metrics {
    title_size: String,
    title_lh: f64,
    body_size: String,
    body_lh: f64,
}

fn dynamic_metrics(title: &str, body: &str, s: &Settings) -> Metrics {
    let title = title.trim();
    let title_len = title.chars().count();
    let longest = title
        .split_whitespace()
        .map(|w| w.chars().count())
        .max()
        .unwrap_or(0);
    let body_len = body.trim().chars().count();

    let mult = match s.title_scale.as_str() {
        "subtle" => 0.9,
        "expressive" => 1.12,
        _ => 1.0,
    };

    let (mut base, lh) = if title_len <= 14 && longest <= 11 {
        (1.5, 1.16)
    } else if title_len <= 28 && longest <= 14 {
        (1.28, 1.2)
    } else if title_len <= 48 && longest <= 18 {
        (1.1, 1.24)
    } else if title_len <= 76 {
        (0.96, 1.26)
    } else {
        (0.88, 1.28)
    };
    if s.density == "compact" {
        base *= 0.9;
    }

    let (mut body_px, body_lh) = if body_len > 420 {
        (12.25, 1.44)
    } else if body_len > 240 {
        (12.75, 1.48)
    } else if body_len > 120 {
        (13.5, 1.52)
    } else {
        (14.0, 1.55)
    };
    if s.density == "compact" {
        body_px = f64::max(11.5, body_px - 0.75);
    }

    let final_rem = f64::max(0.84, (base * mult * 1000.0_f64).round() / 1000.0);
    Metrics {
        title_size: format!("{}rem", final_rem),
        title_lh: lh,
        body_size: format!("{}px", body_px),
        body_lh,
    }
}

fn svg_html(cls: &str, stroke_w: &str, paths: &str) -> String {
    format!(
        r#"<svg class="{cls}" fill="none" stroke="currentColor" stroke-width="{stroke_w}" viewBox="0 0 24 24">{paths}</svg>"#
    )
}

const PLUS: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15"/>"#;
const CLOSE: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12"/>"#;
const SEARCH: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-4.35-4.35m0 0A7.5 7.5 0 103.5 10.5a7.5 7.5 0 0013.15 6.15z"/>"#;
const GEAR: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>"#;
const TRASH: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0"/>"#;
const DOC: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z"/>"#;

const SUNSHINE_SVG: &str = r##"<svg class="w-full h-full object-cover" viewBox="0 0 1440 960" preserveAspectRatio="none" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <filter id="paperShadowLeft" x="-10%" y="-10%" width="130%" height="130%"><feDropShadow dx="4" dy="0" stdDeviation="8" flood-color="#b45309" flood-opacity="0.22" /></filter>
    <filter id="paperShadowRight" x="-15%" y="-10%" width="130%" height="130%"><feDropShadow dx="-6" dy="4" stdDeviation="12" flood-color="#9a3412" flood-opacity="0.24" /></filter>
  </defs>
  <rect width="1440" height="960" fill="#FA9E2E" />
  <linearGradient id="tangerineDepth" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0%" stop-color="#FBAA3B" stop-opacity="0.7" />
    <stop offset="100%" stop-color="#F2891D" stop-opacity="0.85" />
  </linearGradient>
  <rect width="1440" height="960" fill="url(#tangerineDepth)" />
  <rect x="0" y="0" width="310" height="960" fill="#F8D930" filter="url(#paperShadowLeft)" />
  <polygon points="855,0 1370,0 810,960 580,960" fill="#FCE467" filter="url(#paperShadowRight)" />
</svg>"##;

// ───────────────────────── App ─────────────────────────

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let notes: RwSignal<Vec<Note>> =
        create_rw_signal(LocalStorage::get(NOTES_KEY).unwrap_or_default());
    let settings: RwSignal<Settings> =
        create_rw_signal(LocalStorage::get(SETTINGS_KEY).unwrap_or_default());
    let active: RwSignal<Option<String>> = create_rw_signal(None);
    let search: RwSignal<String> = create_rw_signal(String::new());
    let editor_open = create_rw_signal(false);
    let settings_open = create_rw_signal(false);
    let toast: RwSignal<(String, bool)> = create_rw_signal(("Preferences saved".into(), false));
    let editor_ts = create_rw_signal(String::from("Edited just now"));

    let title_ref = create_node_ref::<html::Input>();
    let body_ref = create_node_ref::<html::Textarea>();
    let tag_ref = create_node_ref::<html::Input>();
    let search_ref = create_node_ref::<html::Input>();

    // theme picker helper
    let tp = move |l: &'static str, d: &'static str, s: &'static str| -> &'static str {
        settings.with(|c| pick(&c.theme, l, d, s))
    };

    // persistence
    create_effect(move |_| {
        let _ = LocalStorage::set(NOTES_KEY, notes.get());
    });
    create_effect(move |_| {
        let _ = LocalStorage::set(SETTINGS_KEY, settings.get());
    });

    // body theme class
    create_effect(move |_| {
        let t = settings.with(|c| c.theme.clone());
        if let Some(body) = document().body() {
            body.set_class_name(&format!(
                "theme-{t} antialiased selection:bg-indigo-500/15 selection:text-indigo-700 min-h-screen overflow-x-hidden transition-colors duration-300"
            ));
        }
    });

    let show_toast = move |msg: &str| {
        toast.set((msg.to_string(), true));
        Timeout::new(2200, move || toast.update(|t| t.1 = false)).forget();
    };

    let update_word_count_source = move || -> usize {
        let id = active.get();
        notes.with(|ns| {
            ns.iter()
                .find(|n| Some(&n.id) == id.as_ref())
                .map(|n| n.body.split_whitespace().count())
                .unwrap_or(0)
        })
    };

    let select_and_open = move |id: String| {
        let note = notes.with(|ns| ns.iter().find(|n| n.id == id).cloned());
        let Some(note) = note else { return };
        active.set(Some(id));
        if let Some(el) = title_ref.get() {
            el.set_value(&note.title);
        }
        if let Some(el) = body_ref.get() {
            el.set_value(&note.body);
        }
        if let Some(el) = tag_ref.get() {
            el.set_value(&note.tag);
        }
        editor_ts.set(format!("Updated {}", note.updated_at));
        editor_open.set(true);
    };

    let close_editor = move || editor_open.set(false);

    let close_settings = move || {
        settings_open.set(false);
        show_toast("Preferences updated");
    };

    let create_new_note = move || {
        let id = format!("note-{}", js_sys::Date::now() as u64);
        let n = Note {
            id: id.clone(),
            title: String::new(),
            body: String::new(),
            tag: "Draft".into(),
            color: "#6366f1".into(),
            updated_at: "Just now".into(),
        };
        notes.update(|ns| ns.insert(0, n));
        select_and_open(id);
        Timeout::new(50, move || {
            if let Some(el) = title_ref.get() {
                let _ = el.focus();
            }
        })
        .forget();
    };

    let delete_current = move || {
        let id = active.get();
        notes.update(|ns| ns.retain(|n| Some(&n.id) != id.as_ref()));
        active.set(notes.with(|ns| ns.first().map(|n| n.id.clone())));
        close_editor();
        show_toast("Note deleted");
    };

    let live_edit = move || {
        let Some(id) = active.get() else { return };
        let title = title_ref.get().map(|e| e.value()).unwrap_or_default();
        let body = body_ref.get().map(|e| e.value()).unwrap_or_default();
        let tag = tag_ref.get().map(|e| e.value()).unwrap_or_default();
        notes.update(|ns| {
            if let Some(n) = ns.iter_mut().find(|n| n.id == id) {
                n.title = title;
                n.body = body;
                n.tag = if tag.is_empty() { "Note".into() } else { tag };
                n.updated_at = "Just now".into();
            }
        });
        editor_ts.set("Updated just now".into());
    };

    let set_note_color = move |hex: &'static str| {
        let Some(id) = active.get() else { return };
        notes.update(|ns| {
            if let Some(n) = ns.iter_mut().find(|n| n.id == id) {
                n.color = hex.to_string();
            }
        });
    };

    let update_setting = move |key: &'static str, value: &'static str| {
        settings.update(|s| match key {
            "theme" => s.theme = value.into(),
            "font" => s.font = value.into(),
            "density" => s.density = value.into(),
            "blur" => s.blur = value.into(),
            "titleScale" => s.title_scale = value.into(),
            "radius" => s.radius = value.into(),
            _ => {}
        });
    };

    // keyboard shortcuts
    window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            close_editor();
            close_settings();
        }
        if e.key() == "/" {
            let tag = document()
                .active_element()
                .map(|a| a.tag_name())
                .unwrap_or_default();
            if tag != "INPUT" && tag != "TEXTAREA" {
                e.prevent_default();
                if let Some(el) = search_ref.get() {
                    let _ = el.focus();
                }
            }
        }
    });

    let filtered = move || -> Vec<Note> {
        let q = search.get().to_lowercase();
        let q = q.trim().to_string();
        notes.with(|ns| {
            ns.iter()
                .filter(|n| {
                    q.is_empty()
                        || n.title.to_lowercase().contains(&q)
                        || n.body.to_lowercase().contains(&q)
                        || n.tag.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        })
    };

    let grid_classes = move || {
        if settings.with(|c| c.density == "compact") {
            "grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4"
        } else {
            "grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-5"
        }
    };

    let font_class = move || format!("note-font-{}", settings.with(|c| c.font.clone()));

    // settings-modal button styling
    let sbtn = move |active_now: bool, extra: &'static str| -> String {
        let a = tp(
            "border-slate-900 bg-slate-900 text-white font-medium",
            "border-white bg-white text-slate-900 font-semibold",
            "border-amber-950 bg-amber-950 text-amber-50 font-semibold",
        );
        let i = tp(
            "border-slate-200 bg-white text-slate-700 hover:bg-slate-50",
            "border-white/10 bg-white/[0.04] text-slate-300 hover:bg-white/[0.09]",
            "border-amber-950/15 bg-white/75 text-amber-950 hover:bg-white",
        );
        format!("{} border text-xs transition {}", extra, if active_now { a } else { i })
    };

    let setting_label = "block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-2.5";

    view! {
        // Sunshine geometric backdrop
        <div
            class=move || if settings.with(|c| c.theme == "sunshine") {
                "fixed inset-0 z-0 pointer-events-none overflow-hidden"
            } else {
                "fixed inset-0 z-0 pointer-events-none hidden overflow-hidden"
            }
            aria-hidden="true"
            inner_html=SUNSHINE_SVG
        ></div>

        <div id="appShell" class="relative z-10 min-h-screen flex flex-row transition-colors duration-300">
            // ───────── Sidebar ─────────
            <aside id="sidebarContainer" class="sticky top-0 left-0 z-30 h-screen w-[240px] sm:w-[275px] lg:w-[295px] shrink-0">
                <div id="sidebarBackdropLayer" class="absolute inset-0 overflow-hidden rounded-r-[28px] pointer-events-none select-none transition-colors duration-300"></div>

                <div
                    id="sidebarGlassSurface"
                    class=move || format!(
                        "sidebar-glass-{} relative h-full w-full rounded-r-[28px] rounded-l-none border-y border-r border-l-0 flex flex-col justify-between p-5 sm:p-6 overflow-hidden transition-all duration-300",
                        settings.with(|c| c.blur.clone())
                    )
                >
                    <div>
                        <div class="px-1 pt-1">
                            <h1 class=move || format!(
                                "text-[26px] sm:text-[30px] leading-tight font-bold tracking-tight {}",
                                tp("text-slate-900", "text-white", "text-amber-950")
                            )>"Mosaic Notes"</h1>
                        </div>

                        <button
                            on:click=move |_| { settings_open.set(true) }
                            class=move || format!(
                                "mt-3 w-full flex items-center gap-2.5 px-3 py-2.5 rounded-2xl text-sm font-medium active:scale-[0.99] transition-all group {}",
                                tp(
                                    "text-slate-600 hover:text-slate-900 hover:bg-slate-900/[0.05]",
                                    "text-slate-300 hover:text-white hover:bg-white/[0.07]",
                                    "text-amber-950/80 hover:text-amber-950 hover:bg-amber-950/[0.07]",
                                )
                            )
                        >
                            <span class="contents" inner_html=move || svg_html(
                                &format!("w-[18px] h-[18px] group-hover:rotate-45 transition-transform duration-300 {}",
                                    tp("text-slate-400 group-hover:text-slate-700", "text-slate-400 group-hover:text-white", "text-amber-800/75 group-hover:text-amber-950")),
                                "1.8", GEAR)></span>
                            <span>"Settings"</span>
                        </button>

                        <div class=move || format!("my-4 h-px {}", tp("bg-slate-900/[0.06]", "bg-white/[0.08]", "bg-amber-950/[0.12]"))></div>

                        <div class="flex items-center justify-between px-2 mb-2">
                            <div class="flex items-center gap-2">
                                <span class=move || format!("text-[11px] font-semibold uppercase tracking-wider {}",
                                    tp("text-slate-400", "text-slate-400", "text-amber-900/65"))>"Recent"</span>
                                <span class=move || format!("text-[11px] font-medium {}",
                                    tp("text-slate-400", "text-slate-500", "text-amber-900/65"))>
                                    {move || notes.with(|n| n.len())}
                                </span>
                            </div>
                            <button
                                title="Add note" aria-label="Add note"
                                on:click=move |_| create_new_note()
                                class=move || format!("w-6 h-6 rounded-lg flex items-center justify-center transition {}",
                                    tp("text-slate-400 hover:text-slate-800 hover:bg-slate-900/[0.06]",
                                       "text-slate-400 hover:text-white hover:bg-white/[0.08]",
                                       "text-amber-900/70 hover:text-amber-950 hover:bg-amber-950/[0.08]"))
                                inner_html=svg_html("w-3.5 h-3.5", "2.2", PLUS)
                            ></button>
                        </div>
                    </div>

                    <nav class="flex-1 overflow-y-auto -mx-2 px-2 space-y-1 pr-1" aria-label="Recent notes">
                        {move || {
                            let all = notes.get();
                            if all.is_empty() {
                                let muted = tp("text-xs text-slate-400", "text-xs text-slate-500", "text-xs text-amber-950/60");
                                return view! {
                                    <div class="px-3 py-6 text-center"><p class=muted>"No recent notes yet"</p></div>
                                }.into_view();
                            }
                            let act = active.get();
                            all.into_iter().map(|note| {
                                let is_active = act.as_ref() == Some(&note.id);
                                let item = if is_active {
                                    tp("bg-slate-900/[0.07] text-slate-900 font-semibold shadow-2xs",
                                       "bg-white/[0.11] text-white font-semibold shadow-2xs",
                                       "bg-white/65 text-amber-950 font-semibold shadow-2xs")
                                } else {
                                    tp("text-slate-600 hover:bg-slate-900/[0.04] hover:text-slate-900 font-medium",
                                       "text-slate-300 hover:bg-white/[0.06] hover:text-white font-medium",
                                       "text-amber-950/80 hover:bg-white/40 hover:text-amber-950 font-medium")
                                };
                                let hint = tp("text-[11px] text-slate-400 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity",
                                              "text-[11px] text-slate-400 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity",
                                              "text-[11px] text-amber-900/65 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity");
                                let title = if note.title.trim().is_empty() { "Untitled Note".to_string() } else { note.title.clone() };
                                let id = note.id.clone();
                                view! {
                                    <button
                                        on:click=move |_| select_and_open(id.clone())
                                        class=format!("w-full text-left px-3 py-2.5 rounded-2xl flex items-center justify-between gap-2.5 transition-all group {item}")
                                    >
                                        <div class="flex items-center gap-2.5 min-w-0">
                                            <span class="w-2 h-2 rounded-full shrink-0 transition-transform group-hover:scale-125"
                                                  style=format!("background-color: {}", note.color)></span>
                                            <span class="text-[13.5px] truncate">{title}</span>
                                        </div>
                                        <span class=hint>"Open"</span>
                                    </button>
                                }
                            }).collect_view()
                        }}
                    </nav>
                </div>
            </aside>

            // ───────── Main ─────────
            <main id="mainWorkspace" class="flex-1 min-w-0 flex flex-col px-5 sm:px-8 lg:px-10 pt-6 pb-12">
                <header class="flex items-center justify-between gap-3 mb-7">
                    <div class="flex items-center gap-3 flex-1 max-w-xl">
                        <div class="relative flex-1">
                            <span class="contents" inner_html=move || svg_html(
                                &format!("w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 pointer-events-none {}",
                                    tp("text-slate-400", "text-slate-400", "text-amber-900/60")),
                                "2", SEARCH)></span>
                            <input
                                node_ref=search_ref
                                type="text"
                                placeholder="Search Notes..."
                                on:input=move |e| search.set(event_target_value(&e))
                                prop:value=move || search.get()
                                class=move || format!("w-full pl-10 pr-9 py-2.5 rounded-2xl text-sm transition-all focus:outline-none {}",
                                    tp("bg-slate-50/80 hover:bg-slate-100/70 focus:bg-white border border-slate-200/80 focus:border-slate-300 focus:ring-4 focus:ring-slate-900/[0.04] text-slate-800 placeholder:text-slate-400",
                                       "bg-white/[0.06] hover:bg-white/[0.09] focus:bg-[#161922] border border-white/[0.1] focus:border-white/25 focus:ring-4 focus:ring-white/[0.05] text-white placeholder:text-slate-400",
                                       "bg-white/65 hover:bg-white/80 focus:bg-white/95 backdrop-blur-md border border-white/70 focus:border-white focus:ring-4 focus:ring-amber-950/[0.06] text-amber-950 placeholder:text-amber-900/55 shadow-2xs"))
                            />
                            <button
                                on:click=move |_| search.set(String::new())
                                class=move || if search.with(|s| s.trim().is_empty()) {
                                    "hidden absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 p-0.5 rounded-full"
                                } else {
                                    "absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 p-0.5 rounded-full"
                                }
                                inner_html=svg_html("w-3.5 h-3.5", "2.2", CLOSE)
                            ></button>
                        </div>
                    </div>

                    <div class="flex items-center gap-2">
                        <button
                            title="Add note" aria-label="Add note"
                            on:click=move |_| create_new_note()
                            class=move || format!("w-10 h-10 rounded-2xl flex items-center justify-center transition-all active:scale-[0.97] {}",
                                tp("bg-white hover:bg-slate-50 text-slate-600 hover:text-slate-900 border border-slate-200/90 shadow-2xs",
                                   "bg-white/[0.06] hover:bg-white/[0.12] text-slate-200 hover:text-white border border-white/[0.1]",
                                   "bg-white/70 hover:bg-white/90 backdrop-blur-md text-amber-950 border border-white/75 shadow-2xs"))
                            inner_html=svg_html("w-4 h-4", "2.2", PLUS)
                        ></button>
                    </div>
                </header>

                <div class="flex items-center justify-between mb-5 px-0.5">
                    <div class="flex items-center gap-2.5">
                        <h2 class=move || format!("text-sm font-semibold {}", tp("text-slate-500", "text-slate-300", "text-amber-950/85"))>
                            {move || {
                                let q = search.get();
                                if q.trim().is_empty() { "All Notes".to_string() } else { format!("Results for \"{}\"", q) }
                            }}
                        </h2>
                        <span class=move || format!("px-2 py-0.5 text-xs rounded-full {}",
                            tp("font-medium bg-slate-100 text-slate-600",
                               "font-medium bg-white/[0.08] text-slate-300",
                               "font-semibold bg-white/70 text-amber-950 border border-white/60"))>
                            {move || filtered().len()}
                        </span>
                    </div>
                    <div class=move || format!("text-xs hidden sm:block {}", tp("text-slate-400", "text-slate-500", "font-medium text-amber-950/65"))>
                        {move || if notes.with(|n| n.is_empty()) && search.with(|s| s.trim().is_empty()) {
                            "Your workspace is ready"
                        } else {
                            "Click any square card to edit"
                        }}
                    </div>
                </div>

                // Notes grid
                <div class=move || {
                    if notes.with(|n| n.is_empty()) && search.with(|s| s.trim().is_empty()) {
                        "hidden".to_string()
                    } else { grid_classes().to_string() }
                }>
                    {move || {
                        let list = filtered();
                        let act = active.get();
                        let s = settings.get();
                        let fc = font_class();
                        list.into_iter().map(|note| {
                            let title = if note.title.trim().is_empty() { "Untitled Note".to_string() } else { note.title.clone() };
                            let body = if note.body.trim().is_empty() { "No additional text yet...".to_string() } else { note.body.clone() };
                            let selected = act.as_ref() == Some(&note.id);
                            let m = dynamic_metrics(&title, &body, &s);

                            let (surface, title_c, body_c, foot_b, tag_c, date_c) = match s.theme.as_str() {
                                "dark" => (
                                    format!("bg-[#161922] border border-white/[0.08] hover:border-white/[0.16] shadow-lg shadow-black/30 {}", if selected { "ring-2 ring-white/25" } else { "" }),
                                    "text-white group-hover:text-white", "text-slate-300", "border-white/[0.07]", "text-slate-300", "text-slate-500"),
                                "sunshine" => (
                                    format!("bg-white/75 hover:bg-white/88 backdrop-blur-xl border border-white/80 shadow-md shadow-amber-950/10 {}", if selected { "ring-2 ring-amber-950/30" } else { "" }),
                                    "text-amber-950", "text-amber-950/85", "border-amber-950/[0.08]", "text-amber-950/80", "text-amber-900/60"),
                                _ => (
                                    format!("bg-white shadow-card hover:shadow-card-hover {}", if selected { "ring-2 ring-slate-900/15" } else { "" }),
                                    "text-slate-900 group-hover:text-black", "text-slate-600", "border-slate-100/90", "text-slate-500", "text-slate-400"),
                            };
                            let id = note.id.clone();
                            view! {
                                <article
                                    on:click=move |_| select_and_open(id.clone())
                                    style=format!("border-radius: {};", s.radius)
                                    class=format!("group relative aspect-square p-5 sm:p-6 flex flex-col justify-between cursor-pointer transition-all duration-200 ease-out hover:-translate-y-1 {surface}")
                                >
                                    <div class="min-h-0 flex-1 flex flex-col overflow-hidden">
                                        <h3
                                            style=format!("font-size: {}; line-height: {}; overflow-wrap: anywhere; word-break: normal;", m.title_size, m.title_lh)
                                            class=format!("{fc} font-bold tracking-tight {title_c} shrink-0 transition-[font-size] duration-150")
                                        >{title}</h3>
                                        <div
                                            on:wheel=|e| e.stop_propagation()
                                            style=format!("font-size: {}; line-height: {};", m.body_size, m.body_lh)
                                            class=format!("{fc} card-body-scroll mt-2.5 flex-1 min-h-0 overflow-y-auto pr-1 whitespace-pre-wrap break-words {body_c}")
                                        >{body}</div>
                                    </div>
                                    <div class=format!("pt-3 mt-2 shrink-0 flex items-center justify-between border-t {foot_b} text-xs")>
                                        <div class="inline-flex items-center gap-1.5">
                                            <span class="w-2 h-2 rounded-full" style=format!("background-color: {}", note.color)></span>
                                            <span class=format!("font-medium {tag_c}")>{if note.tag.is_empty() { "Note".to_string() } else { note.tag.clone() }}</span>
                                        </div>
                                        <span class=date_c>{note.updated_at.clone()}</span>
                                    </div>
                                </article>
                            }
                        }).collect_view()
                    }}
                </div>

                // Initial empty state (0 notes)
                <div class=move || {
                    if notes.with(|n| n.is_empty()) && search.with(|s| s.trim().is_empty()) {
                        grid_classes().to_string()
                    } else { format!("hidden {}", grid_classes()) }
                }>
                    <button
                        on:click=move |_| create_new_note()
                        style=move || format!("border-radius: {};", settings.with(|c| c.radius.clone()))
                        class=move || format!("group aspect-square border-2 border-dashed p-7 flex flex-col items-center justify-center text-center transition-all duration-200 hover:-translate-y-1 cursor-pointer {}",
                            tp("border-slate-200/90 hover:border-slate-300 bg-slate-50/40 hover:bg-slate-50/90",
                               "border-white/15 hover:border-white/30 bg-white/[0.03] hover:bg-white/[0.06]",
                               "border-white/75 hover:border-white bg-white/45 hover:bg-white/65 backdrop-blur-md shadow-sm"))
                    >
                        <div
                            class=move || format!("w-12 h-12 rounded-2xl flex items-center justify-center group-hover:scale-105 transition-all mb-3.5 {}",
                                tp("bg-white border border-slate-200/80 shadow-2xs text-slate-400 group-hover:text-slate-800",
                                   "bg-white/[0.07] border border-white/10 text-slate-300 group-hover:text-white",
                                   "bg-white/85 border border-white shadow-2xs text-amber-800 group-hover:text-amber-950"))
                            inner_html=svg_html("w-5 h-5", "2.2", PLUS)
                        ></div>
                        <span class=move || format!("text-base font-semibold {}",
                            tp("text-slate-700 group-hover:text-slate-900", "text-slate-200 group-hover:text-white", "text-amber-950"))>"Start writing"</span>
                        <span class=move || format!("text-xs mt-1 {}", tp("text-slate-400", "text-slate-400", "text-amber-950/75"))>"Click + to create your first note"</span>
                    </button>
                </div>

                // Empty search state
                <div class=move || {
                    let none = notes.with(|n| n.is_empty()) && search.with(|s| s.trim().is_empty());
                    if !none && filtered().is_empty() {
                        "flex flex-col items-center justify-center py-20 text-center"
                    } else { "hidden flex-col items-center justify-center py-20 text-center" }
                }>
                    <div class="w-14 h-14 rounded-3xl bg-slate-100 flex items-center justify-center text-slate-400 mb-4"
                         inner_html=svg_html("w-6 h-6", "1.8", DOC)></div>
                    <h3 class="text-base font-semibold text-slate-800">"No matching notes"</h3>
                    <p class="text-sm text-slate-500 mt-1 max-w-xs">"Try another search term or create a fresh note."</p>
                    <button on:click=move |_| search.set(String::new())
                        class="mt-4 px-4 py-2 rounded-xl bg-slate-900 text-white text-xs font-medium hover:bg-slate-800 transition">"Reset Search"</button>
                </div>
            </main>
        </div>

        // ───────── Editor modal ─────────
        <div
            class=move || format!("fixed inset-0 z-50 {} items-center justify-center p-4 sm:p-6 bg-slate-900/25 backdrop-blur-sm transition-opacity",
                if editor_open.get() { "flex" } else { "hidden" })
        >
            <div class=move || format!("w-full max-w-2xl rounded-[28px] shadow-2xl border overflow-hidden flex flex-col max-h-[88vh] {}",
                tp("bg-white border-slate-200/90", "bg-[#161922] border-white/10", "bg-[#FFFDF5] border-amber-200/90"))>
                <div class=move || format!("flex items-center justify-between px-6 py-4 border-b {}",
                    tp("border-slate-100", "border-white/[0.08]", "border-amber-950/[0.08]"))>
                    <div class="flex items-center gap-2.5">
                        <div class="flex items-center gap-1.5" title="Note accent dot">
                            {[("#6366f1", "bg-indigo-500"), ("#0ea5e9", "bg-sky-500"), ("#10b981", "bg-emerald-500"),
                              ("#f59e0b", "bg-amber-500"), ("#f43f5e", "bg-rose-500")]
                                .into_iter().map(|(hex, bg)| view! {
                                    <button type="button"
                                        on:click=move |_| set_note_color(hex)
                                        class=move || {
                                            let cur = active.get().and_then(|id| notes.with(|ns| ns.iter().find(|n| n.id == id).map(|n| n.color.clone())));
                                            format!("color-dot w-4 h-4 rounded-full {bg} ring-offset-2 transition {}",
                                                if cur.as_deref() == Some(hex) { "ring-2 ring-slate-400 scale-110" } else { "" })
                                        }></button>
                                }).collect_view()}
                        </div>
                        <span class="text-xs text-slate-300">"|"</span>
                        <span class="text-xs text-slate-400">{move || editor_ts.get()}</span>
                    </div>
                    <div class="flex items-center gap-1.5">
                        <button on:click=move |_| delete_current() title="Delete note"
                            class="p-2 rounded-xl text-slate-400 hover:text-rose-600 hover:bg-rose-500/10 transition"
                            inner_html=svg_html("w-4 h-4", "2", TRASH)></button>
                        <button on:click=move |_| close_editor()
                            class=move || format!("px-3.5 py-1.5 rounded-xl text-xs transition {}",
                                tp("bg-slate-900 text-white font-medium hover:bg-slate-800",
                                   "bg-white text-slate-900 font-semibold hover:bg-slate-200",
                                   "bg-amber-950 text-amber-50 font-medium hover:bg-amber-900"))>"Done"</button>
                    </div>
                </div>

                <div class="p-6 sm:p-8 overflow-y-auto flex-1 space-y-4">
                    <input
                        node_ref=title_ref type="text" placeholder="Untitled Note"
                        on:input=move |_| live_edit()
                        class=move || format!("w-full text-2xl sm:text-3xl font-bold tracking-tight focus:outline-none bg-transparent {} {}",
                            tp("text-slate-900 placeholder:text-slate-300", "text-white placeholder:text-slate-600", "text-amber-950 placeholder:text-amber-900/35"),
                            font_class())
                    />
                    <textarea
                        node_ref=body_ref rows="10" placeholder="Start writing your thoughts..."
                        on:input=move |_| live_edit()
                        class=move || format!("w-full text-[15px] sm:text-base leading-relaxed focus:outline-none resize-none bg-transparent {} {}",
                            tp("text-slate-600 placeholder:text-slate-300", "text-slate-300 placeholder:text-slate-600", "text-amber-950/85 placeholder:text-amber-900/35"),
                            font_class())
                    ></textarea>
                </div>

                <div class=move || format!("px-6 py-3 border-t flex items-center justify-between text-xs {}",
                    tp("bg-slate-50/70 border-slate-100 text-slate-400", "bg-white/[0.03] border-white/[0.08] text-slate-400", "bg-amber-100/50 border-amber-950/[0.08] text-amber-900/70"))>
                    <input
                        node_ref=tag_ref type="text" placeholder="Add tag (e.g. Product, Personal)..."
                        on:input=move |_| live_edit()
                        class=move || format!("bg-transparent focus:outline-none font-medium w-48 {}",
                            tp("text-slate-600 placeholder:text-slate-400", "text-slate-300 placeholder:text-slate-500", "text-amber-950 placeholder:text-amber-900/45"))
                    />
                    <span>{move || { let w = update_word_count_source(); format!("{} {}", w, if w == 1 { "word" } else { "words" }) }}</span>
                </div>
            </div>
        </div>

        // ───────── Settings modal ─────────
        <div class=move || format!("fixed inset-0 z-50 {} items-center justify-center p-4 bg-slate-900/25 backdrop-blur-sm",
            if settings_open.get() { "flex" } else { "hidden" })>
            <div class=move || format!("w-full max-w-md rounded-[28px] shadow-2xl border overflow-hidden max-h-[90vh] flex flex-col {}",
                tp("bg-white border-slate-200/90", "bg-[#161922] border-white/10", "bg-[#FFFDF5] border-amber-200/90"))>
                <div class=move || format!("flex items-center justify-between px-6 py-5 border-b shrink-0 {}",
                    tp("border-slate-100", "border-white/[0.08]", "border-amber-950/[0.08]"))>
                    <div>
                        <h3 class=move || format!("text-lg font-bold {}", tp("text-slate-900", "text-white", "text-amber-950"))>"Notes Settings"</h3>
                        <p class=move || format!("text-xs mt-0.5 {}", tp("text-slate-500", "text-slate-400", "text-amber-900/70"))>"Customize your workspace appearance"</p>
                    </div>
                    <button on:click=move |_| close_settings()
                        class="p-2 rounded-xl text-slate-400 hover:text-slate-700 hover:bg-slate-500/10 transition"
                        inner_html=svg_html("w-4 h-4", "2.2", CLOSE)></button>
                </div>

                <div class="p-6 space-y-6 overflow-y-auto flex-1">
                    // Theme
                    <div>
                        <label class=setting_label>"Theme"</label>
                        <div class="grid grid-cols-3 gap-2.5">
                            <button on:click=move |_| update_setting("theme", "light")
                                class=move || sbtn(settings.with(|c| c.theme == "light"), "setting-theme-btn p-2.5 rounded-2xl flex flex-col items-center gap-2")>
                                <span class="w-full h-7 rounded-xl bg-white border border-slate-200 flex items-center overflow-hidden shadow-2xs">
                                    <span class="w-1/3 h-full bg-slate-100 border-r border-slate-200/80"></span>
                                    <span class="w-2/3 h-full bg-white"></span>
                                </span>
                                <span>"Light"</span>
                            </button>
                            <button on:click=move |_| update_setting("theme", "dark")
                                class=move || sbtn(settings.with(|c| c.theme == "dark"), "setting-theme-btn p-2.5 rounded-2xl flex flex-col items-center gap-2")>
                                <span class="w-full h-7 rounded-xl bg-[#0e1015] border border-slate-700 flex items-center overflow-hidden shadow-2xs">
                                    <span class="w-1/3 h-full bg-[#1b1f2a] border-r border-slate-700/80"></span>
                                    <span class="w-2/3 h-full bg-[#0e1015]"></span>
                                </span>
                                <span>"Dark"</span>
                            </button>
                            <button on:click=move |_| update_setting("theme", "sunshine")
                                class=move || sbtn(settings.with(|c| c.theme == "sunshine"), "setting-theme-btn p-2.5 rounded-2xl flex flex-col items-center gap-2")>
                                <span class="w-full h-7 rounded-xl overflow-hidden border border-amber-300/80 relative flex shadow-2xs">
                                    <span class="w-1/4 h-full bg-[#F8D930]"></span>
                                    <span class="w-2/4 h-full bg-[#FA9E2E]"></span>
                                    <span class="w-1/4 h-full bg-[#FCE467]"></span>
                                </span>
                                <span>"Sunshine"</span>
                            </button>
                        </div>
                    </div>

                    // Typography
                    <div>
                        <label class=setting_label>"Note Typography"</label>
                        <div class="grid grid-cols-3 gap-2">
                            <button on:click=move |_| update_setting("font", "sans")
                                class=move || sbtn(settings.with(|c| c.font == "sans"), "py-2.5 px-3 rounded-2xl")>"Inter Sans"</button>
                            <button on:click=move |_| update_setting("font", "serif")
                                class=move || sbtn(settings.with(|c| c.font == "serif"), "py-2.5 px-3 rounded-2xl font-serif")>"Editorial Serif"</button>
                            <button on:click=move |_| update_setting("font", "mono")
                                class=move || sbtn(settings.with(|c| c.font == "mono"), "py-2.5 px-3 rounded-2xl font-mono")>"Mono Code"</button>
                        </div>
                    </div>

                    // Density
                    <div>
                        <label class=setting_label>"Grid Card Size"</label>
                        <div class="grid grid-cols-2 gap-2">
                            <button on:click=move |_| update_setting("density", "comfortable")
                                class=move || sbtn(settings.with(|c| c.density == "comfortable"), "py-2.5 px-3 rounded-2xl")>"Spacious Squares"</button>
                            <button on:click=move |_| update_setting("density", "compact")
                                class=move || sbtn(settings.with(|c| c.density == "compact"), "py-2.5 px-3 rounded-2xl")>"Compact Grid"</button>
                        </div>
                    </div>

                    // Blur
                    <div>
                        <label class=setting_label>"Glass Frost & Tint"</label>
                        <div class="grid grid-cols-3 gap-2">
                            <button on:click=move |_| update_setting("blur", "subtle")
                                class=move || sbtn(settings.with(|c| c.blur == "subtle"), "py-2.5 px-3 rounded-2xl")>"Subtle"</button>
                            <button on:click=move |_| update_setting("blur", "frosted")
                                class=move || sbtn(settings.with(|c| c.blur == "frosted"), "py-2.5 px-3 rounded-2xl")>"Frosted"</button>
                            <button on:click=move |_| update_setting("blur", "heavy")
                                class=move || sbtn(settings.with(|c| c.blur == "heavy"), "py-2.5 px-3 rounded-2xl")>"Heavy Frost"</button>
                        </div>
                    </div>

                    // Title scale
                    <div>
                        <label class=setting_label>"Title Scale"</label>
                        <div class="grid grid-cols-3 gap-2">
                            <button on:click=move |_| update_setting("titleScale", "subtle")
                                class=move || sbtn(settings.with(|c| c.title_scale == "subtle"), "py-2.5 px-3 rounded-2xl")>"Balanced"</button>
                            <button on:click=move |_| update_setting("titleScale", "standard")
                                class=move || sbtn(settings.with(|c| c.title_scale == "standard"), "py-2.5 px-3 rounded-2xl")>"Dynamic"</button>
                            <button on:click=move |_| update_setting("titleScale", "expressive")
                                class=move || sbtn(settings.with(|c| c.title_scale == "expressive"), "py-2.5 px-3 rounded-2xl")>"Expressive"</button>
                        </div>
                    </div>

                    // Radius
                    <div>
                        <label class=setting_label>"Card Corner Radius"</label>
                        <div class="grid grid-cols-3 gap-2">
                            <button on:click=move |_| update_setting("radius", "18px")
                                class=move || sbtn(settings.with(|c| c.radius == "18px"), "py-2.5 px-3 rounded-2xl")>"Soft (18px)"</button>
                            <button on:click=move |_| update_setting("radius", "24px")
                                class=move || sbtn(settings.with(|c| c.radius == "24px"), "py-2.5 px-3 rounded-2xl")>"Mosaic (24px)"</button>
                            <button on:click=move |_| update_setting("radius", "32px")
                                class=move || sbtn(settings.with(|c| c.radius == "32px"), "py-2.5 px-3 rounded-2xl")>"Pill (32px)"</button>
                        </div>
                    </div>
                </div>

                <div class=move || format!("px-6 py-4 border-t flex items-center justify-between shrink-0 {}",
                    tp("bg-slate-50 border-slate-100", "bg-white/[0.03] border-white/[0.08]", "bg-amber-100/50 border-amber-950/[0.08]"))>
                    <span class=move || format!("text-[11px] font-medium tracking-wide {}",
                        tp("text-slate-400", "text-slate-500", "text-amber-900/55"))>"ver. 1.0.0"</span>
                    <button on:click=move |_| close_settings()
                        class=move || format!("px-4 py-2 rounded-xl text-xs transition {}",
                            tp("bg-slate-900 text-white font-medium hover:bg-slate-800",
                               "bg-white text-slate-900 font-semibold hover:bg-slate-200",
                               "bg-amber-950 text-amber-50 font-medium hover:bg-amber-900"))>"Save Preferences"</button>
                </div>
            </div>
        </div>

        // Toast
        <div class=move || format!("fixed bottom-5 right-5 z-50 pointer-events-none transition-all duration-300 bg-slate-900 text-white text-xs font-medium px-4 py-2.5 rounded-2xl shadow-lg {}",
            if toast.with(|t| t.1) { "" } else { "translate-y-16 opacity-0" })>
            {move || toast.with(|t| t.0.clone())}
        </div>
    }
}
