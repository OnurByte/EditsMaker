// Embeds every presets/*.json in the binary, so presets stay plain files anyone can read and change.
use std::fmt::Write;

fn main() {
    println!("cargo:rerun-if-changed=presets");
    let mut files: Vec<String> = std::fs::read_dir("presets").map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".json")).collect()).unwrap_or_default();
    files.sort();
    let mut s = String::from("pub const BUILT_IN: &[(&str, &str)] = &[\n");
    for f in &files {
        writeln!(s, "    ({f:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/presets/{f}\"))),").unwrap();
    }
    s.push_str("];\n");
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("built_in.rs"), s).unwrap();
}
