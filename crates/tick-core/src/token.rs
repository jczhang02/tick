//! API token storage: the GNOME keyring (Secret Service, via `oo7`), with a `TICK_TOKEN`
//! environment override for tests and CI.

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
}

/// Token from `TICK_TOKEN`, when set and non-empty.
pub fn env_token() -> Option<String> {
    std::env::var("TICK_TOKEN")
        .ok()
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
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

/// Remove the token from the keyring (logout).
pub async fn delete() -> Result<(), TokenError> {
    let keyring = oo7::Keyring::new().await?;
    keyring.delete(&attributes()).await?;
    Ok(())
}
