fn main() {
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rustc-cfg=tauri_build");
    tauri_build::build();
}
