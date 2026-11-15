use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Timeout;
use leptos::*;
use serde::{Deserialize, Serialize};

const EVENTS_KEY: &str = "agenda.events";
const SETTINGS_KEY: &str = "agenda.settings";

// ───────────────────────── Date helpers (no external crates) ─────────────────────────

type Ymd = (i32, u32, u32);

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];
const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
// Sunday-first
const DAYS_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = (if m <= 2 { y - 1 } else { y }) as i64;
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> Ymd {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

fn fmt(y: i32, m: u32, d: u32) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

fn parse(s: &str) -> Ymd {
    let mut it = s.split('-');
    let y = it.next().and_then(|v| v.parse().ok());
    let m = it.next().and_then(|v| v.parse().ok());
    let d = it.next().and_then(|v| v.parse().ok());
    match (y, m, d) {
        (Some(y), Some(m), Some(d)) if (1..=12).contains(&m) && (1..=31).contains(&d) => (y, m, d),
        _ => (1970, 1, 1),
    }
}

fn to_days(s: &str) -> i64 {
    let (y, m, d) = parse(s);
    days_from_civil(y, m, d)
}

fn add_days(s: &str, n: i64) -> String {
    let (y, m, d) = civil_from_days(to_days(s) + n);
    fmt(y, m, d)
}

/// 0 = Sunday … 6 = Saturday
fn weekday(s: &str) -> u32 {
    (to_days(s) + 4).rem_euclid(7) as u32
}

fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    (days_from_civil(ny, nm, 1) - days_from_civil(y, m, 1)) as u32
}

fn add_months(s: &str, n: i32) -> String {
    let (y, m, d) = parse(s);
    let total = y * 12 + (m as i32 - 1) + n;
    let ny = total.div_euclid(12);
    let nm = (total.rem_euclid(12) + 1) as u32;
    fmt(ny, nm, d.min(days_in_month(ny, nm)))
}

fn today_str() -> String {
    let d = js_sys::Date::new_0();
    fmt(d.get_full_year() as i32, d.get_month() + 1, d.get_date())
}

fn lead_days(first_of_month: &str, monday_first: bool) -> u32 {
    let wd = weekday(first_of_month);
    if monday_first { (wd + 6) % 7 } else { wd }
}

fn week_start_of(s: &str, monday_first: bool) -> String {
    let wd = weekday(s);
    let off = if monday_first { (wd + 6) % 7 } else { wd };
    add_days(s, -(off as i64))
}

fn month_title(s: &str) -> String {
    let (y, m, _) = parse(s);
    format!("{} {}", MONTHS[(m - 1) as usize], y)
}

fn short_date(s: &str) -> String {
    let (_, m, d) = parse(s);
    format!("{} {}", DAYS_SHORT[weekday(s) as usize], format_args!("{d} {}", MONTHS_SHORT[(m - 1) as usize]))
}

fn long_date(s: &str) -> String {
    let (y, m, d) = parse(s);
    format!("{}, {} {} {}", DAYS_SHORT[weekday(s) as usize], d, MONTHS_SHORT[(m - 1) as usize], y)
}

fn rel_label(date: &str, today: &str) -> String {
    match to_days(date) - to_days(today) {
        0 => "Today".into(),
        1 => "Tomorrow".into(),
        _ => short_date(date),
    }
}

fn week_range_title(start: &str) -> String {
    let end = add_days(start, 6);
    let (y1, m1, d1) = parse(start);
    let (y2, m2, d2) = parse(&end);
    if y1 == y2 && m1 == m2 {
        format!("{d1} – {d2} {} {y1}", MONTHS_SHORT[(m1 - 1) as usize])
    } else if y1 == y2 {
        format!("{d1} {} – {d2} {} {y1}", MONTHS_SHORT[(m1 - 1) as usize], MONTHS_SHORT[(m2 - 1) as usize])
    } else {
        format!(
            "{d1} {} {y1} – {d2} {} {y2}",
            MONTHS_SHORT[(m1 - 1) as usize],
            MONTHS_SHORT[(m2 - 1) as usize]
        )
    }
}

// ───────────────────────── Data ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Appt {
    id: String,
    title: String,
    date: String,
    start: String,
    end: String,
    notes: String,
    color: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    theme: String,
    font: String,
    density: String,
    blur: String,
    radius: String,
    week_start: String,
    view: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "light".into(),
            font: "sans".into(),
            density: "comfortable".into(),
            blur: "frosted".into(),
            radius: "24px".into(),
            week_start: "mon".into(),
            view: "month".into(),
        }
    }
}

fn pick(t: &str, l: &'static str, d: &'static str) -> &'static str {
    if t == "dark" { d } else { l }
}

fn sort_appts(v: &mut [Appt]) {
    v.sort_by(|a, b| (a.date.as_str(), a.start.as_str()).cmp(&(b.date.as_str(), b.start.as_str())));
}

fn time_label(a: &Appt) -> String {
    if a.start.is_empty() {
        "All day".into()
    } else if a.end.is_empty() {
        a.start.clone()
    } else {
        format!("{} – {}", a.start, a.end)
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
const CAL: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5"/>"#;
const CHEV_L: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5L8.25 12l7.5-7.5"/>"#;
const CHEV_R: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5"/>"#;

const COLORS: [(&str, &str); 5] = [
    ("#10b981", "bg-emerald-500"),
    ("#6366f1", "bg-indigo-500"),
    ("#0ea5e9", "bg-sky-500"),
    ("#f59e0b", "bg-amber-500"),
    ("#f43f5e", "bg-rose-500"),
];

// ───────────────────────── App ─────────────────────────

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let events: RwSignal<Vec<Appt>> =
        create_rw_signal(LocalStorage::get::<Vec<Appt>>(EVENTS_KEY).unwrap_or_default());
    let settings: RwSignal<Settings> =
        create_rw_signal(LocalStorage::get::<Settings>(SETTINGS_KEY).unwrap_or_default());
    let cursor: RwSignal<String> = create_rw_signal(today_str());
    let search: RwSignal<String> = create_rw_signal(String::new());
    let editor_open = create_rw_signal(false);
    let settings_open = create_rw_signal(false);
    let editing: RwSignal<Option<String>> = create_rw_signal(None);
    let toast: RwSignal<(String, bool)> = create_rw_signal(("Saved".into(), false));

    // editor draft
    let d_title = create_rw_signal(String::new());
    let d_date = create_rw_signal(String::new());
    let d_start = create_rw_signal(String::new());
    let d_end = create_rw_signal(String::new());
    let d_notes = create_rw_signal(String::new());
    let d_color = create_rw_signal(COLORS[0].0.to_string());

    let title_ref = create_node_ref::<html::Input>();
    let search_ref = create_node_ref::<html::Input>();

    let tp = move |l: &'static str, d: &'static str| -> &'static str {
        settings.with(|c| pick(&c.theme, l, d))
    };

    // persistence
    create_effect(move |_| {
        let _ = LocalStorage::set(EVENTS_KEY, events.get());
    });
    create_effect(move |_| {
        let _ = LocalStorage::set(SETTINGS_KEY, settings.get());
    });

    // body theme class
    create_effect(move |_| {
        let t = settings.with(|c| c.theme.clone());
        if let Some(body) = document().body() {
            body.set_class_name(&format!(
                "theme-{t} antialiased selection:bg-emerald-500/20 min-h-screen overflow-x-hidden transition-colors duration-300"
            ));
        }
    });

    let show_toast = move |msg: &str| {
        toast.set((msg.to_string(), true));
        Timeout::new(2200, move || toast.update(|t| t.1 = false)).forget();
    };

    let focus_title = move || {
        Timeout::new(50, move || {
            if let Some(el) = title_ref.get() {
                let _ = el.focus();
            }
        })
        .forget();
    };

    let open_new = move |date: String| {
        editing.set(None);
        d_title.set(String::new());
        d_date.set(date);
        d_start.set("09:00".into());
        d_end.set("10:00".into());
        d_notes.set(String::new());
        d_color.set(COLORS[0].0.to_string());
        editor_open.set(true);
        focus_title();
    };

    let open_edit = move |id: String| {
        let found = events.with(|es| es.iter().find(|a| a.id == id).cloned());
        let Some(a) = found else { return };
        editing.set(Some(a.id));
        d_title.set(a.title);
        d_date.set(a.date);
        d_start.set(a.start);
        d_end.set(a.end);
        d_notes.set(a.notes);
        d_color.set(a.color);
        editor_open.set(true);
    };

    let close_editor = move || editor_open.set(false);

    let save_event = move || {
        let title = d_title.get_untracked();
        let title = if title.trim().is_empty() { "Untitled event".to_string() } else { title.trim().to_string() };
        let mut date = d_date.get_untracked();
        if date.is_empty() {
            date = today_str();
        }
        let id = editing
            .get_untracked()
            .unwrap_or_else(|| format!("evt-{}", js_sys::Date::now() as u64));
        let appt = Appt {
            id: id.clone(),
            title,
            date: date.clone(),
            start: d_start.get_untracked(),
            end: d_end.get_untracked(),
            notes: d_notes.get_untracked(),
            color: d_color.get_untracked(),
        };
        events.update(|es| {
            if let Some(slot) = es.iter_mut().find(|a| a.id == id) {
                *slot = appt;
            } else {
                es.push(appt);
            }
        });
        cursor.set(date);
        editor_open.set(false);
        show_toast("Event saved");
    };

    let delete_event = move || {
        if let Some(id) = editing.get_untracked() {
            events.update(|es| es.retain(|a| a.id != id));
            editor_open.set(false);
            show_toast("Event deleted");
        }
    };

    let close_settings = move || {
        if settings_open.get_untracked() {
            settings_open.set(false);
            show_toast("Preferences updated");
        }
    };

    let update_setting = move |key: &'static str, value: &'static str| {
        settings.update(|s| match key {
            "theme" => s.theme = value.into(),
            "font" => s.font = value.into(),
            "density" => s.density = value.into(),
            "blur" => s.blur = value.into(),
            "radius" => s.radius = value.into(),
            "week_start" => s.week_start = value.into(),
            "view" => s.view = value.into(),
            _ => {}
        });
    };

    // navigation
    let go_prev = move || {
        let week = settings.with_untracked(|c| c.view == "week");
        cursor.update(|c| *c = if week { add_days(c, -7) } else { add_months(c, -1) });
    };
    let go_next = move || {
        let week = settings.with_untracked(|c| c.view == "week");
        cursor.update(|c| *c = if week { add_days(c, 7) } else { add_months(c, 1) });
    };
    let go_today = move || cursor.set(today_str());

    // keyboard shortcuts
    window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            close_editor();
            close_settings();
        }
        if e.key() == "/" {
            let tag = document().active_element().map(|a| a.tag_name()).unwrap_or_default();
            if tag != "INPUT" && tag != "TEXTAREA" {
                e.prevent_default();
                if let Some(el) = search_ref.get() {
                    let _ = el.focus();
                }
            }
        }
    });

    let font_class = move || format!("app-font-{}", settings.with(|c| c.font.clone()));
    let radius = move || settings.with(|c| c.radius.clone());

    // settings-modal button styling
    let sbtn = move |is_active: bool, extra: &'static str| -> String {
        let a = tp(
            "border-slate-900 bg-slate-900 text-white font-medium",
            "border-white bg-white text-slate-900 font-semibold",
        );
        let i = tp(
            "border-slate-200 bg-white text-slate-700 hover:bg-slate-50",
            "border-white/10 bg-white/[0.04] text-slate-300 hover:bg-white/[0.09]",
        );
        format!("{} border text-xs transition {}", extra, if is_active { a } else { i })
    };
    let setting_label = "block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-2.5";

    // ───── Month view ─────
    let month_view = move || -> View {
        let cur = cursor.get();
        let s = settings.get();
        let dark = s.theme == "dark";
        let mon = s.week_start == "mon";
        let (y, m, _) = parse(&cur);
        let first = fmt(y, m, 1);
        let lead = lead_days(&first, mon);
        let dim = days_in_month(y, m);
        let rows = (lead + dim + 6) / 7;
        let start = add_days(&first, -(lead as i64));
        let all = events.get();
        let today = today_str();
        let compact = s.density == "compact";
        let max_chips: usize = if compact { 2 } else { 3 };
        let min_h = if compact { "min-h-[96px]" } else { "min-h-[124px]" };
        let radius = s.radius.clone();

        let head = (0..7usize)
            .map(|i| {
                let idx = (if mon { 1 } else { 0 } + i) % 7;
                view! {
                    <div class=format!("px-2 pb-2 text-[11px] font-semibold uppercase tracking-wider {}",
                        if dark { "text-slate-500" } else { "text-slate-400" })>{DAYS_SHORT[idx]}</div>
                }
            })
            .collect_view();

        let cells = (0..rows * 7)
            .map(|i| {
                let date = add_days(&start, i as i64);
                let (_, cm, cd) = parse(&date);
                let in_month = cm == m;
                let is_today = date == today;
                let is_sel = date == cur;
                let mut day: Vec<Appt> = all.iter().filter(|a| a.date == date).cloned().collect();
                sort_appts(&mut day);
                let extra = day.len().saturating_sub(max_chips);

                let chips = day
                    .into_iter()
                    .take(max_chips)
                    .map(|a| {
                        let id = a.id.clone();
                        let label = if a.start.is_empty() { a.title.clone() } else { format!("{} {}", a.start, a.title) };
                        view! {
                            <button
                                on:click=move |e| { e.stop_propagation(); open_edit(id.clone()); }
                                style=format!("border-left-color: {}", a.color)
                                class=format!("w-full text-left truncate px-1.5 py-0.5 rounded-md text-[11px] leading-tight border-l-[3px] transition {}",
                                    if dark { "bg-white/[0.07] text-slate-200 hover:bg-white/[0.13]" } else { "bg-slate-100 text-slate-700 hover:bg-slate-200/80" })
                            >{label}</button>
                        }
                    })
                    .collect_view();

                let d_more = date.clone();
                let more = if extra > 0 {
                    view! {
                        <button
                            on:click=move |e| {
                                e.stop_propagation();
                                cursor.set(d_more.clone());
                                settings.update(|s| s.view = "week".into());
                            }
                            class=format!("text-[11px] font-medium px-1.5 {}", if dark { "text-slate-400 hover:text-white" } else { "text-slate-500 hover:text-slate-900" })
                        >{format!("+{extra} more")}</button>
                    }
                    .into_view()
                } else {
                    ().into_view()
                };

                let bg = match (in_month, dark) {
                    (true, false) => "bg-white hover:bg-slate-50",
                    (false, false) => "bg-slate-50/80 hover:bg-slate-50",
                    (true, true) => "bg-[#161922] hover:bg-[#1b1f2a]",
                    (false, true) => "bg-[#12141b] hover:bg-[#161922]",
                };
                let num = if is_today {
                    "bg-emerald-500 text-white font-semibold"
                } else {
                    match (in_month, dark) {
                        (true, false) => "text-slate-700 font-medium",
                        (false, false) => "text-slate-300",
                        (true, true) => "text-slate-200 font-medium",
                        (false, true) => "text-slate-600",
                    }
                };
                let d_new = date.clone();
                view! {
                    <div
                        on:click=move |_| { cursor.set(d_new.clone()); open_new(d_new.clone()); }
                        class=format!("relative p-2 cursor-pointer transition-colors flex flex-col gap-1 {min_h} {bg} {}",
                            if is_sel { "ring-2 ring-inset ring-emerald-500/50" } else { "" })
                    >
                        <span class=format!("w-6 h-6 rounded-full text-xs flex items-center justify-center {num}")>{cd}</span>
                        <div class="flex flex-col gap-1 min-w-0">{chips}{more}</div>
                    </div>
                }
            })
            .collect_view();

        view! {
            <div>
                <div class="grid grid-cols-7">{head}</div>
                <div
                    style=format!("border-radius: {radius};")
                    class=format!("grid grid-cols-7 gap-px overflow-hidden border {}",
                        if dark { "bg-white/[0.07] border-white/[0.07]" } else { "bg-slate-200/70 border-slate-200/70 shadow-card" })
                >{cells}</div>
            </div>
        }
        .into_view()
    };

    // ───── Week view ─────
    let week_view = move || -> View {
        let cur = cursor.get();
        let s = settings.get();
        let dark = s.theme == "dark";
        let mon = s.week_start == "mon";
        let start = week_start_of(&cur, mon);
        let all = events.get();
        let today = today_str();
        let compact = s.density == "compact";
        let radius = s.radius.clone();

        let cards = (0..7i64)
            .map(|i| {
                let date = add_days(&start, i);
                let (_, _, dn) = parse(&date);
                let is_today = date == today;
                let is_sel = date == cur;
                let mut day: Vec<Appt> = all.iter().filter(|a| a.date == date).cloned().collect();
                sort_appts(&mut day);
                let empty = day.is_empty();

                let items = day
                    .into_iter()
                    .map(|a| {
                        let id = a.id.clone();
                        view! {
                            <button
                                on:click=move |e| { e.stop_propagation(); open_edit(id.clone()); }
                                style=format!("border-left-color: {}", a.color)
                                class=format!("w-full text-left px-2.5 py-2 rounded-xl border-l-[3px] transition {}",
                                    if dark { "bg-white/[0.06] hover:bg-white/[0.12]" } else { "bg-slate-50 hover:bg-slate-100" })
                            >
                                <div class=format!("text-[11px] font-medium {}", if dark { "text-slate-400" } else { "text-slate-500" })>{time_label(&a)}</div>
                                <div class=format!("text-[13px] font-semibold leading-snug break-words {}", if dark { "text-white" } else { "text-slate-900" })>{a.title.clone()}</div>
                            </button>
                        }
                    })
                    .collect_view();

                let surface = if dark {
                    "bg-[#161922] border border-white/[0.08] hover:border-white/[0.16]"
                } else {
                    "bg-white shadow-card hover:shadow-card-hover"
                };
                let d_new = date.clone();
                view! {
                    <div
                        on:click=move |_| { cursor.set(d_new.clone()); open_new(d_new.clone()); }
                        style=format!("border-radius: {radius};")
                        class=format!("p-4 cursor-pointer transition-all duration-200 flex flex-col gap-3 {} {surface} {}",
                            if compact { "min-h-[180px]" } else { "min-h-[260px]" },
                            if is_sel { "ring-2 ring-emerald-500/40" } else { "" })
                    >
                        <div class="flex items-center justify-between">
                            <span class=format!("text-[11px] font-semibold uppercase tracking-wider {}", if dark { "text-slate-400" } else { "text-slate-400" })>
                                {DAYS_SHORT[weekday(&date) as usize]}
                            </span>
                            <span class=format!("w-7 h-7 rounded-full text-sm flex items-center justify-center {}",
                                if is_today { "bg-emerald-500 text-white font-semibold" }
                                else if dark { "text-slate-200 font-medium" } else { "text-slate-700 font-medium" })>{dn}</span>
                        </div>
                        <div class="flex flex-col gap-2">{items}</div>
                        {if empty {
                            view! { <span class=format!("text-xs mt-auto {}", if dark { "text-slate-600" } else { "text-slate-300" })>"No events"</span> }.into_view()
                        } else { ().into_view() }}
                    </div>
                }
            })
            .collect_view();

        view! {
            <div class=if compact {
                "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 xl:grid-cols-7 gap-3"
            } else {
                "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 xl:grid-cols-7 gap-4"
            }>{cards}</div>
        }
        .into_view()
    };

    // ───── Search results ─────
    let search_view = move || -> View {
        let q = search.get().to_lowercase();
        let q = q.trim().to_string();
        let dark = settings.with(|c| c.theme == "dark");
        let radius = radius();
        let mut hits: Vec<Appt> = events.with(|es| {
            es.iter()
                .filter(|a| {
                    a.title.to_lowercase().contains(&q)
                        || a.notes.to_lowercase().contains(&q)
                        || a.date.contains(&q)
                })
                .cloned()
                .collect()
        });
        sort_appts(&mut hits);
        if hits.is_empty() {
            return view! {
                <div class="flex flex-col items-center justify-center py-20 text-center">
                    <div class=format!("w-14 h-14 rounded-3xl flex items-center justify-center mb-4 {}",
                        if dark { "bg-white/[0.06] text-slate-400" } else { "bg-slate-100 text-slate-400" })
                        inner_html=svg_html("w-6 h-6", "1.8", CAL)></div>
                    <h3 class=format!("text-base font-semibold {}", if dark { "text-slate-200" } else { "text-slate-800" })>"No matching events"</h3>
                    <p class="text-sm text-slate-500 mt-1 max-w-xs">"Try another search term or create a new event."</p>
                    <button on:click=move |_| search.set(String::new())
                        class=format!("mt-4 px-4 py-2 rounded-xl text-xs font-medium transition {}",
                            if dark { "bg-white text-slate-900 hover:bg-slate-200" } else { "bg-slate-900 text-white hover:bg-slate-800" })>"Reset Search"</button>
                </div>
            }
            .into_view();
        }
        let rows = hits
            .into_iter()
            .map(|a| {
                let id = a.id.clone();
                let (_, _, d) = parse(&a.date);
                let (_, m, _) = parse(&a.date);
                view! {
                    <button
                        on:click=move |_| open_edit(id.clone())
                        style=format!("border-radius: {radius};")
                        class=format!("w-full text-left p-4 flex items-center gap-4 transition-all hover:-translate-y-0.5 {}",
                            if dark { "bg-[#161922] border border-white/[0.08] hover:border-white/[0.16]" } else { "bg-white shadow-card hover:shadow-card-hover" })
                    >
                        <div class=format!("w-12 shrink-0 text-center {}", if dark { "text-white" } else { "text-slate-900" })>
                            <div class="text-lg font-bold leading-none">{d}</div>
                            <div class="text-[10px] uppercase tracking-wider text-slate-400 mt-1">{MONTHS_SHORT[(m - 1) as usize]}</div>
                        </div>
                        <span class="w-1 self-stretch rounded-full" style=format!("background-color: {}", a.color)></span>
                        <div class="min-w-0 flex-1">
                            <div class=format!("text-sm font-semibold truncate {}", if dark { "text-white" } else { "text-slate-900" })>{a.title.clone()}</div>
                            <div class="text-xs text-slate-400 mt-0.5 truncate">{format!("{} · {}", time_label(&a), if a.notes.is_empty() { "No notes".to_string() } else { a.notes.clone() })}</div>
                        </div>
                    </button>
                }
            })
            .collect_view();
        view! { <div class="flex flex-col gap-3 max-w-3xl">{rows}</div> }.into_view()
    };

    // counts for subheader
    let range_count = move || -> usize {
        let cur = cursor.get();
        let s = settings.get();
        events.with(|es| {
            if s.view == "week" {
                let start = week_start_of(&cur, s.week_start == "mon");
                let end = add_days(&start, 6);
                es.iter().filter(|a| a.date >= start && a.date <= end).count()
            } else {
                let (y, m, _) = parse(&cur);
                let prefix = format!("{y:04}-{m:02}");
                es.iter().filter(|a| a.date.starts_with(&prefix)).count()
            }
        })
    };

    let upcoming = move || -> Vec<Appt> {
        let today = today_str();
        let mut v: Vec<Appt> = events.with(|es| es.iter().filter(|a| a.date >= today).cloned().collect());
        sort_appts(&mut v);
        v
    };

    view! {
        <div id="appShell" class="relative z-10 min-h-screen flex flex-row transition-colors duration-300">
            // ───────── Sidebar ─────────
            <aside id="sidebarContainer" class="sticky top-0 left-0 z-30 h-screen w-[250px] sm:w-[280px] lg:w-[300px] shrink-0">
                <div id="sidebarBackdropLayer" class="absolute inset-0 overflow-hidden rounded-r-[28px] pointer-events-none select-none transition-colors duration-300"></div>
                <div
                    id="sidebarGlassSurface"
                    class=move || format!(
                        "sidebar-glass-{} relative h-full w-full rounded-r-[28px] border-y border-r border-l-0 flex flex-col p-5 sm:p-6 overflow-hidden transition-all duration-300",
                        settings.with(|c| c.blur.clone())
                    )
                >
                    <div class="shrink-0">
                        <div class="px-1 pt-1 flex items-center gap-3">
                            <img src="logo.png" alt="Agenda" class="w-9 h-9 object-contain" />
                            <h1 class=move || format!("text-[26px] sm:text-[28px] leading-tight font-bold tracking-tight {}",
                                tp("text-slate-900", "text-white"))>"Agenda"</h1>
                        </div>

                        <button
                            on:click=move |_| settings_open.set(true)
                            class=move || format!(
                                "mt-3 w-full flex items-center gap-2.5 px-3 py-2.5 rounded-2xl text-sm font-medium active:scale-[0.99] transition-all group {}",
                                tp("text-slate-600 hover:text-slate-900 hover:bg-slate-900/[0.05]",
                                   "text-slate-300 hover:text-white hover:bg-white/[0.07]")
                            )
                        >
                            <span class="contents" inner_html=move || svg_html(
                                &format!("w-[18px] h-[18px] group-hover:rotate-45 transition-transform duration-300 {}",
                                    tp("text-slate-400 group-hover:text-slate-700", "text-slate-400 group-hover:text-white")),
                                "1.8", GEAR)></span>
                            <span>"Settings"</span>
                        </button>

                        <div class=move || format!("my-4 h-px {}", tp("bg-slate-900/[0.06]", "bg-white/[0.08]"))></div>

                        // Mini calendar header
                        <div class="flex items-center justify-between px-1 mb-2">
                            <span class=move || format!("text-sm font-semibold {}", tp("text-slate-800", "text-slate-100"))>
                                {move || month_title(&cursor.get())}
                            </span>
                            <div class="flex items-center gap-0.5">
                                <button aria-label="Previous month" on:click=move |_| cursor.update(|c| *c = add_months(c, -1))
                                    class=move || format!("w-6 h-6 rounded-lg flex items-center justify-center transition {}",
                                        tp("text-slate-400 hover:text-slate-800 hover:bg-slate-900/[0.06]", "text-slate-400 hover:text-white hover:bg-white/[0.08]"))
                                    inner_html=svg_html("w-3.5 h-3.5", "2.2", CHEV_L)></button>
                                <button aria-label="Next month" on:click=move |_| cursor.update(|c| *c = add_months(c, 1))
                                    class=move || format!("w-6 h-6 rounded-lg flex items-center justify-center transition {}",
                                        tp("text-slate-400 hover:text-slate-800 hover:bg-slate-900/[0.06]", "text-slate-400 hover:text-white hover:bg-white/[0.08]"))
                                    inner_html=svg_html("w-3.5 h-3.5", "2.2", CHEV_R)></button>
                            </div>
                        </div>

                        {move || {
                            let cur = cursor.get();
                            let s = settings.get();
                            let dark = s.theme == "dark";
                            let mon = s.week_start == "mon";
                            let (y, m, _) = parse(&cur);
                            let first = fmt(y, m, 1);
                            let lead = lead_days(&first, mon);
                            let dim = days_in_month(y, m);
                            let rows = (lead + dim + 6) / 7;
                            let start = add_days(&first, -(lead as i64));
                            let all = events.get();
                            let today = today_str();

                            let head = (0..7usize).map(|i| {
                                let idx = (if mon { 1 } else { 0 } + i) % 7;
                                view! { <div class=format!("text-center text-[10px] font-semibold uppercase {}",
                                    if dark { "text-slate-500" } else { "text-slate-400" })>{&DAYS_SHORT[idx][..2]}</div> }
                            }).collect_view();

                            let cells = (0..rows * 7).map(|i| {
                                let date = add_days(&start, i as i64);
                                let (_, cm, cd) = parse(&date);
                                let in_month = cm == m;
                                let is_today = date == today;
                                let is_sel = date == cur;
                                let has = all.iter().any(|a| a.date == date);
                                let cls = if is_today {
                                    "bg-emerald-500 text-white font-semibold".to_string()
                                } else if is_sel {
                                    (if dark { "bg-white/[0.14] text-white font-semibold" } else { "bg-slate-900/[0.09] text-slate-900 font-semibold" }).to_string()
                                } else if in_month {
                                    (if dark { "text-slate-200 hover:bg-white/[0.08]" } else { "text-slate-700 hover:bg-slate-900/[0.05]" }).to_string()
                                } else {
                                    (if dark { "text-slate-600 hover:bg-white/[0.05]" } else { "text-slate-300 hover:bg-slate-900/[0.04]" }).to_string()
                                };
                                view! {
                                    <button
                                        on:click=move |_| cursor.set(date.clone())
                                        class=format!("relative w-8 h-8 mx-auto rounded-full text-[12px] flex items-center justify-center transition {cls}")
                                    >
                                        {cd}
                                        {if has && !is_today {
                                            view! { <span class="absolute bottom-[3px] w-1 h-1 rounded-full bg-emerald-500"></span> }.into_view()
                                        } else { ().into_view() }}
                                    </button>
                                }
                            }).collect_view();

                            view! {
                                <div class="grid grid-cols-7 gap-y-0.5">{head}{cells}</div>
                            }
                        }}

                        <div class=move || format!("my-4 h-px {}", tp("bg-slate-900/[0.06]", "bg-white/[0.08]"))></div>

                        <div class="flex items-center justify-between px-2 mb-2">
                            <div class="flex items-center gap-2">
                                <span class=move || format!("text-[11px] font-semibold uppercase tracking-wider {}", tp("text-slate-400", "text-slate-400"))>"Upcoming"</span>
                                <span class=move || format!("text-[11px] font-medium {}", tp("text-slate-400", "text-slate-500"))>
                                    {move || upcoming().len()}
                                </span>
                            </div>
                            <button title="Add event" aria-label="Add event"
                                on:click=move |_| open_new(cursor.get_untracked())
                                class=move || format!("w-6 h-6 rounded-lg flex items-center justify-center transition {}",
                                    tp("text-slate-400 hover:text-slate-800 hover:bg-slate-900/[0.06]", "text-slate-400 hover:text-white hover:bg-white/[0.08]"))
                                inner_html=svg_html("w-3.5 h-3.5", "2.2", PLUS)></button>
                        </div>
                    </div>

                    <nav class="flex-1 min-h-0 overflow-y-auto -mx-2 px-2 space-y-1 pr-1" aria-label="Upcoming events">
                        {move || {
                            let list = upcoming();
                            let dark = settings.with(|c| c.theme == "dark");
                            if list.is_empty() {
                                return view! {
                                    <div class="px-3 py-6 text-center">
                                        <p class=if dark { "text-xs text-slate-500" } else { "text-xs text-slate-400" }>"Nothing coming up"</p>
                                    </div>
                                }.into_view();
                            }
                            let today = today_str();
                            list.into_iter().take(12).map(|a| {
                                let id = a.id.clone();
                                let d_go = a.date.clone();
                                let item = if dark {
                                    "text-slate-300 hover:bg-white/[0.07] hover:text-white"
                                } else {
                                    "text-slate-600 hover:bg-slate-900/[0.05] hover:text-slate-900"
                                };
                                view! {
                                    <button
                                        on:click=move |_| { cursor.set(d_go.clone()); open_edit(id.clone()); }
                                        class=format!("w-full text-left px-3 py-2 rounded-2xl flex items-center gap-2.5 transition-all group {item}")
                                    >
                                        <span class="w-2 h-2 rounded-full shrink-0 transition-transform group-hover:scale-125"
                                              style=format!("background-color: {}", a.color)></span>
                                        <div class="min-w-0 flex-1">
                                            <div class="text-[13px] font-medium truncate">{a.title.clone()}</div>
                                            <div class="text-[11px] text-slate-400 truncate">{format!("{} · {}", rel_label(&a.date, &today), time_label(&a))}</div>
                                        </div>
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
                                &format!("w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 pointer-events-none {}", tp("text-slate-400", "text-slate-400")),
                                "2", SEARCH)></span>
                            <input
                                node_ref=search_ref type="text" placeholder="Search events..."
                                on:input=move |e| search.set(event_target_value(&e))
                                prop:value=move || search.get()
                                class=move || format!("w-full pl-10 pr-9 py-2.5 rounded-2xl text-sm transition-all focus:outline-none {}",
                                    tp("bg-slate-50/80 hover:bg-slate-100/70 focus:bg-white border border-slate-200/80 focus:border-slate-300 focus:ring-4 focus:ring-slate-900/[0.04] text-slate-800 placeholder:text-slate-400",
                                       "bg-white/[0.06] hover:bg-white/[0.09] focus:bg-[#161922] border border-white/[0.1] focus:border-white/25 focus:ring-4 focus:ring-white/[0.05] text-white placeholder:text-slate-400"))
                            />
                            <button
                                on:click=move |_| search.set(String::new())
                                class=move || if search.with(|s| s.trim().is_empty()) {
                                    "hidden absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 p-0.5 rounded-full"
                                } else {
                                    "absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 p-0.5 rounded-full"
                                }
                                inner_html=svg_html("w-3.5 h-3.5", "2.2", CLOSE)></button>
                        </div>
                    </div>

                    <div class="flex items-center gap-2">
                        // View switch
                        <div class=move || format!("hidden sm:flex p-1 rounded-2xl border {}",
                            tp("bg-slate-50/80 border-slate-200/80", "bg-white/[0.05] border-white/[0.1]"))>
                            <button on:click=move |_| update_setting("view", "month")
                                class=move || format!("px-3 py-1.5 rounded-xl text-xs transition {}",
                                    if settings.with(|c| c.view == "month") {
                                        tp("bg-white text-slate-900 font-semibold shadow-sm", "bg-white/[0.14] text-white font-semibold")
                                    } else {
                                        tp("text-slate-500 hover:text-slate-900 font-medium", "text-slate-400 hover:text-white font-medium")
                                    })>"Month"</button>
                            <button on:click=move |_| update_setting("view", "week")
                                class=move || format!("px-3 py-1.5 rounded-xl text-xs transition {}",
                                    if settings.with(|c| c.view == "week") {
                                        tp("bg-white text-slate-900 font-semibold shadow-sm", "bg-white/[0.14] text-white font-semibold")
                                    } else {
                                        tp("text-slate-500 hover:text-slate-900 font-medium", "text-slate-400 hover:text-white font-medium")
                                    })>"Week"</button>
                        </div>
                        <button on:click=move |_| go_today()
                            class=move || format!("h-10 px-4 rounded-2xl text-xs font-medium transition-all active:scale-[0.97] border {}",
                                tp("bg-white hover:bg-slate-50 text-slate-700 border-slate-200/90",
                                   "bg-white/[0.06] hover:bg-white/[0.12] text-slate-200 border-white/[0.1]"))>"Today"</button>
                        <button title="Add event" aria-label="Add event"
                            on:click=move |_| open_new(cursor.get_untracked())
                            class=move || format!("w-10 h-10 rounded-2xl flex items-center justify-center transition-all active:scale-[0.97] {}",
                                tp("bg-slate-900 hover:bg-slate-800 text-white", "bg-white hover:bg-slate-200 text-slate-900"))
                            inner_html=svg_html("w-4 h-4", "2.2", PLUS)></button>
                    </div>
                </header>

                <div class="flex items-center justify-between mb-5 px-0.5">
                    <div class="flex items-center gap-3">
                        <div class="flex items-center gap-1">
                            <button aria-label="Previous" on:click=move |_| go_prev()
                                class=move || format!("w-8 h-8 rounded-xl flex items-center justify-center transition {}",
                                    tp("text-slate-500 hover:text-slate-900 hover:bg-slate-900/[0.06]", "text-slate-400 hover:text-white hover:bg-white/[0.08]"))
                                inner_html=svg_html("w-4 h-4", "2.2", CHEV_L)></button>
                            <button aria-label="Next" on:click=move |_| go_next()
                                class=move || format!("w-8 h-8 rounded-xl flex items-center justify-center transition {}",
                                    tp("text-slate-500 hover:text-slate-900 hover:bg-slate-900/[0.06]", "text-slate-400 hover:text-white hover:bg-white/[0.08]"))
                                inner_html=svg_html("w-4 h-4", "2.2", CHEV_R)></button>
                        </div>
                        <h2 class=move || format!("text-xl font-semibold tracking-tight {} {}", tp("text-slate-900", "text-white"), font_class())>
                            {move || {
                                let q = search.get();
                                if !q.trim().is_empty() {
                                    format!("Results for \"{}\"", q)
                                } else if settings.with(|c| c.view == "week") {
                                    let mon = settings.with(|c| c.week_start == "mon");
                                    week_range_title(&week_start_of(&cursor.get(), mon))
                                } else {
                                    month_title(&cursor.get())
                                }
                            }}
                        </h2>
                        <span class=move || format!("px-2 py-0.5 text-xs rounded-full font-medium {}",
                            tp("bg-slate-100 text-slate-600", "bg-white/[0.08] text-slate-300"))>
                            {move || format!("{} {}", range_count(), if range_count() == 1 { "event" } else { "events" })}
                        </span>
                    </div>
                    <div class=move || format!("text-xs hidden md:block {}", tp("text-slate-400", "text-slate-500"))>
                        "Click a day to add an event"
                    </div>
                </div>

                {move || {
                    if !search.with(|q| q.trim().is_empty()) {
                        search_view()
                    } else if settings.with(|c| c.view == "week") {
                        week_view()
                    } else {
                        month_view()
                    }
                }}
            </main>
        </div>

        // ───────── Event editor modal ─────────
        <div class=move || format!("fixed inset-0 z-50 {} items-center justify-center p-4 sm:p-6 bg-slate-900/25 backdrop-blur-sm transition-opacity",
            if editor_open.get() { "flex" } else { "hidden" })
            on:click=move |_| close_editor()>
            <div on:click=|e| e.stop_propagation()
                class=move || format!("w-full max-w-xl rounded-[28px] shadow-2xl border overflow-hidden flex flex-col max-h-[88vh] {}",
                tp("bg-white border-slate-200/90", "bg-[#161922] border-white/10"))>
                <div class=move || format!("flex items-center justify-between px-6 py-4 border-b {}",
                    tp("border-slate-100", "border-white/[0.08]"))>
                    <div class="flex items-center gap-2.5">
                        <div class="flex items-center gap-1.5" title="Event colour">
                            {COLORS.into_iter().map(|(hex, bg)| view! {
                                <button type="button"
                                    on:click=move |_| d_color.set(hex.to_string())
                                    class=move || format!("w-4 h-4 rounded-full {bg} ring-offset-2 transition {}",
                                        if d_color.with(|c| c == hex) { "ring-2 ring-slate-400 scale-110" } else { "" })></button>
                            }).collect_view()}
                        </div>
                        <span class="text-xs text-slate-300">"|"</span>
                        <span class="text-xs text-slate-400">{move || long_date(&d_date.get())}</span>
                    </div>
                    <div class="flex items-center gap-1.5">
                        <button
                            on:click=move |_| delete_event() title="Delete event"
                            class=move || format!("p-2 rounded-xl text-slate-400 hover:text-rose-600 hover:bg-rose-500/10 transition {}",
                                if editing.with(|e| e.is_some()) { "" } else { "hidden" })
                            inner_html=svg_html("w-4 h-4", "2", TRASH)></button>
                        <button on:click=move |_| save_event()
                            class=move || format!("px-3.5 py-1.5 rounded-xl text-xs transition {}",
                                tp("bg-slate-900 text-white font-medium hover:bg-slate-800", "bg-white text-slate-900 font-semibold hover:bg-slate-200"))>"Save"</button>
                    </div>
                </div>

                <div class="p-6 sm:p-8 overflow-y-auto flex-1 space-y-5">
                    <input
                        node_ref=title_ref type="text" placeholder="Event title"
                        prop:value=move || d_title.get()
                        on:input=move |e| d_title.set(event_target_value(&e))
                        on:keydown=move |e| if e.key() == "Enter" { save_event() }
                        class=move || format!("w-full text-2xl sm:text-3xl font-bold tracking-tight focus:outline-none bg-transparent {} {}",
                            tp("text-slate-900 placeholder:text-slate-300", "text-white placeholder:text-slate-600"), font_class())
                    />
                    <div class="grid grid-cols-3 gap-3">
                        <label class="block">
                            <span class=setting_label>"Date"</span>
                            <input type="date"
                                prop:value=move || d_date.get()
                                on:input=move |e| d_date.set(event_target_value(&e))
                                class=move || format!("w-full px-3 py-2 rounded-xl text-sm border focus:outline-none {}",
                                    tp("bg-slate-50 border-slate-200 text-slate-800 focus:border-slate-300", "bg-white/[0.05] border-white/10 text-white focus:border-white/25")) />
                        </label>
                        <label class="block">
                            <span class=setting_label>"Start"</span>
                            <input type="time"
                                prop:value=move || d_start.get()
                                on:input=move |e| d_start.set(event_target_value(&e))
                                class=move || format!("w-full px-3 py-2 rounded-xl text-sm border focus:outline-none {}",
                                    tp("bg-slate-50 border-slate-200 text-slate-800 focus:border-slate-300", "bg-white/[0.05] border-white/10 text-white focus:border-white/25")) />
                        </label>
                        <label class="block">
                            <span class=setting_label>"End"</span>
                            <input type="time"
                                prop:value=move || d_end.get()
                                on:input=move |e| d_end.set(event_target_value(&e))
                                class=move || format!("w-full px-3 py-2 rounded-xl text-sm border focus:outline-none {}",
                                    tp("bg-slate-50 border-slate-200 text-slate-800 focus:border-slate-300", "bg-white/[0.05] border-white/10 text-white focus:border-white/25")) />
                        </label>
                    </div>
                    <textarea
                        rows="5" placeholder="Add notes..."
                        prop:value=move || d_notes.get()
                        on:input=move |e| d_notes.set(event_target_value(&e))
                        class=move || format!("w-full text-[15px] leading-relaxed focus:outline-none resize-none bg-transparent {} {}",
                            tp("text-slate-600 placeholder:text-slate-300", "text-slate-300 placeholder:text-slate-600"), font_class())
                    ></textarea>
                </div>

                <div class=move || format!("px-6 py-3 border-t flex items-center justify-between text-xs {}",
                    tp("bg-slate-50/70 border-slate-100 text-slate-400", "bg-white/[0.03] border-white/[0.08] text-slate-400"))>
                    <button on:click=move |_| { d_start.set(String::new()); d_end.set(String::new()); }
                        class="font-medium hover:underline">"Make all-day"</button>
                    <span>"Esc to close"</span>
                </div>
            </div>
        </div>

        // ───────── Settings modal ─────────
        <div class=move || format!("fixed inset-0 z-50 {} items-center justify-center p-4 bg-slate-900/25 backdrop-blur-sm",
            if settings_open.get() { "flex" } else { "hidden" })
            on:click=move |_| close_settings()>
            <div on:click=|e| e.stop_propagation()
                class=move || format!("w-full max-w-md rounded-[28px] shadow-2xl border overflow-hidden max-h-[90vh] flex flex-col {}",
                tp("bg-white border-slate-200/90", "bg-[#161922] border-white/10"))>
                <div class=move || format!("flex items-center justify-between px-6 py-5 border-b shrink-0 {}",
                    tp("border-slate-100", "border-white/[0.08]"))>
                    <div>
                        <h3 class=move || format!("text-lg font-bold {}", tp("text-slate-900", "text-white"))>"Agenda Settings"</h3>
                        <p class=move || format!("text-xs mt-0.5 {}", tp("text-slate-500", "text-slate-400"))>"Customize your workspace appearance"</p>
                    </div>
                    <button on:click=move |_| close_settings()
                        class="p-2 rounded-xl text-slate-400 hover:text-slate-700 hover:bg-slate-500/10 transition"
                        inner_html=svg_html("w-4 h-4", "2.2", CLOSE)></button>
                </div>

                <div class="p-6 space-y-6 overflow-y-auto flex-1">
                    <div>
                        <label class=setting_label>"Theme"</label>
                        <div class="grid grid-cols-2 gap-2.5">
                            <button on:click=move |_| update_setting("theme", "light")
                                class=move || sbtn(settings.with(|c| c.theme == "light"), "p-2.5 rounded-2xl flex flex-col items-center gap-2")>
                                <span class="w-full h-7 rounded-xl bg-white border border-slate-200 flex items-center overflow-hidden">
                                    <span class="w-1/3 h-full bg-slate-100 border-r border-slate-200/80"></span>
                                    <span class="w-2/3 h-full bg-white"></span>
                                </span>
                                <span>"Light"</span>
                            </button>
                            <button on:click=move |_| update_setting("theme", "dark")
                                class=move || sbtn(settings.with(|c| c.theme == "dark"), "p-2.5 rounded-2xl flex flex-col items-center gap-2")>
                                <span class="w-full h-7 rounded-xl bg-[#0e1015] border border-slate-700 flex items-center overflow-hidden">
                                    <span class="w-1/3 h-full bg-[#1b1f2a] border-r border-slate-700/80"></span>
                                    <span class="w-2/3 h-full bg-[#0e1015]"></span>
                                </span>
                                <span>"Dark"</span>
                            </button>
                        </div>
                    </div>

                    <div>
                        <label class=setting_label>"Typography"</label>
                        <div class="grid grid-cols-3 gap-2">
                            <button on:click=move |_| update_setting("font", "sans")
                                class=move || sbtn(settings.with(|c| c.font == "sans"), "py-2.5 px-3 rounded-2xl")>"Inter Sans"</button>
                            <button on:click=move |_| update_setting("font", "serif")
                                class=move || sbtn(settings.with(|c| c.font == "serif"), "py-2.5 px-3 rounded-2xl font-serif")>"Editorial Serif"</button>
                            <button on:click=move |_| update_setting("font", "mono")
                                class=move || sbtn(settings.with(|c| c.font == "mono"), "py-2.5 px-3 rounded-2xl font-mono")>"Mono Code"</button>
                        </div>
                    </div>

                    <div>
                        <label class=setting_label>"Calendar Density"</label>
                        <div class="grid grid-cols-2 gap-2">
                            <button on:click=move |_| update_setting("density", "comfortable")
                                class=move || sbtn(settings.with(|c| c.density == "comfortable"), "py-2.5 px-3 rounded-2xl")>"Comfortable"</button>
                            <button on:click=move |_| update_setting("density", "compact")
                                class=move || sbtn(settings.with(|c| c.density == "compact"), "py-2.5 px-3 rounded-2xl")>"Compact"</button>
                        </div>
                    </div>

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

                    <div>
                        <label class=setting_label>"Week Starts On"</label>
                        <div class="grid grid-cols-2 gap-2">
                            <button on:click=move |_| update_setting("week_start", "mon")
                                class=move || sbtn(settings.with(|c| c.week_start == "mon"), "py-2.5 px-3 rounded-2xl")>"Monday"</button>
                            <button on:click=move |_| update_setting("week_start", "sun")
                                class=move || sbtn(settings.with(|c| c.week_start == "sun"), "py-2.5 px-3 rounded-2xl")>"Sunday"</button>
                        </div>
                    </div>

                    <div>
                        <label class=setting_label>"Corner Radius"</label>
                        <div class="grid grid-cols-3 gap-2">
                            <button on:click=move |_| update_setting("radius", "18px")
                                class=move || sbtn(settings.with(|c| c.radius == "18px"), "py-2.5 px-3 rounded-2xl")>"Soft (18px)"</button>
                            <button on:click=move |_| update_setting("radius", "24px")
                                class=move || sbtn(settings.with(|c| c.radius == "24px"), "py-2.5 px-3 rounded-2xl")>"Round (24px)"</button>
                            <button on:click=move |_| update_setting("radius", "32px")
                                class=move || sbtn(settings.with(|c| c.radius == "32px"), "py-2.5 px-3 rounded-2xl")>"Pill (32px)"</button>
                        </div>
                    </div>
                </div>

                <div class=move || format!("px-6 py-4 border-t flex items-center justify-between shrink-0 {}",
                    tp("bg-slate-50 border-slate-100", "bg-white/[0.03] border-white/[0.08]"))>
                    <span class=move || format!("text-[11px] font-medium tracking-wide {}", tp("text-slate-400", "text-slate-500"))>"ver. 1.0.0"</span>
                    <button on:click=move |_| close_settings()
                        class=move || format!("px-4 py-2 rounded-xl text-xs transition {}",
                            tp("bg-slate-900 text-white font-medium hover:bg-slate-800", "bg-white text-slate-900 font-semibold hover:bg-slate-200"))>"Save Preferences"</button>
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_roundtrip_and_weekday() {
        assert_eq!(civil_from_days(days_from_civil(2026, 11, 15)), (2026, 11, 15));
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(weekday("1970-01-01"), 4); // Thursday
        assert_eq!(weekday("2026-11-15"), 0); // Sunday
        assert_eq!(weekday("2026-11-14"), 6); // Saturday
    }

    #[test]
    fn month_math() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 12), 31);
        assert_eq!(add_months("2026-01-31", 1), "2026-02-28");
        assert_eq!(add_months("2026-01-15", -1), "2025-12-15");
        assert_eq!(add_days("2026-12-31", 1), "2027-01-01");
        assert_eq!(week_start_of("2026-11-15", true), "2026-11-09");
        assert_eq!(week_start_of("2026-11-15", false), "2026-11-15");
        assert_eq!(week_range_title("2026-11-09"), "9 – 15 Nov 2026");
    }
}
