fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let obj_path = std::path::Path::new(&dir).join("icon.obj");
    if obj_path.exists() {
        println!("cargo:rerun-if-changed=icon.obj");
        println!("cargo:rustc-link-arg={}", obj_path.display());
    }
}
