fn main() {
    // generate_context! embeds the icons, but nothing tells Cargo they're
    // inputs, so a new icon wouldn't show until something else changed.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
