//! Premiere Pro effect presets (.prfpset) as camera moves around a cut: scale, position, rotation and blur
//! keyframes, with "A/B" or "IN/OUT" halves joined into one move and "Start N Frames Before Cut" honoured.
use crate::preset::{Curve, Ease, Motion};
use roxmltree::{Document, Node};
use std::collections::HashMap;

/// Premiere's clock: ticks per second.
const TICKS: f64 = 254_016_000_000.0;

/// A parameter's keyframes: "ticks,value,…;ticks,value,…", values "a" or "x:y". Times start at 0.
fn keys(text: &str, comp: usize) -> Vec<(f64, f64, Ease)> {
    let raw: Vec<(f64, f64, bool)> = text.split(';').filter_map(|k| {
        let f: Vec<&str> = k.split(',').collect();
        let t = f.first()?.trim().parse::<f64>().ok()? / TICKS;
        let v = f.get(1)?.split(':').nth(comp)?.trim().trim_end_matches('.').parse::<f64>().ok()?;
        // non-zero bezier handles: Premiere's smooth keyframes
        let eased = f.iter().skip(4).any(|x| x.parse::<f64>().is_ok_and(|v| v.abs() > 1e-6));
        Some((t, v, eased))
    }).collect();
    let t0 = raw.iter().map(|k| k.0).fold(f64::MAX, f64::min);
    raw.into_iter().map(|(t, v, e)| (t - t0, v, if e { Ease::InOut(2.0) } else { Ease::Linear })).collect()
}

/// Everything an item points to, following ObjectRef links.
fn reachable<'a, 'i>(start: Node<'a, 'i>, objs: &HashMap<&str, Node<'a, 'i>>) -> Vec<Node<'a, 'i>> {
    let mut seen: Vec<Node> = vec![];
    let mut stack = vec![start];
    while let Some(n) = stack.pop() {
        if seen.contains(&n) {
            continue;
        }
        seen.push(n);
        for d in n.descendants() {
            if let Some(r) = d.attribute("ObjectRef").and_then(|r| objs.get(r)) {
                stack.push(*r);
            }
        }
    }
    seen
}

/// Which half of a cut a preset plays: the outgoing shot ("OUT", "- A"), the incoming one ("IN", "- B"), or both
/// ("Start N Frames Before Cut": the cut is N frames in). Returns (stem to pair halves by, cut seconds or None).
fn half(name: &str) -> (String, Option<f64>, bool) {
    let n = name.trim();
    let lower = n.to_lowercase();
    if let Some(i) = lower.find("start ") {
        let frames: f64 = lower[i + 6..].split_whitespace().next().and_then(|x| x.parse().ok()).unwrap_or(0.0);
        if lower[i..].contains("before") {
            return (n.to_string(), Some(frames / 30.0), true);
        }
    }
    for (suffix, out) in [(" - a", true), (" - b", false), (" out", true), (" in", false)] {
        if let Some(stem) = lower.strip_suffix(suffix) {
            return (n[..stem.len()].to_string(), None, out);
        }
    }
    (n.to_string(), None, false)
}

pub fn parse(xml: &str, file: &str) -> Result<Vec<Motion>, String> {
    let doc = Document::parse(xml).map_err(|e| format!("{file}: not a Premiere preset ({e})"))?;
    let objs: HashMap<&str, Node> = doc.root_element().children().filter_map(|n| Some((n.attribute("ObjectID")?, n))).collect();
    // one move per preset item
    let mut halves: Vec<(String, Option<f64>, bool, Motion)> = vec![];
    for item in doc.root_element().children().filter(|n| n.has_tag_name("TreeItem")) {
        let Some(name) = item.descendants().find(|d| d.has_tag_name("Name")).and_then(|n| n.text()) else { continue };
        if name.to_lowercase().contains("base preset") {
            continue;
        }
        let mut m = Motion { name: name.trim().to_string(), dur: 0.0, cut: 0.0, zoom: Curve::default(), x: Curve::default(), y: Curve::default(), rot: Curve::default(), blur: Curve::default() };
        for n in reachable(item, &objs) {
            let (Some(pname), Some(kf)) = (n.children().find(|c| c.has_tag_name("Name")).and_then(|c| c.text()), n.children().find(|c| c.has_tag_name("Keyframes")).and_then(|c| c.text())) else { continue };
            if !kf.contains(';') {
                continue;
            }
            let set = |c: &mut Curve, k: Vec<(f64, f64, Ease)>| if c.keys.is_empty() && k.len() > 1 { *c = Curve { keys: k } };
            match pname.trim() {
                "Scale" | "Scale Height" => set(&mut m.zoom, keys(kf, 0).into_iter().map(|(t, v, e)| (t, v / 100.0, e)).collect()),
                "Position" | "Shift Center To" => {
                    set(&mut m.x, keys(kf, 0).into_iter().map(|(t, v, e)| (t, v - 0.5, e)).collect());
                    set(&mut m.y, keys(kf, 1).into_iter().map(|(t, v, e)| (t, v - 0.5, e)).collect());
                }
                "Rotation" => set(&mut m.rot, keys(kf, 0)),
                "Blurriness" | "Blur Length" => set(&mut m.blur, keys(kf, 0).into_iter().map(|(t, v, e)| (t, (v / 30.0).min(1.0), e)).collect()),
                _ => {}
            }
        }
        let curves = [&m.zoom, &m.x, &m.y, &m.rot, &m.blur];
        m.dur = curves.iter().flat_map(|c| c.keys.iter().map(|k| k.0)).fold(0.0, f64::max);
        if m.dur < 0.05 || m.dur > 3.0 {
            continue; // nothing moves, or a slow drift rather than a move around a cut
        }
        let (stem, cut, out) = half(&m.name);
        halves.push((stem, cut, out, m));
    }
    // curves as fractions of the move's length
    let norm = |c: &Curve, dur: f64, shift: f64| Curve { keys: c.keys.iter().map(|&(t, v, e)| ((t + shift) / dur, v, e)).collect() };
    let mut out: Vec<Motion> = vec![];
    let mut used = vec![false; halves.len()];
    for i in 0..halves.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let (stem, cut, is_out, a) = halves[i].clone();
        // the matching other half, if this is one half of a pair
        let partner = (cut.is_none()).then(|| (0..halves.len()).find(|&j| !used[j] && halves[j].0 == stem && halves[j].2 != is_out && halves[j].1.is_none())).flatten();
        let m = match (cut, partner) {
            (Some(c), _) => Motion { cut: c.min(a.dur), zoom: norm(&a.zoom, a.dur, 0.0), x: norm(&a.x, a.dur, 0.0), y: norm(&a.y, a.dur, 0.0), rot: norm(&a.rot, a.dur, 0.0), blur: norm(&a.blur, a.dur, 0.0), ..a },
            (None, Some(j)) => {
                used[j] = true;
                let b = halves[j].3.clone();
                let (o, n) = if is_out { (a, b) } else { (b, a) };
                let dur = o.dur + n.dur;
                let join = |x: &Curve, y: &Curve| Curve { keys: norm(x, dur, 0.0).keys.into_iter().chain(norm(y, dur, o.dur).keys).collect() };
                Motion { name: stem.clone(), dur, cut: o.dur, zoom: join(&o.zoom, &n.zoom), x: join(&o.x, &n.x), y: join(&o.y, &n.y), rot: join(&o.rot, &n.rot), blur: join(&o.blur, &n.blur) }
            }
            // a lone half: an "out" move ends on the cut, an "in" move starts on it
            (None, None) => Motion { cut: if is_out { a.dur } else { 0.0 }, zoom: norm(&a.zoom, a.dur, 0.0), x: norm(&a.x, a.dur, 0.0), y: norm(&a.y, a.dur, 0.0), rot: norm(&a.rot, a.dur, 0.0), blur: norm(&a.blur, a.dur, 0.0), ..a },
        };
        out.push(Motion { name: format!("{file}: {}", m.name), ..m });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves() {
        assert_eq!(half("Rotation Transition (Clockwise) - A"), ("Rotation Transition (Clockwise)".into(), None, true));
        assert_eq!(half("Cross Spin IN"), ("Cross Spin".into(), None, false));
        let (_, cut, _) = half("Blur Warp In Out (Start 9 Frames Before Cut)");
        assert_eq!(cut, Some(0.3));
    }

    #[test]
    fn keyframe_text() {
        let k = keys("914456685542400,100.,0,0,0,0.1666,2.02,0.1666;915711084288000,110.,0,0,0,0,0,0;", 0);
        assert_eq!(k.len(), 2);
        assert_eq!((k[0].0, k[0].1, k[0].2), (0.0, 100.0, Ease::InOut(2.0)));
        assert!((k[1].0 - 4.938).abs() < 0.001 && k[1].1 == 110.0 && k[1].2 == Ease::Linear);
        assert_eq!(keys("0,0.5:0.25,0,0,0,0,0,0;254016000000,1:0.75,0,0,0,0,0,0", 1).iter().map(|k| k.1).collect::<Vec<_>>(), vec![0.25, 0.75]);
    }

    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PremiereData Version="3">
  <TreeItem ObjectID="5"><TreeItemBase><Name>Spin - A</Name><Data ObjectRef="6"/></TreeItemBase></TreeItem>
  <Comp ObjectID="6"><Param ObjectRef="7"/><Param ObjectRef="8"/></Comp>
  <Param ObjectID="7"><Name>Rotation</Name><Keyframes>0,0.,0,0,0,0,0,0;63504000000,90.,0,0,0,0,0,0;</Keyframes></Param>
  <Param ObjectID="8"><Name>Blurriness</Name><Keyframes>0,0.,0,0,0,0,0,0;63504000000,30.,0,0,0,0,0,0;</Keyframes></Param>
  <TreeItem ObjectID="9"><TreeItemBase><Name>Spin - B</Name><Data ObjectRef="10"/></TreeItemBase></TreeItem>
  <Param ObjectID="10"><Name>Rotation</Name><Keyframes>0,-90.,0,0,0,0,0,0;127008000000,0.,0,0,0,0,0,0;</Keyframes></Param>
  <TreeItem ObjectID="11"><TreeItemBase><Name>0. Base Preset</Name></TreeItemBase></TreeItem>
</PremiereData>"#;

    #[test]
    fn pairs_become_one_move() {
        let m = parse(XML, "spin").unwrap();
        assert_eq!(m.len(), 1);
        let s = &m[0];
        assert_eq!((s.name.as_str(), s.dur, s.cut), ("spin: Spin", 0.75, 0.25));
        // the outgoing shot turns to 90 and blurs; the incoming one comes from -90 back to rest
        assert_eq!(s.rot.keys.iter().map(|k| (k.0 * 100.0).round() / 100.0).collect::<Vec<_>>(), vec![0.0, 0.33, 0.33, 1.0]);
        assert_eq!(s.blur.max(), 1.0);
        assert!(s.strength() > 1.0);
    }
}
