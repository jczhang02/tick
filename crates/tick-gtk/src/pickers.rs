//! Picker popovers (`DESIGN.md` 4.9, 4.10): date, deadline, priority, labels, assignee and
//! "Move to...". Each is a floating-surface `GtkPopover` built on demand, with 44 px rows,
//! reporting the choice through a callback. The caller parents it (a detail field row, or the
//! task list pointing at a row) and unparents it when it closes ([`present`]).

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use gtk::glib;

use crate::format::{self, DateSettings};
use crate::i18n::{fill, gettext};
use crate::rows::color_class;

/// Windows shorter than this start the date picker with the calendar collapsed (4.9).
const SHORT_WINDOW: i32 = 700;

/// What a date or deadline picker chose.
#[derive(Debug, Clone, PartialEq)]
pub enum DateChoice {
    /// A calendar date (quick option or calendar day).
    Date(NaiveDate),
    /// Free text for the server to parse (`due.string`).
    Text(String),
    /// "No Date" / "No Deadline".
    Clear,
}

/// What the date picker needs to resolve its quick options.
#[derive(Debug, Clone, Copy)]
pub struct DateInfo {
    pub today: NaiveDate,
    /// `weekend_start_day` (user setting, 1 = Monday): "This Weekend".
    pub weekend: Weekday,
    /// `next_week` (user setting): "Next Week".
    pub next_week: Weekday,
    /// The current date, if any (selected in the calendar, enables the clear option).
    pub current: Option<NaiveDate>,
    pub settings: DateSettings,
}

impl DateInfo {
    pub fn weekday_from_setting(n: Option<i64>, default: Weekday) -> Weekday {
        match n {
            Some(1) => Weekday::Mon,
            Some(2) => Weekday::Tue,
            Some(3) => Weekday::Wed,
            Some(4) => Weekday::Thu,
            Some(5) => Weekday::Fri,
            Some(6) => Weekday::Sat,
            Some(7) => Weekday::Sun,
            _ => default,
        }
    }

    /// "This Weekend": the next `weekend` day from today (today included).
    pub fn this_weekend(&self) -> NaiveDate {
        let mut d = self.today;
        while d.weekday() != self.weekend {
            d += Duration::days(1);
        }
        d
    }

    /// "Next Week": the `next_week` day of the following week.
    pub fn next_week(&self) -> NaiveDate {
        let mut d = self.today + Duration::days(1);
        // Skip to the start of the next week (Monday-based), then to the chosen weekday.
        while d.weekday() != Weekday::Mon {
            d += Duration::days(1);
        }
        while d.weekday() != self.next_week {
            d += Duration::days(1);
        }
        d
    }
}

/// Show `popover` on `parent` (optionally pointing at `rect` in its coordinates) and
/// unparent it once it closed.
pub fn present(
    popover: &gtk::Popover,
    parent: &impl IsA<gtk::Widget>,
    rect: Option<gtk::gdk::Rectangle>,
) {
    popover.set_parent(parent);
    if let Some(r) = rect {
        popover.set_pointing_to(Some(&r));
    }
    popover.connect_closed(|p| {
        let p = p.clone();
        glib::idle_add_local_once(move || p.unparent());
    });
    if popover.is::<gtk::PopoverMenu>() {
        // A new menu shows the separators between its sections from an idle callback: a menu
        // shown before that measures without them and is cut off at the bottom (4.19).
        let p = popover.clone();
        glib::idle_add_local_full(glib::Priority::LOW, move || {
            if p.parent().is_some() {
                p.popup();
            }
            glib::ControlFlow::Break
        });
    } else {
        popover.popup();
    }
}

/// Keep `popover` within `max` px of height: its scrolled content scrolls instead (4.9).
pub fn limit_height(popover: &gtk::Popover, max: i32) {
    fn find(w: &gtk::Widget) -> Option<gtk::ScrolledWindow> {
        if let Some(s) = w.downcast_ref::<gtk::ScrolledWindow>() {
            return Some(s.clone());
        }
        let mut c = w.first_child();
        while let Some(x) = c {
            if let Some(s) = find(&x) {
                return Some(s);
            }
            c = x.next_sibling();
        }
        None
    }
    let Some(child) = popover.child() else { return };
    if let Some(s) = find(&child) {
        // The rest of the picker (entry, quick options) sits outside a label list's scroller.
        let outside = if s.upcast_ref::<gtk::Widget>() == &child {
            0
        } else {
            60
        };
        let cap = (max - outside - 24).max(132);
        if s.max_content_height() < 0 || s.max_content_height() > cap {
            s.set_max_content_height(cap);
        }
        s.set_propagate_natural_height(true);
    }
}

fn popover(width: i32, name: &str) -> gtk::Popover {
    let p = gtk::Popover::builder()
        .has_arrow(false)
        .css_classes(["tick-picker"])
        .width_request(width)
        .build();
    p.update_property(&[gtk::accessible::Property::Label(name)]);
    // Escape closes the picker and goes no further: a search entry would only clear itself,
    // and the event would reach the navigation view, which pops the page.
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(|c, key, _, _| {
        if key == gtk::gdk::Key::Escape
            && let Some(p) = c.widget().and_downcast::<gtk::Popover>()
        {
            p.popdown();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    p.add_controller(keys);
    p
}

fn list(name: &str) -> gtk::ListBox {
    let l = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["tick-picker-list"])
        .build();
    l.update_property(&[gtk::accessible::Property::Label(name)]);
    l
}

/// A 44 px picker row: `[lead 16][gap 12][label][trailing dim]`.
fn row(lead: Option<gtk::Widget>, text: &str, trailing: Option<&str>) -> gtk::ListBoxRow {
    let b = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    let holder = adw::Bin::builder()
        .width_request(16)
        .accessible_role(gtk::AccessibleRole::Presentation)
        .build();
    if let Some(l) = lead {
        holder.set_child(Some(&l));
    }
    b.append(&holder);
    let label = gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    b.append(&label);
    if let Some(t) = trailing {
        b.append(
            &gtk::Label::builder()
                .label(t)
                .css_classes(["dimmed", "numeric"])
                .build(),
        );
    }
    let r = gtk::ListBoxRow::builder()
        .child(&b)
        .css_classes(["tick-picker-row"])
        .build();
    r.update_property(&[gtk::accessible::Property::Label(text)]);
    r
}

fn icon(name: &str) -> gtk::Widget {
    gtk::Image::builder()
        .icon_name(name)
        .accessible_role(gtk::AccessibleRole::Presentation)
        .build()
        .upcast()
}

fn dot(color: &str) -> gtk::Widget {
    gtk::Box::builder()
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .accessible_role(gtk::AccessibleRole::Presentation)
        .css_classes(["tick-dot".to_owned(), color_class(color)])
        .build()
        .upcast()
}

/// Down from an entry moves into the list below it (4.9, 4.10).
fn down_into(entry: &impl IsA<gtk::Widget>, list: &gtk::ListBox) {
    let keys = gtk::EventControllerKey::new();
    let list = list.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if matches!(key, gtk::gdk::Key::Down | gtk::gdk::Key::KP_Down)
            && let Some(l) = list.upgrade()
        {
            let mut i = 0;
            while let Some(r) = l.row_at_index(i) {
                if r.is_visible() && r.is_child_visible() {
                    r.grab_focus();
                    return glib::Propagation::Stop;
                }
                i += 1;
            }
        }
        glib::Propagation::Proceed
    });
    entry.add_controller(keys);
}

/// Printable keys typed while a row has focus go to the search entry (and move focus there),
/// so typing always filters (4.10) and Enter then acts on the search.
fn type_to_search(list: &gtk::ListBox, search: &gtk::SearchEntry) {
    let keys = gtk::EventControllerKey::new();
    let search = search.downgrade();
    keys.connect_key_pressed(move |_, key, _, mods| {
        let Some(ch) = key.to_unicode().filter(|ch| !ch.is_control() && *ch != ' ') else {
            return glib::Propagation::Proceed;
        };
        if mods.intersects(gtk::gdk::ModifierType::CONTROL_MASK | gtk::gdk::ModifierType::ALT_MASK)
        {
            return glib::Propagation::Proceed;
        }
        let Some(s) = search.upgrade() else {
            return glib::Propagation::Proceed;
        };
        s.grab_focus();
        let mut pos = s.text().chars().count() as i32;
        s.insert_text(&ch.to_string(), &mut pos);
        s.set_position(pos);
        glib::Propagation::Stop
    });
    list.add_controller(keys);
}

/// The date picker (4.9), or the deadline picker without the text entry (`deadline`).
pub fn date_picker(
    info: DateInfo,
    deadline: bool,
    on_pick: impl Fn(DateChoice) + 'static,
) -> gtk::Popover {
    let title = if deadline {
        gettext("Deadline")
    } else {
        gettext("Date")
    };
    let pop = popover(340, &title);
    let on_pick: Rc<dyn Fn(DateChoice)> = Rc::new(on_pick);
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let s = info.settings;
    let pick = {
        let pop = pop.downgrade();
        let on_pick = on_pick.clone();
        Rc::new(move |c: DateChoice| {
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
            on_pick(c);
        })
    };
    let entry = (!deadline).then(|| {
        let e = gtk::Entry::builder()
            .placeholder_text(gettext("Type a date, e.g. \"every Monday\""))
            .css_classes(["tick-entry"])
            .margin_bottom(6)
            .build();
        e.update_property(&[gtk::accessible::Property::Label(&gettext("Date text"))]);
        let pick = pick.clone();
        e.connect_activate(move |e| {
            let text = e.text().trim().to_owned();
            if !text.is_empty() {
                pick(DateChoice::Text(text));
            }
        });
        content.append(&e);
        e
    });
    let options = list(&gettext("Quick options"));
    let mut quick: Vec<(&str, String, NaiveDate)> = vec![
        ("weather-clear-symbolic", gettext("Today"), info.today),
        (
            "daytime-sunrise-symbolic",
            gettext("Tomorrow"),
            info.today + Duration::days(1),
        ),
    ];
    if !deadline {
        quick.push((
            "tick-weekend-symbolic",
            gettext("This Weekend"),
            info.this_weekend(),
        ));
    }
    quick.push((
        "tick-next-week-symbolic",
        gettext("Next Week"),
        info.next_week(),
    ));
    let mut actions: Vec<DateChoice> = Vec::new();
    let next_week = gettext("Next Week");
    for (icon_name, label, date) in quick {
        // The resolved day on the right: a weekday within the week, else the date (4.9).
        // "Next Week" always shows its date: on a Sunday it is the same weekday as
        // "Tomorrow" and would read like the same day.
        let right = if (date - info.today).num_days() < 7 && label != next_week {
            format::weekday_short(date.weekday())
        } else {
            fill(
                &gettext("{weekday} {date}"),
                &[
                    ("weekday", &format::weekday_short(date.weekday())),
                    ("date", &format::date_plain(date, info.today, s)),
                ],
            )
        };
        options.append(&row(Some(icon(icon_name)), &label, Some(&right)));
        actions.push(DateChoice::Date(date));
    }
    if info.current.is_some() {
        let label = if deadline {
            gettext("No Deadline")
        } else {
            gettext("No Date")
        };
        options.append(&row(Some(icon("tick-clear-symbolic")), &label, None));
        actions.push(DateChoice::Clear);
    }
    {
        let pick = pick.clone();
        options.connect_row_activated(move |_, r| {
            if let Some(a) = usize::try_from(r.index()).ok().and_then(|i| actions.get(i)) {
                pick(a.clone());
            }
        });
    }
    content.append(&options);
    if let Some(e) = &entry {
        down_into(e, &options);
    }
    let separator = gtk::Separator::builder()
        .margin_top(6)
        .margin_bottom(6)
        .build();
    content.append(&separator);

    // GtkCalendar's own heading has month and year steppers; Tick shows one "November 2026"
    // heading with previous and next month buttons (4.9).
    // Seven 44 px day columns (4.9, 2.7). GtkCalendar's grid measures the hidden week-number
    // column too (8 columns) but allocates 7: the cells ask for 38 px in CSS (8 x 38 = 304)
    // and the calendar is given 7 x 44 = 308, so each day gets exactly 44.
    let calendar = gtk::Calendar::builder()
        .show_heading(false)
        .halign(gtk::Align::Center)
        .width_request(308)
        .build();
    calendar.update_property(&[gtk::accessible::Property::Label(&gettext("Calendar"))]);
    if let Some(d) = info.current.or(Some(info.today))
        && let Some(dt) =
            glib::DateTime::from_local(d.year(), d.month() as i32, d.day() as i32, 0, 0, 0.0).ok()
    {
        calendar.set_date(&dt);
    }
    // Set while the month buttons move the calendar: that is not a pick.
    let paging = Rc::new(std::cell::Cell::new(false));
    {
        let pick = pick.clone();
        let paging = paging.clone();
        calendar.connect_day_selected(move |c| {
            if paging.get() {
                return;
            }
            let d = c.date();
            if let Some(date) =
                NaiveDate::from_ymd_opt(d.year(), d.month() as u32, d.day_of_month() as u32)
            {
                pick(DateChoice::Date(date));
            }
        });
    }
    // Collapsed, the calendar is hidden, not only unrevealed: a crossfading revealer keeps
    // its child's height, and the popover would keep the full calendar's height.
    let heading = gtk::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .margin_start(12)
        .css_classes(["heading"])
        .build();
    let month_button = |icon_name: &str, name: &str, step: i32| {
        let b = gtk::Button::builder()
            .icon_name(icon_name)
            .tooltip_text(name)
            .css_classes(["flat", "tick-icon-44"])
            .build();
        b.update_property(&[gtk::accessible::Property::Label(name)]);
        let calendar = calendar.downgrade();
        let paging = paging.clone();
        b.connect_clicked(move |_| {
            let Some(c) = calendar.upgrade() else { return };
            let d = c.date();
            // The same day in the next or previous month, clamped to its length.
            let (y, m) = (d.year(), d.month() - 1 + step);
            let (y, m) = (y + m.div_euclid(12), m.rem_euclid(12) + 1);
            let last = (28..=31)
                .rev()
                .find(|day| NaiveDate::from_ymd_opt(y, m as u32, *day).is_some())
                .unwrap_or(28);
            let day = d.day_of_month().min(last as i32);
            if let Ok(dt) = glib::DateTime::from_local(y, m, day, 0, 0, 0.0) {
                paging.set(true);
                c.set_date(&dt);
                paging.set(false);
            }
        });
        b
    };
    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .build();
    header.append(&heading);
    header.append(&month_button(
        "go-previous-symbolic",
        &gettext("Previous Month"),
        -1,
    ));
    header.append(&month_button("go-next-symbolic", &gettext("Next Month"), 1));
    let update_heading = {
        let heading = heading.downgrade();
        move |c: &gtk::Calendar| {
            let d = c.date();
            if let (Some(h), Some(date)) = (
                heading.upgrade(),
                NaiveDate::from_ymd_opt(d.year(), d.month() as u32, 1),
            ) {
                h.set_label(&format::month_year(date));
            }
        }
    };
    update_heading(&calendar);
    {
        // Page Up/Down and other-month days move the calendar too.
        let u = update_heading.clone();
        calendar.connect_notify_local(Some("month"), move |c, _| u(c));
        calendar.connect_notify_local(Some("year"), move |c, _| update_heading(c));
    }
    let calendar_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    calendar_box.append(&header);
    calendar_box.append(&calendar);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&calendar_box)
        .reveal_child(true)
        .build();
    // "Pick a Date" row for short windows (4.9).
    let pick_row_list = list(&gettext("Pick a Date"));
    let pick_row = row(
        Some(icon("x-office-calendar-symbolic")),
        &gettext("Pick a Date"),
        None,
    );
    if let Some(b) = pick_row.child().and_downcast::<gtk::Box>() {
        b.append(&icon("pan-down-symbolic"));
    }
    pick_row_list.append(&pick_row);
    pick_row_list.set_visible(false);
    {
        // The calendar takes the quick options' place (change form, not size: P10): entry,
        // month header and grid fit a 600 px window; options and calendar together do not.
        let revealer = revealer.downgrade();
        let calendar = calendar.downgrade();
        let options = options.downgrade();
        let separator = separator.downgrade();
        pick_row_list.connect_row_activated(move |l, _| {
            l.set_visible(false);
            if let Some(o) = options.upgrade() {
                o.set_visible(false);
            }
            if let Some(s) = separator.upgrade() {
                s.set_visible(false);
            }
            if let (Some(r), Some(c)) = (revealer.upgrade(), calendar.upgrade()) {
                r.set_visible(true);
                r.set_reveal_child(true);
                c.grab_focus();
            }
        });
    }
    content.append(&pick_row_list);
    content.append(&revealer);
    {
        // Decided when the popover is realized, before it measures itself for the first
        // time: a short window, or little room at the anchor (see `limit_height`).
        let revealer = revealer.downgrade();
        let pick_row_list = pick_row_list.downgrade();
        pop.connect_realize(move |p| {
            let limited = p
                .child()
                .and_downcast::<gtk::ScrolledWindow>()
                .is_some_and(|s| (0..560).contains(&s.max_content_height()));
            let short = limited
                || p.parent()
                    .and_then(|w| w.root())
                    .is_some_and(|r| r.upcast_ref::<gtk::Widget>().height() < SHORT_WINDOW);
            if let (Some(r), Some(l)) = (revealer.upgrade(), pick_row_list.upgrade()) {
                r.set_reveal_child(!short);
                r.set_visible(!short);
                l.set_visible(short);
            }
        });
    }
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .focusable(false)
        .propagate_natural_height(true)
        .child(&content)
        .build();
    pop.set_child(Some(&scroller));
    {
        // Little room at the anchor can cap the popover below even the calendar-only form:
        // once revealed, scroll the grid (and the focused day) into view, not the top.
        let scroller = scroller.downgrade();
        revealer.connect_child_revealed_notify(move |r| {
            if r.is_child_revealed()
                && let Some(s) = scroller.upgrade()
            {
                let adj = s.vadjustment();
                adj.set_value(adj.upper() - adj.page_size());
            }
        });
    }
    // Focus starts in the entry; in the deadline picker on the first quick option (4.9).
    {
        let entry = entry.clone();
        let options = options.downgrade();
        pop.connect_show(move |_| {
            if let Some(e) = &entry {
                e.grab_focus();
            } else if let Some(r) = options.upgrade().and_then(|l| l.row_at_index(0)) {
                r.grab_focus();
            }
        });
    }
    pop
}

/// The priority picker (4.10): four rows, keys 1 to 4 pick directly.
pub fn priority_picker(current: u8, on_pick: impl Fn(u8) + 'static) -> gtk::Popover {
    let pop = popover(220, &gettext("Priority"));
    let on_pick: Rc<dyn Fn(u8)> = Rc::new(on_pick);
    let options = list(&gettext("Priority"));
    for p in 1..=4u8 {
        let ring = gtk::Box::builder()
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .accessible_role(gtk::AccessibleRole::Presentation)
            .css_classes(["tick-ring-16".to_owned(), format!("p{p}")])
            .build();
        if p == current {
            ring.add_css_class("current");
        }
        let name = fill(&gettext("Priority {n}"), &[("n", &p.to_string())]);
        let r = row(Some(ring.upcast()), &name, Some(&p.to_string()));
        if p == current {
            r.update_state(&[gtk::accessible::State::Selected(Some(true))]);
        }
        options.append(&r);
    }
    let pick = {
        let pop = pop.downgrade();
        let on_pick = on_pick.clone();
        move |p: u8| {
            if let Some(pop) = pop.upgrade() {
                pop.popdown();
            }
            on_pick(p);
        }
    };
    {
        let pick = pick.clone();
        options.connect_row_activated(move |_, r| pick(r.index() as u8 + 1));
    }
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, _| {
        let p = match key {
            gtk::gdk::Key::_1 | gtk::gdk::Key::KP_1 => 1,
            gtk::gdk::Key::_2 | gtk::gdk::Key::KP_2 => 2,
            gtk::gdk::Key::_3 | gtk::gdk::Key::KP_3 => 3,
            gtk::gdk::Key::_4 | gtk::gdk::Key::KP_4 => 4,
            _ => return glib::Propagation::Proceed,
        };
        pick(p);
        glib::Propagation::Stop
    });
    pop.add_controller(keys);
    pop.set_child(Some(&options));
    let opts = options.downgrade();
    pop.connect_show(move |_| {
        if let Some(r) = opts
            .upgrade()
            .and_then(|l| l.row_at_index(i32::from(current.clamp(1, 4)) - 1))
        {
            r.grab_focus();
        }
    });
    pop
}

/// One label in the label picker.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelOption {
    pub name: String,
    pub color: String,
    pub checked: bool,
}

fn tristate(b: bool) -> gtk::AccessibleTristate {
    if b {
        gtk::AccessibleTristate::True
    } else {
        gtk::AccessibleTristate::False
    }
}

fn check_icon(checked: bool) -> gtk::Image {
    let i = gtk::Image::builder()
        .icon_name("object-select-symbolic")
        .accessible_role(gtk::AccessibleRole::Presentation)
        .build();
    i.set_opacity(if checked { 1.0 } else { 0.0 });
    i
}

/// A row with a check column, a lead widget and a name; checked state for AT.
fn check_row(checked: bool, lead: gtk::Widget, text: &str) -> (gtk::ListBoxRow, gtk::Image) {
    let b = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    let check = check_icon(checked);
    b.append(&check);
    let holder = adw::Bin::builder()
        .width_request(20)
        .accessible_role(gtk::AccessibleRole::Presentation)
        .child(&lead)
        .build();
    b.append(&holder);
    b.append(
        &gtk::Label::builder()
            .label(text)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );
    let r = gtk::ListBoxRow::builder()
        .child(&b)
        .css_classes(["tick-picker-row"])
        .accessible_role(gtk::AccessibleRole::Checkbox)
        .build();
    r.update_property(&[gtk::accessible::Property::Label(text)]);
    r.update_state(&[gtk::accessible::State::Checked(tristate(checked))]);
    (r, check)
}

/// The label picker (4.10): search, toggle live, create a missing label. Changes apply as
/// they happen (`on_toggle(name, checked)`, `on_create(name)`).
pub fn label_picker(
    options: Vec<LabelOption>,
    on_toggle: impl Fn(String, bool) + 'static,
    on_create: impl Fn(String) + 'static,
) -> gtk::Popover {
    let pop = popover(300, &gettext("Labels"));
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .build();
    let search = gtk::SearchEntry::builder()
        .placeholder_text(gettext("Search or create a label"))
        .css_classes(["tick-entry", "tick-search"])
        .build();
    search.update_property(&[gtk::accessible::Property::Label(&gettext(
        "Search or create a label",
    ))]);
    content.append(&search);
    let listbox = list(&gettext("Labels"));
    let state: Rc<RefCell<Vec<(LabelOption, gtk::ListBoxRow, gtk::Image)>>> = Rc::default();
    let on_toggle: Rc<dyn Fn(String, bool)> = Rc::new(on_toggle);
    let on_create: Rc<dyn Fn(String)> = Rc::new(on_create);
    // Row 0: "Create label "x"", shown when no label matches exactly.
    let create = row(Some(icon("list-add-symbolic")), "", None);
    create.set_visible(false);
    listbox.append(&create);
    let add_option = {
        let listbox = listbox.downgrade();
        let state = state.clone();
        move |o: LabelOption| {
            let Some(listbox) = listbox.upgrade() else {
                return;
            };
            let lead = icon("tick-label-symbolic");
            let (r, check) = check_row(o.checked, lead, &o.name);
            listbox.append(&r);
            state.borrow_mut().push((o, r, check));
        }
    };
    for o in options {
        add_option(o);
    }
    let filter = {
        let state = state.clone();
        let create = create.downgrade();
        move |text: &str| {
            let q = text.trim().to_lowercase();
            let mut exact = false;
            for (o, r, _) in state.borrow().iter() {
                let n = o.name.to_lowercase();
                r.set_visible(q.is_empty() || n.contains(&q));
                exact |= n == q;
            }
            if let Some(c) = create.upgrade() {
                let show = !q.is_empty() && !exact;
                c.set_visible(show);
                if show {
                    let text = fill(
                        &gettext("Create label \"{name}\""),
                        &[("name", text.trim())],
                    );
                    if let Some(l) = c
                        .child()
                        .and_downcast::<gtk::Box>()
                        .and_then(|b| b.first_child()?.next_sibling())
                        .and_downcast::<gtk::Label>()
                    {
                        l.set_label(&text);
                    }
                    c.update_property(&[gtk::accessible::Property::Label(&text)]);
                }
            }
        }
    };
    let filter = Rc::new(filter);
    {
        let filter = filter.clone();
        search.connect_changed(move |e| filter(&e.text()));
    }
    {
        let state = state.clone();
        let search = search.downgrade();
        let filter = filter.clone();
        let add_option = add_option.clone();
        listbox.connect_row_activated(move |_, r| {
            if r.index() == 0 {
                let Some(e) = search.upgrade() else { return };
                let name = e.text().trim().to_owned();
                if name.is_empty() {
                    return;
                }
                on_create(name.clone());
                add_option(LabelOption {
                    name,
                    color: "charcoal".into(),
                    checked: true,
                });
                e.set_text("");
                filter("");
                return;
            }
            let mut st = state.borrow_mut();
            if let Some((o, row, check)) = st.iter_mut().find(|(_, row, _)| row == r) {
                o.checked = !o.checked;
                check.set_opacity(if o.checked { 1.0 } else { 0.0 });
                row.update_state(&[gtk::accessible::State::Checked(tristate(o.checked))]);
                let (name, checked) = (o.name.clone(), o.checked);
                drop(st);
                on_toggle(name, checked);
            }
        });
    }
    // Enter in the search entry: create when nothing matches, else toggle the first match.
    {
        let listbox = listbox.downgrade();
        search.connect_activate(move |_| {
            if let Some(l) = listbox.upgrade() {
                let mut i = 0;
                while let Some(r) = l.row_at_index(i) {
                    if r.is_visible() {
                        r.activate();
                        break;
                    }
                    i += 1;
                }
            }
        });
    }
    down_into(&search, &listbox);
    // Escape in the search entry closes the picker (a search entry would only clear itself).
    {
        let pop = pop.downgrade();
        search.connect_stop_search(move |_| {
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
        });
    }
    // Typing anywhere in the picker filters (4.10), even after a click moved focus to a row.
    type_to_search(&listbox, &search);
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .focusable(false)
        .propagate_natural_height(true)
        .max_content_height(360)
        .child(&listbox)
        .build();
    content.append(&scroller);
    pop.set_child(Some(&content));
    let s = search.downgrade();
    pop.connect_show(move |_| {
        if let Some(s) = s.upgrade() {
            s.grab_focus();
        }
    });
    pop
}

/// A person in the assignee picker (`id` `None`: "Unassigned").
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub id: Option<String>,
    pub name: String,
}

/// The assignee picker (4.8: the label picker layout with avatars).
pub fn assignee_picker(
    people: Vec<Person>,
    current: Option<String>,
    on_pick: impl Fn(Option<String>) + 'static,
) -> gtk::Popover {
    let pop = popover(300, &gettext("Assignee"));
    let listbox = list(&gettext("Assignee"));
    let mut ids = Vec::new();
    let mut all = vec![Person {
        id: None,
        name: gettext("Unassigned"),
    }];
    all.extend(people);
    for p in all {
        let lead: gtk::Widget = match &p.id {
            Some(_) => adw::Avatar::new(20, Some(&p.name), true).upcast(),
            None => icon("system-users-symbolic"),
        };
        let checked = p.id == current;
        let (r, _) = check_row(checked, lead, &p.name);
        r.set_accessible_role(gtk::AccessibleRole::ListItem);
        listbox.append(&r);
        ids.push(p.id);
    }
    {
        let pop = pop.downgrade();
        listbox.connect_row_activated(move |_, r| {
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
            if let Some(id) = usize::try_from(r.index()).ok().and_then(|i| ids.get(i)) {
                on_pick(id.clone());
            }
        });
    }
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .focusable(false)
        .propagate_natural_height(true)
        .max_content_height(420)
        .child(&listbox)
        .build();
    pop.set_child(Some(&scroller));
    let l = listbox.downgrade();
    pop.connect_show(move |_| {
        if let Some(r) = l.upgrade().and_then(|l| l.row_at_index(0)) {
            r.grab_focus();
        }
    });
    pop
}

/// A destination in "Move to...": a project, or a section of one.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveOption {
    pub project_id: String,
    pub section_id: Option<String>,
    /// Project name, or the section name (shown indented under its project).
    pub name: String,
    pub color: String,
    pub depth: usize,
    pub current: bool,
    /// The Inbox (its own icon instead of a color dot).
    pub inbox: bool,
}

/// "Move to..." (4.4, 4.8): projects and their sections, with a search entry.
pub fn move_picker(
    options: Vec<MoveOption>,
    on_pick: impl Fn(MoveOption) + 'static,
) -> gtk::Popover {
    let pop = popover(300, &gettext("Move to"));
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .build();
    let search = gtk::SearchEntry::builder()
        .placeholder_text(gettext("Search projects"))
        .css_classes(["tick-entry", "tick-search"])
        .build();
    search.update_property(&[gtk::accessible::Property::Label(&gettext(
        "Search projects",
    ))]);
    content.append(&search);
    let listbox = list(&gettext("Projects"));
    let mut rows = Vec::new();
    for o in &options {
        // Sections sit one step in under their project with a dimmed section icon, so they
        // do not read as sub-projects; the Inbox has its own icon (4.1).
        let lead = if o.section_id.is_some() {
            let i = icon("tick-section-symbolic");
            i.add_css_class("dimmed");
            i
        } else if o.inbox {
            icon("tick-inbox-symbolic")
        } else {
            dot(&o.color)
        };
        let (r, _) = check_row(o.current, lead, &o.name);
        r.set_accessible_role(gtk::AccessibleRole::ListItem);
        if let Some(b) = r.child() {
            b.set_margin_start(18 * o.depth as i32);
        }
        listbox.append(&r);
        rows.push(r);
    }
    let names: Vec<String> = options.iter().map(|o| o.name.to_lowercase()).collect();
    {
        let rows = rows.clone();
        search.connect_changed(move |e| {
            let q = e.text().trim().to_lowercase();
            for (r, n) in rows.iter().zip(&names) {
                r.set_visible(q.is_empty() || n.contains(&q));
            }
        });
    }
    {
        let pop = pop.downgrade();
        listbox.connect_row_activated(move |_, r| {
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
            if let Some(o) = usize::try_from(r.index()).ok().and_then(|i| options.get(i)) {
                on_pick(o.clone());
            }
        });
    }
    {
        let listbox = listbox.downgrade();
        search.connect_activate(move |_| {
            if let Some(l) = listbox.upgrade() {
                let mut i = 0;
                while let Some(r) = l.row_at_index(i) {
                    if r.is_visible() {
                        r.activate();
                        break;
                    }
                    i += 1;
                }
            }
        });
    }
    down_into(&search, &listbox);
    // Escape in the search entry closes the picker (a search entry would only clear itself).
    {
        let pop = pop.downgrade();
        search.connect_stop_search(move |_| {
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
        });
    }
    type_to_search(&listbox, &search);
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .focusable(false)
        .propagate_natural_height(true)
        .max_content_height(420)
        .child(&listbox)
        .build();
    content.append(&scroller);
    pop.set_child(Some(&content));
    let s = search.downgrade();
    pop.connect_show(move |_| {
        if let Some(s) = s.upgrade() {
            s.grab_focus();
        }
    });
    pop
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, day).unwrap()
    }

    #[test]
    fn quick_options_follow_the_user_settings() {
        // Sunday 22 November 2026.
        let mut info = DateInfo {
            today: d(11, 22),
            weekend: Weekday::Sat,
            next_week: Weekday::Mon,
            current: None,
            settings: DateSettings::default(),
        };
        assert_eq!(info.this_weekend(), d(11, 28));
        assert_eq!(info.next_week(), d(11, 23));
        info.today = d(11, 25); // Wednesday
        assert_eq!(info.this_weekend(), d(11, 28));
        assert_eq!(info.next_week(), d(11, 30));
        info.next_week = Weekday::Wed;
        assert_eq!(info.next_week(), d(12, 2));
        info.weekend = Weekday::Wed;
        assert_eq!(info.this_weekend(), d(11, 25));
        assert_eq!(
            DateInfo::weekday_from_setting(Some(6), Weekday::Mon),
            Weekday::Sat
        );
    }
}
