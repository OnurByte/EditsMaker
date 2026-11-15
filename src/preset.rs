//! Alight Motion projects (.xml) as presets: an edit's timing, speed ramps, camera moves, shakes, blurs, exposure
//! hits and flashes, read from the file and replayed over your own footage.
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ease {
    Linear,
    /// fast start, settles: 1-(1-u)^k
    Out(f64),
    /// slow start: u^k
    In(f64),
    InOut(f64),
}

/// A CSS-style cubic bezier easing: y at x = `u`, found by bisection on x(s).
pub fn bezier(x1: f64, y1: f64, x2: f64, y2: f64, u: f64) -> f64 {
    let at = |a: f64, b: f64, s: f64| 3.0 * a * s * (1.0 - s).powi(2) + 3.0 * b * s * s * (1.0 - s) + s.powi(3);
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..50 {
        let mid = (lo + hi) / 2.0;
        if at(x1.clamp(0.0, 1.0), x2.clamp(0.0, 1.0), mid) < u { lo = mid } else { hi = mid }
    }
    at(y1, y2, (lo + hi) / 2.0)
}

/// "cubicBezier x1 y1 x2 y2" as the power curve closest to it, since ffmpeg expressions cannot solve a bezier.
pub fn ease(e: Option<&str>) -> Ease {
    let p: Vec<f64> = e.unwrap_or("").split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
    let [x1, y1, x2, y2] = p[..] else { return Ease::Linear };
    let us: Vec<f64> = (1..10).map(|i| i as f64 / 10.0).collect();
    let truth: Vec<f64> = us.iter().map(|&u| bezier(x1, y1, x2, y2, u)).collect();
    let err = |e: Ease| us.iter().zip(&truth).map(|(&u, t)| (e.at(u) - t).powi(2)).sum::<f64>();
    let mut best = (err(Ease::Linear), Ease::Linear);
    for i in 0..=48 {
        let k = 1.2 + i as f64 * 0.1;
        for e in [Ease::Out(k), Ease::In(k), Ease::InOut(k)] {
            if err(e) < best.0 - 1e-9 {
                best = (err(e), e);
            }
        }
    }
    match best.1 {
        Ease::Out(k) => Ease::Out((k * 10.0).round() / 10.0),
        Ease::In(k) => Ease::In((k * 10.0).round() / 10.0),
        Ease::InOut(k) => Ease::InOut((k * 10.0).round() / 10.0),
        l => l,
    }
}

/// In files an easing is a word: "linear", "out 4", "in 2.5", "inout 3".
impl Serialize for Ease {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&match self {
            Ease::Linear => "linear".to_string(),
            Ease::Out(k) => format!("out {k}"),
            Ease::In(k) => format!("in {k}"),
            Ease::InOut(k) => format!("inout {k}"),
        })
    }
}

impl<'de> Deserialize<'de> for Ease {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        let mut w = text.split_whitespace();
        let (kind, k) = (w.next().unwrap_or("linear").to_lowercase(), w.next().and_then(|x| x.parse::<f64>().ok()).unwrap_or(2.0).clamp(0.2, 10.0));
        match kind.as_str() {
            "linear" => Ok(Ease::Linear),
            "out" => Ok(Ease::Out(k)),
            "in" => Ok(Ease::In(k)),
            "inout" => Ok(Ease::InOut(k)),
            _ => Err(serde::de::Error::custom(format!("easing \"{text}\" is not linear, out K, in K or inout K"))),
        }
    }
}

impl Ease {
    fn at(self, u: f64) -> f64 {
        let u = u.clamp(0.0, 1.0);
        match self {
            Ease::Linear => u,
            Ease::Out(k) => 1.0 - (1.0 - u).powf(k),
            Ease::In(k) => u.powf(k),
            Ease::InOut(k) => if u < 0.5 { (2.0 * u).powf(k) / 2.0 } else { 1.0 - (2.0 - 2.0 * u).powf(k) / 2.0 },
        }
    }
    fn expr(self, u: &str) -> String {
        match self {
            Ease::Linear => u.to_string(),
            Ease::Out(k) => format!("(1-pow(1-{u},{k:.2}))"),
            Ease::In(k) => format!("pow({u},{k:.2})"),
            Ease::InOut(k) => format!("if(lt({u},0.5),pow(2*{u},{k:.2})/2,1-pow(2-2*{u},{k:.2})/2)"),
        }
    }
}

/// A value over a layer's life: keys at 0..1 of its duration, each eased in from the one before.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Curve {
    pub keys: Vec<(f64, f64, Ease)>,
}

impl Curve {
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    pub fn constant(v: f64) -> Self {
        Curve { keys: vec![(0.0, v, Ease::Linear)] }
    }
    pub fn is_moving(&self) -> bool {
        self.keys.windows(2).any(|w| (w[0].1 - w[1].1).abs() > 1e-6)
    }
    pub fn max(&self) -> f64 {
        self.keys.iter().map(|k| k.1).fold(f64::MIN, f64::max)
    }
    pub fn min(&self) -> f64 {
        self.keys.iter().map(|k| k.1).fold(f64::MAX, f64::min)
    }
    pub fn map(&self, f: impl Fn(f64) -> f64) -> Self {
        Curve { keys: self.keys.iter().map(|&(t, v, e)| (t, f(v), e)).collect() }
    }
    /// Value at `u` (0..1 of the layer).
    pub fn at(&self, u: f64) -> f64 {
        let k = &self.keys;
        match k.iter().position(|x| x.0 > u) {
            _ if k.is_empty() => 0.0,
            Some(0) => k[0].1,
            None => k[k.len() - 1].1,
            Some(i) => {
                let (a, b) = (k[i - 1], k[i]);
                a.1 + (b.1 - a.1) * b.2.at((u - a.0) / (b.0 - a.0).max(1e-9))
            }
        }
    }
    /// An ffmpeg expression for the value at `var` seconds into a layer lasting `dur` seconds.
    pub fn expr(&self, var: &str, dur: f64) -> String {
        let k = &self.keys;
        if k.is_empty() {
            return "0".into();
        }
        let n = k.len();
        let mut e = format!("{:.4}", k[n - 1].1);
        for i in (0..n - 1).rev() {
            let (a, b) = (k[i], k[i + 1]);
            let (ta, tb) = (a.0 * dur, b.0 * dur);
            let seg = if tb - ta < 1e-4 {
                format!("{:.4}", b.1)
            } else {
                let u = format!("clip(({var}-{ta:.4})/{:.4},0,1)", tb - ta);
                format!("({:.4}+{:.4}*{})", a.1, b.1 - a.1, b.2.expr(&u))
            };
            e = format!("if(lt({var},{tb:.4}),{seg},{e})");
        }
        format!("if(lt({var},{:.4}),{:.4},{e})", k[0].0 * dur, k[0].1)
    }
}

/// One shot of the edit, and everything that happens to it. Curves are [time 0..1 of the shot, value, easing].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Slot {
    pub start: f64,
    pub end: f64,
    /// playback speed of the footage (velocity edits run 2-4x)
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub speed: f64,
    /// 1 = framed, above = zoomed in
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub zoom: Curve,
    /// offsets as a fraction of the frame width / height
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub x: Curve,
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub y: Curve,
    /// degrees
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub rot: Curve,
    /// shake amplitude as a fraction of the frame width, and its frequency
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub shake: Curve,
    #[serde(default = "four", skip_serializing_if = "is_four")]
    pub shake_hz: f64,
    /// 0..1, blur on when it is high
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub blur: Curve,
    /// 0..1
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub exposure: Curve,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub saturation: f64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub rgb: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub invert: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub motion_blur: bool,
    /// a speed ramp over the shot, multiplying `speed` (2.5 = rushing, 0.5 = slow motion)
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub ramp: Curve,
}

impl Slot {
    /// Seconds of footage the shot plays in `len` seconds of edit.
    pub fn source_len(&self, len: f64) -> f64 {
        if self.ramp.is_empty() {
            len * self.speed
        } else {
            crate::fx::ramp_parts(&self.ramp, self.speed, len, RAMP_PARTS).iter().map(|p| p.1).sum()
        }
    }
}

const RAMP_PARTS: usize = 6;

/// What a shot gets beyond its preset: where its frame looks (an expression in `t`, 0..1 across), filters on the
/// source frame before the crop (blurs), filters at the end (beat hits) and a freeze at its start.
#[derive(Clone, Debug, Default)]
pub struct Extra {
    pub focus: String,
    pub pre: String,
    pub post: String,
    pub freeze: Option<f64>,
}

/// A coloured layer over the picture: flashes, dips, light hits.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flash {
    pub start: f64,
    pub end: f64,
    pub color: String,
    pub opacity: Curve,
    #[serde(default, skip_serializing_if = "is_false")]
    pub blink: bool,
}

/// The song an edit was cut to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub title: String,
    /// seconds into the song where the edit starts
    pub from: f64,
}

fn one() -> f64 {
    1.0
}

fn four() -> f64 {
    4.0
}

fn is_one(v: &f64) -> bool {
    *v == 1.0
}

fn is_four(v: &f64) -> bool {
    *v == 4.0
}

fn is_false(v: &bool) -> bool {
    !*v
}

/// A whole edit as data: what an agent or a person reads, changes and saves as JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    #[serde(default)]
    pub about: String,
    pub w: u32,
    pub h: u32,
    /// seconds
    pub total: f64,
    #[serde(default)]
    pub song: Option<Song>,
    /// colour look (see `edits looks`); None keeps the footage's colours
    #[serde(default)]
    pub look: Option<String>,
    /// 1 = as made; 0.5 calmer, 2 wilder: scales every zoom, slide, turn, shake and blur
    #[serde(default = "one")]
    pub intensity: f64,
    /// with a song other than its own, move each cut onto the nearest beat
    #[serde(default = "yes")]
    pub fit_beats: bool,
    #[serde(rename = "shots")]
    pub slots: Vec<Slot>,
    #[serde(default)]
    pub flashes: Vec<Flash>,
}

fn yes() -> bool {
    true
}

/// JSON for people and agents to read: indented, but each curve key and each list of numbers on one line.
pub fn pretty<T: Serialize>(v: &T) -> String {
    let text = serde_json::to_string_pretty(v).unwrap_or_default();
    let inner = regex::Regex::new(r"\[\s*([^\[\]{}]*?)\s*\]").unwrap();
    let space = regex::Regex::new(r",\s+").unwrap();
    let mut out = text;
    // twice: keys first, then the lists of keys made only of one-line keys
    for _ in 0..2 {
        out = inner.replace_all(&out, |c: &regex::Captures| format!("[{}]", space.replace_all(c[1].trim(), ", "))).into_owned();
    }
    out
}

fn r4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

impl Curve {
    fn tidy(&self) -> Self {
        Curve { keys: self.keys.iter().map(|&(t, v, e)| (r4(t), r4(v), e)).collect() }
    }
}

impl Preset {
    /// Numbers rounded to 4 decimals: the same edit, a file a third the size.
    pub fn tidy(mut self) -> Self {
        self.total = r4(self.total);
        for s in &mut self.slots {
            (s.start, s.end, s.speed, s.shake_hz, s.saturation) = (r4(s.start), r4(s.end), r4(s.speed), r4(s.shake_hz), r4(s.saturation));
            for c in [&mut s.zoom, &mut s.x, &mut s.y, &mut s.rot, &mut s.shake, &mut s.blur, &mut s.exposure] {
                *c = c.tidy();
            }
        }
        for f in &mut self.flashes {
            (f.start, f.end, f.opacity) = (r4(f.start), r4(f.end), f.opacity.tidy());
        }
        self
    }

    /// Every move scaled around its resting value by `k`.
    pub fn with_intensity(mut self, k: f64) -> Self {
        let k = k.clamp(0.0, 3.0);
        for s in &mut self.slots {
            s.zoom = s.zoom.map(|v| 1.0 + (v - 1.0) * k);
            for c in [&mut s.x, &mut s.y, &mut s.rot, &mut s.shake] {
                *c = c.map(|v| v * k);
            }
            s.blur = s.blur.map(|v| (v * k).min(1.0));
        }
        self.intensity = 1.0;
        self
    }

    /// Each cut moved onto the nearest beat (within 0.25 s) of the song it is now cut to.
    pub fn fit_to(mut self, beats: &[f64]) -> Self {
        for i in 1..self.slots.len() {
            let t = self.slots[i].start;
            if let Some(b) = beats.iter().copied().filter(|b| (b - t).abs() <= 0.25).min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs())) {
                if b > self.slots[i - 1].start + 0.1 {
                    self.slots[i].start = b;
                    self.slots[i - 1].end = self.slots[i - 1].end.max(b);
                }
            }
        }
        self
    }
}

fn nums(s: &str) -> Vec<f64> {
    s.split(',').filter_map(|x| x.trim().parse().ok()).collect()
}

/// A property (a transform child or an effect property) as a curve of one component.
fn curve(n: Node, comp: usize) -> Option<Curve> {
    if let Some(v) = n.attribute("value") {
        return nums(v).get(comp).map(|v| Curve::constant(*v));
    }
    let mut keys: Vec<(f64, f64, Ease)> = n.children().filter(|c| c.has_tag_name("kf")).filter_map(|k| {
        Some((k.attribute("t")?.parse::<f64>().ok()?.clamp(0.0, 1.0), *nums(k.attribute("v")?).get(comp)?, ease(k.attribute("e"))))
    }).collect();
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    (!keys.is_empty()).then_some(Curve { keys })
}

fn transform(n: Node, prop: &str, comp: usize) -> Option<Curve> {
    curve(n.children().find(|c| c.has_tag_name("transform"))?.children().find(|c| c.has_tag_name(prop))?, comp)
}

fn effect_prop(n: Node, effect: &str, prop: &str) -> Option<Curve> {
    n.children().filter(|c| c.has_tag_name("effect") && c.attribute("id").is_some_and(|i| i.ends_with(effect)))
        .find_map(|e| curve(e.children().find(|p| p.has_tag_name("property") && p.attribute("name") == Some(prop))?, 0))
}

fn has_effect(n: Node, effect: &str) -> bool {
    n.children().any(|c| c.has_tag_name("effect") && c.attribute("id").is_some_and(|i| i.ends_with(effect)))
}

fn ms(n: Node, a: &str) -> Option<f64> {
    n.attribute(a)?.parse::<f64>().ok().map(|v| v / 1000.0)
}

/// The footage layer inside a group, if any: the group is a slot when it holds a video.
fn media<'a, 'i>(n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    // a video or a still: both are a shot to fill
    let is = |d: Node| d.has_tag_name("shape") && d.attribute("fillType") == Some("media") && (d.attribute("fillVideo").is_some() || d.attribute("fillImage").is_some());
    if is(n) {
        return Some(n);
    }
    n.descendants().skip(1).find(|d| is(*d))
}

/// The null object a layer is parented to: its moves are the layer's moves.
fn parent<'a, 'i>(n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    let id = n.attribute("parent")?;
    n.document().descendants().find(|d| d.has_tag_name("nullobj") && d.attribute("id") == Some(id))
}

fn slot(n: Node, w: f64, h: f64) -> Option<Slot> {
    let m = media(n)?;
    let (start, end) = (ms(n, "startTime")?, ms(n, "endTime")?);
    if end - start < 0.05 {
        return None;
    }
    let speed = |x: Node| x.attribute("speed").and_then(|s| s.parse::<f64>().ok()).unwrap_or(1.0);
    let both = |f: &dyn Fn(Node) -> Option<Curve>| f(n).or_else(|| if m != n { f(m) } else { None });
    // a layer on a null moves with the null
    let tf = |prop: &str, comp: usize| transform(n, prop, comp).filter(Curve::is_moving).or_else(|| parent(n).and_then(|p| transform(p, prop, comp))).or_else(|| transform(n, prop, comp));
    // zoom relative to the layer's own widest framing; position as movement away from where it rests
    let scale = tf("scale", 0).unwrap_or(Curve::constant(1.0));
    let lo = scale.min().max(1e-3);
    let moved = |c: Option<Curve>, size: f64| c.filter(Curve::is_moving).map(|c| {
        let rest = c.keys.iter().map(|k| k.1).min_by(|a, b| (a - size / 2.0).abs().total_cmp(&(b - size / 2.0).abs())).unwrap_or(size / 2.0);
        c.map(|v| (v - rest) / size)
    }).unwrap_or_default();
    let mut shake = both(&|x| effect_prop(x, "oscillate3", "mag").or_else(|| effect_prop(x, "shake2", "mag"))).unwrap_or_default().map(|v| v / w);
    if shake.keys.is_empty() {
        shake = both(&|x| effect_prop(x, "randomdisplace", "mag")).unwrap_or_default().map(|v| v / w * 0.5);
    }
    let hz = both(&|x| effect_prop(x, "oscillate3", "freq").or_else(|| effect_prop(x, "shake2", "freq"))).map_or(4.0, |c| c.max().clamp(0.5, 16.0));
    let blur = both(&|x| effect_prop(x, "gaussianblur", "strength").map(|c| c.map(|v| v / 0.1))
        .or_else(|| effect_prop(x, "lensblurobs", "progressa").map(|c| c.map(|v| v / 0.3)))
        .or_else(|| effect_prop(x, "zoomblur3", "strength"))).unwrap_or_default();
    let saturation = both(&|x| effect_prop(x, "satvib", "saturation")).map_or(1.0, |c| (1.0 + c.max()).clamp(0.0, 2.0));
    Some(Slot {
        start,
        end,
        speed: (speed(n) * if m != n { speed(m) } else { 1.0 }).clamp(0.1, 8.0),
        zoom: Some(scale.map(|v| v / lo)).filter(|z| z.is_moving() || z.max() > 1.001).unwrap_or_default(),
        x: moved(tf("location", 0), w),
        y: moved(tf("location", 1), h),
        rot: tf("rotation", 0).filter(Curve::is_moving).unwrap_or_default(),
        shake,
        shake_hz: hz,
        blur,
        exposure: both(&|x| effect_prop(x, "exposure", "exposure")).unwrap_or_default(),
        saturation,
        rgb: [n, m].iter().any(|x| has_effect(*x, "rgbsepnew") || has_effect(*x, "chromabbr")),
        invert: has_effect(n, "invert") || has_effect(m, "invert"),
        ramp: Curve::default(),
        motion_blur: ["motionblur4", "motionblur2", "motionblur", "moblurplus", "zoomblur3", "spinblur2"].iter().any(|e| has_effect(n, e) || has_effect(m, e)),
    })
}

/// A layer's slots: itself, or for a group that cuts between several videos inside it, each of those, moved to
/// the outer timeline (`at` is where the group's own time starts).
fn collect(n: Node, at: f64, w: f64, h: f64, out: &mut Vec<Slot>) {
    let inner: Vec<Node> = n.children().filter(|c| c.has_tag_name("scene")).flat_map(|sc| sc.children().filter(|c| c.is_element() && media(*c).is_some())).collect();
    let starts = |v: &Vec<Node>| v.iter().filter_map(|c| ms(*c, "startTime")).fold(Vec::<f64>::new(), |mut a, t| { if !a.iter().any(|x| (x - t).abs() < 0.05) { a.push(t) } a });
    if n.has_tag_name("embedScene") && starts(&inner).len() >= 2 {
        let begin = at + ms(n, "startTime").unwrap_or(0.0);
        let (gw, gh) = n.children().find(|c| c.has_tag_name("scene"))
            .map(|sc| (sc.attribute("width").and_then(|v| v.parse().ok()).unwrap_or(w), sc.attribute("height").and_then(|v| v.parse().ok()).unwrap_or(h)))
            .unwrap_or((w, h));
        for c in inner {
            collect(c, begin, gw, gh, out);
        }
    } else if let Some(mut s) = slot(n, w, h) {
        s.start += at;
        s.end += at;
        out.push(s);
    }
}

fn flash(n: Node, w: f64, h: f64) -> Option<Flash> {
    if n.attribute("fillType") != Some("color") {
        return None;
    }
    // only layers that cover the frame: a rectangle scaled up past the canvas
    let size = n.children().find(|c| c.has_tag_name("property") && c.attribute("name") == Some("size")).and_then(|p| p.attribute("value")).map(nums).unwrap_or(vec![200.0, 200.0]);
    let scale = transform(n, "scale", 0).map_or(1.0, |c| c.max());
    if size.first().copied().unwrap_or(0.0) * scale < w * 0.9 || size.get(1).copied().unwrap_or(0.0) * transform(n, "scale", 1).map_or(1.0, |c| c.max()) < h * 0.9 {
        return None;
    }
    let color = n.children().find(|c| c.has_tag_name("fillColor")).and_then(|c| c.attribute("value")).unwrap_or("#ffffffff");
    let rgb = color.trim_start_matches('#');
    let rgb = if rgb.len() == 8 { &rgb[2..] } else { rgb };
    let opacity = transform(n, "opacity", 0).unwrap_or(Curve::constant(1.0));
    // a mask or a full, steady colour is a background, not a flash
    if n.attribute("blending") == Some("mask") || (!opacity.is_moving() && !has_effect(n, "blink2") && n.attribute("blending").is_none()) {
        return None;
    }
    Some(Flash { start: ms(n, "startTime")?, end: ms(n, "endTime")?, color: rgb.to_string(), opacity, blink: has_effect(n, "blink2") })
}

pub fn parse(xml: &str, name: &str) -> Result<Preset, String> {
    let doc = Document::parse(xml).map_err(|e| format!("{name}: not an Alight Motion project ({e})"))?;
    let root = doc.root_element();
    if !root.has_tag_name("scene") {
        return Err(format!("{name}: not an Alight Motion project"));
    }
    let num = |a: &str| root.attribute(a).and_then(|v| v.parse::<f64>().ok());
    let (w, h) = (num("width").unwrap_or(1080.0), num("height").unwrap_or(1920.0));
    let top: Vec<Node> = root.children().filter(|c| c.is_element()).collect();
    let mut slots: Vec<Slot> = vec![];
    for n in &top {
        collect(*n, 0.0, w, h, &mut slots);
    }
    slots.sort_by(|a, b| a.start.total_cmp(&b.start));
    let flashes = top.iter().filter_map(|n| flash(*n, w, h)).collect();
    // the song is the longest sound in the project
    let song = root.descendants().filter(|d| d.has_tag_name("audio"))
        .max_by(|a, b| (ms(*a, "endTime").unwrap_or(0.0) - ms(*a, "startTime").unwrap_or(0.0)).total_cmp(&(ms(*b, "endTime").unwrap_or(0.0) - ms(*b, "startTime").unwrap_or(0.0))))
        .and_then(|a| {
            let label = a.attribute("label")?.trim().trim_end_matches(".mp3").trim_end_matches(".mp4").trim_end_matches(" Mp4").trim_end_matches(" mp4").trim().to_string();
            // only a real title can be found again: two words with letters, not a camera file name or "Copy"
            let words = label.split_whitespace().filter(|w| w.chars().any(char::is_alphabetic)).count();
            let file = ["VID_", "IMG_", "AUD", "REC"].iter().any(|p| label.starts_with(p))
                || [".aac", ".m4a", ".wav", ".ogg", ".opus"].iter().any(|e| label.to_lowercase().ends_with(e))
                || label.chars().collect::<Vec<_>>().windows(6).any(|w| w.iter().all(char::is_ascii_digit));
            (words >= 2 && !file).then(|| Song { title: label, from: ms(a, "inTime").unwrap_or(0.0) })
        });
    if slots.is_empty() {
        return Err(format!("{name}: no video layers to fill"));
    }
    Ok(Preset {
        name: name.to_string(),
        about: format!("Alight Motion project \"{}\"", root.attribute("title").unwrap_or(name)),
        w: (w as u32) / 2 * 2,
        h: (h as u32) / 2 * 2,
        total: num("totalTime").map_or(slots.iter().map(|s| s.end).fold(0.0, f64::max), |t| t / 1000.0),
        song,
        look: None,
        intensity: 1.0,
        fit_beats: true,
        slots,
        flashes,
    }.tidy())
}

/// A preset by file (.json) or by (part of) its name among the built-in and saved ones.
pub fn load(name: &str) -> Result<Preset, String> {
    if name.ends_with(".json") && Path::new(name).is_file() {
        let text = std::fs::read_to_string(name).map_err(|e| format!("{name}: {e}"))?;
        return serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"));
    }
    let want = name.to_lowercase().replace(['-', '_'], " ");
    let all = crate::presets::all();
    let names: Vec<String> = all.iter().map(|p| p.name.clone()).collect();
    all.iter().find(|p| p.name.to_lowercase() == want.trim())
        .or_else(|| all.iter().find(|p| p.name.to_lowercase().replace(['-', '_'], " ").contains(want.trim())))
        .cloned()
        .ok_or(format!("no preset matching \"{name}\"; there are: {}", names.join(", ")))
}

/// A file name for a preset: ascii letters and digits, dashes between.
pub fn slug(name: &str) -> String {
    let s: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "preset".into() } else { s }
}

/// Every Alight Motion project (.xml) in a zip, folder or file.
pub fn read_all(src: &Path) -> Result<Vec<Preset>, String> {
    let tmp = std::env::temp_dir().join(format!("edits-preset-{}", std::process::id()));
    let root = if src.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
        let _ = std::fs::remove_dir_all(&tmp);
        crate::util::run(None, "unzip", &["-q", "-o", &src.to_string_lossy(), "-d", &tmp.to_string_lossy()])?;
        tmp.clone()
    } else {
        src.to_path_buf()
    };
    let mut files = vec![];
    let mut stack = vec![root.clone()];
    while let Some(p) = stack.pop() {
        if p.is_dir() {
            stack.extend(std::fs::read_dir(&p).map_err(|e| e.to_string())?.flatten().map(|e| e.path()));
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")) && !p.to_string_lossy().to_lowercase().contains(".prfpset") {
            files.push(p); // Premiere's .prfpset.xml are cut moves, read elsewhere
        }
    }
    files.sort();
    let mut out = vec![];
    for f in files {
        let xml = std::fs::read_to_string(&f).map_err(|e| e.to_string())?;
        match parse(&xml, f.file_stem().unwrap_or_default().to_string_lossy().trim()) {
            Ok(p) => out.push(p),
            Err(e) => eprintln!("edits: skipped {}: {e}", f.display()),
        }
    }
    if root == tmp {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    Ok(out)
}

/// The ffmpeg chain for one slot's footage, already cut to `len` seconds at the slot's speed. Labels use `p`.
pub fn chain(s: &Slot, i: usize, w: u32, h: u32, x: &Extra, len: f64) -> String {
    let fill = crate::fx::follow(w, h, if x.focus.is_empty() { "0.5" } else { &x.focus });
    let mut f = if s.ramp.is_empty() {
        format!("[{i}:v]setpts=(PTS-STARTPTS)/{:.4},fps=30", s.speed)
    } else {
        // a speed ramp: the shot cut into stretches, each played at its own speed, joined back up
        let parts = crate::fx::ramp_parts(&s.ramp, s.speed, len, RAMP_PARTS);
        let mut g = format!("[{i}:v]fps=30,split={}{};", parts.len(), (0..parts.len()).map(|k| format!("[r{i}x{k}]")).collect::<String>());
        for (k, (from, src, out)) in parts.iter().enumerate() {
            g.push_str(&format!("[r{i}x{k}]trim=start={from:.3}:end={:.3},setpts=(PTS-STARTPTS)/{:.4},fps=30[r{i}y{k}];", from + src, src / out));
        }
        g.push_str(&format!("{}concat=n={}:v=1:a=0,fps=30", (0..parts.len()).map(|k| format!("[r{i}y{k}]")).collect::<String>(), parts.len()));
        g
    };
    f.push_str(&x.pre);
    if let Some(secs) = x.freeze {
        f.push_str(&crate::fx::freeze(secs));
    }
    f.push(',');
    f.push_str(&fill);
    if s.saturation != 1.0 || s.exposure.max() > 0.01 {
        f.push_str(&format!(",eq=saturation={:.2}:brightness='{}':eval=frame", s.saturation, s.exposure.map(|v| v * 0.25).expr("t", len)));
    }
    if s.zoom.is_moving() || s.zoom.max() > 1.001 {
        // capped: a preset framed for a tight crop zooms other footage into a blank patch of colour
        f.push_str(&format!(",zoompan=z='min(2.2,max(1,{}))':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=1:s={w}x{h}:fps=30", s.zoom.expr("it", len)));
    }
    let shaking = s.shake.max() > 1e-4;
    if s.x.is_moving() || s.y.is_moving() || s.rot.is_moving() || shaking {
        // a mirrored 3x3 tile, so moving and turning never shows an edge (Alight's "tile" with mirror)
        let sh = |c: &Curve, phase: f64| if shaking { format!("+{}*sin(2*PI*{:.2}*t+{phase})", c.expr("t", len), s.shake_hz) } else { String::new() };
        let rot = if s.rot.is_moving() { s.rot.expr("t", len) } else { String::new() };
        f.push_str(&moves(&format!("p{i}"), w, h, &format!("{}{}", s.x.expr("t", len), sh(&s.shake, 0.0)), &format!("{}{}", s.y.expr("t", len), sh(&s.shake, 1.3)), &rot));
    }
    if s.blur.max() > 0.25 {
        f.push_str(&format!(",gblur=sigma=14:enable='gt({},0.25)'", s.blur.expr("t", len)));
    }
    if s.motion_blur && (s.speed > 1.3 || s.zoom.is_moving() || s.x.is_moving() || shaking) {
        f.push_str(",tmix=frames=3");
    }
    if s.rgb {
        f.push_str(",rgbashift=rh=-7:bh=7");
    }
    if s.invert {
        f.push_str(",negate");
    }
    f.push_str(&x.post);
    // every shot square-pixelled, or concat refuses to join them
    f.push_str(",setsar=1");
    f.push_str(&format!(",trim=duration={len:.3},setpts=PTS-STARTPTS[v{i}];"));
    f
}

/// Moving and turning a picture without ever showing an edge: a mirrored 3x3 tile (Alight's "tile" with mirror,
/// Premiere's "Mirror"/"Replicate" trick), turned by `rot` degrees, then cut back to w x h offset by `x`, `y`
/// (fractions of the frame; positive moves the picture right / down). Expressions in `t`. `p` makes labels unique.
pub fn moves(p: &str, w: u32, h: u32, x: &str, y: &str, rot: &str) -> String {
    let mut f = format!(
        ",split=3[{p}a][{p}b][{p}c];[{p}a]hflip[{p}l];[{p}c]hflip[{p}r];[{p}l][{p}b][{p}r]hstack=3,split=3[{p}t][{p}m][{p}d];\
         [{p}t]vflip[{p}t2];[{p}d]vflip[{p}d2];[{p}t2][{p}m][{p}d2]vstack=3"
    );
    if !rot.is_empty() {
        f.push_str(&format!(",rotate=a='({rot})*PI/180':c=black"));
    }
    f.push_str(&format!(",crop={w}:{h}:x='{w}-({x})*{w}':y='{h}-({y})*{h}'"));
    f
}

/// A camera move around a cut, from a Premiere Pro preset: the outgoing shot plays the part before `cut`
/// seconds, the incoming one the rest. Curves run over `dur` seconds.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Motion {
    pub name: String,
    pub dur: f64,
    pub cut: f64,
    pub zoom: Curve,
    pub x: Curve,
    pub y: Curve,
    pub rot: Curve,
    pub blur: Curve,
}

impl Motion {
    /// How hard it moves: used to give the hardest beats the biggest moves.
    pub fn strength(&self) -> f64 {
        let span = |c: &Curve| if c.keys.is_empty() { 0.0 } else { c.max() - c.min() };
        span(&self.zoom) + 2.0 * (span(&self.x) + span(&self.y)) + span(&self.rot) / 90.0 + span(&self.blur)
    }
}

/// Flashes over the finished picture, as drawbox layers switched on by their opacity.
pub fn flashes(p: &Preset) -> String {
    p.flashes.iter().filter(|f| f.opacity.max() > 0.05).map(|f| {
        let dur = (f.end - f.start).max(0.01);
        let on = format!("between(t,{:.3},{:.3})*gt({},0.3)", f.start, f.end, f.opacity.expr(&format!("(t-{:.3})", f.start), dur));
        let blink = if f.blink { "*lt(mod(t*12,1),0.5)" } else { "" };
        // white and black flashes hit full; coloured ones were blended in the app (screen, add), so they tint
        let plain = matches!(f.color.to_lowercase().as_str(), "ffffff" | "000000");
        let alpha = (f.opacity.max() * if plain { 0.8 } else { 0.4 }).clamp(0.1, 0.85);
        format!(",drawbox=x=0:y=0:w=iw:h=ih:t=fill:color=0x{}@{alpha:.2}:enable='{on}{blink}'", f.color)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing() {
        assert_eq!(ease(None), Ease::Linear);
        assert_eq!(ease(Some("cubicBezier 0.0 0.0 1.0 1.0")), Ease::Linear);
        assert!((bezier(0.0, 1.0, 0.74, 1.0, 0.5) - 0.94).abs() < 0.02, "{}", bezier(0.0, 1.0, 0.74, 1.0, 0.5));
        assert!(matches!(ease(Some("cubicBezier 0.0 1.0 0.74 1.0")), Ease::Out(k) if k > 3.0));
        assert!(matches!(ease(Some("cubicBezier 0.42 0.0 0.58 1.0")), Ease::InOut(_)));
        assert!(matches!(ease(Some("cubicBezier 0.8 0.0 1.0 0.2")), Ease::In(_)));
        assert!((Ease::Out(4.0).at(0.5) - 0.9375).abs() < 1e-9);
    }

    #[test]
    fn easing_as_words() {
        let c = Curve { keys: vec![(0.0, 1.0, Ease::Linear), (0.5, 2.0, Ease::Out(2.9))] };
        assert_eq!(serde_json::to_string(&c).unwrap(), r#"[[0.0,1.0,"linear"],[0.5,2.0,"out 2.9"]]"#);
        assert_eq!(serde_json::from_str::<Curve>(r#"[[0,1,"inout 3"],[1,2,"linear"]]"#).unwrap().keys[0].2, Ease::InOut(3.0));
        assert!(serde_json::from_str::<Curve>(r#"[[0,1,"bounce"]]"#).is_err());
    }

    #[test]
    fn curves_match_their_expressions() {
        let c = Curve { keys: vec![(0.2, 1.0, Ease::Linear), (0.6, 2.0, Ease::Out(2.0)), (1.0, 1.5, Ease::Linear)] };
        assert_eq!((c.at(0.0), c.at(0.2), c.at(1.0)), (1.0, 1.0, 1.5));
        assert!((c.at(0.4) - 1.75).abs() < 1e-9); // eased out halfway: 1 + 1*(1-0.25)
        let e = c.expr("t", 10.0);
        assert!(e.starts_with("if(lt(t,2.0000),1.0000,if(lt(t,6.0000),(1.0000+1.0000*(1-pow(1-clip((t-2.0000)/4.0000,0,1),2.00)))"), "{e}");
        assert_eq!(Curve::default().expr("t", 1.0), "0");
        assert!(!Curve::constant(3.0).is_moving());
    }

    const XML: &str = r##"<?xml version='1.0' encoding='UTF-8' ?>
<scene title="T" width="1080" height="1200" totalTime="2000" fps="60">
  <bookmark t="1000" />
  <shape id="1" label="bg" startTime="0" endTime="2000" fillType="color"><fillColor value="#ff000000"/><transform><scale value="6,6"/></transform></shape>
  <embedScene id="2" label="Group 1" startTime="0" endTime="1000" fillType="intrinsic" speed="2.0">
    <transform>
      <location value="540,600,0" />
      <scale><kf t="0.0" v="1.6,1.6" /><kf t="0.5" v="1.0,1.0" e="cubicBezier 0.0 1.0 0.74 1.0" /></scale>
    </transform>
    <effect id="com.alightcreative.effects.oscillate3"><property name="mag" type="float"><kf t="0" v="0"/><kf t="0.2" v="54"/><kf t="0.4" v="0"/></property><property name="freq" type="float" value="6"/></effect>
    <effect id="com.alightcreative.effects.motionblur4"><property name="tune" type="float" value="1"/></effect>
    <scene width="2160" height="1200" totalTime="1000">
      <shape id="3" label="clip.mp4" startTime="0" endTime="1000" fillType="media" fillVideo="content://x" speed="1.5"><transform><scale value="1.9,1.9"/></transform></shape>
    </scene>
  </embedScene>
  <shape id="4" label="flash" startTime="1000" endTime="1100" fillType="color" s=".rect"><fillColor value="#ffffffff"/><transform><scale value="5.4,6"/><opacity><kf t="0" v="1"/><kf t="1" v="0"/></opacity></transform><property name="size" type="vec2" value="200,200"/></shape>
  <shape id="5" label="clip2.mp4" startTime="1000" endTime="2000" fillType="media" fillVideo="content://y" speed="1.0">
    <transform><location><kf t="0" v="1620,600,0"/><kf t="0.4" v="540,600,0" e="cubicBezier 0.0 1.0 0.74 1.0"/></location></transform>
    <effect id="com.alightcreative.effects.gaussianblur"><property name="strength" type="float"><kf t="0" v="0.1"/><kf t="0.3" v="0"/></property></effect>
  </shape>
  <audio id="6" label="Montagem Royal - Slowed &amp; Reverb Mp4" startTime="0" endTime="2000" src="content://a" inTime="6484" outTime="8484"/>
</scene>"##;

    #[test]
    fn reads_a_project() {
        let p = parse(XML, "t").unwrap();
        assert_eq!((p.w, p.h, p.total, p.slots.len(), p.flashes.len()), (1080, 1200, 2.0, 2, 1));
        let a = &p.slots[0];
        assert_eq!((a.start, a.end, a.speed), (0.0, 1.0, 3.0)); // group 2x and clip 1.5x
        assert_eq!((a.zoom.at(0.0), a.zoom.at(1.0)), (1.6, 1.0));
        assert!((a.shake.max() - 0.05).abs() < 1e-9 && a.shake_hz == 6.0 && a.motion_blur);
        let b = &p.slots[1];
        assert_eq!((b.x.at(0.0), b.x.at(1.0)), (1.0, 0.0)); // slides in from one frame to the right
        assert!(b.blur.at(0.0) > 0.99 && b.blur.at(1.0) == 0.0);
        assert_eq!((p.flashes[0].color.as_str(), p.flashes[0].blink), ("ffffff", false));
        assert_eq!(p.song, Some(Song { title: "Montagem Royal - Slowed & Reverb".into(), from: 6.484 }));
        assert!(parse("<nope/>", "x").is_err() && parse("garbage", "x").is_err());
    }

    #[test]
    fn chains_for_slots() {
        let p = parse(XML, "t").unwrap();
        let a = chain(&p.slots[0], 0, 1080, 1200, &Extra::default(), 1.0);
        assert!(a.starts_with("[0:v]setpts=(PTS-STARTPTS)/3.0000,fps=30,crop=") && a.contains("zoompan=z='min(2.2,max(1,") && a.contains("hstack=3") && a.contains("tmix=frames=3"), "{a}");
        assert!(a.ends_with("trim=duration=1.000,setpts=PTS-STARTPTS[v0];"), "{a}");
        let b = chain(&p.slots[1], 1, 1080, 1200, &Extra { post: ",negate".into(), freeze: Some(0.3), ..Default::default() }, 1.0);
        assert!(b.contains(",tpad=start_mode=clone:start_duration=0.30,crop=") && b.contains(",negate,setsar=1,trim=duration"), "{b}");
        let mut ramped = p.slots[1].clone();
        ramped.ramp = crate::fx::velocity();
        let r = chain(&ramped, 1, 1080, 1200, &Extra::default(), 1.0);
        assert!(r.starts_with("[1:v]fps=30,split=6[r1x0]") && r.contains("concat=n=6:v=1:a=0,fps=30,crop="), "{r}");
        assert!(ramped.source_len(1.0) > 1.0);
        assert!(b.contains("gblur=sigma=14:enable=") && b.contains("crop=1080:1200:x='1080-(") && !b.contains("zoompan"), "{b}");
        // a preset survives being written out as JSON and read back
        let back: Preset = serde_json::from_str(&serde_json::to_string_pretty(&p).unwrap()).unwrap();
        assert_eq!((back.slots.len(), back.slots[0].zoom.clone(), back.song.clone()), (2, p.slots[0].zoom.clone(), p.song.clone()));
        let calm = p.clone().with_intensity(0.5);
        assert_eq!((calm.slots[0].zoom.at(0.0), calm.slots[1].x.at(0.0)), (1.3, 0.5));
        let fitted = p.clone().fit_to(&[0.0, 0.9, 1.2]);
        assert_eq!(fitted.slots[1].start, 0.9);
        assert_eq!(slug("ʟɪᴋᴋ ᴘʀᴇꜱᴇᴛ #33"), "33");
        let fl = flashes(&p);
        assert!(fl.starts_with(",drawbox=x=0:y=0:w=iw:h=ih:t=fill:color=0xffffff@0.80:enable='between(t,1.000,1.100)*gt("), "{fl}");
    }
}
