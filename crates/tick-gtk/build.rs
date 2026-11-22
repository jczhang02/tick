use std::{env, path::PathBuf, process::Command};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let ui_out = out_dir.join("ui");

    // Compile Blueprint files to GtkBuilder XML in OUT_DIR.
    let blueprints = [
        "data/ui/window.blp",
        "data/ui/login.blp",
        "data/ui/quick_add.blp",
    ];
    for blp in blueprints {
        println!("cargo:rerun-if-changed={blp}");
    }
    let status = Command::new("blueprint-compiler")
        .arg("batch-compile")
        .arg(&ui_out)
        .arg("data/ui")
        .args(blueprints)
        .status()
        .expect("blueprint-compiler not found; install blueprint-compiler");
    assert!(status.success(), "blueprint-compiler failed");

    println!("cargo:rerun-if-changed=data/resources.gresource.xml");
    println!("cargo:rerun-if-changed=data/style.css");
    println!("cargo:rerun-if-changed=data/icons");
    glib_build_tools::compile_resources(
        &[out_dir.to_str().unwrap(), "data"],
        "data/resources.gresource.xml",
        "resources.gresource",
    );
}
