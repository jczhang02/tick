//! AT-SPI access: find widgets by role and accessible name, read their state, act on them.

use std::time::Duration;

use anyhow::Context as _;
use atspi::proxy::accessible::AccessibleProxy;
use atspi::proxy::action::ActionProxy;
use atspi::proxy::component::ComponentProxy;
use atspi::proxy::editable_text::EditableTextProxy;
use atspi::proxy::text::TextProxy;
use atspi::{CoordType, ObjectRefOwned, Role, State, StateSet};
use zbus::names::OwnedBusName;
use zbus::zvariant::OwnedObjectPath;

use crate::{Result, poll};

const REGISTRY: &str = "org.a11y.atspi.Registry";
const ROOT_PATH: &str = "/org/a11y/atspi/accessible/root";
/// Guard against cycles or runaway trees during a search.
const MAX_NODES: usize = 20_000;

/// Connection to the session's accessibility bus.
pub struct A11y {
    conn: zbus::Connection,
}

impl A11y {
    /// Asks `org.a11y.Bus` on the session bus for the accessibility bus address
    /// (D-Bus-activates at-spi-bus-launcher) and connects to it.
    pub async fn connect(session_bus: &zbus::Connection, timeout: Duration) -> Result<Self> {
        let reply = session_bus
            .call_method(
                Some("org.a11y.Bus"),
                "/org/a11y/bus",
                Some("org.a11y.Bus"),
                "GetAddress",
                &(),
            )
            .await
            .context("org.a11y.Bus.GetAddress")?;
        let address: String = reply.body().deserialize()?;
        let conn = zbus::connection::Builder::address(address.as_str())?
            .method_timeout(timeout)
            .build()
            .await
            .context("connecting to the accessibility bus")?;
        Ok(Self { conn })
    }

    /// The raw accessibility bus connection.
    pub fn connection(&self) -> &zbus::Connection {
        &self.conn
    }

    /// The desktop root (its children are the registered applications).
    pub fn desktop(&self) -> Node {
        Node::new(
            self.conn.clone(),
            OwnedBusName::try_from(REGISTRY).expect("valid name"),
            OwnedObjectPath::try_from(ROOT_PATH).expect("valid path"),
        )
    }

    /// Waits until the process `pid` registers an application on AT-SPI.
    ///
    /// Matching by PID (not by name) makes relaunches unambiguous.
    pub async fn wait_for_app(&self, pid: u32, timeout: Duration) -> Result<Node> {
        let dbus = zbus::fdo::DBusProxy::new(&self.conn).await?;
        let desktop = self.desktop();
        poll(timeout, &format!("app pid {pid} on AT-SPI"), || {
            let dbus = &dbus;
            let desktop = &desktop;
            async move {
                for app in desktop.children().await.ok()? {
                    let owner = app.dest.as_ref().clone();
                    if dbus.get_connection_unix_process_id(owner).await.ok() == Some(pid) {
                        return Some(app);
                    }
                }
                None
            }
        })
        .await
    }

    /// Waits for an application by its AT-SPI name (the binary name for GTK apps).
    pub async fn wait_for_app_named(&self, name: &str, timeout: Duration) -> Result<Node> {
        let desktop = self.desktop();
        let query = Query::default().name(name);
        poll(timeout, &format!("app {name:?} on AT-SPI"), || {
            let desktop = &desktop;
            let query = &query;
            async move {
                for app in desktop.children().await.ok()? {
                    if query.matches(&app).await {
                        return Some(app);
                    }
                }
                None
            }
        })
        .await
    }
}

/// How to match an accessible name.
#[derive(Clone, Debug, Default)]
enum NameMatch {
    #[default]
    Any,
    Exact(String),
    Contains(String),
}

/// A search for an accessible object by role, name and states.
#[derive(Clone, Debug, Default)]
pub struct Query {
    role: Option<Role>,
    name: NameMatch,
    states: Vec<State>,
}

impl Query {
    /// Matches objects with this role.
    pub fn role(role: Role) -> Self {
        Self {
            role: Some(role),
            ..Self::default()
        }
    }

    /// Requires this exact accessible name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = NameMatch::Exact(name.into());
        self
    }

    /// Requires the accessible name to contain this text.
    pub fn name_contains(mut self, part: impl Into<String>) -> Self {
        self.name = NameMatch::Contains(part.into());
        self
    }

    /// Requires this state (e.g. [`State::Showing`]).
    pub fn state(mut self, state: State) -> Self {
        self.states.push(state);
        self
    }

    /// Requires the object to be visible on screen (`showing`).
    pub fn showing(self) -> Self {
        self.state(State::Showing)
    }

    async fn matches(&self, node: &Node) -> bool {
        if let Some(role) = self.role
            && node.role().await.ok() != Some(role)
        {
            return false;
        }
        match &self.name {
            NameMatch::Any => {}
            NameMatch::Exact(want) => {
                if node.name().await.ok().as_deref() != Some(want.as_str()) {
                    return false;
                }
            }
            NameMatch::Contains(part) => {
                if !node.name().await.is_ok_and(|n| n.contains(part.as_str())) {
                    return false;
                }
            }
        }
        if !self.states.is_empty() {
            let Ok(states) = node.states().await else {
                return false;
            };
            if !self.states.iter().all(|s| states.contains(*s)) {
                return false;
            }
        }
        true
    }
}

/// Bounding box of an object (window coordinates).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// The center point.
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }
}

/// One accessible object on the accessibility bus.
#[derive(Clone, Debug)]
pub struct Node {
    conn: zbus::Connection,
    dest: OwnedBusName,
    path: OwnedObjectPath,
}

impl Node {
    fn new(conn: zbus::Connection, dest: OwnedBusName, path: OwnedObjectPath) -> Self {
        Self { conn, dest, path }
    }

    fn child_from_ref(&self, r: &ObjectRefOwned) -> Option<Self> {
        if r.is_null() {
            return None;
        }
        let dest = OwnedBusName::try_from(r.name_as_str()?).ok()?;
        let path = OwnedObjectPath::try_from(r.path_as_str()).ok()?;
        Some(Self::new(self.conn.clone(), dest, path))
    }

    /// D-Bus object path (useful in failure messages).
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    async fn accessible(&self) -> Result<AccessibleProxy<'static>> {
        Ok(AccessibleProxy::builder(&self.conn)
            .destination(self.dest.clone())?
            .path(self.path.clone())?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?)
    }

    /// Accessible name.
    pub async fn name(&self) -> Result<String> {
        Ok(self.accessible().await?.name().await?)
    }

    /// Accessible description.
    pub async fn description(&self) -> Result<String> {
        Ok(self.accessible().await?.description().await?)
    }

    /// The accessible parent, if any.
    pub async fn parent(&self) -> Result<Option<Node>> {
        let r = self.accessible().await?.parent().await?;
        Ok(self.child_from_ref(&r))
    }

    /// Accessible role.
    pub async fn role(&self) -> Result<Role> {
        Ok(self.accessible().await?.get_role().await?)
    }

    /// Current states.
    pub async fn states(&self) -> Result<StateSet> {
        Ok(self.accessible().await?.get_state().await?)
    }

    /// True if the object currently has `state`.
    pub async fn has_state(&self, state: State) -> Result<bool> {
        Ok(self.states().await?.contains(state))
    }

    /// Direct children.
    pub async fn children(&self) -> Result<Vec<Node>> {
        let refs = self.accessible().await?.get_children().await?;
        Ok(refs.iter().filter_map(|r| self.child_from_ref(r)).collect())
    }

    /// Text content (Text interface), e.g. of an entry or label.
    pub async fn text(&self) -> Result<String> {
        let text = TextProxy::builder(&self.conn)
            .destination(self.dest.clone())?
            .path(self.path.clone())?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?;
        Ok(text.get_text(0, -1).await?)
    }

    /// Extents relative to the window (`CoordType::Window`, reliable in GTK4).
    pub async fn extents(&self) -> Result<Rect> {
        let component = ComponentProxy::builder(&self.conn)
            .destination(self.dest.clone())?
            .path(self.path.clone())?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?;
        let (x, y, width, height) = component.get_extents(CoordType::Window).await?;
        Ok(Rect {
            x,
            y,
            width,
            height,
        })
    }

    /// Runs an accessible action: the one named `name` (case-insensitive; GTK4 buttons
    /// have "Click"), or the first one.
    /// A fallback for when real input is not the point of the test.
    pub async fn do_action(&self, name: Option<&str>) -> Result<()> {
        let action = ActionProxy::builder(&self.conn)
            .destination(self.dest.clone())?
            .path(self.path.clone())?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?;
        let index = match name {
            None => 0,
            Some(name) => {
                let actions = action.get_actions().await?;
                let i = actions
                    .iter()
                    .position(|a| a.name.eq_ignore_ascii_case(name))
                    .with_context(|| format!("no action {name:?} in {actions:?}"))?;
                i32::try_from(i)?
            }
        };
        anyhow::ensure!(action.do_action(index).await?, "DoAction({index}) failed");
        Ok(())
    }

    /// Replaces the text of an editable object (EditableText interface). A fallback for
    /// text that real key events cannot produce.
    pub async fn set_text(&self, text: &str) -> Result<()> {
        let editable = self.editable_text().await?;
        anyhow::ensure!(
            editable.set_text_contents(text).await?,
            "SetTextContents failed"
        );
        Ok(())
    }

    /// Inserts text at `position` (character offset) of an editable object.
    pub async fn insert_text(&self, position: i32, text: &str) -> Result<()> {
        let editable = self.editable_text().await?;
        let length = i32::try_from(text.chars().count())?;
        anyhow::ensure!(
            editable.insert_text(position, text, length).await?,
            "InsertText failed"
        );
        Ok(())
    }

    async fn editable_text(&self) -> Result<EditableTextProxy<'static>> {
        Ok(EditableTextProxy::builder(&self.conn)
            .destination(self.dest.clone())?
            .path(self.path.clone())?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?)
    }

    /// All descendants matching `query`, depth-first in tree order. Objects that vanish
    /// during the walk are skipped.
    pub async fn find_all(&self, query: &Query) -> Vec<Node> {
        let mut found = Vec::new();
        let mut stack = vec![self.clone()];
        let mut visited = 0;
        while let Some(node) = stack.pop() {
            visited += 1;
            if visited > MAX_NODES {
                break;
            }
            if visited > 1 && query.matches(&node).await {
                found.push(node.clone());
            }
            if let Ok(children) = node.children().await {
                stack.extend(children.into_iter().rev());
            }
        }
        found
    }

    /// The first descendant matching `query`, if any.
    pub async fn find(&self, query: &Query) -> Option<Node> {
        let mut stack = vec![self.clone()];
        let mut visited = 0;
        while let Some(node) = stack.pop() {
            visited += 1;
            if visited > MAX_NODES {
                break;
            }
            if visited > 1 && query.matches(&node).await {
                return Some(node);
            }
            if let Ok(children) = node.children().await {
                stack.extend(children.into_iter().rev());
            }
        }
        None
    }

    /// Polls until a descendant matches `query`. The error includes a dump of the tree.
    pub async fn wait_for(&self, query: &Query, timeout: Duration) -> Result<Node> {
        let result = poll(timeout, &format!("{query:?}"), || self.find(query)).await;
        match result {
            Ok(node) => Ok(node),
            Err(err) => Err(err.context(format!("accessible tree:\n{}", self.dump().await))),
        }
    }

    /// Polls until no descendant matches `query`.
    pub async fn wait_gone(&self, query: &Query, timeout: Duration) -> Result<()> {
        poll(timeout, &format!("{query:?} to disappear"), || async {
            self.find(query).await.is_none().then_some(())
        })
        .await
    }

    /// Indented `role "name" [states]` lines of this subtree, for failure messages.
    pub async fn dump(&self) -> String {
        let mut out = String::new();
        let mut stack = vec![(self.clone(), 0usize)];
        let mut visited = 0;
        while let Some((node, depth)) = stack.pop() {
            visited += 1;
            if visited > 2_000 {
                out.push_str("...\n");
                break;
            }
            let role = node
                .role()
                .await
                .map(|r| format!("{r:?}"))
                .unwrap_or_else(|_| "?".into());
            let name = node.name().await.unwrap_or_default();
            let states: Vec<String> = node
                .states()
                .await
                .map(|s| {
                    s.iter()
                        .map(|st| format!("{st:?}").to_lowercase())
                        .collect()
                })
                .unwrap_or_default();
            out.push_str(&format!(
                "{}{role} {name:?} [{}]\n",
                "  ".repeat(depth),
                states.join(" ")
            ));
            if let Ok(children) = node.children().await {
                stack.extend(children.into_iter().rev().map(|c| (c, depth + 1)));
            }
        }
        out
    }
}
