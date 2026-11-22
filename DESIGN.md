# Tick: design specification

This document is the binding visual specification for Tick. Every UI change implements it;
every UI review checks against it (see "Design review checklist" at the end). When this file and
an implementation disagree, the implementation is wrong unless the owner changes this file.

- Platform: GTK 4.22 + libadwaita 1.9 (`v4_22`, `v1_9` crate features). All libadwaita values
  quoted here were read from the stylesheet compiled into libadwaita 1.9.4
  (`/org/gnome/Adwaita/styles/gtk.css` in `libadwaita-1.so`). CI runs a rolling Arch image that
  may ship a newer libadwaita; re-check the numbers in section 2 when libadwaita changes its
  palette.
- Units: `px` are logical pixels at scale 1. Percent type sizes are relative to the system UI font
  size (GNOME default: Adwaita Sans 11 pt = 14.67 px). Breakpoints use `sp` (scales with the
  text-scaling setting).
- Owner rules: `~/.agents/rules/design-principles.md`. Where this file departs from a default rule
  it says so and why. It never departs from a hard rule.

Contents

1. Principles and references
2. Tokens
3. Layout
4. Components
5. Motion
6. Copy
7. GTK implementation notes
8. App icon concept
9. Design review checklist

---

## 1. Principles and references

### 1.1 The idea: "a quiet desk"

Tick is a working tool that people open dozens of times a day. It should feel like a clean desk
with one sheet of paper on it: the task text is the most prominent thing on screen, chrome
recedes, and color appears only where it carries meaning (priority, due state, project identity,
the system accent for focus and links). Beauty comes from rhythm, type and restraint, not from
decoration.

Four signature moves make Tick recognizable without costume:

1. **The priority ring.** One shape, a round checkbox ring colored by priority, is used
   everywhere priority appears: task rows, the priority picker, quick-add token chips, the
   detail pane. Completing a task fills the ring and draws the check.
2. **The large title hand-off.** Each view starts with a large in-content title. When it scrolls
   away, the same title fades into the header bar.
3. **Upcoming numerals.** Upcoming groups days under large tabular day numbers, so a week reads
   like a calendar page.
4. **One meta line.** Everything about a task except its title sits in one quiet caption line
   with small icons, never in colored pills.

### 1.2 Principles

| # | Principle | What it means in Tick | Owner rule it implements |
|---|---|---|---|
| P1 | Content first | Task titles use full-contrast body text; everything else is secondary. No cards around tasks, no per-row icon buttons at rest. | Quiet (default) |
| P2 | One accent | The GNOME system accent is used for focus rings, links, the selected day in the calendar, drop indicators and the suggested button. Never for selection backgrounds or decoration. | One accent (default) |
| P3 | Color means something | The only extra colors: three priority colors (fixed by SPEC), overdue red (shared with p1), today green, and user-chosen project colors. Each is justified in 2.4. | One accent (default) |
| P4 | Native first | Standard libadwaita widgets, header bars, split views, dialogs, popovers. Tick restyles only what a hard rule requires. | Find a reference first (default) |
| P5 | One floating surface | Popovers, menus, tooltips and toasts share one surface style (2.6). | One floating surface (hard) |
| P6 | Designed dark mode | Every custom color has its own dark value tuned for contrast; floating surfaces get a faint light ring in dark mode where shadows vanish. | Dark mode (hard) |
| P7 | AA everywhere | All text 4.5:1 (including secondary text and captions), all graphical state indicators 3:1, in both themes, on every background the text can sit on (hover and selected included). Tick raises libadwaita's secondary-text opacity to get there (2.4.2). | Contrast (hard) |
| P8 | 44 px targets | Every pointer or touch target is at least 44 x 44 px. Visual size may be smaller; the hit area is not (2.7). | Tap targets (hard) |
| P9 | Input parity | Anything hover reveals, keyboard focus reveals too; Escape dismisses every popover, overlay and dialog; every pointer action has a keyboard path. | Input parity (hard) |
| P10 | Change form, not size | Narrow windows change the presentation (overlay detail, collapsed sidebar, full-width detail), they do not shrink the wide layout. | Change form, not size (default) |
| P11 | Calm motion | Short state transitions only. Reduced motion keeps fades and drops movement. | Motion (hard) |
| P12 | Bilingual by design | English and Simplified Chinese strings are designed together; Chinese uses ASCII punctuation and spaces around Latin words and numbers; font stacks are Latin first. | CJK rules (hard) |

Departures from owner defaults, stated once:

- **Type roles.** Tick is sans-serif throughout (the system font). Task descriptions are short
  notes, not long reading, so the serif-for-reading default does not apply. Code spans use the
  system monospace font.
- **Hairlines vs boxes.** Lists are flat on the view background with hairline section rules,
  as the default asks. The only boxed lists are in the login page and preferences, where they
  are the GNOME convention for forms.

### 1.3 References: what Tick adopts and rejects

Screenshots were reviewed for Planify (repo `data/resources/screenshot/*.png`), Errands, Fractal,
GNOME Calendar (Flathub screenshots) and Things 3 (culturedcode.com marketing images). Todoist web
and the GNOME HIG are described from the owner's daily use and the HIG pages
(developer.gnome.org/hig).

| Reference | Adopt | Reject |
|---|---|---|
| Todoist web | Round checkbox ring colored by priority with a faint tinted fill; meta line under the title (due, labels, project at the end); quick-add token highlighting; date picker with typed date + quick options (Today, Tomorrow, This Weekend, Next Week, No Date) showing the resolved weekday on the right; Undo toast after complete/delete; single-key shortcuts. | Red brand chrome; the task modal that covers the list (Tick uses a side pane); colored date text for every future date (tomorrow orange, this week purple). |
| Things 3 | Large in-content view title with the header bar staying quiet; generous left margin and a clamped content column; project name as a second line in cross-project views; section headers with a hairline rule; Upcoming as a sequence of day groups. | Section titles in the accent color (Tick keeps headings neutral so the accent stays rare); square checkboxes. |
| GNOME HIG + libadwaita | Header bar per pane, split views with breakpoints, `.navigation-sidebar`, boxed lists for forms, `AdwStatusPage` for empty states, `AdwToast`, `AdwPreferencesDialog`, `AdwShortcutsDialog`, the 360 x 294 px minimum window and 1024 x 600 desktop support. | Default 36 px sidebar rows and 32 px menu items (below the 44 px rule); 55 % secondary text opacity (fails AA in light, see 2.4.2); the dark translucent tooltip and the dark pill toast (break the one-surface rule). |
| GNOME Calendar | Mini month calendar with a filled circle on the selected day and the agenda's relative day labels (Today, Tomorrow, then weekday + date). | Colored event blocks with a thick left bar. |
| Fractal | Sidebar section headers as small dim caption headings with a collapse chevron; counts as quiet text right-aligned; composer pinned to the bottom of its pane. | Accent-colored count pills for everything (Tick shows plain dim counts). |
| Errands | 44 px tall sidebar rows (proof that the 44 px rule looks right in a GNOME sidebar); task details in a pane that slides over the list on narrow windows. | One card per task, saturated colored parent cards, a "+" and a menu button on every row, a bottom view switcher inside the detail pane. |

### 1.4 What Planify gets wrong (and the rule that prevents it in Tick)

Observed in Planify's own screenshots (light and dark, around 1240 x 940):

1. **Six-color tile grid at the top of the sidebar** (Inbox blue, Today green, Scheduled purple,
   Pinboard red, Labels brown, Completed orange). Six competing hues appear before the user's own
   project colors, and in dark mode the tinted tiles drop text contrast (brown "Labels" on a brown
   tile). Tick: system views are plain sidebar rows with neutral symbolic icons (P2, P3).
2. **A Material floating action button** in the bottom-right corner, overlapping content. It is a
   phone pattern, not a GNOME one. Tick: "Add Task" lives in the header bar and `Q` (P4).
3. **Unclamped rows.** At 1240 px the task title starts at the left and the project name sits
   700 px away at the right edge. Tick: the list column is clamped to 720 px (3.2).
4. **Due dates as tinted pills before the title.** Every row in Today repeats a green "Today"
   pill, and titles start at different x positions depending on pill width. Tick: the due date
   goes in the meta line under the title, is omitted where the view already implies it (Today,
   Upcoming), and titles always align (4.4).
5. **Square checkboxes with no priority.** Tick: the priority ring (signature move 1).
6. **Form-like detail pane**: bold "Title / Properties / Description / Sub-tasks / Attachments"
   labels, a 2 x 3 grid of property cards, empty boxes with "No subtasks added yet. Get started!".
   Tick: the title is the heading, fields are flat rows with an icon, empty fields show a dim
   placeholder value, not a box (4.8).
7. **Icon noise.** "+" and a menu button on every section header at rest, a sort button, a
   separate close button. Tick: secondary actions appear on hover and on keyboard focus and stay
   reachable through the context menu and shortcuts (P9).
8. **Undesigned dark mode.** Near-black content (#0d0d10-ish) next to a mid-grey sidebar, colored
   rings and emoji mixed as project icons, low-contrast dim text. Tick: Adwaita dark surfaces,
   per-theme custom colors with measured contrast (2.4), project color dots only.
9. **Left-bar colored blocks** for calendar events in Today and Scheduled. Tick shows no calendar
   events and uses no colored side bars anywhere.

---

## 2. Tokens

Color and shadow tokens are CSS custom properties defined in `crates/tick-gtk/data/style.css`
(section 7.2) and use the `--tick-` prefix. Spacing and radius tokens are names used in this
spec and in reviews; code writes their literal values.

### 2.1 Spacing

A 6 px grid, matching libadwaita (which spaces in 6, 12, 18, 24).

| Token | Value | Use |
|---|---|---|
| `space-1` | 3 px | icon-to-text inside a meta item |
| `space-2` | 6 px | gap between meta items' icon groups; row inner gaps; sidebar row side margin |
| `space-3` | 12 px | row horizontal padding; gap between title and trailing controls; gap between meta items |
| `space-4` | 18 px | padding inside popovers and dialogs (libadwaita uses 12 to 24) |
| `space-5` | 24 px | space above a section header; view header bottom margin |
| `space-6` | 36 px | space above an Upcoming month header; empty-state top offset |
| `space-7` | 48 px | view header top margin on wide windows |

Rhythm rules: more space above a heading than below it (section header: 24 above, 6 below);
inside a row, 2 px between the title line and the description line, 2 px before the meta line.

The spacing and radius tokens are names for reviews and specs. In CSS and widget properties,
write the literal values (as libadwaita does); only colors and shadows are defined as CSS
custom properties (7.2).

### 2.2 Radii

| Token | Value | Use |
|---|---|---|
| `radius-row` | 9 px | task rows, sidebar rows, menu items, field rows (same as libadwaita `.navigation-sidebar` rows and buttons) |
| `radius-card` | 12 px | boxed lists (login, preferences), the drag icon of a task row (libadwaita `.card`) |
| `radius-float` | 15 px | every floating surface: popovers, menus, tooltips, toasts (libadwaita popover value) |
| `radius-round` | 9999 px | priority ring, color dots, avatars, the selected calendar day |

### 2.3 Typography

**Family.** The system UI font (Adwaita Sans on GNOME 48+), never a bundled face. Tick does not
set a font size; it inherits the user's size and text scaling.

**Latin-first CJK fallback (hard rule).** At startup Tick reads `Gtk.Settings:gtk-font-name`,
takes its family (for example `Adwaita Sans`), and installs one CSS provider at
`STYLE_PROVIDER_PRIORITY_APPLICATION`:

```css
window, popover, tooltip, dialog {
  font-family: "Adwaita Sans", "Noto Sans CJK SC", "Source Han Sans SC", "Noto Sans SC", sans-serif;
}
```

with the first family replaced by the user's family. Latin letters and digits therefore always
come from the UI font; Han characters use the Simplified Chinese glyph forms even under an
English locale (fontconfig would otherwise pick the Japanese variant for unlabelled Han text).
Re-install the provider on `notify::gtk-font-name`. Monospace (`.monospace`, inline code in
descriptions) uses libadwaita's `--monospace-font-family` (Adwaita Mono) unchanged.

**Scale.** Libadwaita style classes only. Pixel sizes assume the 14.67 px default.

| Role in Tick | Class | Size | Weight | Line height | Where |
|---|---|---|---|---|---|
| View title | `.title-1` | 181 % (26.5 px) | 800 | normal | large in-content title of every view |
| Upcoming day number | `.title-2` + `.numeric` | 136 % (20 px) | 800 | normal | Upcoming day group header |
| Task title in detail | `.title-3` | 136 % (20 px) | 700 | 1.3 | detail pane title field |
| Empty-state title | `.title-2` | 136 % (20 px) | 800 | normal | inside `AdwStatusPage.compact` |
| Month header, quick-add entry | `.title-4` | 118 % (17.3 px) | 700 (month) / 400 (entry, override) | normal | Upcoming month header; quick-add text |
| Section header, field label | `.heading` | 100 % | 700 | normal | project section headers, Upcoming relative-day label, comment author |
| Task title in list, description, comments | `.body` | 100 % | 400 | 140 % | task title (wraps up to 3 lines), description text, comment text |
| Meta line, description preview, sidebar counts | `.caption` | 82 % (12 px) | 400 | 140 % | everything under a task title; counts |
| Sidebar section header | `.caption-heading` | 82 % (12 px) | 700 | normal | "Favorites", "Projects", "Labels", "Filters" |
| Counts, times, dates, day numbers | `.numeric` | inherit | inherit | inherit | add to every label that shows a number that can change |

Rules:

- Weights come only from the classes above. No other weights, no letter-spacing changes, no
  uppercase transforms (they break Chinese and look foreign in GNOME).
- Task titles wrap (`wrap-mode: word-char`, `lines: 3`, `ellipsize: end`), so long URLs and
  Chinese lines break cleanly. The full title is always visible in the detail pane.
- Measure: the list column is clamped to 720 px, which keeps task titles under about 80
  characters per line at 14.67 px. Descriptions in the detail pane are under 60 characters per
  line by the pane width (340 to 420 px).
- Chinese text is never justified (every Tick column is narrow).

### 2.4 Color

#### 2.4.1 Roles from libadwaita (use the variables, never the hex)

| Role | Variable | Light | Dark | Used for |
|---|---|---|---|---|
| Window | `--window-bg-color` / `--window-fg-color` | `#fafafb` / `rgb(0 0 6 / 80%)` | `#222226` / `#ffffff` | login page, behind boxed lists |
| View | `--view-bg-color` / `--view-fg-color` | `#ffffff` / `rgb(0 0 6 / 80%)` | `#1d1d20` / `#ffffff` | task list pane and detail pane |
| Sidebar | `--sidebar-bg-color` | `#ebebed` | `#2e2e32` | the navigation sidebar (set by `AdwNavigationSplitView`) |
| Header bar | `--headerbar-bg-color` | `#ffffff` | `#2e2e32` | all header bars (libadwaita default) |
| Card | `--card-bg-color` | `#ffffff` | `rgb(255 255 255 / 8%)` | boxed lists only |
| Popover | `--popover-bg-color` / `--popover-fg-color` | `#ffffff` | `#36363a` | the floating surface (2.6) |
| Dialog | `--dialog-bg-color` | `#fafafb` | `#36363a` | quick add, preferences, shortcuts |
| Accent | `--accent-bg-color`, `--accent-fg-color`, `--accent-color` | system accent | system accent | 2.4.4 |
| Borders | `--border-color` (`currentColor` at `--border-opacity` 15 %) | | | hairlines: section rules, pane separators |
| Error | `--error-color` | | | inline login error text only |

The detail pane uses the view background, not a sidebar color: it is a second content pane, and
a white or near-black surface is what text fields need. The pane separator is the split view's
own 1 px line.

#### 2.4.2 Secondary text: raise `--dim-opacity` to 72 %

Libadwaita renders secondary text (`.dimmed`, row subtitles, placeholders, window subtitles) at
`--dim-opacity: 55%`. Measured in light mode that is 3.23:1 on white and 3.15:1 on the sidebar,
which fails AA for the 12 px captions Tick uses everywhere. Tick sets:

```css
:root { --dim-opacity: 72%; }
@media (prefers-contrast: more) { :root { --dim-opacity: 90%; } }
```

The second rule restores libadwaita's high-contrast value, which the first would otherwise
override. All secondary text in Tick uses `.dimmed` (the 1.9 name; `.dim-label` is the old alias).

Contrast of primary and secondary (72 %) text on every surface Tick uses. Hover and selected rows
are the row tint (2.4.6) over the surface. Values are WCAG 2 contrast ratios, computed from the
composited colors.

| Surface | Light bg | primary | secondary 72 % | Dark bg | primary | secondary 72 % |
|---|---|---|---|---|---|---|
| view | `#ffffff` | 12.57 | 5.22 | `#1d1d20` | 16.82 | 9.22 |
| window | `#fafafb` | 12.22 | 5.16 | `#222226` | 15.85 | 8.82 |
| sidebar | `#ebebed` | 11.18 | 4.97 | `#2e2e32` | 13.52 | 7.80 |
| popover | `#ffffff` | 12.57 | 5.22 | `#36363a` | 12.03 | 7.10 |
| dialog | `#fafafb` | 12.22 | 5.16 | `#36363a` | 12.03 | 7.10 |
| card on window | `#ffffff` | 12.57 | 5.22 | `#343437` | 12.41 | 7.28 |
| card on dialog | `#ffffff` | 12.57 | 5.22 | `#46464a` | 9.39 | 5.81 |
| list row hover (view + 4 %) | `#f7f7f7` | 12.01 | 5.13 | `#262629` | 15.09 | 8.50 |
| list row selected (view + 10 %) | `#ebebeb` | 11.17 | 4.97 | `#343436` | 12.42 | 7.29 |
| list row selected + hover (view + 13 %) | `#e4e4e5` | 10.69 | 4.88 | `#3a3a3d` | 11.34 | 6.77 |
| sidebar row hover (sidebar + 7 %) | `#dedee0` | 10.28 | 4.79 | `#3d3d40` | 10.83 | 6.52 |
| sidebar row selected (sidebar + 10 %) | `#d8d8db` | 9.87 | 4.71 | `#434346` | 9.86 | 6.04 |
| sidebar row selected + hover (sidebar + 13 %) | `#d3d3d5` | 9.52 | 4.63 | `#49494d` | 8.96 | 5.59 |

For comparison, the libadwaita default (55 %) gives 3.02 to 3.23 in light: every light row
above would fail.

Disabled controls keep libadwaita's `--disabled-opacity` (50 %); WCAG exempts inactive controls.

#### 2.4.3 Tick's own colors

Each extra color is justified by a job no neutral can do:

| Token | Light | Dark | Job | Why it is allowed |
|---|---|---|---|---|
| `--tick-red` | `#bb2f28` | `#ff7f70` | p1 ring; overdue date text; overdue deadline text | Priority colors are fixed by SPEC; overdue is the one state a user must not miss. One red for both keeps the palette small. |
| `--tick-orange` | `#c36400` | `#eb8909` | p2 ring | Fixed by SPEC. |
| `--tick-blue` | `#246fe0` | `#55a0ff` | p3 ring | Fixed by SPEC. Distinct from the default GNOME blue accent (`#3584e4`) in lightness, and the accent is never used for rings, so they do not compete. |
| `--tick-green` | `#007335` | `#78e9ab` | "Today" due text outside the Today view | Lets a user scan a project for what is due today; Todoist users expect it. |
| p4 ring | `currentColor` at 72 % | same | default priority | Neutral; no extra color. |

The starting hues were Todoist's commonly seen priority colors; each was then moved in OKLab
lightness only (hue and chroma kept, per the owner's "adjust within its hue" rule) until it met
its target on every background it can sit on. Rings are graphical objects (target 3:1); red and
green are also used as text (target 4.5:1). Orange and blue are never used as text: the
priority picker labels them "Priority 2" in normal text next to a colored ring.

Light:

| Color | hex | view | popover | dialog | row hover | row selected | row selected + hover | target |
|---|---|---|---|---|---|---|---|---|
| red | `#bb2f28` | 5.92 | 5.92 | 5.67 | 5.52 | 4.96 | 4.66 | 4.5 (text) |
| orange | `#c36400` | 4.08 | 4.08 | 3.91 | 3.81 | 3.42 | 3.21 | 3.0 (ring) |
| blue | `#246fe0` | 4.75 | 4.75 | 4.55 | 4.43 | 3.98 | 3.73 | 3.0 (ring) |
| green | `#007335` | 5.99 | 5.99 | 5.75 | 5.59 | 5.03 | 4.72 | 4.5 (text) |

Dark:

| Color | hex | view | popover | dialog | row hover | row selected | row selected + hover | target |
|---|---|---|---|---|---|---|---|---|
| red | `#ff7f70` | 6.82 | 4.88 | 4.88 | 6.12 | 5.04 | 4.60 | 4.5 (text) |
| orange | `#eb8909` | 6.50 | 4.65 | 4.65 | 5.83 | 4.80 | 4.38 | 3.0 (ring) |
| blue | `#55a0ff` | 6.29 | 4.50 | 4.50 | 5.65 | 4.65 | 4.24 | 3.0 (ring) |
| green | `#78e9ab` | 11.24 | 8.04 | 8.04 | 10.09 | 8.31 | 7.58 | 4.5 (text) |

Check marks inside a filled ring: white in light mode (red 5.92, orange 4.08, blue 4.75 against
the fill); `#1d1d20` in dark mode (red 6.82, orange 6.50, blue 6.29; white would be 2.46 to 2.67).

Due-date states. Only three:

| State | Color | Shown where |
|---|---|---|
| Overdue (date or date-time in the past) | `--tick-red` | everywhere, including Today |
| Today | `--tick-green` | all views except Today and Upcoming (there the grouping already says "today") |
| Any other date | secondary text | everywhere |

Deadlines follow the same rule but are only red (overdue) or secondary; a deadline today is
secondary with the label "Today".

#### 2.4.4 Accent

Tick never sets the accent. It follows the GNOME system accent through `--accent-bg-color` and
uses the standalone `--accent-color` (libadwaita derives it with `oklab(... min(l, 0.5))` in light
and `max(l, 0.85)` in dark) for text and lines.

Allowed accent uses: focus rings, links, the selected day in the date picker, today's day number
in the date picker, drop indicators while dragging, the suggested-action button, the quick-add
token tint, switches and check buttons (libadwaita defaults).

Not allowed: selection backgrounds of task or sidebar rows (they are neutral, 2.4.6), headings,
icons at rest, priority rings, and accent-colored text on the sidebar background (on the selected
sidebar row some accents drop to 3.98:1).

Contrast of the derived `--accent-color` for all nine GNOME accents. Focus ring = Tick's ring
color (accent at 80 %, 2.4.5) composited over the surface.

| Accent | `--accent-bg-color` | light `--accent-color` | on view | on selected sidebar row | focus ring on view | focus ring on selected row | dark `--accent-color` | on view | on popover | focus ring on view | focus ring on selected row |
|---|---|---|---|---|---|---|---|---|---|---|---|
| blue | `#3584e4` | `#0461be` | 6.07 | 4.27 | 4.09 | 3.61 | `#81d0ff` | 9.93 | 7.10 | 6.81 | 5.30 |
| teal | `#2190a4` | `#007184` | 5.68 | 4.00 | 3.85 | 3.40 | `#7bdff4` | 10.97 | 7.85 | 7.46 | 5.80 |
| green | `#3a944a` | `#15772e` | 5.66 | 3.98 | 3.81 | 3.37 | `#8de698` | 11.13 | 7.97 | 7.56 | 5.88 |
| yellow | `#c88800` | `#905300` | 6.13 | 4.31 | 4.01 | 3.55 | `#ffc057` | 10.35 | 7.41 | 7.07 | 5.50 |
| orange | `#ed5b00` | `#b62200` | 6.53 | 4.59 | 4.66 | 4.11 | `#ff9c5b` | 8.13 | 5.82 | 5.66 | 4.42 |
| red | `#e62d42` | `#c00023` | 6.43 | 4.52 | 5.03 | 4.43 | `#ff888c` | 7.33 | 5.24 | 5.15 | 4.03 |
| pink | `#d56199` | `#a2326c` | 6.54 | 4.60 | 4.36 | 3.86 | `#ffa0d8` | 9.03 | 6.46 | 6.23 | 4.86 |
| purple | `#9141ac` | `#8939a4` | 6.59 | 4.64 | 4.30 | 3.81 | `#fba7ff` | 9.67 | 6.92 | 6.64 | 5.17 |
| slate | `#6f8396` | `#526678` | 5.95 | 4.18 | 3.81 | 3.38 | `#bbd1e5` | 10.70 | 7.66 | 7.30 | 5.68 |

#### 2.4.5 Focus ring

Libadwaita draws focus rings at 50 % of `--accent-color`, which measures 2.14 to 2.75:1 on white:
below the 3:1 non-text minimum. Tick draws every focus ring at 80 % (libadwaita's own
`prefers-contrast: more` value): 3.37:1 or better on every surface and accent above.

```css
*:focus:focus-visible { outline-color: color-mix(in srgb, var(--accent-color) 80%, transparent); }
```

Ring geometry stays libadwaita's: 2 px wide; offset -2 px on rows (inside the row), libadwaita's
offsets on buttons and entries.

#### 2.4.6 Row tints (neutral selection)

| State | Value | Notes |
|---|---|---|
| hover | `color-mix(in srgb, currentColor 4%, transparent)` (task list); 7 % (sidebar) | libadwaita values |
| pressed | 8 % (task list); 16 % (sidebar) | libadwaita values |
| selected (the task shown in the detail pane; the current view in the sidebar) | 10 % | replaces libadwaita's `listview > row:selected` accent at 25 %, which would vary with nine accents and compete with priority rings |
| selected + hover | 13 % | |

#### 2.4.7 Project, label and filter colors

The API returns one of 20 color names (`docs/api/pages/colors.md`); unknown names render as
`charcoal`. Tick shows them only as 10 px dots (sidebar, project crumb, meta line project) and in
the color picker. Each dot was adjusted in OKLab lightness to reach 3:1 against the sidebar, view
and popover backgrounds of its theme. `charcoal` light was darkened further than needed so it
stays distinguishable from `grey`.

| API name | API hex | Light dot | worst light | on selected row (light) | Dark dot | worst dark | on selected row (dark) |
|---|---|---|---|---|---|---|---|
| berry_red | `#b8255f` | `#b8255f` | 5.09 | 4.26 | `#db497b` | 3.01 | 2.47 |
| red | `#dc4c3e` | `#dc4c3e` | 3.43 | 2.87 | `#de4e40` | 3.02 | 2.47 |
| orange | `#c77100` | `#c77100` | 3.05 | 2.55 | `#c77100` | 3.32 | 2.72 |
| yellow | `#b29104` | `#a48300` | 3.03 | 2.53 | `#b29104` | 3.98 | 3.26 |
| olive_green | `#949c31` | `#868d1d` | 3.02 | 2.53 | `#949c31` | 4.04 | 3.31 |
| lime_green | `#65a33a` | `#59962c` | 3.03 | 2.54 | `#65a33a` | 3.93 | 3.22 |
| green | `#369307` | `#369307` | 3.31 | 2.77 | `#369307` | 3.06 | 2.50 |
| mint_green | `#42a393` | `#339686` | 3.02 | 2.52 | `#42a393` | 3.95 | 3.24 |
| teal | `#148fad` | `#148fad` | 3.17 | 2.65 | `#148fad` | 3.19 | 2.61 |
| sky_blue | `#319dc0` | `#2292b5` | 3.02 | 2.53 | `#319dc0` | 3.85 | 3.15 |
| light_blue | `#6988a4` | `#6988a4` | 3.12 | 2.61 | `#6988a4` | 3.24 | 2.66 |
| blue | `#4180ff` | `#4180ff` | 3.06 | 2.56 | `#4180ff` | 3.30 | 2.70 |
| grape | `#692ec2` | `#692ec2` | 6.37 | 5.33 | `#9361f5` | 3.02 | 2.47 |
| violet | `#ca3fee` | `#ca3fee` | 3.23 | 2.71 | `#ca3fee` | 3.13 | 2.56 |
| lavender | `#a4698c` | `#a4698c` | 3.54 | 2.97 | `#a86d90` | 3.01 | 2.47 |
| magenta | `#e05095` | `#e05095` | 3.07 | 2.57 | `#e05095` | 3.29 | 2.70 |
| salmon | `#c9766f` | `#c27069` | 3.03 | 2.54 | `#c9766f` | 3.61 | 2.96 |
| charcoal | `#808080` | `#6f6f6f` | 4.22 | 3.53 | `#808080` | 3.05 | 2.50 |
| grey | `#999999` | `#878787` | 3.02 | 2.53 | `#999999` | 4.22 | 3.46 |
| taupe | `#8f7a69` | `#8f7a69` | 3.42 | 2.86 | `#917c6b` | 3.04 | 2.49 |

**Stated exemption.** On the selected sidebar row (sidebar + 10 % tint) most dots fall to about
2.5:1. This is accepted: the dot is a redundant cue next to the project name, which stays at
9.87:1 (light) and 9.86:1 (dark). Do not "fix" it by darkening the selected row, which would
break the text values in 2.4.2. Always send the API name, never the display hex, back to the
server.

Avatars: `AdwAvatar` with the collaborator's image when available, otherwise libadwaita's
initials colors. Those colors identify people and are the platform standard, so they are
allowed; avatars always sit next to the person's name or have the name as accessible label.

#### 2.4.8 How the numbers were computed

WCAG 2.x relative luminance and contrast ratio on sRGB. Translucent colors (`rgb(0 0 6 / 80%)`
text, tints, `--dim-opacity`) were alpha-composited onto the surface before measuring. The
derived accent colors and the lightness adjustments use OKLab (Bjorn Ottosson's matrices) with
clipping to sRGB, which is how GTK evaluates `oklab(from ...)`. GTK may round a channel
differently by 1/255; every ratio above has at least 0.2 headroom except where the table shows a
value within 0.05 of its target (orange ring on a selected + hovered row: 3.21, project dots: 3.0x),
which pass by construction.

### 2.5 Elevation

Tick is flat. Depth comes from surface color (sidebar, view, popover) and hairlines. Shadows
appear only on the floating surface and on the drag icon.

| Name | Value | Use |
|---|---|---|
| floating | `0 1px 5px 1px rgb(0 0 0 / 9%), 0 2px 14px 3px rgb(0 0 0 / 5%)` | popovers, menus, toasts, tooltips, the drag icon (libadwaita's popover shadow) |
| none | | everything else, including task rows and section headers |

### 2.6 The floating surface (hard rule)

One style for everything that floats above the window: popovers (date picker, label picker,
priority picker, sync status), menus (`popover.menu`), tooltips, toasts, and the task drag icon.

| Property | Light | Dark |
|---|---|---|
| background | `--popover-bg-color` (`#ffffff`) | `--popover-bg-color` (`#36363a`) |
| text | `--popover-fg-color` | `--popover-fg-color` |
| border | 1 px `rgb(0 0 0 / 14%)` | 1 px `rgb(255 255 255 / 8%)` (the faint ring where shadows vanish) |
| radius | 15 px | 15 px |
| shadow | floating (2.5) | floating (2.5) |
| inner padding | 6 px (menus), 12 px (tooltips: 6 px x 12 px), 18 px (pickers) | same |

Libadwaita's defaults differ for two of these and Tick overrides them (7.2):

- Tooltips are dark translucent (`rgb(0 0 6 / 80%)`, radius 9 px) by default. Tick restyles them
  to the floating surface. A one-line tooltip at 15 px radius reads as a pill; that is expected.
- Toasts are a dark grey pill (`#505053`, radius 150 px) by default. Tick restyles them to the
  floating surface and resets the light-on-dark accent overrides libadwaita sets on toasts, so
  the "Undo" button uses the normal accent in light mode.

Modal dialogs (`AdwDialog`, `AdwAlertDialog`, preferences, shortcuts) are not floating surfaces in
this sense: they dim the window and keep libadwaita's sheet style.

### 2.7 Hit targets (hard rule: 44 x 44 px)

Every pointer and touch target is at least 44 x 44 px. Three techniques, in order of preference:

1. **Make the widget 44 px.** Rows (task, sidebar, field, menu item, picker option) have
   `min-height: 44px`. The priority ring button is 44 x 44 with an 18 px ring centered inside.
2. **Transparent border.** For small buttons whose visual size must stay small (header bar icon
   buttons are 34 x 34), add `.tick-hit`: a transparent border that extends the hit area with a
   matching negative margin so layout does not move. Verified on GTK 4.22 in headless mutter: a
   flat 34 px header-style button with `.tick-hit` reports 44 x 44 bounds and `Gtk.Widget.pick`
   hits it 21 px above its center; without it, picking misses at 18 px.

   ```css
   .tick-hit { border: 5px solid transparent; margin: -5px; background-clip: padding-box; }
   ```

   (`background-clip: padding-box` keeps hover and pressed backgrounds at the visual size.)
3. **Make it display-only.** Meta-line items (due, deadline, labels, counts, project) are not
   buttons. They are read in the row and edited in the detail pane.

Per-control targets:

| Control | Visual | Hit area | Technique |
|---|---|---|---|
| Sidebar row | full width x 44 | full width x 44 | 1 |
| Sidebar section header (collapse) | full width x 44 | full width x 44 | 1 |
| Header bar icon buttons | 34 x 34 | 44 x 44 | 2 |
| Priority ring | 18 x 18 ring | 44 x 44 | 1 |
| Task row | full width x >= 44 | full width x >= 44 | 1 |
| Sub-task collapse chevron (trailing) | 16 px icon | 44 x 44 | 1 |
| Section header chevron and menu | 16 px icons | 44 x 44 each | 1 |
| Detail field rows | full width x 44 | full width x 44 | 1 |
| Menu items (`popover.menu modelbutton`) | full width x 44 | same | 1 (libadwaita default is 32) |
| Date picker quick options | full width x 44 | same | 1 |
| Calendar days | 36 px circle | 44 x 44 cell | 1 (`calendar > grid > label.day-number { min-width: 44px; min-height: 44px; }`) |
| Toast "Undo" button | text button | 44 px tall | 1 |
| Links in descriptions and comments | inline text | text line height (exempt: WCAG 2.5.8 exempts inline links in sentences) | none |

---

## 3. Layout

### 3.1 Structure

```
AdwApplicationWindow
└─ AdwToastOverlay                         (toasts span the whole window)
   └─ AdwNavigationSplitView               (sidebar | content)
      ├─ sidebar: AdwNavigationPage "Tick"
      │  └─ AdwToolbarView: header bar + sidebar list
      └─ content: AdwNavigationPage (view title)
         └─ AdwOverlaySplitView            (task list | task detail), sidebar-position = end
            ├─ content: AdwToolbarView: header bar + task list
            └─ sidebar: AdwToolbarView: header bar + task detail
```

Window: `default-width 1180`, `default-height 760`, `width-request 360`, `height-request 294`
(the HIG phone minimum). The login page replaces the split view as the window content.

### 3.2 Geometry

| Pane | Property | Value |
|---|---|---|
| Sidebar (`AdwNavigationSplitView`) | `min-sidebar-width` / `max-sidebar-width` / `sidebar-width-fraction` | 240 / 300 / 0.22 (libadwaita defaults are 180 / 280 / 0.25) |
| Task list | `AdwClampScrollable` `maximum-size` / `tightening-threshold` | 720 / 560 |
| Task list | horizontal margin inside the clamp | 12 px (narrow) to 24 px (when the clamp has room) |
| Detail (`AdwOverlaySplitView`) | `min-sidebar-width` / `max-sidebar-width` / `sidebar-width-fraction` | 340 / 420 / 0.38 |
| Detail content | `AdwClamp` `maximum-size` | 420 (fills the pane) |
| View header | top margin | 48 px wide, 24 px when narrow (B2) |

Computed at the screenshot sizes:

- 1280 x 800, detail open: sidebar 282, list pane 619 (column 619, no clamp needed), detail 379.
- 1280 x 800, detail closed: sidebar 282, list pane 998 with the column clamped to 720 and
  centered.
- 1024 x 600 (HIG desktop minimum), detail open: sidebar 240, list 444, detail 340.

The detail pane is shown when a task is opened (click, Enter) and hidden with Escape or its
close button. On wide windows it is docked (pushes the list); otherwise it overlays the list.
When nothing is open there is no detail pane and no "select a task" placeholder: the list
takes the room.

### 3.3 Breakpoints

`AdwBreakpoint` conditions are inclusive ("max-width: 980sp" applies at 980 and below). None of
the breakpoints sits on the screenshot widths (1280, 600).

| Name | Condition | Sidebar | Task list | Detail |
|---|---|---|---|---|
| Wide | above 980 sp | docked | docked | docked, 340 to 420 px, pushes the list |
| B1 medium | `max-width: 980sp` | docked | docked | `AdwOverlaySplitView.collapsed = true`: slides over the list from the end edge, same width rules, list stays full width underneath; click on the list or Escape closes it |
| B2 narrow | `max-width: 720sp` | `AdwNavigationSplitView.collapsed = true`: the sidebar becomes the root page; choosing a view pushes the list | full window width | overlay as in B1 (340 px) |
| B3 phone | `max-width: 500sp` | as B2 | as B2 | overlay at full width: setters `min-sidebar-width = 0`, `max-sidebar-width = 500`, `sidebar-width-fraction = 1.0`; the close button becomes a back arrow at the start |

At 600 x 800 (narrow screenshots) Tick is in B2: the sidebar is a page, the list fills the
window, an open task overlays the right 340 px.

### 3.4 Header bars

All header bars are `AdwHeaderBar` inside `AdwToolbarView` (`top-bar-style: flat`; the bar gets
its bottom hairline only when content scrolls under it, which `AdwToolbarView` does
automatically). Libadwaita decides which header bar shows the window controls.

| Pane | Start | Title | End |
|---|---|---|---|
| Sidebar | "Add Task" (`list-add-symbolic`), only in B2/B3 (breakpoint setter `visible = true`) | none | Search (`system-search-symbolic`, toggles the Search view), Main menu (`open-menu-symbolic`: Preferences, Keyboard Shortcuts, About Tick, Log Out, Quit) |
| Task list | back button (automatic in B2/B3), "Add Task" (`list-add-symbolic`) | `AdwWindowTitle` with the view name, hidden until the large title scrolls out (4.2) | Sync status (only when not idle, 4.14), View menu (`view-more-symbolic`) |
| Detail | B3 only: Back (`go-previous-symbolic`, "Close Task") | none | Task menu (`view-more-symbolic`), Close (`sidebar-show-right-symbolic`, "Close Task"; hidden in B3) |

The detail close button deliberately uses `sidebar-show-right-symbolic`, not a cross, because the
window close button can sit in the same header bar.

All header bar icon buttons: `.flat` (libadwaita default in header bars) plus `.tick-hit`, a
tooltip that names the action and its shortcut ("Add Task (Q)"), and the same text as accessible
label without the shortcut.

### 3.5 Wireframes

Wide, 1280 x 800, a project open with a task selected:

```
┌ sidebar 282 ───────────┬ list 619 ─────────────────────────────┬ detail 379 ─────────────┐
│                 ⌕  ☰   │ +                                  ⋮  │                   ⋮  ▶│ │
│ ▢ Inbox            4   │                                       │ ◯ Book flights          │
│ ☆ Today            7   │  ● Vacation                           │                         │
│ ▦ Upcoming             │    12 tasks                           │ Compare prices on the   │
│                        │                                       │ two airlines...         │
│ FAVORITES          ⌄   │  ◯ Check passport validity            │                         │
│ ● Work            18   │  ◯ Book flights               ⌄       │ ▦ Date      Fri 28 Nov  │
│ PROJECTS       +   ⌄   │    Compare prices on the two...       │ ⚑ Deadline  Mon 1 Dec   │
│ ● Vacation        12   │    ▦ Fri 28 Nov  ⚑ 1 Dec  ◔ 1/3       │ ◯ Priority  Priority 2  │
│   ● Packing        4   │      ◯ Pick seats                     │ ⌗ Labels    travel      │
│ ● Home             6   │                                       │                         │
│ LABELS             ⌄   │  Preparation  3                ⌄  ⋮   │ Sub-tasks          1/3  │
│ ⌗ errands          3   │  ───────────────────────────────────  │ Comments             2  │
│ FILTERS            ⌄   │  ◯ Rent a car                         │ [ Write a comment     ] │
└────────────────────────┴───────────────────────────────────────┴─────────────────────────┘
```

Glyphs in the wireframes stand in for the symbolic icons in 7.3; they are not literal.

Narrow, 600 x 800 (B2), list page with a task open:

```
┌ list 600 ──────────────────────────────────────────────┐
│ ‹  +            Today                       ⋮          │
│                                   ┌ detail 340 ───────┐ │
│  Today                            │          ⋮   ▶│   │ │
│  Sunday 22 November   7 tasks     │ ◯ Book flights    │ │
│                                   │ ...               │ │
│  ◯ Book flights                   │                   │ │
│    ▦ 14:00  ● Vacation            │                   │ │
└───────────────────────────────────┴───────────────────┘
```

---

## 4. Components

Every component lists anatomy, sizes, states and keyboard behavior. States not listed use the
libadwaita default. "Accessible name" is the AT-SPI name the E2E tests use; it is part of the
test contract (`docs/architecture.md`).

### 4.1 Sidebar

**Widget.** `GtkListView` with `.navigation-sidebar`, model = `GtkTreeListModel` over sections and
the project tree, rows built with `GtkTreeExpander` for sub-projects. (`AdwSidebar`, new in 1.9,
was considered and rejected: its items are flat, it has no expanders and no drag source, and
Tick needs a project tree with drag-to-nest. Revisit when it gains hierarchy.)

**Sections, in order.**

1. System views (no header): Inbox, Today, Upcoming, Completed (Completed sits last in this group).
   Search is reached through the header bar button and `/`, not a row.
2. Favorites (header "Favorites"; hidden when empty): favorite projects, labels and filters,
   each with its own icon or dot.
3. Projects (header "Projects", trailing "Add Project" button): the tree. Inbox is not repeated
   here.
4. Labels (header "Labels", trailing "Add Label").
5. Filters (header "Filters", trailing "Add Filter").

**Row anatomy** (height 44, side margin 6, radius 9, horizontal padding 12):

```
[indent 18 x depth][expander 16 or 16 spacer][gap 9][icon 16 or dot 10 centered in 16][gap 12][name, body, ellipsize end][gap 12][count, caption numeric dimmed]
```

- Icons: Inbox `tick-inbox-symbolic`, Today `tick-today-symbolic` (a calendar page with the day
  number area), Upcoming `x-office-calendar-symbolic`, Completed `object-select-symbolic`,
  labels `tick-label-symbolic`, filters `tick-filter-symbolic`. Icons at rest use the row's text
  color at full opacity (they are 16 px symbolic, 3:1 is met by text color).
- Projects show the color dot (2.4.7) instead of an icon. Shared projects add
  `system-users-symbolic` (12 px, dimmed) after the name.
- Count = number of active tasks (Inbox, Today: including overdue; projects: excluding
  sub-projects). Hidden when 0. Upcoming and Completed show no count.
- Counts are always neutral, even when they include overdue tasks. Overdue is visible inside
  the view; the sidebar stays calm.

**Section header** (44 px, not selectable): label in `.caption-heading .dimmed`, left-aligned
with row icons; trailing: optional "Add ..." `list-add-symbolic` button (shown on hover and
focus-within of the header, always exposed to AT), then a chevron `pan-down-symbolic` rotating to
`pan-end` when collapsed. The whole header toggles collapse on click. Collapsed state is saved in
the settings table.

**States.** rest: transparent; hover: 7 %; pressed: 16 %; selected (current view): 10 %;
selected + hover: 13 %; focus-visible: 2 px ring inside the row; drop target while dragging a
task over a project row: `box-shadow: inset 0 0 0 2px var(--accent-color)` and, after 600 ms
hover, the project row expands if collapsed; dragging a project: the row's slot stays with 40 %
opacity and a 2 px `--accent-color` insertion line (with a 6 px dot at its start) marks the drop
position; dropping onto the middle third of another project makes it a sub-project (row shows the
inset accent outline instead of the line). Disabled: none.

**Keyboard.** Up/Down move focus; Enter or Space opens the view (and on B2 pushes the list
page); Left collapses a project or moves to its parent; Right expands; Menu key or Shift+F10
opens the row's context menu (Rename, Color, Favorite, Archive, Delete for projects; Edit,
Delete for labels and filters). Ctrl+Up/Down are not used for reordering; reordering projects
is pointer-only in v1 (stated gap; the context menu offers "Move to..." instead).

**Accessible names.** Row: "Inbox, 4 tasks" (name plus count). Section header: "Projects,
expanded". Add buttons: "Add Project".

### 4.2 View header

At the top of every list, scrolling with it (it is the first item of the scrolled content, not
a list row):

```
[top margin 48 (24 in B2)]
[dot 12 (projects only)][gap 9][title .title-1]
[gap 3]
[subtitle .body .dimmed: date or description][gap 12][count .body .dimmed .numeric]
[bottom margin 24]
```

| View | Title | Subtitle |
|---|---|---|
| Inbox | Inbox | count only |
| Today | Today | "Sunday 22 November" + count |
| Upcoming | Upcoming | count only |
| Project | project name + color dot | count; for shared projects also the member avatars (up to 3, 20 px) |
| Label | label name with `tick-label-symbolic` | count |
| Filter | filter name | the query in `.monospace .dimmed`, then count |
| Search | the search entry replaces the title (4.15) | result count |
| Completed | Completed | "Last 30 days" |

**Title hand-off (signature move 2).** The header bar's `AdwWindowTitle` shows the view name
with `opacity: 0` while the large title is visible. When the scroll offset passes the large
title's bottom edge, the large title stays where it is (it scrolls away) and the header bar
title fades to 1 over 150 ms; scrolling back fades it out. In B2/B3 the same applies.

**Project title editing.** Double-click or F2 on a project title makes it an inline entry
(same font); Enter saves, Escape cancels. Rename is also in the view menu.

### 4.3 Priority ring (checkbox)

The signature control. `GtkCheckButton` with class `.tick-ring` and one of `.p1`, `.p2`, `.p3`,
`.p4`, sized 44 x 44, `valign: start` so it aligns to the first title line.

| State | Ring (18 px, 2 px stroke) | Fill | Check mark |
|---|---|---|---|
| rest | priority color | priority color at 12 % | none |
| hover / focus-visible | priority color | priority color at 20 % | `object-select-symbolic` 12 px, priority color |
| pressed | priority color | priority color at 30 % | same |
| checked (completing) | priority color | priority color 100 % | white (light) / `#1d1d20` (dark), 12 px |
| completed-pending (recurring, waiting for sync) | as checked | as checked | as checked, plus `AdwSpinner` 12 px in the meta line with "Completing" |
| disabled (Completed history view) | `currentColor` at 72 % | none | check in `currentColor` at 72 % |

p4 uses `currentColor` at 72 % for the ring and 0 % fill at rest (no tint for "no priority").
Focus ring: the 2 px accent ring around the 44 px button, radius 9999.

Accessible name: "Complete task" when unchecked, "Mark as not done" when checked; the row's
title is the accessible description. Space toggles.

### 4.4 Task row

**Anatomy** (inside a `GtkListView` with `.tick-task-list`; row radius 9, side margin 6 inside the
clamp, padding 0 12 0 0):

```
[indent 32 x depth][ring 44 x 44][content box, vertical, padding 12 0 12 0, hexpand]
                                    title          .body, wraps <= 3 lines
                                    (2 px)
                                    description    .caption .dimmed, 1 line, ellipsized   (if any)
                                    (2 px)
                                    meta line      .caption, AdwWrapBox, child spacing 12, line spacing 2 (if any)
                                  [trailing box, valign center: assignee avatar 20 | collapse chevron 44 x 44]
```

- The ring column is 44 wide and starts at the row's leading edge; the title starts at 44.
- Sub-task indentation is 32 px per level, which puts a child's ring at its parent's title edge.
  Depth is capped visually at 4 levels (Todoist's limit); deeper tasks indent like level 4.
- Description preview: the first non-empty line of the description, Markdown stripped.
- Row height: 44 with title only (12 + 20.5 + 12 rounded; the ring button sets the 44 minimum);
  about 61 with a meta line; about 80 with description and meta.

**Meta line items**, in this order, each `[icon 12][3 px][label .caption]`, items 12 px apart,
icons and labels individually `.dimmed` unless colored:

| Item | Icon | Text | Color | Hidden when |
|---|---|---|---|---|
| Due | `task-due-symbolic` (recurring: `media-playlist-repeat-symbolic`) | "Today", "Tomorrow", "Wed", "28 Nov", "28 Nov 2027", with time "14:00" when set | 2.4.3 states | no due date; in Today and Upcoming the date part is dropped and only a time remains (no time: item hidden, except overdue which always shows) |
| Deadline | `tick-deadline-symbolic` (a target) | same date format, prefixed by nothing | red when overdue | no deadline |
| Sub-tasks | `tick-subtasks-symbolic` | "1/3" (done/total, numeric) | dimmed | no sub-tasks |
| Comments | `tick-comment-symbolic` | "2" | dimmed | 0 comments |
| Labels | `tick-label-symbolic` once, then names | "travel, errands" (max 2, then "+2") | dimmed | no labels |
| Project | color dot 8 px | "Vacation" or "Vacation / Packing" (section after a slash) | dimmed | in the task's own project view |

The project item is pushed to the end of the meta line (it wraps to the next line rather than
truncating other items).

**Trailing.** Assignee `AdwAvatar` 20 px (shared projects only, accessible name "Assigned to
Name"). For parents, a collapse button: `pan-down-symbolic` 16 px in a 44 x 44 flat button,
rotated -90 degrees when collapsed, `.dimmed` at rest, accessible name "Collapse sub-tasks" /
"Expand sub-tasks".

**States.**

| State | Treatment |
|---|---|
| rest | transparent |
| hover | 4 % tint; ring shows the faint check |
| focus-visible | 2 px focus ring inside the row (offset -2), radius 9 |
| pressed | 8 % tint |
| selected (open in the detail pane) | 10 % tint; selected + hover 13 % |
| completing | ring fills and the check appears (150 ms), title gets `text-decoration: line-through` and `.dimmed`; after 800 ms the row fades to 0 (250 ms) and is removed; the toast appears (4.13) |
| completed-pending (recurring) | ring filled, title not struck, meta line shows the spinner and "Completing" until the sync returns with the next date; then the ring empties and the due date updates |
| failed change | trailing `dialog-warning-symbolic` 16 px in `--tick-red`, tooltip with the server message; the row shows the server state (the change was rolled back) |
| dragging (source) | the row stays in place at 40 % opacity |
| drag icon | the row rendered on the floating surface (2.6) at its own width, radius 12, 95 % opacity |
| drop position | 2 px `--accent-color` line with a 6 px dot at its start, inset by the target depth's indent, between rows; dropping on the right half of a row's content nests it as a sub-task (row gets the inset accent outline instead) |
| disabled | not used (read-only Completed rows use the ring's disabled state and full-contrast titles) |

**Keyboard.** Up/Down (and J/K) move focus between rows; Enter opens the detail pane; Space or
E completes; Left collapses or moves to the parent, Right expands; 1 to 4 set priority; T opens
the date picker anchored to the row; L opens the label picker; Shift+Delete deletes (with toast,
no confirmation dialog); Menu key or Shift+F10 opens the context menu (Open, Complete, Date,
Priority, Labels, Move to..., Delete). Every hover-only affordance (the faint check) also shows
on focus.

**Accessible name.** The row's name is the title; its description is the meta line read as text
("Due today 14:00, deadline 1 December, 1 of 3 sub-tasks, 2 comments, labels travel, project
Vacation"). The visible meta icons are hidden from AT.

### 4.5 Section header (project sections)

```
[space 24 above]
[chevron button 44 x 44, pan-down 16][name .heading][gap 9][count .caption .dimmed .numeric][spacer][menu button 44 x 44, view-more 16]
[hairline 1 px var(--border-color), starts at the name's x, 6 px below the text]
```

- Height 44 plus the hairline. Not selectable; focusable.
- The menu button (Rename, Add Task Here, Delete Section) is visible on hover and on focus within
  the header, and always exposed to AT. On touch, the context menu is reached by long-press.
- Collapsed: chevron rotates to point at the end edge; the section's tasks are removed from the
  model (not hidden widgets); the count stays visible. State persists per section.
- The "(No section)" group has no header.
- After the last task of each section, an "Add task" ghost row (44 px, `list-add-symbolic` in the
  ring column, "Add task" `.dimmed`) opens quick add with `#Project /Section` prefilled.
  Shown only in project views.

Keyboard: the header is in the row focus order; Enter or Space toggles collapse; the Menu key
opens its menu.

### 4.6 Sub-task indentation

Covered in 4.4. Additional rules: collapsing a parent removes descendants from the model;
expanding restores them; the parent's sub-task count stays visible either way. Dragging a task
horizontally while dragging changes its depth in 32 px steps (the insertion line indents to show
the target depth).

### 4.7 Upcoming day groups

The list is a sequence of day groups, starting with "Overdue" (if any) and then today.
The next 7 days always appear, even when empty; after that only days with tasks.

```
Day header (not a row; 56 px tall, top space 18, bottom 6):
[day number .title-2 .numeric, min-width 36, right-aligned][gap 12][relative label .heading][gap 9][weekday/date .body .dimmed]
[hairline below, from the day number's x]
```

| Day | Number | Relative label | Dim text |
|---|---|---|---|
| Overdue group | none (`task-past-due-symbolic` 20 px in `--tick-red`) | "Overdue" | count |
| Today | 22 | "Today" | "Sunday" |
| Tomorrow | 23 | "Tomorrow" | "Monday" |
| Other | 26 | "Thursday" | none |

Month change: a month header ("December", with the year when it is not the current year) in
`.title-4`, 36 px above. An empty day shows only its header plus an "Add task" ghost row (which
prefills the date in the quick-add text: "Thursday" in English, "周四" in Chinese, which the
server parses).

The date part of each task's due chip is dropped (only the time shows). Dragging a task onto
another day's group reschedules it (keeps the time).

### 4.8 Task detail pane

`AdwToolbarView` (header bar 3.4, bottom bar: comment composer), content in a `GtkScrolledWindow`
with an `AdwClamp` (max 420), padding 18 px sides, 6 px top.

```
[project crumb button: dot 8 + "Vacation / Preparation", .flat, 44 tall]    -> Move to... popover
[ring 44][title, .title-3, GtkTextView, wraps, no frame]
[description: rendered Markdown .body, or placeholder "Add a description" .dimmed]
[space 12]
[field rows]
  Date        [task-due-symbolic]       Fri 28 Nov 14:00   (or "No date" .dimmed)
  Deadline    [tick-deadline-symbolic]  Mon 1 Dec          (or "No deadline" .dimmed)
  Priority    [ring 16]                 Priority 2
  Labels      [tick-label-symbolic]     travel, errands    (or "No labels" .dimmed)
  Assignee    [avatar 20]               Ana                (shared projects only; "Unassigned")
[space 24]
[Sub-tasks .heading + "1/3" .caption .dimmed]
  task rows (4.4, compact: no description line), "Add sub-task" ghost row
[space 24]
[Comments .heading + count]
  comment: [avatar 24][name .heading][gap 9][time .caption .dimmed]
           [text .body, selectable; attachment links as "file name" links with tick-attachment-symbolic]
[bottom bar: composer entry "Write a comment", send button 44 x 44 (`tick-send-symbolic`, accessible name "Send Comment")]
```

- **Field row:** 44 px, radius 9, `[icon 16][gap 12][label .dimmed, fixed 96 px column][value .body]`.
  The whole row is a button: hover 4 %, pressed 8 %, focus ring inside. Activating opens the
  matching popover anchored to the value (date picker 4.9, label picker 4.10, priority picker
  4.10, assignee picker = label picker layout with avatars). A clear button (`edit-clear-symbolic`,
  44 x 44) appears at the row end on hover and focus when the field has a value.
- **Title:** always editable (Things-style). Enter commits (no newlines), Escape reverts, focus-out
  commits.
- **Description:** rendered as `GtkLabel` with Pango markup (bold, italic, `code` in
  `.monospace`, links in `--accent-color` underlined, bullet and numbered lists as indented lines
  with "•" or "1."). Click (or Enter when focused) swaps to a `GtkTextView` with the raw text in
  the same position and font (crossfade 150 ms). Ctrl+Enter or focus-out saves; Escape reverts.
  Links open on click without entering edit mode.
- **Completed history tasks** open read-only: fields have no hover, the title is not editable.

Keyboard: Tab cycles title, description, field rows, sub-task rows, comments, composer. Escape
closes the pane (when not editing) and returns focus to the task row. Ctrl+Enter in the composer
sends; Enter also sends; Shift+Enter inserts a newline.

### 4.9 Date picker popover

Anchored to the field row value, the due item, or the task row (T). Floating surface, width 340,
padding 12.

```
[entry: placeholder 'Type a date, e.g. "every Monday"', 44 tall]
[quick options, rows 44:]
   [weather-clear-symbolic]     Today         Sun       (dim, right)
   [daytime-sunrise-symbolic]   Tomorrow      Mon
   [tick-weekend-symbolic]      This Weekend  Sat
   [go-next-symbolic]           Next Week     Mon 30 Nov
   [edit-clear-symbolic]        No Date                  (only when a date is set)
[separator 6]
[GtkCalendar, 7 x 44 day cells]
```

- Typed text is sent as `due_string` and parsed by the server (SPEC: no local date parsing).
  Enter submits; while waiting, the entry shows an `AdwSpinner` as its end icon; the popover
  closes when the change is queued. Offline: the text is queued like any change.
- "This Weekend" and "Next Week" resolve with the user's `weekend_start_day` and `next_week`
  settings; the calendar's first weekday is `start_day`.
- Calendar: today's number in `--accent-color`, weight 700; the selected day is a 36 px filled
  circle `--accent-bg-color` with `--accent-fg-color` text; other-month days use `.dimmed`
  (72 %, replacing libadwaita's 30 %); the header shows "November 2026" with previous/next month
  buttons (44 x 44).
- Deadline uses the same popover with three changes: the entry placeholder is 'Type a date,
  e.g. "Dec 1"' (deadlines have no time and no recurrence), the quick options are Today,
  Tomorrow, Next Week and No Deadline, and the typed text is sent as the deadline date string.

Keyboard: focus starts in the entry; Down moves into the quick options, then into the calendar
(arrow keys move days, Page Up/Down months, Enter picks). Escape closes without changes.

### 4.10 Label picker and priority picker

**Label picker** (floating surface, width 300, max height 420): search entry at top ("Search or
create a label"), list of labels as 44 px rows `[check 16][label icon 16 or dot][name]`, checked
state shown with `object-select-symbolic` at the start. Typing filters; when no label matches
exactly, the first row is "Create label "name"" (`list-add-symbolic`). Space or click toggles,
Enter toggles and keeps the popover open, Escape closes and applies (changes apply live; Undo is
not needed).

**Priority picker** (floating surface, a `popover.menu`-style list, width 220): four 44 px rows
`[ring 16 in the priority color, filled for the current one][gap 12]["Priority 1"]...["Priority
4"]`, with the shortcut digit right-aligned in `.dimmed .numeric`. Keys 1 to 4 pick directly.

### 4.11 Quick add dialog

`AdwDialog` (`content-width 560`; becomes a bottom sheet automatically when the window is
narrow), no visible title, padding 18. Opened by Q, the header bar "+", or an "Add task" ghost
row.

```
[entry, .title-4 weight 400, no frame, 44 tall, placeholder "e.g. Call Mom tomorrow 6pm p2 #Family"]
[token preview: AdwWrapBox of chips, hidden from AT]
[separator]
[footer: hint .caption .dimmed][spacer][Cancel .flat][Add Task .suggested-action, disabled while empty]
```

- **Token highlighting (local, SPEC).** `#project`, `@label` and `p1` to `p4` tokens are found by
  the tick-core highlighter and drawn with Pango attributes on the entry text: background = the
  token tint (accent at 16 % for `#` and `@`; the priority color at 16 % for `pN`), foreground
  unchanged, weight 600. Primary text on the tint measures 10.40:1 or better in light (worst:
  red tint) and 9.41:1 or better in dark (worst: blue tint) on the dialog background. Pango needs
  concrete colors: resolve `--accent-bg-color` from `AdwStyleManager:accent-color-rgba` and the
  priority hexes from 2.4.3 for the current theme, and re-apply on theme or accent change. Dates
  are not highlighted (no local date parsing).
- **Token preview chips** (display-only, `accessible-role: presentation` or not in the AT tree):
  one chip per token, `[dot or ring or label icon 12][name]`, `.caption`, radius 9999, background
  `color-mix(in srgb, currentColor 6%, transparent)`, padding 3 x 9. Unknown projects show the
  name with a `dialog-warning-symbolic` 12 px and the hint changes to "No project named "x".
  The task goes to the Inbox."
- **Prefill.** From a project view: `#Project ` ; from a section ghost row: `#Project /Section ` ;
  from an Upcoming day: the weekday word. The prefill is real, highlighted text the user can
  delete.
- **Footer hint:** online "Enter to add, Esc to close"; offline "Offline. The task will be added
  when you're back online."
- **Submit:** Enter or "Add Task". The dialog stays open and clears for the next task when Ctrl+
  Enter is used ("add another"); plain Enter closes it. A toast "Task added" with "Open" appears
  when the new task is not in the current view.
- **Escape** closes and keeps the draft for the next opening in this session.

### 4.12 Toasts with Undo

`AdwToastOverlay` around the whole split view. Floating surface style (2.6).

| Event | Title | Button | Timeout |
|---|---|---|---|
| Complete one task | "Task completed" | "Undo" | 5 s |
| Complete several in a row | "3 tasks completed" (the existing toast's title updates, timeout restarts) | "Undo" (undoes all) | 5 s |
| Delete | "Task deleted" | "Undo" | 5 s |
| Move by drag to a project | "Moved to Vacation" | "Undo" | 5 s |
| Quick add outside current view | "Task added" | "Open" | 4 s |
| Change rejected by server | "Couldn't save "Buy milk"" | "Details" (opens the sync popover) | 8 s |

Ctrl+Z triggers the visible toast's Undo. Toasts never stack; a new toast replaces the current
one (`AdwToastPriority.HIGH` for errors).

### 4.13 Empty states

`AdwStatusPage` with `.compact`, icon 64 px symbolic at `.dimmed`, title `.title-2`, description
`.body .dimmed`, placed 36 px below the view header (the header stays visible). No illustrations.

| View | Icon | English title / description | Chinese title / description |
|---|---|---|---|
| Inbox | `tick-inbox-symbolic` | "Inbox is empty" / "New tasks land here. Press Q to add one." | "收件箱是空的" / "新任务会先放在这里. 按 Q 添加一个." |
| Today | `weather-clear-symbolic` | "All done for today" / "Enjoy the rest of your day." | "今天的任务都完成了" / "好好享受剩下的时间." |
| Upcoming (no dated tasks at all) | `x-office-calendar-symbolic` | "Nothing scheduled" / "Tasks with a date show up here, day by day." | "近期没有安排" / "设置了日期的任务会按天显示在这里." |
| Project | `tick-project-symbolic` | "No tasks yet" / "Press Q to add the first one." | "还没有任务" / "按 Q 添加第一个任务." |
| Label | `tick-label-symbolic` | "No tasks with @travel" / "Add @travel to a task to see it here." | "没有带 @travel 的任务" / "给任务加上 @travel, 它就会出现在这里." |
| Filter | `tick-filter-symbolic` | "No matching tasks" / "Nothing matches this filter right now." | "没有匹配的任务" / "目前没有任务符合这个过滤器." |
| Search, before typing | `system-search-symbolic` | "Search your tasks" / "Titles and descriptions of active tasks." | "搜索任务" / "搜索进行中任务的标题和描述." |
| Search, no results | `system-search-symbolic` | "No results for "milk"" / "Completed tasks and comments are not searched." | "没有找到 "milk"" / "不会搜索已完成的任务和评论." |
| Completed | `object-select-symbolic` | "No completed tasks" / "Tasks you complete in the next 30 days show up here." | "没有已完成的任务" / "最近 30 天内完成的任务会显示在这里." |

### 4.14 Offline and pending-sync indicator

One place for all sync state: a header bar button in the task list header (end side, left of
the view menu). It is hidden when online with nothing pending (the normal case).

| State | Icon (16, `.tick-hit`) | Label | Tooltip / accessible name |
|---|---|---|---|
| Offline, nothing pending | `network-offline-symbolic`, `.dimmed` | none | "Offline. Changes are saved on this computer." |
| Pending changes (online or offline) | `network-offline-symbolic` (offline) or `network-transmit-symbolic` (online) | count, `.caption .numeric` | "3 changes waiting to sync" |
| A change failed | `dialog-warning-symbolic` in `--tick-red` | count of failures | "1 change could not be saved" |

Clicking opens a floating-surface popover (width 320): status line (`.heading`), the list of
pending and failed changes as 44 px rows ("Complete "Buy milk"", "Rename project "Home""), each
failed row with the server message in `.caption .dimmed`; footer "Sync Now" (`.suggested-action`
when online, insensitive offline) and "Dismiss Errors". Failed changes were already rolled back
(server wins); the list explains what happened.

No banner, no blocking dialog. Startup offline renders the cache with the indicator only.

### 4.15 Search

The Search view replaces the view header title with a `GtkSearchEntry` (`.title-4` size, 44 tall,
placeholder "Search tasks"), focused on open. Results update while typing (FTS5), as normal task
rows with the project item in the meta line; matched words are not highlighted in v1. Escape
clears the entry; Escape on an empty entry returns to the previous view.

### 4.16 Login page

Window content when there is no token. `AdwToolbarView` with a flat header bar (no title,
window controls only), content centered vertically in an `AdwClamp` (max 400), padding 24.

```
[app icon 96 px]
[space 24]
["Welcome to Tick" .title-1, centered]
[space 6]
["Paste your Todoist API token to sign in." .body .dimmed, centered]
[space 24]
[boxed list: AdwPasswordEntryRow "API Token"]
[space 6]
[error line .body in --error-color, only on failure]
[space 12]
[Log In: .suggested-action .pill, centered, 44 tall, insensitive while empty; AdwSpinner while checking]
[space 18]
[link button "Find your token in Todoist settings" -> https://app.todoist.com/app/settings/integrations]
[spacer]
["Your token is stored in the GNOME keyring." .caption .dimmed, centered, 24 from the bottom]
```

Errors: "That token was not accepted. Check it and try again." / "Can't reach Todoist. Check your
connection and try again." Enter in the entry submits.

### 4.17 Preferences

`AdwPreferencesDialog`, one page (no page switcher), title "Preferences".

- Group "Appearance": three `AdwActionRow`s with a radio `GtkCheckButton` as prefix (grouped):
  "Follow System", "Light", "Dark". Rows are 50 px or taller (44 px rule met without a toggle
  group whose buttons would be 34 px). Choosing applies immediately via
  `AdwStyleManager:color-scheme` and is saved in the settings table; the saved scheme is applied
  before the first window is presented (no flash).
- Group "Background": `AdwSwitchRow` "Run at Login", subtitle "Keep syncing and sending reminders
  after the window is closed."
- Group "Account": `AdwButtonRow` "Log Out" with `.destructive-action`; confirms with an
  `AdwAlertDialog` ("Log out of Tick?" / "The local cache and unsynced changes on this computer
  will be removed." / Cancel, Log Out).

### 4.18 Keyboard shortcuts window

`AdwShortcutsDialog` (available since libadwaita 1.8), opened with Ctrl+? and from the main
menu. Sections and items:

| Section | Items |
|---|---|
| General | Quick add `Q`; Search `/`; Preferences `Ctrl+,`; Keyboard shortcuts `Ctrl+?`; Close window `Ctrl+W`; Quit `Ctrl+Q`; Undo `Ctrl+Z` |
| Navigation | Inbox `G I`; Today `G T`; Upcoming `G U`; Move focus `Up`/`Down` or `K`/`J`; Collapse/expand `Left`/`Right`; Back `Alt+Left` |
| Tasks | Open `Enter`; Close details `Escape`; Complete `E` or `Space`; Set date `T`; Set priority `1`-`4`; Add labels `L`; Delete `Shift+Delete`; Context menu `Shift+F10` |

Sequences use `AdwShortcutLabel`'s sequential syntax with `+` (`g+t` renders G, then T);
alternatives are space-separated (`e space`). Single-letter shortcuts are
inactive while a text field has focus.

### 4.19 Context menus and the view menu

Plain `GMenuModel` menus (`popover.menu`) on the floating surface with 44 px items, icons off,
destructive items last after a separator and labelled clearly ("Delete Project..."; the dialog
names the consequence). View menu (project): Rename, Color, Add Section, Favorite / Unfavorite,
Archive, Delete. View menu (label, filter): Edit, Favorite / Unfavorite, Delete. Inbox, Today,
Upcoming, Search and Completed have no view menu in v1; hide the button there instead of
showing an empty or disabled menu.

---

## 5. Motion

### 5.1 Tokens

| Token | Value | Use |
|---|---|---|
| `fast` | 150 ms | hover and press tints, ring fill, title hand-off fade, description edit crossfade |
| `base` | 200 ms | libadwaita's own transitions (outline, background), popover open |
| `exit` | 250 ms | completed row fade-out |
| `hold` | 800 ms | delay between checking a task and its fade-out (gives time to uncheck) |
| easing (CSS) | `cubic-bezier(0.25, 0.46, 0.45, 0.94)` | libadwaita's standard ease-out |
| easing (code) | `AdwEasing::EaseOutCubic` | `AdwTimedAnimation` in Rust |
| springs | libadwaita defaults | split view slides, navigation push/pop, dialog presentation |

### 5.2 What moves and what fades

| Element | Kind | Animation |
|---|---|---|
| Split views (detail overlay, sidebar collapse), navigation push/pop, bottom sheet | movement | libadwaita springs |
| Chevron rotation (sections, sub-tasks, sidebar) | movement | `transition: -gtk-icon-transform 150ms` |
| Drag insertion line | movement | follows the pointer, no easing |
| Completed row | fade | opacity 1 to 0 over 250 ms after the 800 ms hold, then removed. `GtkListView` cannot animate row height, so the rows below close the gap in one step. This is accepted; do not fake it with per-row height animations (recycled rows make it flicker). |
| Header bar title hand-off, description edit swap, toast content changes | fade | 150 ms |
| Popovers, toasts | libadwaita | default |

No entrance animations on view switch, no list staggering, no animated counters.

### 5.3 Reduced motion

GTK 4.22 exposes two system preferences; Tick handles both:

1. `Gtk.Settings:gtk-enable-animations = false` (GNOME "Animations" off; also forced in test
   mode). GTK and libadwaita skip all animations, including fades. Tick's own animations are
   `AdwAnimation`s that follow this setting (the default `follow-enable-animations-setting =
   true`), so they also become instant. The user asked for no animation at all; that is stricter
   than reduced motion and wins.
2. `Gtk.Settings:gtk-interface-reduced-motion = reduce` (new in GTK 4.22; CSS
   `@media (prefers-reduced-motion: reduce)`). Tick keeps fades and drops movement:
   - CSS: chevron rotation transitions are removed inside the media query (fades stay).
   - Code: when the setting is `Reduce`, Tick replaces `scroll_to` smooth scrolling with jumps
     and keeps its opacity animations.
   - Libadwaita 1.9 reads this setting too (the library references
     `gtk-interface-reduced-motion`); whether it turns split-view slides into crossfades must be
     verified in a screenshot run with the setting forced. If it does not, Tick sets
     `AdwOverlaySplitView`/`AdwNavigationSplitView` transitions to their instant form while the
     setting is `Reduce` (accepting no animation over movement).

---

## 6. Copy

### 6.1 Tone

Plain, calm, short. Name the action, not the feature. No exclamation marks, no "Oops", no
emoji, no jokes in errors. Errors say what happened and what to do next. Buttons use title case
in English ("Add Task", "Log In") per GNOME HIG; labels and sentences use sentence case.

### 6.2 Chinese rules (hard)

- ASCII punctuation with English spacing: `已完成 3 个任务.`, `离线. 更改已保存在本机.`,
  `日期, 截止日期, 标签.` Never `，。：；？！“”（）`.
- A space between Chinese and adjacent Latin words or numbers: `按 Q 添加`, `3 个任务`,
  `11 月 22 日 周日`, `Todoist 设置`, `最近 30 天`.
- Quotes are ASCII double quotes with spaces outside: `没有找到 "milk"`.
- User content (task titles, project names) is never rewritten; the rules apply to Tick's own
  strings and to formatted dates and counts.
- Plurals go through `ngettext` even in Chinese (it has one form) so English stays correct.

### 6.3 Dates and times

Formats depend on the user's Todoist settings `date_format` (0 = day first, 1 = month first) and
`time_format` (0 = 24 h, 1 = 12 h), read from the synced user object.

| Case | English, day first | English, month first | Chinese |
|---|---|---|---|
| today / tomorrow / yesterday | Today / Tomorrow / Yesterday | same | 今天 / 明天 / 昨天 |
| within the next 6 days | Wed | Wed | 周三 |
| this year | 28 Nov | Nov 28 | 11 月 28 日 |
| other year | 28 Nov 2027 | Nov 28, 2027 | 2027 年 11 月 28 日 |
| with time | Tomorrow 14:00 / Tomorrow 2:00 PM | same | 明天 14:00 / 明天 下午 2:00 |
| view subtitle (Today) | Sunday 22 November | Sunday, November 22 | 11 月 22 日 周日 |

### 6.4 Key strings

| Key | English | Chinese |
|---|---|---|
| App subtitle | A Todoist client | Todoist 客户端 |
| Views | Inbox, Today, Upcoming, Completed, Search | 收件箱, 今天, 即将到来, 已完成, 搜索 |
| Sidebar headers | Favorites, Projects, Labels, Filters | 收藏, 项目, 标签, 过滤器 |
| Task count | 1 task / 7 tasks | 1 个任务 / 7 个任务 |
| Add Task button | Add Task | 添加任务 |
| Overdue group | Overdue | 已逾期 |
| Field labels | Date, Deadline, Priority, Labels, Assignee | 日期, 截止日期, 优先级, 标签, 负责人 |
| Empty field values | No date, No deadline, No labels, Unassigned | 无日期, 无截止日期, 无标签, 未分配 |
| Priority names | Priority 1 ... Priority 4 | 优先级 1 ... 优先级 4 |
| Date quick options | Today, Tomorrow, This Weekend, Next Week, No Date | 今天, 明天, 本周末, 下周, 无日期 |
| Date entry placeholder | Type a date, e.g. "every Monday" | 输入日期, 例如 "每周一" |
| Quick add placeholder | e.g. Call Mom tomorrow 6pm p2 #Family | 例如: 明天 6pm 给妈妈打电话 p2 #Family |
| Quick add hint, offline | Offline. The task will be added when you're back online. | 离线. 恢复联网后会自动添加这个任务. |
| Toasts | Task completed / 3 tasks completed / Task deleted / Undo | 任务已完成 / 已完成 3 个任务 / 任务已删除 / 撤销 |
| Sync tooltip, offline | Offline. Changes are saved on this computer. | 离线. 更改已保存在本机. |
| Sync tooltip, pending | 3 changes waiting to sync | 3 项更改等待同步 |
| Sync error toast | Couldn't save "Buy milk" | 无法保存 "Buy milk" |
| Sync error detail | Todoist rejected the change. Your task shows the version on the server. | Todoist 拒绝了这项更改. 任务已恢复为服务器上的版本. |
| Login title / body | Welcome to Tick / Paste your Todoist API token to sign in. | 欢迎使用 Tick / 粘贴你的 Todoist API 令牌以登录. |
| Login button / link | Log In / Find your token in Todoist settings | 登录 / 在 Todoist 设置中查找令牌 |
| Login errors | That token was not accepted. Check it and try again. / Can't reach Todoist. Check your connection and try again. | 这个令牌无效. 请检查后重试. / 无法连接 Todoist. 请检查网络后重试. |
| Keyring note | Your token is stored in the GNOME keyring. | 令牌保存在 GNOME 密钥环中. |
| Preferences | Appearance, Follow System, Light, Dark, Run at Login | 外观, 跟随系统, 浅色, 深色, 登录时运行 |
| Run at Login subtitle | Keep syncing and sending reminders after the window is closed. | 关闭窗口后继续同步并发送提醒. |
| Detail placeholders | Add a description / Add sub-task / Write a comment | 添加描述 / 添加子任务 / 写评论 |
| Completed history | Load More | 加载更多 |
| Notification actions | Complete / Open | 完成 / 打开 |

---

## 7. GTK implementation notes

### 7.1 Widget map

| Need | Widget | Notes |
|---|---|---|
| Window | `AdwApplicationWindow` | content: `AdwToastOverlay` |
| Sidebar + content | `AdwNavigationSplitView` | breakpoint B2 sets `collapsed` |
| List + detail | `AdwOverlaySplitView` (`sidebar-position: end`) | B1 sets `collapsed`, B3 widths; `show-sidebar` bound to "a task is open" |
| Breakpoints | `AdwBreakpoint` on the window | conditions in `sp`, inclusive |
| Pane chrome | `AdwToolbarView` + `AdwHeaderBar` | `top-bar-style: flat` |
| Sidebar list | `GtkListView` + `GtkTreeListModel` + `GtkTreeExpander`, `.navigation-sidebar` | not `AdwSidebar` (no tree, no drag source in 1.9) |
| Task list | `GtkListView` (`.tick-task-list`) in `AdwClampScrollable` | `gio::ListStore` of `TaskObject`; section headers via `GtkListView` header factory (`GtkListHeader`) for project sections and Upcoming days |
| Wrapping meta line | `AdwWrapBox` (1.7) | |
| Empty states | `AdwStatusPage.compact` | |
| Quick add, alerts | `AdwDialog`, `AdwAlertDialog` | |
| Preferences | `AdwPreferencesDialog` | |
| Shortcuts | `AdwShortcutsDialog` (1.8) | not `GtkShortcutsWindow` (deprecated) |
| Toasts | `AdwToast` | `button-label`, `action-name` for Undo |
| Pickers | `GtkPopover` | floating surface class applies to all popovers |
| Spinners | `AdwSpinner` (1.6) | |
| Avatars | `AdwAvatar` | `show-initials: true` |
| Theme | `AdwStyleManager:color-scheme` | `PREFER_LIGHT` is not used; "Follow System" = `DEFAULT`, Light = `FORCE_LIGHT`, Dark = `FORCE_DARK` |
| Animations | `AdwTimedAnimation` | `follow-enable-animations-setting` left true |

### 7.2 Starter stylesheet (`crates/tick-gtk/data/style.css`)

Libadwaita loads `style.css` from the application's resource base path automatically
(`/dev/chengrui/Tick/style.css`) at application priority, so these rules override libadwaita
regardless of selector specificity. Verification done for this sketch on GTK 4.22.5 /
libadwaita 1.9.4 in headless mutter: no `parsing-error`; a rendered mock in light and dark
showed the priority rings (rest, checked, dark check mark), neutral selected rows overriding the
accent, red and green due text switching per theme, 44 px sidebar rows, a 44 x 44 `.tick-hit`
button and the toast on the floating surface. Popovers did not map in that minimal test, so the
popover, menu, tooltip and calendar rules are parsed but not yet seen; the first screenshot run
must check them.

```css
/* ---------- tokens ---------- */
:root {
  --dim-opacity: 72%;                       /* 2.4.2: AA for secondary text */
  --tick-red: #bb2f28;
  --tick-orange: #c36400;
  --tick-blue: #246fe0;
  --tick-green: #007335;
  --tick-on-fill: #ffffff;                  /* check mark on a filled ring */
  --tick-float-border: rgb(0 0 0 / 14%);
  --tick-float-shadow: 0 1px 5px 1px rgb(0 0 0 / 9%), 0 2px 14px 3px rgb(0 0 0 / 5%);
}
@media (prefers-color-scheme: dark) {
  :root {
    --tick-red: #ff7f70;
    --tick-orange: #eb8909;
    --tick-blue: #55a0ff;
    --tick-green: #78e9ab;
    --tick-on-fill: #1d1d20;
    --tick-float-border: rgb(255 255 255 / 8%);
  }
}
@media (prefers-contrast: more) {
  :root { --dim-opacity: 90%; }
}

/* ---------- focus ring at 80 % (2.4.5) ---------- */
*:focus:focus-visible {
  outline-color: color-mix(in srgb, var(--accent-color) 80%, transparent);
}

/* ---------- 44 px hit area for small buttons (2.7) ---------- */
.tick-hit {
  border: 5px solid transparent;
  margin: -5px;
  background-clip: padding-box;
}

/* ---------- one floating surface (2.6) ---------- */
popover > contents,
popover > arrow,
tooltip.background,
toast {
  background-color: var(--popover-bg-color);
  color: var(--popover-fg-color);
  border: 1px solid var(--tick-float-border);
  box-shadow: var(--tick-float-shadow);
}
popover > contents,
tooltip.background,
toast {
  border-radius: 15px;
}
tooltip.background { padding: 6px 12px; }
toast {
  /* libadwaita gives toasts light-on-dark accent colors; undo that in light mode */
  --standalone-color-oklab: min(l, 0.5) a b;
}
@media (prefers-color-scheme: dark) {
  toast { --standalone-color-oklab: max(l, 0.85) a b; }
}
popover.menu modelbutton { min-height: 44px; }

/* ---------- sidebar (4.1) ---------- */
.navigation-sidebar > row {
  min-height: 44px;
  margin: 0 6px;
}
.navigation-sidebar .tick-count { font-variant-numeric: tabular-nums; }

/* ---------- task list (4.4) ---------- */
listview.tick-task-list { background: none; }
listview.tick-task-list > row {
  padding: 0 12px 0 0;
  margin: 0 6px;
  border-radius: 9px;
}
listview.tick-task-list > row:hover { background-color: color-mix(in srgb, currentColor 4%, transparent); }
listview.tick-task-list > row:active { background-color: color-mix(in srgb, currentColor 8%, transparent); }
listview.tick-task-list > row:selected { background-color: color-mix(in srgb, currentColor 10%, transparent); }
listview.tick-task-list > row:selected:hover { background-color: color-mix(in srgb, currentColor 13%, transparent); }
listview.tick-task-list > row:focus:focus-visible { outline-offset: -2px; }
listview.tick-task-list > row.tick-drag-source { opacity: 0.4; }

.tick-drop-line {
  min-height: 2px;
  background-color: var(--accent-color);
}
.tick-drop-into { box-shadow: inset 0 0 0 2px var(--accent-color); }
.tick-drag-icon {
  background-color: var(--popover-bg-color);
  color: var(--popover-fg-color);
  border: 1px solid var(--tick-float-border);
  box-shadow: var(--tick-float-shadow);
  border-radius: 12px;
  opacity: 0.95;
}

/* ---------- priority ring (4.3) ---------- */
checkbutton.tick-ring {
  min-width: 44px;
  min-height: 44px;
  padding: 0;
  border-radius: 9999px;
  --tick-ring: color-mix(in srgb, currentColor 72%, transparent);
  --tick-ring-rest: transparent;
}
checkbutton.tick-ring.p1 { --tick-ring: var(--tick-red); }
checkbutton.tick-ring.p2 { --tick-ring: var(--tick-orange); }
checkbutton.tick-ring.p3 { --tick-ring: var(--tick-blue); }
checkbutton.tick-ring.p1,
checkbutton.tick-ring.p2,
checkbutton.tick-ring.p3 { --tick-ring-rest: color-mix(in srgb, var(--tick-ring) 12%, transparent); }
checkbutton.tick-ring > check {
  min-width: 18px;
  min-height: 18px;
  margin: 13px;
  padding: 0;
  border-radius: 9999px;
  border: none;
  background-image: none;
  background-color: var(--tick-ring-rest);
  box-shadow: inset 0 0 0 2px var(--tick-ring);
  color: var(--tick-ring);
  -gtk-icon-source: none;
  -gtk-icon-size: 12px;
  transition: background-color 150ms cubic-bezier(0.25, 0.46, 0.45, 0.94);
}
checkbutton.tick-ring:hover > check,
checkbutton.tick-ring:focus-visible > check {
  background-color: color-mix(in srgb, var(--tick-ring) 20%, transparent);
  -gtk-icon-source: -gtk-icontheme("object-select-symbolic");
}
checkbutton.tick-ring:active > check {
  background-color: color-mix(in srgb, var(--tick-ring) 30%, transparent);
}
checkbutton.tick-ring > check:checked {
  background-color: var(--tick-ring);
  color: var(--tick-on-fill);
  -gtk-icon-source: -gtk-icontheme("object-select-symbolic");
}

/* ---------- meta line and due states (4.4, 2.4.3) ---------- */
.tick-meta image { -gtk-icon-size: 12px; }
.tick-due.overdue,
.tick-deadline.overdue { color: var(--tick-red); }
.tick-due.today { color: var(--tick-green); }
.tick-dot {
  min-width: 10px;
  min-height: 10px;
  border-radius: 9999px;
}

/* ---------- collapse chevrons (5.2, 5.3) ---------- */
.tick-chevron image { transition: -gtk-icon-transform 150ms cubic-bezier(0.25, 0.46, 0.45, 0.94); }
.tick-chevron.collapsed image { -gtk-icon-transform: rotate(-90deg); }
@media (prefers-reduced-motion: reduce) {
  .tick-chevron image { transition: none; }
}

/* ---------- section and Upcoming headers (4.5, 4.7) ---------- */
.tick-section-header { margin-top: 24px; }
.tick-section-rule {
  min-height: 1px;
  background-color: var(--border-color);
}
.tick-day-header { margin-top: 18px; min-height: 56px; }
.tick-month-header { margin-top: 36px; }

/* ---------- detail field rows (4.8) ---------- */
.tick-field-row {
  min-height: 44px;
  padding: 0 12px;
  border-radius: 9px;
}
.tick-field-row:hover { background-color: color-mix(in srgb, currentColor 4%, transparent); }
.tick-field-row:active { background-color: color-mix(in srgb, currentColor 8%, transparent); }

/* ---------- quick-add chips (4.11) ---------- */
.tick-token-chip {
  padding: 3px 9px;
  border-radius: 9999px;
  background-color: color-mix(in srgb, currentColor 6%, transparent);
}

/* ---------- date picker calendar (4.9) ---------- */
popover calendar { border: none; background: none; }
popover calendar > grid > label.day-number {
  min-width: 44px;
  min-height: 44px;
  padding: 0;
  border-radius: 9999px;
}
popover calendar > grid > label.day-number.other-month {
  color: inherit;
  opacity: var(--dim-opacity);
}
popover calendar > grid > label.today {
  box-shadow: none;
  color: var(--accent-color);
  font-weight: 700;
}
popover calendar > grid > label.day-number:selected {
  background-color: var(--accent-bg-color);
  color: var(--accent-fg-color);
}

/* ---------- project colors (2.4.7); class = tick-color-<api name> ---------- */
.tick-color-berry_red { background-color: #b8255f; }
.tick-color-red { background-color: #dc4c3e; }
.tick-color-orange { background-color: #c77100; }
.tick-color-yellow { background-color: #a48300; }
.tick-color-olive_green { background-color: #868d1d; }
.tick-color-lime_green { background-color: #59962c; }
.tick-color-green { background-color: #369307; }
.tick-color-mint_green { background-color: #339686; }
.tick-color-teal { background-color: #148fad; }
.tick-color-sky_blue { background-color: #2292b5; }
.tick-color-light_blue { background-color: #6988a4; }
.tick-color-blue { background-color: #4180ff; }
.tick-color-grape { background-color: #692ec2; }
.tick-color-violet { background-color: #ca3fee; }
.tick-color-lavender { background-color: #a4698c; }
.tick-color-magenta { background-color: #e05095; }
.tick-color-salmon { background-color: #c27069; }
.tick-color-charcoal { background-color: #6f6f6f; }
.tick-color-grey { background-color: #878787; }
.tick-color-taupe { background-color: #8f7a69; }
@media (prefers-color-scheme: dark) {
  .tick-color-berry_red { background-color: #db497b; }
  .tick-color-red { background-color: #de4e40; }
  .tick-color-orange { background-color: #c77100; }
  .tick-color-yellow { background-color: #b29104; }
  .tick-color-olive_green { background-color: #949c31; }
  .tick-color-lime_green { background-color: #65a33a; }
  .tick-color-green { background-color: #369307; }
  .tick-color-mint_green { background-color: #42a393; }
  .tick-color-teal { background-color: #148fad; }
  .tick-color-sky_blue { background-color: #319dc0; }
  .tick-color-light_blue { background-color: #6988a4; }
  .tick-color-blue { background-color: #4180ff; }
  .tick-color-grape { background-color: #9361f5; }
  .tick-color-violet { background-color: #ca3fee; }
  .tick-color-lavender { background-color: #a86d90; }
  .tick-color-magenta { background-color: #e05095; }
  .tick-color-salmon { background-color: #c9766f; }
  .tick-color-charcoal { background-color: #808080; }
  .tick-color-grey { background-color: #999999; }
  .tick-color-taupe { background-color: #917c6b; }
}
```

### 7.3 Icons

Symbolic icons only, 16 px (12 px in the meta line). Use the Adwaita theme name when it exists;
otherwise bundle an SVG from GNOME's Icon Library (`org.gnome.design.IconLibrary`) into the
gresource under `/dev/chengrui/Tick/icons/scalable/actions/` with the `tick-` name below.

| Role | Name | Source |
|---|---|---|
| Inbox | `tick-inbox-symbolic` (tray) | Icon Library |
| Today | `tick-today-symbolic` (calendar page with a dot) | Icon Library |
| Upcoming | `x-office-calendar-symbolic` | Adwaita |
| Completed | `object-select-symbolic` | Adwaita |
| Project (empty state) | `tick-project-symbolic` (list with bullets) | Icon Library |
| Label | `tick-label-symbolic` (tag) | Icon Library |
| Filter | `tick-filter-symbolic` (funnel) | Icon Library |
| Due date | `task-due-symbolic` | Adwaita |
| Recurring | `media-playlist-repeat-symbolic` | Adwaita |
| Deadline | `tick-deadline-symbolic` (target / flag) | Icon Library |
| Sub-tasks | `tick-subtasks-symbolic` (indented list) | Icon Library |
| Comments | `tick-comment-symbolic` (speech bubble) | Icon Library |
| Attachment | `tick-attachment-symbolic` (paperclip) | Icon Library |
| Send | `tick-send-symbolic` (paper plane) | Icon Library |
| Weekend | `tick-weekend-symbolic` (sofa or sun-and-moon) | Icon Library |
| Overdue group | `task-past-due-symbolic` | Adwaita |
| Offline / pending / error | `network-offline-symbolic`, `network-transmit-symbolic`, `dialog-warning-symbolic` | Adwaita |
| Detail close | `sidebar-show-right-symbolic` | Adwaita |
| Shared project | `system-users-symbolic` | Adwaita |

### 7.4 Known pitfalls

1. **Opacity multiplies.** `.dimmed` on a container dims every child, including red or green due
   text, and breaks the contrast tables. Apply `.dimmed` to individual labels and icons, never
   to the meta box or the row.
2. **`listview > row:selected` uses the accent at 25 %.** Tick's override in 7.2 must stay at
   application priority; check a selected row in screenshots with a non-blue accent.
3. **`.navigation-sidebar` rows default to 36 px** with a 2 px bottom margin. The override sets
   44 px and removes the gap; selected and hover backgrounds then touch. That is intended.
4. **`GtkListView` rows get 2 px padding** from libadwaita (`listview > row { padding: 2px }`).
   The task-row rule resets it; do not add padding on the row's child as a workaround.
5. **Negative margins.** GTK accepts negative CSS margins (libadwaita uses them); the hit area is
   the border box. `.tick-hit` was verified with `Gtk.Widget.pick`; E2E tests should assert
   44 x 44 extents through AT-SPI for header buttons.
6. **Focus ring offsets with `.tick-hit`.** The outline is drawn outside the border box, so on a
   `.tick-hit` button it sits 5 px outside the visible button. If that looks detached in
   screenshots, add `outline-offset: -5px` to `.tick-hit:focus:focus-visible`.
7. **Tooltip shadows.** Tooltips are separate surfaces; if GTK clips the floating shadow on
   tooltips, drop the shadow for tooltips only and keep background, border and radius. Verify in
   a screenshot.
8. **Toast accent.** Libadwaita sets light-on-dark `--accent-color` on toasts. Without the
   `--standalone-color-oklab` reset in 7.2, the "Undo" button text would be a pale accent on a
   white toast.
9. **Pango attributes need real colors.** The quick-add token tint cannot use CSS variables;
   compute RGBA from `AdwStyleManager` (`accent-color-rgba`, `dark`) and re-apply on
   `notify::accent-color` and `notify::dark`.
10. **API priority is inverted.** `priority: 4` in the API is p1 (`docs/api/pages/sync_tasks.md`).
    Map once in tick-core; the UI only sees p1 to p4.
11. **Unknown project colors** must render as `charcoal` and must not be sent back.
12. **Theme before first paint.** Read the saved scheme from SQLite and set
    `AdwStyleManager:color-scheme` before presenting the window; `TICK_COLOR_SCHEME` overrides it
    for screenshots.
13. **Font provider.** The CJK fallback provider must be rebuilt when `gtk-font-name` changes;
    do not hard-code "Adwaita Sans" (users may choose another UI font).
14. **Header factories.** Use `GtkListView`'s section header factory for section and day
    headers rather than mixing header rows into the task model; it keeps keyboard navigation and
    selection on tasks only. Section headers still need focusable controls (chevron, menu).
15. **Color-scheme media queries need the provider's scheme.** The auto-loaded `style.css`
    follows `AdwStyleManager` (verified: dark values apply). A `GtkCssProvider` that Tick creates
    in code (the font provider in 2.3, any test provider) ignores `@media (prefers-color-scheme:
    dark)` unless its `prefers-color-scheme` property is bound to `AdwStyleManager:dark`. Keep
    all colors in `style.css`.
16. **CI differs from the dev machine.** CI installs the current Arch `gtk4`/`libadwaita`
    (newer than 4.22/1.9 at the time of writing); `gtk4` pulls `adwaita-fonts`, and the workflow
    installs `noto-fonts-cjk`, so the font stack resolves the same way. Compare screenshots from
    the same environment only.

---

## 8. App icon concept (later milestone)

Follows the GNOME app icon style: a 128 x 128 canvas, front view with a slight top-down
perspective, a visible base that gives the object thickness, simple shapes, colors from the
GNOME palette, no text, no fine detail.

**Concept: "the card and the ring".** A white index card (light_1 face `#ffffff`, light_3 base
`#deddda` for its thickness) lying on the desk, slightly taller than wide, with rounded corners.
On it, one large priority ring in GNOME blue (blue_3 `#3584e4` stroke, blue_5 `#1a5fb4` for the
ring's inner shadow) with a bold check mark whose long stroke leaves the ring at the top right,
as if drawn in one motion. Two short grey lines (light_4 `#c0bfbc`) to the right of the ring
suggest a task title. The card never uses red and never shows stacked check marks, so it cannot
be mistaken for the Todoist logo.

**Symbolic icon** (`dev.chengrui.Tick-symbolic`, 16 px): the ring and the escaping check only,
2 px strokes on the 16 px grid.

Deliverables: `crates/tick-gtk/data/icons/dev.chengrui.Tick.svg` and
`dev.chengrui.Tick-symbolic.svg`, checked with App Icon Preview against light and dark
backgrounds at 128, 64 and 32 px.

---

## 9. Design review checklist

Reviewers run `just screenshots` (light and dark, 1280 x 800 and 600 x 800) and check every item.
A "no" blocks the PR unless the item is marked as a known gap in the PR body.

**Identity and quiet**

- [ ] Task titles are the most prominent text; no cards around tasks; no per-row buttons visible
      at rest except the ring and (for parents) the collapse chevron.
- [ ] The accent appears only on focus rings, links, the suggested button, the calendar's
      selected and today days, and drag indicators. Selected rows are neutral grey.
- [ ] No colors beyond: accent, red, orange, blue, green (2.4.3), project dots, avatars.
- [ ] Due dates are in the meta line, not before the title; the Today view shows times only;
      titles start at the same x in every row of a level.

**Light and dark**

- [ ] Both themes show the same layout; dark is not an inversion (popovers lighter than the view,
      faint light ring on floating surfaces).
- [ ] Secondary text is readable on hover and selected rows (it uses 72 %, not 55 %).
- [ ] Priority rings and project dots are visible on every background in both themes.

**Wide and narrow**

- [ ] 1280: three panes when a task is open; the list column is clamped to 720 when the detail
      is closed.
- [ ] 600: sidebar collapsed to a page; the list fills the window; an open task overlays at
      340 px; nothing is cut off or overlapping.
- [ ] Header bars carry exactly the controls in 3.4 for the current breakpoint.

**Floating surfaces**

- [ ] Popovers, menus, tooltips and toasts share background, border, 15 px radius and shadow.
- [ ] Menu items and picker rows are 44 px tall.

**Targets, focus, motion**

- [ ] Every control is at least 44 x 44 (rows, ring, header buttons via `.tick-hit`, calendar
      days). E2E AT-SPI extents confirm header buttons.
- [ ] Focus rings are visible (80 % accent) on rows, buttons and calendar days.
- [ ] Anything that appears on hover also appears on keyboard focus; Escape closes every popover,
      dialog and the detail pane.
- [ ] With reduced motion, nothing slides or rotates; fades remain.

**Copy and language**

- [ ] Chinese screenshots: ASCII punctuation only, spaces between Chinese and Latin or digits,
      dates as `11 月 22 日 周日`.
- [ ] Latin letters and digits in Chinese strings come from the UI font, not the CJK font.
- [ ] Every interactive control has an accessible name (E2E finds it by name).
- [ ] Empty states use the copy in 4.13; errors say what happened and what to do.
