//! One isolated desktop session: private D-Bus bus, temp HOME/XDG dirs, headless mutter
//! and the app under test.

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

use anyhow::{Context as _, bail};
use tokio::sync::{OnceCell, OwnedSemaphorePermit};

use crate::a11y::{A11y, Node, Query};
use crate::input::Input;
use crate::notify::Notifications;
use crate::{Result, Role, poll};

/// App ID (and D-Bus name) of Tick.
pub const TICK_APP_ID: &str = "dev.chengrui.Tick";
/// Wayland socket name inside the session's private `XDG_RUNTIME_DIR`.
const WAYLAND_DISPLAY: &str = "tick-e2e-0";
/// Budget for each startup step (bus, compositor, app on AT-SPI). Generous for CI.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
/// Timeout of every D-Bus method call made by the harness.
const METHOD_TIMEOUT: Duration = Duration::from_secs(15);

/// Inherited variables that are safe and useful to pass to session processes.
/// Everything else is cleared so the user's real session (DISPLAY, WAYLAND_DISPLAY,
/// DBUS_SESSION_BUS_ADDRESS, GTK_IM_MODULE, AT_SPI_BUS_ADDRESS, ...) cannot leak in.
const PASSTHROUGH_ENV: &[&str] = &[
    "PATH",
    "XDG_DATA_DIRS",
    "XDG_CONFIG_DIRS",
    "RUST_BACKTRACE",
    "RUST_LOG",
    "G_MESSAGES_DEBUG",
    "GSK_RENDERER",
    "GDK_DEBUG",
    "GTK_DEBUG",
    "LIBGL_ALWAYS_SOFTWARE",
    "MUTTER_DEBUG",
];

/// Forced light or dark style (`TICK_COLOR_SCHEME`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorScheme {
    Light,
    Dark,
}

/// Options for [`Session::builder`].
#[derive(Debug, Clone)]
pub struct SessionBuilder {
    width: u32,
    height: u32,
    env: Vec<(String, String)>,
    app_bin: Option<PathBuf>,
    app_id: String,
    launch: bool,
    gtk_notifications: bool,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
            env: vec![
                ("LC_ALL".into(), "C.UTF-8".into()),
                // Input::type_text enters non-ASCII text through the simple context's
                // Ctrl+Shift+U hex input; the default Wayland context ignores it.
                ("GTK_IM_MODULE".into(), "gtk-im-context-simple".into()),
            ],
            app_bin: None,
            app_id: TICK_APP_ID.into(),
            launch: true,
            gtk_notifications: true,
        }
    }
}

impl SessionBuilder {
    /// Size of mutter's virtual monitor (default 1280x800).
    pub fn monitor(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// A 600x800 virtual monitor (narrow layout).
    pub fn narrow(self) -> Self {
        self.monitor(600, 800)
    }

    /// Sets an environment variable for the app (replaces an earlier value).
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key = key.into();
        self.env.retain(|(k, _)| *k != key);
        self.env.push((key, value.into()));
        self
    }

    /// Forces the light or dark style.
    pub fn color_scheme(self, scheme: ColorScheme) -> Self {
        let value = match scheme {
            ColorScheme::Light => "light",
            ColorScheme::Dark => "dark",
        };
        self.env("TICK_COLOR_SCHEME", value)
    }

    /// Points the app at an API server (a fake server in tests): sets `TICK_API_BASE`
    /// and `TICK_TOKEN`.
    pub fn api(self, base_url: impl Into<String>, token: impl Into<String>) -> Self {
        self.env("TICK_API_BASE", base_url).env("TICK_TOKEN", token)
    }

    /// Fixes the app's notion of "now" (RFC 3339, `TICK_NOW`).
    pub fn now(self, rfc3339: impl Into<String>) -> Self {
        self.env("TICK_NOW", rfc3339)
    }

    /// Runs the app in this locale (default `C.UTF-8`, so GTK's own accessible names
    /// such as "Close" are English).
    pub fn locale(self, locale: impl Into<String>) -> Self {
        let locale = locale.into();
        self.env("LANG", locale.clone()).env("LC_ALL", locale)
    }

    /// Runs another GTK binary instead of `tick` (harness self-tests).
    pub fn app_binary(mut self, bin: impl Into<PathBuf>, app_id: impl Into<String>) -> Self {
        self.app_bin = Some(bin.into());
        self.app_id = app_id.into();
        self
    }

    /// Starts the compositor but not the app; call [`Session::launch_app`] later.
    pub fn no_launch(mut self) -> Self {
        self.launch = false;
        self
    }

    /// Owns only `org.freedesktop.Notifications`, not `org.gtk.Notifications`, so GLib
    /// falls back to its freedesktop backend.
    pub fn freedesktop_notifications_only(mut self) -> Self {
        self.gtk_notifications = false;
        self
    }

    /// Starts the session. Waits for a free `TICK_E2E_JOBS` slot first.
    pub async fn start(self) -> Result<Session> {
        Session::start(self).await
    }
}

/// Child processes and temp dirs; killed and removed on drop.
struct Procs {
    dir: Option<tempfile::TempDir>,
    root: PathBuf,
    pgid: Option<i32>,
    children: Vec<Child>,
    app: Option<Child>,
}

impl Procs {
    fn spawn(&mut self, mut cmd: Command, log: &str) -> Result<()> {
        let program = cmd.get_program().to_string_lossy().into_owned();
        let out = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join(log))?;
        cmd.stdin(Stdio::null())
            .stdout(out.try_clone()?)
            .stderr(out)
            .process_group(self.pgid.unwrap_or(0));
        let child = cmd.spawn().with_context(|| format!("spawning {program}"))?;
        if self.pgid.is_none() {
            self.pgid = i32::try_from(child.id()).ok();
        }
        self.children.push(child);
        Ok(())
    }

    fn log_tail(&self, log: &str, lines: usize) -> String {
        let text = std::fs::read_to_string(self.root.join(log)).unwrap_or_default();
        let all: Vec<&str> = text.lines().collect();
        all[all.len().saturating_sub(lines)..].join("\n")
    }

    fn check_alive(&mut self) -> Result<()> {
        for child in &mut self.children {
            if let Some(status) = child.try_wait()? {
                bail!("session process {} exited early: {status}", child.id());
            }
        }
        Ok(())
    }

    /// Lazily unmounts FUSE mounts below the session dir (gvfsd-fuse, the document
    /// portal) left behind by killed daemons; otherwise the temp dir cannot be removed.
    fn unmount_fuse(&self) {
        let Ok(mountinfo) = std::fs::read_to_string("/proc/self/mountinfo") else {
            return;
        };
        let root = self.root.to_string_lossy();
        let mut mounts: Vec<&str> = mountinfo
            .lines()
            .filter_map(|line| line.split(' ').nth(4))
            .filter(|m| m.starts_with(root.as_ref()))
            .collect();
        mounts.sort_by_key(|m| std::cmp::Reverse(m.len()));
        for mount in mounts {
            let unmounted = ["fusermount3", "fusermount"].iter().any(|bin| {
                Command::new(bin)
                    .args(["-u", "-z", mount])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_ok_and(|s| s.success())
            });
            if !unmounted {
                eprintln!("tick-e2e: could not unmount {mount}");
            }
        }
    }
}

impl Drop for Procs {
    fn drop(&mut self) {
        if let Some(app) = &mut self.app {
            let _ = app.kill();
            let _ = app.wait();
        }
        for child in self.children.iter_mut().rev() {
            let _ = child.kill();
        }
        if let Some(pgid) = self.pgid {
            // D-Bus-activated services (at-spi, portals) are not our direct children but
            // share the process group.
            // SAFETY: kill(2) has no memory-safety preconditions.
            unsafe {
                libc::kill(-pgid, libc::SIGKILL);
            }
        }
        for child in &mut self.children {
            let _ = child.wait();
        }
        self.unmount_fuse();
        let keep = std::env::var_os("TICK_E2E_KEEP").is_some();
        if (std::thread::panicking() || keep)
            && let Some(dir) = self.dir.take()
        {
            let root = dir.keep();
            eprintln!("tick-e2e: kept session dir {}", root.display());
            for log in ["app.log", "mutter.log", "dbus.log"] {
                eprintln!("tick-e2e: {}", root.join(log).display());
            }
            if std::thread::panicking() {
                for log in ["mutter.log", "app.log"] {
                    eprintln!("---- tail of {log} ----\n{}", self.log_tail(log, 40));
                }
            }
        }
    }
}

/// A running E2E session. Drop kills all processes and removes the temp dirs (kept and
/// printed when the test panics, or always with `TICK_E2E_KEEP=1`).
pub struct Session {
    // Field order matters for drop: connections close before the processes die.
    a11y: OnceCell<A11y>,
    input: OnceCell<Input>,
    notifications: Notifications,
    bus: zbus::Connection,
    procs: Procs,
    _slot: OwnedSemaphorePermit,
    base_env: Vec<(String, String)>,
    app_env: Vec<(String, String)>,
    app_bin: PathBuf,
    app_id: String,
    app_log_offset: u64,
    width: u32,
    height: u32,
}

impl Session {
    /// Options for a new session.
    pub fn builder() -> SessionBuilder {
        SessionBuilder::default()
    }

    /// Starts a default session (1280x800, the `tick` binary).
    pub async fn start_default() -> Result<Session> {
        Self::builder().start().await
    }

    async fn start(opts: SessionBuilder) -> Result<Session> {
        let slot = crate::job_slots().acquire_owned().await?;
        let app_bin = match opts.app_bin.clone() {
            Some(bin) => bin,
            None => crate::tick_bin()?,
        };

        let dir = tempfile::Builder::new().prefix("tick-e2e-").tempdir()?;
        let root = dir.path().to_path_buf();
        let runtime = root.join("runtime");
        let home = root.join("home");
        let dirs = [
            ("XDG_CONFIG_HOME", home.join(".config")),
            ("XDG_DATA_HOME", home.join(".local/share")),
            ("XDG_CACHE_HOME", home.join(".cache")),
            ("XDG_STATE_HOME", home.join(".local/state")),
        ];
        std::fs::create_dir_all(&runtime)?;
        std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o700))?;
        for (_, d) in &dirs {
            std::fs::create_dir_all(d)?;
        }

        let bus_socket = runtime.join("bus");
        let bus_address = format!("unix:path={}", bus_socket.display());
        let mut base_env: Vec<(String, String)> = PASSTHROUGH_ENV
            .iter()
            .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v)))
            .collect();
        base_env.extend([
            ("HOME".into(), path_str(&home)),
            ("XDG_RUNTIME_DIR".into(), path_str(&runtime)),
            ("DBUS_SESSION_BUS_ADDRESS".into(), bus_address.clone()),
            ("LANG".into(), "C.UTF-8".into()),
            ("LC_ALL".into(), "C.UTF-8".into()),
            // No dconf: settings stay in memory, defaults only.
            ("GSETTINGS_BACKEND".into(), "memory".into()),
            // No gvfs: GIO would D-Bus-activate gvfsd, which FUSE-mounts into the
            // runtime dir and outlives the session.
            ("GIO_USE_VFS".into(), "local".into()),
            ("GIO_USE_VOLUME_MONITOR".into(), "unix".into()),
            ("GVFS_DISABLE_FUSE".into(), "1".into()),
        ]);
        base_env.extend(dirs.iter().map(|(k, d)| (k.to_string(), path_str(d))));

        let mut procs = Procs {
            dir: Some(dir),
            root: root.clone(),
            pgid: None,
            children: Vec::new(),
            app: None,
        };

        let started = async {
            let mut dbus = Command::new("dbus-daemon");
            dbus.args(["--session", "--nofork", "--nopidfile"])
                .arg(format!("--address={bus_address}"))
                .env_clear()
                .envs(base_env.iter().cloned());
            procs.spawn(dbus, "dbus.log")?;
            wait_for_path(&mut procs, &bus_socket, "D-Bus socket").await?;

            let bus = zbus::connection::Builder::address(bus_address.as_str())?
                .method_timeout(METHOD_TIMEOUT)
                .build()
                .await
                .context("connecting to the private session bus")?;
            // Own the notification names before anything can activate a real server.
            let notifications = Notifications::install(&bus, opts.gtk_notifications).await?;

            let mut mutter = Command::new("mutter");
            mutter
                .args(["--headless", "--wayland", "--no-x11", "--virtual-monitor"])
                .arg(format!("{}x{}", opts.width, opts.height))
                .args(["--wayland-display", WAYLAND_DISPLAY])
                .env_clear()
                .envs(base_env.iter().cloned());
            procs.spawn(mutter, "mutter.log")?;
            wait_for_path(&mut procs, &runtime.join(WAYLAND_DISPLAY), "Wayland socket").await?;
            let dbus_proxy = zbus::fdo::DBusProxy::new(&bus).await?;
            poll(STARTUP_TIMEOUT, "org.gnome.Mutter.RemoteDesktop", || {
                let dbus_proxy = &dbus_proxy;
                async move {
                    let name = "org.gnome.Mutter.RemoteDesktop".try_into().ok()?;
                    dbus_proxy.name_has_owner(name).await.ok().filter(|o| *o)
                }
            })
            .await?;
            anyhow::Ok((bus, notifications))
        }
        .await;
        let (bus, notifications) = match started {
            Ok(v) => v,
            Err(err) => {
                let mutter = procs.log_tail("mutter.log", 30);
                let dbus = procs.log_tail("dbus.log", 30);
                return Err(err.context(format!(
                    "session startup failed\n---- mutter.log ----\n{mutter}\n---- dbus.log ----\n{dbus}"
                )));
            }
        };

        let mut session = Session {
            a11y: OnceCell::new(),
            input: OnceCell::new(),
            notifications,
            bus,
            procs,
            _slot: slot,
            base_env,
            app_env: opts.env,
            app_bin,
            app_id: opts.app_id,
            app_log_offset: 0,
            width: opts.width,
            height: opts.height,
        };
        if opts.launch {
            session.launch_app()?;
        }
        Ok(session)
    }

    /// Starts the app (again). Fails if it is already running. HOME and the cache persist
    /// across launches within a session.
    pub fn launch_app(&mut self) -> Result<()> {
        if self.app_running() {
            bail!("the app is already running");
        }
        let log_path = self.procs.root.join("app.log");
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        self.app_log_offset = log.metadata()?.len();
        let mut cmd = Command::new(&self.app_bin);
        cmd.env_clear()
            .envs(self.base_env.iter().cloned())
            .env("WAYLAND_DISPLAY", WAYLAND_DISPLAY)
            .env("GDK_BACKEND", "wayland")
            .env("TICK_TEST_MODE", "1")
            .envs(self.app_env.iter().cloned())
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log.try_clone()?)
            .process_group(self.procs.pgid.unwrap_or(0));
        let child = cmd
            .spawn()
            .with_context(|| format!("spawning {}", self.app_bin.display()))?;
        writeln!(log, "tick-e2e: launched pid {}", child.id())?;
        self.procs.app = Some(child);
        Ok(())
    }

    /// True while the app process runs.
    pub fn app_running(&mut self) -> bool {
        match &mut self.procs.app {
            Some(app) => matches!(app.try_wait(), Ok(None)),
            None => false,
        }
    }

    /// PID of the current app process.
    pub fn app_pid(&self) -> Option<u32> {
        self.procs.app.as_ref().map(Child::id)
    }

    /// Waits until the app exits and returns its status.
    pub async fn wait_app_exit(&mut self, timeout: Duration) -> Result<ExitStatus> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let app = self
                .procs
                .app
                .as_mut()
                .context("the app was not launched")?;
            if let Some(status) = app.try_wait()? {
                return Ok(status);
            }
            if tokio::time::Instant::now() >= deadline {
                bail!("the app did not exit within {timeout:?}");
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Kills the app (SIGKILL) and waits for it.
    pub fn kill_app(&mut self) -> Result<()> {
        if let Some(mut app) = self.procs.app.take() {
            let _ = app.kill();
            app.wait()?;
        }
        Ok(())
    }

    /// Output (stdout + stderr) of the current app launch.
    pub fn app_log(&self) -> String {
        let text = std::fs::read(self.procs.root.join("app.log")).unwrap_or_default();
        let start = usize::try_from(self.app_log_offset)
            .unwrap_or(0)
            .min(text.len());
        String::from_utf8_lossy(&text[start..]).into_owned()
    }

    /// Waits for a line `<key>=<value>` in the current launch's output and returns the value.
    pub async fn wait_for_log_value(&self, key: &str, timeout: Duration) -> Result<String> {
        let prefix = format!("{key}=");
        poll(timeout, key, || {
            let log = self.app_log();
            let value = log
                .lines()
                .find_map(|l| l.strip_prefix(&prefix).map(|v| v.trim().to_string()));
            async move { value }
        })
        .await
    }

    /// `TICK_FIRST_DATA_FRAME_MS` of the current launch.
    pub async fn first_frame_ms(&self, timeout: Duration) -> Result<u64> {
        let value = self
            .wait_for_log_value("TICK_FIRST_DATA_FRAME_MS", timeout)
            .await?;
        Ok(value.parse()?)
    }

    /// The private session bus.
    pub fn bus(&self) -> &zbus::Connection {
        &self.bus
    }

    /// The session's `DBUS_SESSION_BUS_ADDRESS`.
    pub fn bus_address(&self) -> &str {
        self.base_env
            .iter()
            .find(|(k, _)| k == "DBUS_SESSION_BUS_ADDRESS")
            .map(|(_, v)| v.as_str())
            .unwrap_or_default()
    }

    /// Root of the session's temp dir (logs live here).
    pub fn root(&self) -> &Path {
        &self.procs.root
    }

    /// The session's `HOME`.
    pub fn home(&self) -> PathBuf {
        self.procs.root.join("home")
    }

    /// The session's `XDG_DATA_HOME` (the app's cache lives below it).
    pub fn data_home(&self) -> PathBuf {
        self.home().join(".local/share")
    }

    /// Virtual monitor size.
    pub fn monitor_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The fake notification servers.
    pub fn notifications(&self) -> &Notifications {
        &self.notifications
    }

    /// AT-SPI access (connects to the accessibility bus on first use).
    pub async fn a11y(&self) -> Result<&A11y> {
        self.a11y
            .get_or_try_init(|| A11y::connect(&self.bus, METHOD_TIMEOUT))
            .await
    }

    /// The app's AT-SPI root, waiting until the current process registers.
    pub async fn app(&self) -> Result<Node> {
        let pid = self.app_pid().context("the app was not launched")?;
        self.a11y().await?.wait_for_app(pid, STARTUP_TIMEOUT).await
    }

    /// The app's first window (role `Frame`), waiting until it appears.
    pub async fn window(&self) -> Result<Node> {
        self.app()
            .await?
            .wait_for(&Query::role(Role::Frame), STARTUP_TIMEOUT)
            .await
    }

    /// Real input through mutter's RemoteDesktop (created and warmed up on first use).
    pub async fn input(&self) -> Result<&Input> {
        self.input
            .get_or_try_init(|| Input::connect(&self.bus))
            .await
    }

    /// Clicks the center of `node`. The window is maximized, so window coordinates are
    /// screen coordinates.
    pub async fn click(&self, node: &Node) -> Result<()> {
        let rect = node.extents().await?;
        let (x, y) = rect.center();
        self.input().await?.click(x, y).await
    }

    /// Presses and releases one key.
    pub async fn key(&self, keysym: u32) -> Result<()> {
        self.input().await?.key(keysym).await
    }

    /// Presses keys in order and releases them in reverse (e.g. Ctrl+Q).
    pub async fn chord(&self, keysyms: &[u32]) -> Result<()> {
        self.input().await?.chord(keysyms).await
    }

    /// Types text with real key events.
    pub async fn type_text(&self, text: &str) -> Result<()> {
        self.input().await?.type_text(text).await
    }

    /// Renders the app's main window to a PNG at `path` through `app.test-screenshot`.
    pub async fn screenshot(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let path = std::path::absolute(path.as_ref())?;
        let _ = std::fs::remove_file(&path);
        self.activate_action(
            "test-screenshot",
            Some(zbus::zvariant::Value::from(path_str(&path))),
        )
        .await?;
        let p = &path;
        poll(Duration::from_secs(15), "screenshot file", || async move {
            p.exists().then_some(())
        })
        .await?;
        Ok(path)
    }

    /// Activates an application action (`app.<name>`) through `org.gtk.Actions`.
    pub async fn activate_action(
        &self,
        name: &str,
        parameter: Option<zbus::zvariant::Value<'_>>,
    ) -> Result<()> {
        let path = format!("/{}", self.app_id.replace('.', "/"));
        let params: Vec<zbus::zvariant::Value<'_>> = parameter.into_iter().collect();
        let platform: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
        self.bus
            .call_method(
                Some(self.app_id.as_str()),
                path.as_str(),
                Some("org.gtk.Actions"),
                "Activate",
                &(name, params, platform),
            )
            .await
            .with_context(|| format!("activating app.{name}"))?;
        Ok(())
    }
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

async fn wait_for_path(procs: &mut Procs, path: &Path, what: &str) -> Result<()> {
    let deadline = tokio::time::Instant::now() + STARTUP_TIMEOUT;
    while !path.exists() {
        procs.check_alive()?;
        if tokio::time::Instant::now() >= deadline {
            bail!("timed out waiting for {what} at {}", path.display());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}
