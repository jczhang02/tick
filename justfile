# Tick task runner

default: check

# fmt, clippy, unit and integration tests
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --exclude tick-e2e

# E2E tests against the real app in headless mutter (builds tick and the harness fixture)
e2e:
    cargo build -p tick-gtk --bins --examples
    cargo test -p tick-e2e

# Design-review screenshots into target/screenshots/
screenshots:
    cargo build -p tick-gtk --bins --examples
    TICK_SCREENSHOTS=1 cargo test -p tick-e2e screenshots -- --nocapture

# Run against the real Todoist API
run:
    cargo run -p tick-gtk

# Run against a seeded fake server
run-fake:
    cargo run -p tick-fake-server -- --seed demo --launch

# Install release build, desktop file and icon into ~/.local
install:
    cargo build --release -p tick-gtk
    install -Dm755 target/release/tick ~/.local/bin/tick
    install -Dm644 crates/tick-gtk/data/dev.chengrui.Tick.desktop ~/.local/share/applications/dev.chengrui.Tick.desktop
    install -Dm644 crates/tick-gtk/data/icons/dev.chengrui.Tick.svg ~/.local/share/icons/hicolor/scalable/apps/dev.chengrui.Tick.svg
    install -Dm644 crates/tick-gtk/data/icons/dev.chengrui.Tick-symbolic.svg ~/.local/share/icons/hicolor/symbolic/apps/dev.chengrui.Tick-symbolic.svg

# Regenerate po/tick.pot from the sources listed in po/POTFILES (gettext, textdomain "tick")
pot:
    xgettext --from-code=UTF-8 -L C --keyword=_ --keyword=gettext --keyword=ngettext:1,2 \
        --keyword=pgettext:1c,2 --keyword=ncount:1,2 --add-comments=Translators \
        --package-name=tick --msgid-bugs-address=https://github.com/jczhang02/tick/issues \
        -f po/POTFILES -o po/tick.pot
    msgmerge --update --backup=none po/zh_CN.po po/tick.pot
