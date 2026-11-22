// Embeds the data files (data/*.json) and every pack in packs/ (its manifest.json, preset.json, SKILL.md and
// automation.js) in
// the binary, so they stay plain files anyone can read and change.
use std::fmt::Write;

fn main() {
    println!("cargo:rerun-if-changed=data");
    println!("cargo:rerun-if-changed=packs");
    let list = |dir: &str| -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
        v.sort();
        v
    };
    let file = |path: String| format!("include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{path}\"))");
    let mut s = String::from("pub const DATA_FILES: &[(&str, &str)] = &[\n");
    for f in list("data").iter().filter(|f| f.ends_with(".json")) {
        writeln!(s, "    ({f:?}, {}),", file(format!("data/{f}"))).unwrap();
    }
    s.push_str("];\n\n/// (folder, manifest.json, preset.json, SKILL.md, automation.js)\npub type BuiltIn = (&'static str, &'static str, Option<&'static str>, Option<&'static str>, Option<&'static str>);\npub const BUILT_IN_PACKS: &[BuiltIn] = &[\n");
    for d in list("packs").iter().filter(|d| std::path::Path::new("packs").join(d).join("manifest.json").is_file()) {
        let opt = |f: &str| if std::path::Path::new("packs").join(d).join(f).is_file() { format!("Some({})", file(format!("packs/{d}/{f}"))) } else { "None".into() };
        writeln!(s, "    ({d:?}, {}, {}, {}, {}),", file(format!("packs/{d}/manifest.json")), opt("preset.json"), opt("SKILL.md"), opt("automation.js")).unwrap();
    }
    s.push_str("];\n");
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("built_in.rs"), s).unwrap();
}
