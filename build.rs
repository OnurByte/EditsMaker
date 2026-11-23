// Embeds the data files (data/*.json) and every pack in packs/ (its manifest.json, preset.json, SKILL.md and
// automation.js, scene.js, presets/) in
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
    s.push_str("];\n\n/// (folder, manifest.json, preset.json, SKILL.md, automation.js, scene.js, presets/*.json, skills/*.md, sites/*.js as (site, script), preview.gif)\npub type BuiltIn = (&'static str, &'static str, Option<&'static str>, Option<&'static str>, Option<&'static str>, Option<&'static str>, &'static [&'static str], &'static [&'static str], &'static [(&'static str, &'static str)], Option<&'static [u8]>);\npub const BUILT_IN_PACKS: &[BuiltIn] = &[\n");
    for d in list("packs").iter().filter(|d| std::path::Path::new("packs").join(d).join("manifest.json").is_file()) {
        let opt = |f: &str| if std::path::Path::new("packs").join(d).join(f).is_file() { format!("Some({})", file(format!("packs/{d}/{f}"))) } else { "None".into() };
        // a family of presets: presets/NAME.json, each found by its own name
        let family: Vec<String> = list(&format!("packs/{d}/presets")).into_iter().filter(|f| f.ends_with(".json")).map(|f| file(format!("packs/{d}/presets/{f}"))).collect();
        // more agent skills: skills/NAME.md, each its own skill
        let skills: Vec<String> = list(&format!("packs/{d}/skills")).into_iter().filter(|f| f.ends_with(".md")).map(|f| file(format!("packs/{d}/skills/{f}"))).collect();
        // more sites an automation posts to: sites/SITE.js
        let sites: Vec<String> = list(&format!("packs/{d}/sites")).into_iter().filter(|f| f.ends_with(".js")).map(|f| format!("({:?}, {})", f.trim_end_matches(".js"), file(format!("packs/{d}/sites/{f}")))).collect();
        // a moving cover, for the marketplace's card
        let gif = if std::path::Path::new("packs").join(d).join("preview.gif").is_file() { format!("Some(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/packs/{d}/preview.gif\")))") } else { "None".into() };
        writeln!(s, "    ({d:?}, {}, {}, {}, {}, {}, &[{}], &[{}], &[{}], {gif}),", file(format!("packs/{d}/manifest.json")), opt("preset.json"), opt("SKILL.md"), opt("automation.js"), opt("scene.js"), family.join(", "), skills.join(", "), sites.join(", ")).unwrap();
    }
    s.push_str("];\n");
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("built_in.rs"), s).unwrap();
}
