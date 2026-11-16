//! CapCut projects (a draft folder's draft_content.json, or draft_info.json in older ones) as presets: each clip on
//! the main video track becomes a shot with its speed (ramps too), zoom, slide and turn, keyframed or still; effect
//! clips named like a flash, shake, blur, RGB split or invert hit the shots under them; the music clip is the song.
//! Times in a draft are microseconds; positions are centre-out with 1 = half the canvas, y pointing up.
use crate::preset::{Curve, Ease, Flash, Preset, Slot, Song};
use serde_json::Value;

const US: f64 = 1_000_000.0;

fn num(v: &Value) -> Option<f64> {
    v.as_f64()
}

/// The material a segment points at, by id, from any list in `materials`.
fn material<'a>(m: &'a Value, list: &str, id: &str) -> Option<&'a Value> {
    m[list].as_array()?.iter().find(|x| x["id"].as_str() == Some(id))
}

/// A segment's materials named in `extra_material_refs`, from `list`.
fn extras<'a>(m: &'a Value, list: &str, seg: &Value) -> Vec<&'a Value> {
    seg["extra_material_refs"].as_array().into_iter().flatten().filter_map(|r| material(m, list, r.as_str()?)).collect()
}

fn ease(curve: &str) -> Ease {
    match curve {
        "Line" | "" => Ease::Linear,
        "EaseIn" => Ease::In(2.0),
        "EaseOut" => Ease::Out(2.0),
        _ => Ease::InOut(2.0),
    }
}

/// One keyframed property of a segment as a curve over the shot (0..1), each value through `f`.
fn keyed(seg: &Value, kind: &str, len_us: f64, f: impl Fn(f64) -> f64) -> Curve {
    let list = seg["common_keyframes"].as_array().into_iter().flatten().find(|k| k["property_type"].as_str() == Some(kind));
    let mut keys: Vec<(f64, f64, Ease)> = list.and_then(|k| k["keyframe_list"].as_array()).into_iter().flatten().filter_map(|k| {
        let t = (num(&k["time_offset"])? / len_us.max(1.0)).clamp(0.0, 1.0);
        Some((t, f(num(k["values"].get(0)?)?), ease(k["curveType"].as_str().unwrap_or(""))))
    }).collect();
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    // the easing of a key is how it is left; ours is how the next one is reached
    let eases: Vec<Ease> = keys.iter().map(|k| k.2).collect();
    for i in (1..keys.len()).rev() {
        keys[i].2 = eases[i - 1];
    }
    if let Some(k) = keys.first_mut() {
        k.2 = Ease::Linear;
    }
    Curve { keys }
}

/// A still value as a curve, empty when it is the resting one.
fn still(v: f64, rest: f64) -> Curve {
    if (v - rest).abs() > 1e-4 { Curve::constant(v) } else { Curve::default() }
}

/// A shot from a video segment.
fn shot(seg: &Value, m: &Value) -> Option<Slot> {
    let (start, len) = (num(&seg["target_timerange"]["start"])?, num(&seg["target_timerange"]["duration"])?);
    let clip = &seg["clip"];
    let (sx, tx, ty, rot) = (num(&clip["scale"]["x"]).unwrap_or(1.0), num(&clip["transform"]["x"]).unwrap_or(0.0), num(&clip["transform"]["y"]).unwrap_or(0.0), num(&clip["rotation"]).unwrap_or(0.0));
    let pick = |kind: &str, f: fn(f64) -> f64, rest: f64, v: f64| {
        let c = keyed(seg, kind, len, f);
        if c.is_empty() { still(f(v), rest) } else { c }
    };
    // a speed curve: points at 0..1 of the clip, each a multiple of normal speed
    let ramp: Vec<(f64, f64, Ease)> = extras(m, "speeds", seg).iter().filter_map(|s| s["curve_speed"]["speed_points"].as_array()).flatten()
        .filter_map(|p| Some((num(&p["x"])?.clamp(0.0, 1.0), num(&p["y"])?.max(0.05), Ease::InOut(2.0)))).collect();
    let mut s: Slot = serde_json::from_value(serde_json::json!({ "start": start / US, "end": (start + len) / US })).ok()?;
    if ramp.len() >= 2 {
        s.ramp = Curve { keys: ramp };
    } else {
        s.speed = num(&seg["speed"]).unwrap_or(1.0).clamp(0.1, 8.0);
    }
    s.zoom = pick("KFTypeScaleX", |v| v, 1.0, sx);
    s.x = pick("KFTypePositionX", |v| v / 2.0, 0.0, tx);
    s.y = pick("KFTypePositionY", |v| -v / 2.0, 0.0, ty);
    s.rot = pick("KFTypeRotation", |v| v, 0.0, rot);
    Some(s)
}

/// What an effect clip does to the shots under it, from its name.
fn effect(name: &str, from: f64, to: f64, slots: &mut [Slot], flashes: &mut Vec<Flash>) {
    let n = name.to_lowercase();
    let has = |ws: &[&str]| ws.iter().any(|w| n.contains(w));
    if has(&["flash", "flicker", "blink", "strobe", "white"]) {
        flashes.push(Flash { start: from, end: to.min(from + 0.4), color: "ffffff".into(), opacity: Curve { keys: vec![(0.0, 1.0, Ease::Linear), (1.0, 0.0, Ease::Out(3.0))] }, blink: has(&["strobe", "flicker", "blink"]) });
    }
    for s in slots.iter_mut().filter(|s| s.start < to && s.end > from) {
        if has(&["shake", "vibrat", "earthquake", "jitter"]) {
            s.shake = Curve { keys: vec![(0.0, 0.03, Ease::Linear), (1.0, 0.0, Ease::Out(3.0))] };
            s.shake_hz = 12.0;
        }
        if has(&["blur"]) {
            s.blur = Curve { keys: vec![(0.0, 0.6, Ease::Linear), (0.3, 0.0, Ease::Out(2.0))] };
        }
        if has(&["rgb", "chromatic", "glitch", "split", "aberration"]) {
            s.rgb = true;
        }
        if has(&["invert", "negative"]) {
            s.invert = true;
        }
        if has(&["motion blur", "speed"]) {
            s.motion_blur = true;
        }
    }
}

pub fn parse(text: &str, name: &str) -> Result<Preset, String> {
    if !text.trim_start().starts_with('{') {
        return Err(format!("{name}: an encrypted CapCut draft (newer CapCut / JianYing 6+ lock their drafts); open it in an older CapCut or export the edit and use `edits learn` on the video"));
    }
    let d: Value = serde_json::from_str(text).map_err(|e| format!("{name}: not a CapCut draft ({e})"))?;
    let tracks = d["tracks"].as_array().ok_or(format!("{name}: not a CapCut draft (no tracks)"))?;
    let m = &d["materials"];
    let segs = |t: &Value| t["segments"].as_array().cloned().unwrap_or_default();
    // the main track: the video track with the most clips (overlays sit on the others)
    let main = tracks.iter().filter(|t| t["type"].as_str() == Some("video")).max_by_key(|t| segs(t).len()).ok_or(format!("{name}: no video track"))?;
    let mut slots: Vec<Slot> = segs(main).iter().filter_map(|s| shot(s, m)).collect();
    slots.sort_by(|a, b| a.start.total_cmp(&b.start));
    if slots.is_empty() {
        return Err(format!("{name}: no video clips to fill"));
    }
    let mut flashes = vec![];
    for t in tracks.iter().filter(|t| t["type"].as_str() == Some("effect")) {
        for s in segs(t) {
            let (Some(a), Some(l)) = (num(&s["target_timerange"]["start"]), num(&s["target_timerange"]["duration"])) else { continue };
            if let Some(name) = s["material_id"].as_str().and_then(|id| material(m, "video_effects", id)).and_then(|e| e["name"].as_str()) {
                effect(name, a / US, (a + l) / US, &mut slots, &mut flashes);
            }
        }
    }
    // transitions named like a flash become one on the cut
    for (seg, slot) in segs(main).iter().zip(slots.clone()) {
        for t in extras(m, "transitions", seg) {
            let n = t["name"].as_str().unwrap_or("").to_lowercase();
            if n.contains("flash") || n.contains("white") {
                let half = num(&t["duration"]).unwrap_or(300_000.0) / US / 2.0;
                flashes.push(Flash { start: (slot.end - half).max(0.0), end: slot.end + half, color: "ffffff".into(), opacity: Curve { keys: vec![(0.0, 0.0, Ease::Linear), (0.5, 1.0, Ease::In(2.0)), (1.0, 0.0, Ease::Out(2.0))] }, blink: false });
            }
        }
    }
    // the song: the longest audio clip that is music, named well enough to find again
    let song = tracks.iter().filter(|t| t["type"].as_str() == Some("audio")).flat_map(segs)
        .filter_map(|s| {
            let a = material(m, "audios", s["material_id"].as_str()?)?;
            let title = a["name"].as_str()?.trim().trim_end_matches(".mp3").trim_end_matches(".m4a").trim().to_string();
            let named = title.split_whitespace().filter(|w| w.chars().any(char::is_alphabetic)).count() >= 2;
            (a["type"].as_str() != Some("voice") && named).then(|| (num(&s["target_timerange"]["duration"]).unwrap_or(0.0), Song { title, from: num(&s["source_timerange"]["start"]).unwrap_or(0.0) / US }))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1);
    let num_u = |v: &Value, d: f64| num(v).unwrap_or(d) as u32 / 2 * 2;
    Ok(Preset {
        name: d["name"].as_str().filter(|n| !n.trim().is_empty()).unwrap_or(name).to_string(),
        about: format!("CapCut project \"{name}\""),
        w: num_u(&d["canvas_config"]["width"], 1080.0),
        h: num_u(&d["canvas_config"]["height"], 1920.0),
        total: num(&d["duration"]).map_or(slots.iter().map(|s| s.end).fold(0.0, f64::max), |t| t / US),
        song,
        look: None,
        intensity: 1.0,
        fit_beats: true,
        slots,
        flashes,
    }.tidy())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn a_draft_becomes_a_preset() {
        let d = json!({
            "name": "velo", "duration": 3_000_000, "canvas_config": {"width": 1080, "height": 1920},
            "materials": {
                "speeds": [{"id": "sp2", "speed": 1.0, "curve_speed": {"speed_points": [{"x": 0.0, "y": 3.0}, {"x": 0.5, "y": 0.4}, {"x": 1.0, "y": 3.0}]}}],
                "video_effects": [{"id": "fx1", "name": "Shake 2"}],
                "transitions": [{"id": "tr1", "name": "Flash White", "duration": 400_000}],
                "audios": [{"id": "au1", "type": "music", "name": "Metro Boomin Superhero.mp3"}]
            },
            "tracks": [
                {"type": "video", "segments": [
                    {"target_timerange": {"start": 0, "duration": 1_000_000}, "speed": 2.0, "extra_material_refs": ["tr1"],
                     "clip": {"scale": {"x": 1.2, "y": 1.2}, "transform": {"x": 0.5, "y": 0.2}, "rotation": 0.0},
                     "common_keyframes": [{"property_type": "KFTypeScaleX", "keyframe_list": [
                        {"time_offset": 0, "values": [1.5], "curveType": "EaseOut"}, {"time_offset": 500_000, "values": [1.0], "curveType": "Line"}]}]},
                    {"target_timerange": {"start": 1_000_000, "duration": 2_000_000}, "speed": 1.0, "extra_material_refs": ["sp2"]}
                ]},
                {"type": "effect", "segments": [{"material_id": "fx1", "target_timerange": {"start": 1_200_000, "duration": 300_000}}]},
                {"type": "audio", "segments": [{"material_id": "au1", "target_timerange": {"start": 0, "duration": 3_000_000}, "source_timerange": {"start": 42_000_000, "duration": 3_000_000}}]}
            ]
        });
        let p = super::parse(&d.to_string(), "folder").unwrap();
        assert_eq!((p.name.as_str(), p.w, p.h, p.total, p.slots.len()), ("velo", 1080, 1920, 3.0, 2));
        let a = &p.slots[0];
        assert_eq!(a.speed, 2.0);
        // keyframes win over the still scale; the first key's easing carries to the second
        assert_eq!(a.zoom.keys.len(), 2);
        assert_eq!((a.zoom.keys[0].1, a.zoom.keys[1].0, a.zoom.keys[1].1), (1.5, 0.5, 1.0));
        assert!(matches!(a.zoom.keys[1].2, crate::preset::Ease::Out(_)));
        assert_eq!((a.x.keys[0].1, a.y.keys[0].1), (0.25, -0.1), "half-canvas units, y up");
        let b = &p.slots[1];
        assert_eq!(b.ramp.keys.len(), 3);
        assert!(b.shake.is_moving() && a.shake.is_empty(), "the shake effect sits over the second shot only");
        assert_eq!(p.flashes.len(), 1, "the white flash transition on the cut");
        assert_eq!(p.song.as_ref().map(|s| (s.title.as_str(), s.from)), Some(("Metro Boomin Superhero", 42.0)));
    }

    #[test]
    fn an_encrypted_draft_says_so() {
        assert!(super::parse("\u{1}\u{2}binary", "x").unwrap_err().contains("encrypted"));
    }
}
