//! Colour corrections ("cc") from Alight Motion packs: stacks of adjustment layers, each with colour effects, an
//! opacity and a blending mode. A stack is kept as JSON (editable like a preset), worked out per colour in Rust
//! and baked into a 3D LUT that any edit can wear with `--look NAME`.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Fx {
    /// effect name as Alight calls it: exposure, brightcont2, satvib, highlightshadow, channelmap, colorize, ...
    pub id: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub p: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    #[serde(default = "one")]
    pub opacity: f64,
    /// normal, screen, multiply, overlay, add, softlight, lighten, darken
    #[serde(default = "normal", skip_serializing_if = "is_normal")]
    pub blend: String,
    /// the layer's own fill (#rrggbb); its `lift` effect says how much of it shows over the picture (none: it is
    /// an adjustment layer)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub fx: Vec<Fx>,
}

fn one() -> f64 {
    1.0
}
fn normal() -> String {
    "normal".into()
}
fn is_normal(s: &String) -> bool {
    s == "normal"
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Grade {
    pub name: String,
    #[serde(default)]
    pub about: String,
    pub layers: Vec<Layer>,
}

/// Effects that work on neighbours or over time (glows, streaks, blurs): left out of the LUT, and a layer built on
/// a threshold or a streak is a glow pass, so it is left out whole.
const SPATIAL: &[&str] = &["gaussianblur", "sharpen", "flicker2", "innerblur", "feather", "vignette", "gradientoverlay", "gradientoverlay2", "lightglow", "softglow"];
const GLOW_PASS: &[&str] = &["threshold", "spectralmap", "stretchsegment", "wipe2"];

pub fn all() -> Vec<Grade> {
    crate::presets::file("grades.json").and_then(|t| serde_json::from_str(&t).map_err(|e| eprintln!("edits: grades.json is not valid: {e}")).ok()).unwrap_or_default()
}

pub fn find(name: &str) -> Option<Grade> {
    all().into_iter().find(|g| g.name.eq_ignore_ascii_case(name))
}

fn num(fx: &Fx, k: &str, d: f64) -> f64 {
    fx.p.get(k).and_then(|v| v.split(',').next()?.trim().parse().ok()).unwrap_or(d)
}

fn vec3(fx: &Fx, k: &str) -> [f64; 3] {
    let v: Vec<f64> = fx.p.get(k).map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default();
    [v.first().copied().unwrap_or(0.0), v.get(1).copied().unwrap_or(0.0), v.get(2).copied().unwrap_or(0.0)]
}

fn luma(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn hex(s: &str) -> Option<[f64; 3]> {
    let h = s.trim_start_matches('#');
    let h = if h.len() == 8 { &h[2..] } else { h }; // Alight writes #aarrggbb
    let b = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok().map(|v| v as f64 / 255.0);
    Some([b(0)?, b(2)?, b(4)?])
}

/// A pure colour at `deg` on the wheel, centred so it shifts hue without changing brightness much.
fn wheel(deg: f64, amount: f64) -> [f64; 3] {
    let h = deg.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let c = match h as u32 { 0 => [1.0, x, 0.0], 1 => [x, 1.0, 0.0], 2 => [0.0, 1.0, x], 3 => [0.0, x, 1.0], 4 => [x, 0.0, 1.0], _ => [1.0, 0.0, x] };
    let m = (c[0] + c[1] + c[2]) / 3.0;
    [(c[0] - m) * amount, (c[1] - m) * amount, (c[2] - m) * amount]
}

fn hue_rotate(c: [f64; 3], deg: f64) -> [f64; 3] {
    // rotation about the grey axis
    let (s, co) = deg.to_radians().sin_cos();
    let (a, d, e) = (co + (1.0 - co) / 3.0, (1.0 - co) / 3.0 - (1.0f64 / 3.0).sqrt() * s, (1.0 - co) / 3.0 + (1.0f64 / 3.0).sqrt() * s);
    [a * c[0] + d * c[1] + e * c[2], e * c[0] + a * c[1] + d * c[2], d * c[0] + e * c[1] + a * c[2]]
}

/// One effect on one colour.
fn effect(fx: &Fx, c: [f64; 3]) -> [f64; 3] {
    let map = |f: &dyn Fn(f64) -> f64| [f(c[0]), f(c[1]), f(c[2])];
    match fx.id.as_str() {
        "exposure" => {
            let (e, g, o) = (num(fx, "exposure", 0.0), num(fx, "gamma", 1.0).max(0.05), num(fx, "offset", 0.0));
            map(&|x| (x * 2f64.powf(e) + o).max(0.0).powf(1.0 / g))
        }
        "brightcont2" | "brightcont" => {
            let (b, k) = (num(fx, "brightness", 0.0), num(fx, "contrast", 0.0));
            map(&|x| (x - 0.5) * (1.0 + k) + 0.5 + b)
        }
        "satvib" => {
            let (s, v) = (num(fx, "saturation", 0.0), num(fx, "vib", 1.0));
            let y = luma(c);
            let sat = c.iter().cloned().fold(f64::MIN, f64::max) - c.iter().cloned().fold(f64::MAX, f64::min);
            let k = (1.0 + s) * (1.0 + (v - 1.0) * (1.0 - sat.clamp(0.0, 1.0)));
            map(&|x| y + (x - y) * k)
        }
        // shortcut: a smooth shadow/highlight curve with 1 as neutral; Alight's exact curve is not published
        "highlightshadow" => {
            let (h, s) = (num(fx, "highlights", 1.0), num(fx, "shadows", 1.0));
            map(&|x| x + (s - 1.0) * x * (1.0 - x).powi(2) * 1.5 + (h - 1.0) * x * x * (1.0 - x) * 1.5)
        }
        "channelmap" => {
            let pick = |k: &str, d: usize| match num(fx, k, d as f64) as usize { i @ 0..=2 => c[i], 3 => 1.0, _ => luma(c) };
            [pick("red", 0), pick("green", 1), pick("blue", 2)]
        }
        "colorize" => {
            let [deg, a, _] = vec3(fx, "tint");
            let w = wheel(deg, a);
            [c[0] + w[0], c[1] + w[1], c[2] + w[2]]
        }
        // colour wheels: `color`/`gain` push the highlights, `tint`/`lift` the shadows
        "colorhot" | "colortune2" | "colortune" => {
            let (hi, lo) = if fx.id == "colorhot" { (vec3(fx, "color"), vec3(fx, "tint")) } else { (vec3(fx, "gain"), vec3(fx, "lift")) };
            let (wh, wl) = (wheel(hi[0], hi[1]), wheel(lo[0], lo[1]));
            let l = luma(c);
            [0, 1, 2].map(|i| c[i] + wh[i] * l + wl[i] * (1.0 - l))
        }
        "hueshift" => hue_rotate(c, num(fx, "hue", 0.0)),
        "colortemperature" => {
            let t = num(fx, "temperature", 0.0) * num(fx, "strength", 1.0) * 0.03;
            [c[0] + t, c[1] + t * 0.3, c[2] - t]
        }
        _ => c,
    }
}

fn blend(mode: &str, a: f64, b: f64) -> f64 {
    // Alight writes soft-light, linear-dodge, color-burn
    match mode.replace(['-', '_', ' '], "").as_str() {
        "screen" => 1.0 - (1.0 - a) * (1.0 - b),
        "multiply" => a * b,
        "add" | "lineardodge" => a + b,
        "overlay" => if a < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) },
        "softlight" => (1.0 - 2.0 * b) * a * a + 2.0 * b * a,
        "subtract" => a - b,
        "difference" => (a - b).abs(),
        "hardlight" => if b < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) },
        "colordodge" => if b >= 1.0 { 1.0 } else { (a / (1.0 - b)).min(1.0) },
        "colorburn" => if b <= 0.0 { 0.0 } else { 1.0 - ((1.0 - a) / b).min(1.0) },
        "linearburn" => a + b - 1.0,
        "lighten" => a.max(b),
        "darken" => a.min(b),
        _ => b,
    }
}

/// What the grade does to one colour (0..1 each).
pub fn apply(g: &Grade, mut c: [f64; 3]) -> [f64; 3] {
    for l in &g.layers {
        if l.fx.iter().any(|f| GLOW_PASS.contains(&f.id.as_str())) {
            continue;
        }
        let mut x = l.color.as_deref().and_then(hex).unwrap_or(c);
        for f in l.fx.iter().filter(|f| !SPATIAL.contains(&f.id.as_str())) {
            x = if f.id == "lift" {
                // fill 0 shows the picture under the layer, 1 the layer's own colour
                let k = num(f, "fill", 1.0).clamp(0.0, 1.0);
                [0, 1, 2].map(|i| c[i] + (x[i] - c[i]) * k)
            } else {
                effect(f, x).map(|v| v.clamp(0.0, 1.0))
            };
        }
        let o = l.opacity.clamp(0.0, 1.0);
        c = [0, 1, 2].map(|i| (c[i] + (blend(&l.blend, c[i], x[i]) - c[i]) * o).clamp(0.0, 1.0));
    }
    c
}

/// The grade as a .cube LUT in the cache, and the filter that wears it.
pub fn lut(g: &Grade, cache: &Path) -> Result<String, String> {
    const N: usize = 33;
    let dir = cache.join("luts");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut s = format!("TITLE \"{}\"\nLUT_3D_SIZE {N}\n", g.name);
    for b in 0..N {
        for gr in 0..N {
            for r in 0..N {
                let k = |v: usize| v as f64 / (N - 1) as f64;
                let o = apply(g, [k(r), k(gr), k(b)]);
                s.push_str(&format!("{:.5} {:.5} {:.5}\n", o[0], o[1], o[2]));
            }
        }
    }
    let p = dir.join(format!("grade-{}.cube", crate::fetch::safe(&g.name)));
    std::fs::write(&p, s).map_err(|e| e.to_string())?;
    Ok(format!(",lut3d=file={}", p.to_string_lossy()))
}

/// The colour corrections in an Alight Motion project: its adjustment layers grouped by when they show, one grade
/// per group (a cc pack shows several, one after another).
pub fn from_xml(xml: &str, name: &str) -> Result<Vec<Grade>, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let first_v = |n: roxmltree::Node| n.attribute("value").map(String::from).or_else(|| n.children().find(|k| k.has_tag_name("kf")).and_then(|k| k.attribute("v").map(String::from)));
    let start = |n: roxmltree::Node| n.attribute("startTime").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    // the "before" slot of a pack (a title saying WITHOUT CC / NO CC / ORIGINAL) is not a look
    let plain: Vec<i64> = doc.descendants().filter(|n| n.has_tag_name("text")).filter(|t| {
        let s = t.children().find(|c| c.has_tag_name("content")).and_then(|c| c.text()).unwrap_or("").to_lowercase();
        s.contains("without") || s.contains("no cc") || s.contains("original") || s.contains("before")
    }).map(start).collect();
    let mut groups: Vec<(i64, Vec<Layer>)> = vec![];
    for sh in doc.descendants().filter(|n| n.has_tag_name("shape")) {
        let fx: Vec<Fx> = sh.children().filter(|n| n.has_tag_name("effect")).map(|e| Fx {
            id: e.attribute("id").unwrap_or("").rsplit('.').next().unwrap_or("").to_string(),
            p: e.children().filter(|p| p.has_tag_name("property")).filter_map(|p| Some((p.attribute("name")?.to_string(), first_v(p)?))).collect(),
        }).collect();
        if fx.is_empty() || sh.attribute("fillType").is_some_and(|t| t == "media") {
            continue;
        }
        let opacity = sh.descendants().find(|n| n.has_tag_name("opacity")).and_then(first_v).and_then(|v| v.parse().ok()).unwrap_or(1.0);
        // the layer's own fill: its colour, or a gradient's average (shortcut: the gradient's shape is lost)
        let gradient = sh.children().find(|n| n.has_tag_name("gradient")).filter(|_| sh.attribute("fillType") == Some("gradient"));
        let color = match gradient.and_then(|g| Some((hex(g.attribute("startColor")?)?, hex(g.attribute("endColor")?)?))) {
            Some((a, b)) => Some(format!("#{:02x}{:02x}{:02x}", ((a[0] + b[0]) * 127.5) as u8, ((a[1] + b[1]) * 127.5) as u8, ((a[2] + b[2]) * 127.5) as u8)),
            None => sh.children().find(|n| n.has_tag_name("fillColor")).and_then(first_v).filter(|v| hex(v).is_some()),
        };
        let layer = Layer { opacity, blend: sh.attribute("blending").unwrap_or("normal").to_lowercase(), color, fx };
        // layers of one look start within a few frames of each other
        let when = start(sh);
        if plain.iter().any(|p| (p - when).abs() < 150) {
            continue;
        }
        match groups.iter_mut().find(|g| (g.0 - when).abs() < 150) {
            Some(g) => g.1.push(layer),
            None => groups.push((when, vec![layer])),
        }
    }
    if groups.is_empty() {
        return Err(format!("{name}: no colour layers"));
    }
    let base = crate::preset::slug(name);
    let n = groups.len();
    Ok(groups.into_iter().enumerate().map(|(i, (_, layers))| Grade {
        name: if n > 1 { format!("{base}-{}", i + 1) } else { base.clone() },
        about: format!("colour from {name}{}", if n > 1 { format!(", look {}", i + 1) } else { String::new() }),
        layers,
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(id: &str, p: &[(&str, &str)]) -> Fx {
        Fx { id: id.into(), p: p.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() }
    }

    #[test]
    fn colour_maths() {
        let g = |layers| Grade { name: "t".into(), about: String::new(), layers };
        let grey = [0.5, 0.5, 0.5];
        // no layers, no change; a desaturating layer at half opacity, half way
        assert_eq!(apply(&g(vec![]), grey), grey);
        let red = [0.8, 0.2, 0.2];
        let half = apply(&g(vec![Layer { opacity: 0.5, blend: "normal".into(), color: None, fx: vec![fx("satvib", &[("saturation", "-1")])] }]), red);
        assert!(half[0] < 0.8 && half[0] > luma(red), "{half:?}");
        // exposure +1 doubles; screen never darkens; glow passes are skipped whole
        let up = apply(&g(vec![Layer { opacity: 1.0, blend: "normal".into(), color: None, fx: vec![fx("exposure", &[("exposure", "1")])] }]), [0.25; 3]);
        assert!((up[0] - 0.5).abs() < 1e-9);
        let scr = apply(&g(vec![Layer { opacity: 1.0, blend: "screen".into(), color: None, fx: vec![fx("brightcont2", &[("brightness", "-0.64"), ("contrast", "3")])] }]), grey);
        assert!(scr[0] >= 0.5);
        let glow = Layer { opacity: 1.0, blend: "normal".into(), color: None, fx: vec![fx("threshold", &[]), fx("exposure", &[("exposure", "3")])] };
        assert_eq!(apply(&g(vec![glow]), grey), grey);
        // Alight's hyphenated names blend, not replace
        assert_eq!(blend("soft-light", 0.5, 0.5), blend("softlight", 0.5, 0.5));
        assert!((blend("linear-dodge", 0.3, 0.3) - 0.6).abs() < 1e-9 && blend("subtract", 0.3, 0.5) < 0.0);
        assert_eq!(hex("#ffc0f4d9"), Some([192.0 / 255.0, 244.0 / 255.0, 217.0 / 255.0]));
        // a warm wheel pushes red up and blue down
        let w = wheel(30.0, 0.2);
        assert!(w[0] > 0.0 && w[2] < 0.0);
    }

    #[test]
    fn reads_a_cc_pack() {
        let xml = r##"<scene title="cc" width="1080" height="1080" totalTime="2000" fps="30">
          <shape id="1" startTime="0" endTime="999" fillType="color" blending="screen"><transform><opacity value="0.4"/></transform>
            <fillColor value="#ffc0f4d9"/><effect id="com.alightcreative.effects.lift"><property name="fill" type="float" value="0"/></effect>
            <effect id="com.alightcreative.effects.exposure"><property name="exposure" type="float" value="0.4"/></effect></shape>
          <shape id="2" startTime="1000" endTime="2000" fillType="color"><effect id="com.alightcreative.effects.lift"><property name="fill" type="float" value="0"/></effect>
            <effect id="com.alightcreative.effects.satvib"><property name="saturation" type="float" value="-0.5"/></effect></shape>
        </scene>"##;
        let g = from_xml(xml, "My CC").unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!((g[0].name.as_str(), g[0].layers[0].blend.as_str(), g[0].layers[0].opacity, g[0].layers[0].color.as_deref()), ("my-cc-1", "screen", 0.4, Some("#ffc0f4d9")));
        // lift 0 over a colour fill is an adjustment layer: grey stays grey with no effects
        let adj = Layer { opacity: 1.0, blend: "normal".into(), color: Some("#ff0000".into()), fx: vec![fx("lift", &[("fill", "0")])] };
        assert_eq!(apply(&Grade { name: "a".into(), about: String::new(), layers: vec![adj.clone()] }, [0.5; 3]), [0.5; 3]);
        // a little fill tints a little
        let tint = Layer { fx: vec![fx("lift", &[("fill", "0.1")])], ..adj };
        let t = apply(&Grade { name: "a".into(), about: String::new(), layers: vec![tint] }, [0.5; 3]);
        assert!((t[0] - 0.55).abs() < 1e-9 && (t[1] - 0.45).abs() < 1e-9);
        let dir = std::env::temp_dir().join("edits-grade-test");
        let f = lut(&g[1], &dir).unwrap();
        let cube = std::fs::read_to_string(f.trim_start_matches(",lut3d=file=")).unwrap();
        assert_eq!(cube.lines().count(), 2 + 33 * 33 * 33);
        assert!(cube.lines().nth(2).unwrap().starts_with("0.00000 0.00000 0.00000"));
    }
}
