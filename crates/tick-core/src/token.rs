//! API token storage: the GNOME keyring (Secret Service, via `oo7`), with a `TICK_TOKEN`
//! environment override for tests and CI.
//!
//! Keyring calls can block for a long time (D-Bus, an unlock prompt), so none of these
//! functions may run on the GTK main thread; see [`load_blocking`].

use std::collections::HashMap;

/// Attributes that identify Tick's keyring item.
pub fn attributes() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("application", "dev.chengrui.Tick"),
        ("service", "todoist-api-token"),
    ])
}

const LABEL: &str = "Tick: Todoist API token";

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("keyring error: {0}")]
    Keyring(#[from] oo7::Error),
    #[error("the stored token is not valid UTF-8")]
    Encoding,
    #[error("cannot start the keyring runtime: {0}")]
    Runtime(std::io::Error),
}

/// Token from `TICK_TOKEN`, when set and non-empty.
pub fn env_token() -> Option<String> {
    clean(std::env::var("TICK_TOKEN").ok())
}

fn clean(t: Option<String>) -> Option<String> {
    t.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty())
}

/// Load the token: `TICK_TOKEN` first, then the keyring.
pub async fn load() -> Result<Option<String>, TokenError> {
    if let Some(t) = env_token() {
        return Ok(Some(t));
    }
    load_from(&oo7::Keyring::new().await?).await
}

/// Load the token from a given keyring.
pub async fn load_from(keyring: &oo7::Keyring) -> Result<Option<String>, TokenError> {
    keyring.unlock().await?;
    let items = keyring.search_items(&attributes()).await?;
    let Some(item) = items.first() else {
        return Ok(None);
    };
    let secret = item.secret().await?;
    let token = std::str::from_utf8(&secret)
        .map_err(|_| TokenError::Encoding)?
        .trim()
        .to_owned();
    Ok(Some(token).filter(|t| !t.is_empty()))
}

/// Store (or replace) the token in the keyring.
pub async fn store(token: &str) -> Result<(), TokenError> {
    store_in(&oo7::Keyring::new().await?, token).await
}

pub async fn store_in(keyring: &oo7::Keyring, token: &str) -> Result<(), TokenError> {
    keyring.unlock().await?;
    keyring
        .create_item(LABEL, &attributes(), token.trim(), true)
        .await?;
    Ok(())
}

fn runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

/// Blocking [`load`] for a worker thread outside any tokio runtime. `oo7`'s D-Bus backend
/// needs tokio, so this runs a short-lived current-thread runtime.
///
/// Never call it on the GTK main thread: it makes D-Bus calls and `unlock()` may show a
/// keyring unlock prompt and wait for the user indefinitely, which would freeze the UI
/// (`docs/architecture.md`: the main thread never blocks). From GTK, run it with
/// `gio::spawn_blocking` (or a `std::thread`) and await the result with
/// `glib::spawn_future_local`. Do not call it from inside a tokio runtime either; there,
/// await [`load`] directly.
pub fn load_blocking() -> Result<Option<String>, TokenError> {
    if let Some(t) = env_token() {
        return Ok(Some(t));
    }
    runtime().map_err(TokenError::Runtime)?.block_on(load())
}

/// Blocking [`store`] for a worker thread; same rules as [`load_blocking`] (never on the GTK
/// main thread).
pub fn store_blocking(token: &str) -> Result<(), TokenError> {
    runtime()
        .map_err(TokenError::Runtime)?
        .block_on(store(token))
}

/// Blocking [`delete`] for a worker thread; same rules as [`load_blocking`] (never on the GTK
/// main thread).
pub fn delete_blocking() -> Result<(), TokenError> {
    runtime().map_err(TokenError::Runtime)?.block_on(delete())
}

/// Remove the token from the keyring (logout).
pub async fn delete() -> Result<(), TokenError> {
    let keyring = oo7::Keyring::new().await?;
    keyring.delete(&attributes()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_is_trimmed() {
        assert_eq!(clean(Some("  abc \n".into())).as_deref(), Some("abc"));
        assert_eq!(clean(Some("   ".into())), None);
        assert_eq!(clean(None), None);
    }

    /// The keyring code path against oo7's file backend (no Secret Service needed).
    #[tokio::test]
    async fn store_and_load_with_a_file_keyring() {
        let dir = tempfile::tempdir().unwrap();
        let keyring = oo7::Keyring::sandboxed_with_path(
            dir.path().join("test.keyring"),
            oo7::Secret::random().unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(load_from(&keyring).await.unwrap(), None);
        store_in(&keyring, " first-token ").await.unwrap();
        assert_eq!(
            load_from(&keyring).await.unwrap().as_deref(),
            Some("first-token")
        );
        store_in(&keyring, "second-token").await.unwrap();
        assert_eq!(
            load_from(&keyring).await.unwrap().as_deref(),
            Some("second-token")
        );
        assert_eq!(keyring.search_items(&attributes()).await.unwrap().len(), 1);
        keyring.delete(&attributes()).await.unwrap();
        assert_eq!(load_from(&keyring).await.unwrap(), None);
    }
}
