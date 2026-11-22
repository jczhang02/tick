//! Screenshots through `app.test-screenshot`. With `TICK_SCREENSHOTS=1` (`just
//! screenshots`) the PNGs are also kept in `target/screenshots/` for design review.

use std::path::{Path, PathBuf};

use tick_e2e::{ColorScheme, Session, SessionBuilder};

/// Takes a screenshot of the app's window and returns the PNG bytes.
async fn shoot(builder: SessionBuilder, name: &str) -> (PathBuf, Vec<u8>) {
    let s = builder.start().await.unwrap();
    s.window().await.unwrap();
    // Let the first frames settle (animations are off in test mode).
    s.first_frame_ms(crate::WAIT).await.unwrap();
    let path = if std::env::var_os("TICK_SCREENSHOTS").is_some() {
        tick_e2e::screenshot_dir()
            .unwrap()
            .join(format!("{name}.png"))
    } else {
        s.root().join(format!("{name}.png"))
    };
    let path = s.screenshot(&path).await.unwrap();
    let bytes = std::fs::read(&path).unwrap();
    (path, bytes)
}

fn png_size(path: &Path, png: &[u8]) -> (u32, u32) {
    assert!(
        png.len() > 24 && png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "{} is not a PNG",
        path.display()
    );
    let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_state_light_and_dark() {
    let light = Session::builder().color_scheme(ColorScheme::Light);
    let dark = Session::builder().color_scheme(ColorScheme::Dark);
    let (light_path, light) = shoot(light, "empty-wide-light").await;
    let (dark_path, dark) = shoot(dark, "empty-wide-dark").await;
    assert_eq!(png_size(&light_path, &light), (1280, 800));
    assert_eq!(png_size(&dark_path, &dark), (1280, 800));
    assert_ne!(light, dark, "light and dark screenshots are identical");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_state_narrow() {
    let narrow = Session::builder().narrow().color_scheme(ColorScheme::Light);
    let (path, png) = shoot(narrow, "empty-narrow-light").await;
    assert_eq!(png_size(&path, &png), (600, 800));
}
