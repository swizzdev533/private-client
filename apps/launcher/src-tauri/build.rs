use std::path::Path;

fn main() {
    // Cargo only reruns a build script when something it was told to watch
    // changes. The Windows icon resource is embedded here, but nothing declares
    // it as an input, so replacing the icon relinks the binary against the
    // previously generated, now stale, resource: the executable keeps the old
    // icon while every timestamp suggests it was rebuilt.
    println!("cargo:rerun-if-changed=tauri.conf.json");
    for icon in [
        "icons/icon.ico",
        "icons/icon.png",
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
    ] {
        if Path::new(icon).exists() {
            println!("cargo:rerun-if-changed={icon}");
        }
    }

    tauri_build::build();
}
