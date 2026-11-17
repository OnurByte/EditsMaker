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
    /// colour turned round the wheel, degrees (a curve from 0 to 360 cycles every colour across the shot)
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub hue: Curve,
    /// 1 = as shot, 1.5 = hard, 0.7 = flat
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub contrast: f64,
    /// colour temperature in kelvin, 0 = as shot (3000 orange, 6500 neutral, 12000 blue)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub kelvin: f64,
    /// 0..1: dark corners
    #[serde(default, skip_serializing_if = "is_zero")]
    pub vignette: f64,
    /// 0..1: moving film grain
    #[serde(default, skip_serializing_if = "is_zero")]
    pub grain: f64,
    /// 0..1: bright parts bleed light (glow / bloom)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub glow: f64,
    /// 0..1: sharper edges
    #[serde(default, skip_serializing_if = "is_zero")]
    pub sharpen: f64,
    /// mosaic block size in pixels, 0 = off
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pixelate: f64,
    /// -1..1: lens, above 0 bulges the middle out (fisheye), below pinches it in
    #[serde(default, skip_serializing_if = "is_zero")]
    pub lens: f64,
    /// 0..1: bright moving parts leave ghost trails (echo)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub trails: f64,
    /// mirrored left to right
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub rgb: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub invert: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub motion_blur: bool,
    /// the drop: a --say line ends as this shot starts, wherever the song hits hardest
    #[serde(default, skip_serializing_if = "is_false")]
    pub drop: bool,
    /// a speed ramp over the shot, multiplying `speed` (2.5 = rushing, 0.5 = slow motion)
    #[serde(default, skip_serializing_if = "Curve::is_empty")]
    pub ramp: Curve,
    /// played backwards (rewind)
    #[serde(default, skip_serializing_if = "is_false")]
    pub reverse: bool,
    /// tone curve: a name from `fx::CURVES` or master points "0/0 0.4/0.3 1/1"
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub curves: String,
    /// colour pop: only this colour stays (red, yellow, green, cyan, blue, magenta), the rest goes grey
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pop: String,
    /// 0..1: black cinema bars top and bottom (1 = a 2.39:1 strip on a 16:9 frame)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub letterbox: f64,
    /// split screen: the shot tiled N x N (2 or 3)
    #[serde(default, skip_serializing_if = "is_zero_u")]
    pub split: u32,
    /// left half mirrored onto the right (kaleidoscope symmetry)
    #[serde(default, skip_serializing_if = "is_false")]
    pub mirror: bool,
    /// flicker: light pulses per second, 0 = off
    #[serde(default, skip_serializing_if = "is_zero")]
    pub strobe: f64,
    /// comic / cartoon: edges drawn over flattened colour
    #[serde(default, skip_serializing_if = "is_false")]
    pub comic: bool,
    /// 0..1: VHS / CRT scan lines
    #[serde(default, skip_serializing_if = "is_zero")]
    pub scanlines: f64,
    /// 0..1: a warm light leak drifting over the shot
    #[serde(default, skip_serializing_if = "is_zero")]
    pub leak: f64,
    /// seconds faded in from black at the start, out to black at the end
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fade_in: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fade_out: f64,
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

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_zero_u(v: &u32) -> bool {
    *v == 0
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
    /// earlier names (the project it was imported from): `--preset` still finds it by them
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aka: Vec<String>,
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
    /// What the edit does, measured from its shots, for picking one: pace, the moves it is made of, its frame and
    /// where it came from.
    pub fn tags(&self) -> Vec<&'static str> {
        let n = self.slots.len().max(1) as f64;
        let share = |f: &dyn Fn(&Slot) -> bool| self.slots.iter().filter(|s| f(s)).count() as f64 / n;
        let avg = self.total / n;
        let mut t = vec![if avg < 0.6 { "fast" } else if avg < 1.1 { "medium" } else { "slow" }];
        let r = self.w as f64 / self.h.max(1) as f64;
        t.push(if r < 0.7 { "9:16" } else if r < 0.95 { "4:5" } else if r < 1.15 { "1:1" } else { "wide" });
        for (tag, f, at) in [
            ("zoom", &(|s: &Slot| s.zoom.is_moving()) as &dyn Fn(&Slot) -> bool, 0.3),
            ("slide", &|s: &Slot| s.x.is_moving() || s.y.is_moving(), 0.25),
            ("spin", &|s: &Slot| s.rot.keys.iter().any(|k| k.1.abs() >= 45.0), 0.05),
            ("shake", &|s: &Slot| s.shake.is_moving() || s.shake.max() > 0.0, 0.3),
            ("blur", &|s: &Slot| !s.blur.is_empty(), 0.2),
            ("rgb", &|s: &Slot| s.rgb, 0.2),
            ("velocity", &|s: &Slot| !s.ramp.is_empty() || s.speed > 1.4, 0.2),
            ("split", &|s: &Slot| s.split >= 2 || s.mirror, 0.2),
            ("strobe", &|s: &Slot| s.strobe > 0.0, 0.2),
            ("comic", &|s: &Slot| s.comic, 0.3),
            ("retro", &|s: &Slot| s.scanlines > 0.0 || s.leak > 0.0, 0.3),
            ("cinema", &|s: &Slot| s.letterbox > 0.0, 0.5),
            ("fade", &|s: &Slot| s.fade_in > 0.0 || s.fade_out > 0.0, 0.5),
        ] {
            if share(f) >= at {
                t.push(tag);
            }
        }
        if self.flashes.len() as f64 >= n * 0.1 {
            t.push("flash");
        }
        if self.slots.len() <= 3 {
            t.push("transition");
        }
        if self.song.is_some() {
            t.push("song");
        }
        t
    }

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
        Some((k.attribute("t")?.parse::<f64>().ok()?, *nums(k.attribute("v")?).get(comp)?, ease(k.attribute("e"))))
    }).collect();
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    (!keys.is_empty()).then(|| Curve { keys: clamp_keys(keys) })
}

/// Keys at times outside the layer (0..1), as the layer shows them: the last one before its start becomes its start,
/// the first one after its end its end, and the rest are dropped instead of all stacking on 0 and 1.
fn clamp_keys(keys: Vec<(f64, f64, Ease)>) -> Vec<(f64, f64, Ease)> {
    let before = keys.iter().rposition(|k| k.0 <= 0.0);
    let after = keys.iter().position(|k| k.0 >= 1.0);
    keys.iter().enumerate()
        .filter(|(i, k)| (k.0 > 0.0 && k.0 < 1.0) || Some(*i) == before || Some(*i) == after)
        .map(|(_, k)| (k.0.clamp(0.0, 1.0), k.1, k.2)).collect()
}

/// The scale a layer rests at, which zoom is measured against: its smallest real framing. A layer that pops in from
/// 0 or is flipped (negative) once divided every value by ~0, and its shots came out as a frozen 2.2x crop.
fn rest_scale(scale: &Curve) -> f64 {
    let framed = scale.keys.iter().map(|k| k.1.abs()).filter(|v| *v > 0.05).fold(f64::MAX, f64::min);
    if framed == f64::MAX { 1.0 } else { framed }
}

/// Two curves into one, `f(a, b)` at every key of either; an empty `a` counts as `id`. Each key keeps the easing of
/// the curve it came from.
fn combine(a: &Curve, b: Option<&Curve>, id: f64, f: impl Fn(f64, f64) -> f64) -> Curve {
    let Some(b) = b.filter(|b| !b.is_empty()) else { return a.clone() };
    let a = if a.is_empty() { Curve::constant(id) } else { a.clone() };
    let mut keys: Vec<(f64, f64, Ease)> = a.keys.iter().chain(&b.keys).map(|k| (k.0, f(a.at(k.0), b.at(k.0)), k.2)).collect();
    keys.sort_by(|p, q| p.0.total_cmp(&q.0));
    keys.dedup_by(|p, q| (p.0 - q.0).abs() < 1e-6);
    Curve { keys }
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
    let lo = rest_scale(&scale);
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
    // Alight's Transform effect: a zoom (posz, a factor), a turn (angle, degrees) and an offset on top of the layer's own
    let t2 = |p: &str| both(&|x| effect_prop(x, "transform2", p)).or_else(|| parent(n).and_then(|q| effect_prop(q, "transform2", p)));
    let zoom = combine(&scale.map(|v| (v.abs() / lo).max(0.05)), t2("posz").as_ref(), 1.0, |a, b| a * b.abs().max(0.05));
    // shortcut: posx/posy read as pixels; every pack seen so far leaves them at 0
    let mut x = combine(&moved(tf("location", 0), w), t2("posx").map(|c| c.map(|v| v / w)).as_ref(), 0.0, |a, b| a + b);
    let mut y = combine(&moved(tf("location", 1), h), t2("posy").map(|c| c.map(|v| v / h)).as_ref(), 0.0, |a, b| a + b);
    let rot = combine(&tf("rotation", 0).filter(Curve::is_moving).unwrap_or_default(), t2("angle").as_ref(), 0.0, |a, b| a + b);
    // a wipe revealing the shot: it slides in from the side the wipe opens from, over the wipe's own keys
    if let Some((c, from_end)) = both(&|x| effect_prop(x, "wipe2", "end")).filter(Curve::is_moving).map(|c| (c, true))
        .or_else(|| both(&|x| effect_prop(x, "wipe2", "start")).filter(Curve::is_moving).map(|c| (c, false))) {
        let shown = |v: f64| if from_end { v } else { 1.0 - v }.clamp(0.0, 1.0);
        let slide = c.map(|v| (shown(v) - 1.0) * 0.5);
        let angle = both(&|x| effect_prop(x, "wipe2", "angle")).map_or(90.0, |c| c.at(0.0));
        let side = if (angle.rem_euclid(180.0) - 90.0).abs() < 45.0 { &mut x } else { &mut y };
        if !side.is_moving() {
            *side = slide;
        }
    }
    let shadow = has_effect(n, "dropshadow") || has_effect(m, "dropshadow");
    Some(Slot {
        start,
        end,
        speed: (speed(n) * if m != n { speed(m) } else { 1.0 }).clamp(0.1, 8.0),
        zoom: Some(zoom).filter(|z| z.is_moving() || z.max() > 1.001).unwrap_or_default(),
        x: Some(x).filter(Curve::is_moving).unwrap_or_default(),
        y: Some(y).filter(Curve::is_moving).unwrap_or_default(),
        rot: Some(rot).filter(Curve::is_moving).unwrap_or_default(),
        shake,
        shake_hz: hz,
        blur,
        exposure: both(&|x| effect_prop(x, "exposure", "exposure")).unwrap_or_default(),
        saturation,
        hue: Curve::default(),
        contrast: 1.0,
        kelvin: 0.0,
        // a drop shadow on footage darkens what is around it
        vignette: if shadow { 0.25 } else { 0.0 },
        grain: 0.0,
        glow: if has_effect(n, "edgeglow") || has_effect(m, "edgeglow") { 0.3 } else { 0.0 },
        sharpen: 0.0,
        pixelate: 0.0,
        lens: 0.0,
        trails: 0.0,
        flip: false,
        rgb: [n, m].iter().any(|x| has_effect(*x, "rgbsepnew") || has_effect(*x, "chromabbr")),
        invert: has_effect(n, "invert") || has_effect(m, "invert"),
        ramp: Curve::default(),
        reverse: false,
        curves: String::new(),
        pop: String::new(),
        motion_blur: ["motionblur4", "motionblur2", "motionblur", "moblurplus", "zoomblur3", "spinblur2"].iter().any(|e| has_effect(n, e) || has_effect(m, e)),
        drop: false,
        letterbox: 0.0,
        split: 0,
        mirror: false,
        strobe: 0.0,
        comic: false,
        scanlines: 0.0,
        leak: 0.0,
        fade_in: 0.0,
        fade_out: 0.0,
    })
}

/// Effects every importer path understands (on a footage layer or an adjustment layer); the rest are reported.
const READ: &[&str] = &["lift", "tile", "mirror", "oscillate3", "shake2", "randomdisplace", "jitter", "gaussianblur", "innerblur", "fastboxblur", "lensblurobs",
    "zoomblur3", "spinblur2", "motionblur", "motionblur2", "motionblur4", "moblurplus", "exposure", "brightcont2", "highlightshadow", "threshold",
    "deepglow", "deepglow2", "softglow", "edgeglow", "satvib", "rgbsepnew", "chromabbr", "chromaticzoomblur", "invert", "blink2",
    "transform2", "wipe2", "fade", "ccjaws", "turbulence2", "dropshadow"];

/// Effects that only dress titles and logos (letter spacing, bevels, a gradient plate, a line drawing itself): they
/// belong to a caption style (captions.json: `spacing`, `glow`, `outline`, `shadow`), not to a shot.
const TITLE: &[&str] = &["textspacing", "smoothbevel", "smoothbevelnew", "gradientoverlay2", "progress"];

/// An adjustment layer ("copy background" on a shape: Alight's way of putting an effect over everything under it
/// for a moment) as hits on the shots it covers: a blur pulse, a brightening (exposure, contrast, glow passes),
/// a shake, an RGB split, an inversion, a saturation pop.
fn adjust(n: Node, slots: &mut [Slot], w: f64) {
    if !has_effect(n, "lift") || media(n).is_some() {
        return;
    }
    let (Some(a), Some(b)) = (ms(n, "startTime"), ms(n, "endTime")) else { return };
    let any = |ids: &[&str]| ids.iter().any(|e| has_effect(n, e));
    let peak = |e: &str, p: &str, scale: f64, dflt: f64| effect_prop(n, e, p).map_or(dflt, |c| (c.max().abs() * scale).clamp(0.0, 1.0));
    let blur = any(&["gaussianblur", "innerblur", "fastboxblur", "lensblurobs", "zoomblur3", "chromaticzoomblur"]).then(|| peak("gaussianblur", "strength", 10.0, 0.6));
    let bright = any(&["exposure", "brightcont2", "highlightshadow", "threshold", "deepglow", "deepglow2", "softglow", "edgeglow"]).then(|| peak("exposure", "exposure", 1.0, 0.35));
    let shake = any(&["oscillate3", "shake2", "randomdisplace", "jitter"]).then(|| effect_prop(n, "oscillate3", "mag").or_else(|| effect_prop(n, "shake2", "mag")).map_or(0.03, |c| (c.max() / w).clamp(0.005, 0.1)));
    let sat = effect_prop(n, "satvib", "saturation").map(|c| (1.0 + c.max()).clamp(0.0, 2.0));
    for s in slots.iter_mut().filter(|s| s.start < b && s.end > a) {
        let len = (s.end - s.start).max(1e-3);
        let (ua, ub) = (((a - s.start) / len).clamp(0.0, 1.0), ((b - s.start) / len).clamp(0.0, 1.0));
        // a pulse: on at the layer's start, gone by its end
        let pulse = |v: f64| {
            let mut keys = if ua > 1e-3 { vec![(0.0, 0.0, Ease::Linear), (ua - 1e-3, 0.0, Ease::Linear)] } else { vec![] };
            keys.extend([(ua, v, Ease::Linear), (ub.max(ua + 2e-3).min(1.0), 0.0, Ease::Out(2.0))]);
            Curve { keys }
        };
        if let (Some(v), true) = (blur, s.blur.is_empty()) {
            s.blur = pulse(v);
        }
        if let (Some(v), true) = (bright, s.exposure.is_empty()) {
            s.exposure = pulse(v);
        }
        if let (Some(v), true) = (shake, s.shake.is_empty()) {
            s.shake = pulse(v);
            s.shake_hz = 12.0;
        }
        s.rgb |= any(&["rgbsepnew", "chromabbr", "chromaticzoomblur"]);
        s.invert |= any(&["invert"]);
        if let (Some(v), true) = (sat, ub - ua >= 0.5) {
            s.saturation = v;
        }
    }
}

/// Where a layer sits on the project's timeline: its own start plus the starts of the groups around it.
fn abs_span(n: Node) -> Option<(f64, f64)> {
    let at: f64 = n.ancestors().skip(1).filter(|a| a.has_tag_name("embedScene")).filter_map(|a| ms(a, "startTime")).sum();
    Some((at + ms(n, "startTime")?, at + ms(n, "endTime")?))
}

/// A colour property of an effect, "#AARRGGBB" in the project, as "rrggbb".
fn effect_color(n: Node, effect: &str, prop: &str) -> Option<String> {
    let e = n.children().find(|c| c.has_tag_name("effect") && c.attribute("id").is_some_and(|i| i.ends_with(effect)))?;
    let v = e.children().find(|p| p.has_tag_name("property") && p.attribute("name") == Some(prop))?.attribute("value")?.trim_start_matches('#');
    Some(if v.len() == 8 { &v[2..] } else { v }.to_string())
}

/// Effects that join shots rather than sit on one: fades from and to black (fade), jaws closing over the picture
/// and opening again (ccjaws) as flashes in their colour, and a noise texture over the frame (turbulence2) as grain.
fn transitions(doc: &Document, slots: &mut [Slot], flashes: &mut Vec<Flash>) {
    for n in doc.descendants().filter(|d| d.is_element()) {
        let Some((a, b)) = abs_span(n) else { continue };
        if has_effect(n, "fade") && media(n).is_some() {
            let secs = |p: &str| effect_prop(n, "fade", p).map_or(0.0, |c| c.at(0.0)).clamp(0.0, (b - a) / 2.0);
            let (fin, fout) = (secs("inTime"), secs("outTime"));
            let dip = |start: f64, end: f64, keys: [(f64, f64); 2]| Flash { start, end, color: "000000".into(), blink: false,
                opacity: Curve { keys: keys.iter().map(|k| (k.0, k.1, Ease::Linear)).collect() } };
            if fin > 0.01 {
                flashes.push(dip(a, a + fin, [(0.0, 1.0), (1.0, 0.0)]));
            }
            if fout > 0.01 {
                flashes.push(dip(b - fout, b, [(0.0, 0.0), (1.0, 1.0)]));
            }
        }
        if has_effect(n, "ccjaws") {
            // PROGRESS runs between open and closed: its own range, as 0 (open) .. 1 (closed)
            if let Some(c) = effect_prop(n, "ccjaws", "PROGRESS").filter(Curve::is_moving) {
                let (lo, hi) = (c.min(), c.max());
                flashes.push(Flash { start: a, end: b, color: effect_color(n, "ccjaws", "color1").unwrap_or("000000".into()), blink: false,
                    opacity: c.map(|v| (v - lo) / (hi - lo)) });
            }
        }
        if has_effect(n, "turbulence2") && n.has_tag_name("shape") && n.attribute("fillType") == Some("color") {
            let k = effect_prop(n, "turbulence2", "intensity").map_or(1.0, |c| c.max()) * effect_prop(n, "turbulence2", "alpha").map_or(1.0, |c| c.max());
            for s in slots.iter_mut().filter(|s| s.start < b && s.end > a) {
                s.grain = s.grain.max((k * 0.35).clamp(0.0, 1.0));
            }
        }
    }
}

/// Says which effects a project uses that no preset can carry yet, so the gaps are known instead of silent.
fn report_unread(doc: &Document, name: &str) {
    let mut unread: std::collections::BTreeMap<&str, usize> = Default::default();
    let mut titles = 0;
    for e in doc.descendants().filter(|d| d.has_tag_name("effect")) {
        if let Some(id) = e.attribute("id").and_then(|i| i.rsplit('.').next()) {
            if TITLE.contains(&id) {
                titles += 1;
            } else if !READ.contains(&id) {
                *unread.entry(id).or_default() += 1;
            }
        }
    }
    if titles > 0 {
        eprintln!("edits: {name}: {titles} title/logo styling effects (spacing, bevels, gradient plates) left to caption styles");
    }
    if !unread.is_empty() {
        let mut v: Vec<_> = unread.into_iter().collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.1));
        eprintln!("edits: {name}: effects not carried over: {}", v.iter().map(|(k, n)| format!("{k} x{n}")).collect::<Vec<_>>().join(", "));
    }
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
    // a "copy background" layer shows the picture with its effects, not a solid colour: see `adjust`
    if n.attribute("fillType") != Some("color") || has_effect(n, "lift") {
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
    let mut flashes: Vec<Flash> = top.iter().filter_map(|n| flash(*n, w, h)).collect();
    for n in &top {
        adjust(*n, &mut slots, w);
    }
    transitions(&doc, &mut slots, &mut flashes);
    flashes.sort_by(|a, b| a.start.total_cmp(&b.start));
    report_unread(&doc, name);
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
        aka: vec![],
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
    let known = |p: &Preset| std::iter::once(p.name.clone()).chain(p.aka.clone()).map(|n| n.to_lowercase().replace(['-', '_'], " ")).collect::<Vec<_>>();
    all.iter().find(|p| known(p).iter().any(|n| n.trim() == want.trim()))
        .or_else(|| all.iter().find(|p| known(p).iter().any(|n| n.contains(want.trim()))))
        .cloned()
        .ok_or(format!("no preset matching \"{name}\"; there are: {}", names.join(", ")))
}

/// A file name for a preset: ascii letters and digits, dashes between.
pub fn slug(name: &str) -> String {
    let s: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "preset".into() } else { s }
}

/// A CapCut draft's timeline: draft_content.json, or draft_info.json when there is no draft_content.json beside it.
fn is_capcut(p: &Path) -> bool {
    match p.file_name().and_then(|n| n.to_str()) {
        Some("draft_content.json") => true,
        Some("draft_info.json") => !p.with_file_name("draft_content.json").exists(),
        _ => false,
    }
}

/// Every Alight Motion project (.xml) and CapCut draft in a zip, folder or file.
pub fn read_all(src: &Path) -> Result<Vec<Preset>, String> {
    let tmp = std::env::temp_dir().join(format!("edits-preset-{}", std::process::id()));
    let root = if src.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
        let _ = std::fs::remove_dir_all(&tmp);
        crate::util::unzip(src, &tmp)?;
        tmp.clone()
    } else {
        src.to_path_buf()
    };
    let (mut files, mut drafts) = (vec![], vec![]);
    let mut stack = vec![root.clone()];
    while let Some(p) = stack.pop() {
        if p.is_dir() {
            stack.extend(std::fs::read_dir(&p).map_err(|e| e.to_string())?.flatten().map(|e| e.path()));
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")) && !p.to_string_lossy().to_lowercase().contains(".prfpset") {
            files.push(p); // Premiere's .prfpset.xml are cut moves, read elsewhere
        } else if is_capcut(&p) {
            drafts.push(p);
        }
    }
    files.sort();
    drafts.sort();
    let mut out = vec![];
    for f in drafts {
        // the draft's name as CapCut shows it; its folder is often only an id
        let dir = f.parent().unwrap_or(Path::new("."));
        let shown = std::fs::read_to_string(dir.join("draft_meta_info.json")).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["draft_name"].as_str().map(String::from)).filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| dir.file_name().unwrap_or_default().to_string_lossy().into_owned());
        let text = std::fs::read_to_string(&f).unwrap_or_else(|_| String::from_utf8_lossy(&std::fs::read(&f).unwrap_or_default()).into_owned());
        match crate::capcut::parse(&text, shown.trim()) {
            Ok(p) => out.push(p),
            Err(e) => eprintln!("edits: skipped {}: {e}", f.display()),
        }
    }
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
    if s.reverse {
        // only the shot's own footage is buffered and turned round, so its first frame is the one it would end on
        f.push_str(&format!(",trim=end={len:.3},setpts=PTS-STARTPTS,reverse"));
    }
    f.push_str(&x.pre);
    if let Some(secs) = x.freeze {
        f.push_str(&crate::fx::freeze(secs));
    }
    f.push(',');
    f.push_str(&fill);
    if s.saturation != 1.0 || s.contrast != 1.0 || s.exposure.max() > 0.01 {
        f.push_str(&format!(",eq=saturation={:.2}:contrast={:.2}:brightness='{}':eval=frame", s.saturation, s.contrast, s.exposure.map(|v| v * 0.25).expr("t", len)));
    }
    if s.kelvin > 0.0 {
        f.push_str(&format!(",colortemperature=temperature={:.0}", s.kelvin.clamp(1000.0, 40000.0)));
    }
    if s.flip {
        f.push_str(",hflip");
    }
    if s.lens != 0.0 {
        // ffmpeg's k1 below 0 is barrel: the middle swells toward the viewer; a pinch stays mild, past 0.2 it shows black corners
        let k = -s.lens.clamp(-1.0, 1.0) * if s.lens > 0.0 { 0.5 } else { 0.2 };
        f.push_str(&format!(",lenscorrection=k1={k:.3}:k2={:.3}:i=bilinear", k * 0.5));
        if k > 0.0 {
            // a pinch pulls the edges in: crop past the black and fill the frame again
            f.push_str(&format!(",crop=iw*{0:.3}:ih*{0:.3},scale={w}:{h}", 1.0 - k * 0.9));
        }
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
        f.push_str(&moves(w, h, &format!("{}{}", s.x.expr("t", len), sh(&s.shake, 0.0)), &format!("{}{}", s.y.expr("t", len), sh(&s.shake, 1.3)), &rot, reach(&[&s.x, &s.y]) + if shaking { reach(&[&s.shake]) } else { 0.0 }));
    }
    if s.mirror {
        f.push_str(&format!(",crop=iw/2:ih:0:0,split[mi{i}a][mi{i}b];[mi{i}b]hflip[mi{i}c];[mi{i}a][mi{i}c]hstack,scale={w}:{h}"));
    }
    if (2..=3).contains(&s.split) {
        let n = s.split;
        let (cw, ch) = (w / n / 2 * 2, h / n / 2 * 2);
        let tiles: String = (0..n * n).map(|k| format!("[sp{i}x{k}]")).collect();
        let layout: Vec<String> = (0..n * n).map(|k| format!("{}_{}", k % n * cw, k / n * ch)).collect();
        f.push_str(&format!(",scale={cw}:{ch},split={}{tiles};{tiles}xstack=inputs={}:layout={},scale={w}:{h}", n * n, n * n, layout.join("|")));
    }
    if s.blur.max() > 0.25 {
        f.push_str(&format!(",gblur=sigma=14:enable='gt({},0.25)'", s.blur.expr("t", len)));
    }
    if s.motion_blur && (s.speed > 1.3 || s.zoom.is_moving() || s.x.is_moving() || shaking) {
        f.push_str(",tmix=frames=3");
    }
    if !s.hue.is_empty() && (s.hue.max().abs() > 0.5 || s.hue.is_moving()) {
        f.push_str(&format!(",hue=h='{}'", s.hue.expr("t", len)));
    }
    f.push_str(&crate::fx::curves(&s.curves).unwrap_or_default());
    f.push_str(&crate::fx::pop(&s.pop).unwrap_or_default());
    if s.trails > 0.0 {
        f.push_str(&format!(",lagfun=decay={:.3}", 0.8 + 0.18 * s.trails.min(1.0)));
    }
    if s.glow > 0.0 {
        // blended in RGB: a screen blend on YUV planes tints the picture purple
        f.push_str(&format!(",format=gbrp,split[gl{i}a][gl{i}b];[gl{i}b]gblur=sigma=24[gl{i}c];[gl{i}a][gl{i}c]blend=all_mode=screen:all_opacity={:.2},format=yuv420p", s.glow.min(1.0)));
    }
    if s.sharpen > 0.0 {
        f.push_str(&format!(",unsharp=5:5:{:.2}", s.sharpen.min(1.0) * 2.0));
    }
    if s.pixelate >= 2.0 {
        f.push_str(&format!(",pixelize=w={0:.0}:h={0:.0}", s.pixelate.min(200.0)));
    }
    if s.comic {
        f.push_str(",edgedetect=mode=colormix:high=0.15:low=0.05,eq=saturation=1.4");
    }
    if s.rgb {
        f.push_str(",rgbashift=rh=-7:bh=7");
    }
    if s.invert {
        f.push_str(",negate");
    }
    if s.grain > 0.0 {
        f.push_str(&format!(",noise=alls={:.0}:allf=t+u", s.grain.min(1.0) * 60.0));
    }
    if s.vignette > 0.0 {
        f.push_str(&format!(",vignette=angle={:.3}", 0.3 + s.vignette.min(1.0) * 0.6));
    }
    if s.leak > 0.0 {
        // screened in RGB like the glow; the gradient's colours drift round as the shot plays
        f.push_str(&format!(",format=gbrp[lk{i}a];gradients=s={w}x{h}:r=30:c0=0xff7a1f:c1=0x000000:c2=0xffc070:c3=0x000000:nb_colors=4:speed=0.04:d={:.3}[lk{i}g];[lk{i}g]format=gbrp[lk{i}h];[lk{i}a][lk{i}h]blend=all_mode=screen:all_opacity={:.2}:shortest=1,format=yuv420p", len + 1.0, s.leak.min(1.0) * 0.7));
    }
    if s.scanlines > 0.0 {
        f.push_str(&format!(",drawgrid=w=iw:h=4:t=1:color=black@{:.2}", 0.15 + 0.45 * s.scanlines.min(1.0)));
    }
    if s.strobe > 0.0 {
        f.push_str(&format!(",eq=brightness='0.35*lt(mod(t*{:.2},1),0.5)':eval=frame", s.strobe.min(15.0)));
    }
    f.push_str(&x.post);
    if s.letterbox > 0.0 {
        // after the beat hits, so a shake moves the picture and not the bars
        let bar = (h as f64 * 0.12 * s.letterbox.min(1.0)).round();
        f.push_str(&format!(",drawbox=x=0:y=0:w=iw:h={bar}:t=fill:color=black,drawbox=x=0:y=ih-{bar}:w=iw:h={bar}:t=fill:color=black"));
    }
    if s.fade_in > 0.0 {
        f.push_str(&format!(",fade=t=in:st=0:d={:.3}", s.fade_in.min(len)));
    }
    if s.fade_out > 0.0 {
        let d = s.fade_out.min(len);
        f.push_str(&format!(",fade=t=out:st={:.3}:d={d:.3}", len - d));
    }
    // every shot square-pixelled, or concat refuses to join them
    f.push_str(",setsar=1");
    f.push_str(&format!(",trim=duration={len:.3},setpts=PTS-STARTPTS[v{i}];"));
    f
}

/// Moving and turning a picture without ever showing an edge: a mirrored 3x3 tile (Alight's "tile" with mirror,
/// Premiere's "Mirror"/"Replicate" trick), turned by `rot` degrees, then cut back to w x h offset by `x`, `y`
/// (fractions of the frame; positive moves the picture right / down). Expressions in `t`. `reach`: the largest
/// offset the expressions give, 1 when unknown.
pub fn moves(w: u32, h: u32, x: &str, y: &str, rot: &str, reach: f64) -> String {
    let mut f = format!(",pad=3*iw:3*ih:iw:ih,fillborders=left={w}:right={w}:top={h}:bottom={h}:mode=mirror");
    let (mut mx, mut my) = (w, h);
    if !rot.is_empty() {
        // turning costs by the canvas: turn only as much of the tile as the slide can reach
        let margin = |d: u32| (((reach * 1.25).clamp(0.0, 1.0) * d as f64).ceil() as u32).div_ceil(2) * 2;
        (mx, my) = (margin(w), margin(h));
        f.push_str(&format!(",rotate=a='({rot})*PI/180':ow={}:oh={}:c=black", w + 2 * mx, h + 2 * my));
    }
    f.push_str(&format!(",crop={w}:{h}:x='{mx}-({x})*{w}':y='{my}-({y})*{h}'"));
    f
}

/// The furthest these curves go from zero: how far a slide or shake can push the picture.
pub fn reach(curves: &[&Curve]) -> f64 {
    curves.iter().flat_map(|c| c.keys.iter().map(|k| k.1.abs())).fold(0.0, f64::max)
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
    fn turning_uses_only_the_reach() {
        // no slide: the turn runs on the frame itself, not the 3x3 tile
        assert!(moves(1080, 1920, "0", "0", "t*90", 0.0).ends_with("rotate=a='(t*90)*PI/180':ow=1080:oh=1920:c=black,crop=1080:1920:x='0-(0)*1080':y='0-(0)*1920'"));
        assert!(moves(1080, 1920, "0.2*t", "0", "t*90", 0.2).contains(":ow=1620:oh=2880:c=black,crop=1080:1920:x='270-(0.2*t)*1080'"));
        // unknown or large slides keep the whole tile; without a turn nothing changes
        assert!(moves(1080, 1920, "x", "y", "r", 9.0).contains(":ow=3240:oh=5760:"));
        assert!(moves(1080, 1920, "x", "0", "", 0.1).ends_with("mode=mirror,crop=1080:1920:x='1080-(x)*1080':y='1920-(0)*1920'"));
        assert_eq!(reach(&[&Curve::default(), &Curve { keys: vec![(0.0, -0.4, Ease::Linear), (1.0, 0.3, Ease::Linear)] }]), 0.4);
    }

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
    fn transform_fade_and_jaws_carry_over() {
        let xml = r##"<scene width="1000" height="1000" totalTime="3000">
<embedScene startTime="1000" endTime="3000"><scene width="1000" height="1000">
 <shape startTime="0" endTime="2000" fillType="media" fillVideo="a.mp4">
  <effect id="com.alightcreative.effects.transform2"><property name="posz"><kf t="0" v="1"/><kf t="1" v="1.5"/></property>
   <property name="angle"><kf t="0" v="0"/><kf t="1" v="20"/></property><property name="posx" value="0"/></effect>
  <effect id="com.alightcreative.effects.fade"><property name="inTime" value="0"/><property name="outTime" value="0.2"/></effect>
 </shape></scene></embedScene>
<shape startTime="0" endTime="1000" fillType="media" fillVideo="b.mp4"/>
<shape startTime="500" endTime="1000" fillType="color"><effect id="com.alightcreative.effects.lift"/>
 <effect id="com.alightcreative.effects.ccjaws"><property name="PROGRESS"><kf t="0" v="-6"/><kf t="1" v="-4"/></property><property name="color1" value="#ffffffff"/></effect></shape>
</scene>"##;
        let p = parse(xml, "t").unwrap();
        let a = p.slots.iter().find(|s| s.start == 0.0).unwrap();
        assert!(a.zoom.is_empty() && a.x.is_empty());
        let b = p.slots.iter().find(|s| s.start == 1.0).unwrap();
        assert_eq!((b.zoom.at(0.0), b.zoom.at(1.0), b.rot.at(1.0)), (1.0, 1.5, 20.0));
        assert!(b.x.is_empty()); // a still posx of 0 is no move
        // the group's fade-out ends on the timeline at 3 s, the jaws close white over 0.5..1 s
        let f: Vec<_> = p.flashes.iter().map(|f| (f.start, f.end, f.color.as_str(), f.opacity.at(1.0))).collect();
        assert!(f.contains(&(0.5, 1.0, "ffffff", 1.0)) && f.iter().any(|x| (x.0 - 2.8).abs() < 1e-9 && x.1 == 3.0 && x.2 == "000000"), "{f:?}");
    }

    #[test]
    fn chains_for_slots() {
        let p = parse(XML, "t").unwrap();
        let a = chain(&p.slots[0], 0, 1080, 1200, &Extra::default(), 1.0);
        assert!(a.starts_with("[0:v]setpts=(PTS-STARTPTS)/3.0000,fps=30,crop=") && a.contains("zoompan=z='min(2.2,max(1,") && a.contains("fillborders=left=") && a.contains("tmix=frames=3"), "{a}");
        assert!(a.ends_with("trim=duration=1.000,setpts=PTS-STARTPTS[v0];"), "{a}");
        let b = chain(&p.slots[1], 1, 1080, 1200, &Extra { post: ",negate".into(), freeze: Some(0.3), ..Default::default() }, 1.0);
        assert!(b.contains(",tpad=start_mode=clone:start_duration=0.30,crop=") && b.contains(",negate,setsar=1,trim=duration"), "{b}");
        let mut ramped = p.slots[1].clone();
        ramped.ramp = crate::fx::velocity();
        let r = chain(&ramped, 1, 1080, 1200, &Extra::default(), 1.0);
        assert!(r.starts_with("[1:v]fps=30,split=6[r1x0]") && r.contains("concat=n=6:v=1:a=0,fps=30,crop="), "{r}");
        assert!(ramped.source_len(1.0) > 1.0);
        let mut tinted = p.slots[1].clone();
        assert!(!chain(&tinted, 1, 1080, 1200, &Extra::default(), 1.0).contains(",hue="));
        tinted.hue = Curve { keys: vec![(0.0, 0.0, Ease::Linear), (1.0, 360.0, Ease::Linear)] };
        assert!(chain(&tinted, 1, 1080, 1200, &Extra::default(), 1.0).contains(",hue=h='"));
        let plain = chain(&p.slots[1], 1, 1080, 1200, &Extra::default(), 1.0);
        let all: Slot = serde_json::from_value(serde_json::json!({"start": 0.0, "end": 1.0, "contrast": 1.4, "kelvin": 3000.0, "vignette": 0.5, "grain": 0.3,
            "glow": 0.6, "sharpen": 0.5, "pixelate": 12.0, "lens": 0.6, "trails": 0.5, "flip": true, "reverse": true, "curves": "vintage", "pop": "red",
            "letterbox": 0.5, "split": 2, "mirror": true, "strobe": 6.0, "comic": true, "scanlines": 0.5, "leak": 0.5, "fade_in": 0.2, "fade_out": 0.3})).unwrap();
        let c = chain(&all, 1, 1080, 1200, &Extra::default(), 1.0);
        for want in ["contrast=1.40", "colortemperature=temperature=3000", "vignette=angle=", "noise=alls=18", "blend=all_mode=screen:all_opacity=0.60",
            "unsharp=5:5:1.00", "pixelize=w=12:h=12", "lenscorrection=k1=-0.300", "lagfun=decay=0.890", "hflip", "trim=end=1.000,setpts=PTS-STARTPTS,reverse",
            "curves=preset=vintage", "huesaturation=colors=y+g+c+b+m", "hflip[mi1c]", "xstack=inputs=4:layout=0_0|540_0|0_600|540_600", "edgedetect=mode=colormix",
            "drawgrid=w=iw:h=4", "gradients=s=1080x1200", "lt(mod(t*6.00,1),0.5)", "drawbox=x=0:y=ih-72:w=iw:h=72", "fade=t=in:st=0:d=0.200", "fade=t=out:st=0.700:d=0.300"] {
            assert!(c.contains(want) && !plain.contains(want), "{want}: {c}");
        }
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

    #[test]
    fn every_shot_effect_runs_in_ffmpeg() {
        let Ok(_) = std::process::Command::new("ffmpeg").arg("-version").output() else { return };
        let all: Slot = serde_json::from_value(serde_json::json!({"start": 0.0, "end": 1.0, "zoom": [[0.0, 1.0, "linear"], [1.0, 1.2, "out 2"]],
            "shake": [[0.0, 0.02, "linear"]], "glow": 0.5, "letterbox": 0.5, "split": 3, "mirror": true, "strobe": 6.0, "comic": true,
            "scanlines": 0.5, "leak": 0.5, "fade_in": 0.2, "fade_out": 0.3})).unwrap();
        let post = crate::fx::hit(&["strobe".into(), "bw".into(), "pulse".into()], 360, 640);
        let g = chain(&all, 0, 360, 640, &Extra { post, ..Default::default() }, 1.0);
        let out = std::process::Command::new("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "testsrc=s=640x360:d=2:r=30", "-filter_complex", g.trim_end_matches(';'), "-map", "[v0]", "-f", "null", "-"]).output().unwrap();
        assert!(out.status.success(), "{}\n{g}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn zoom_from_a_pop_in_or_a_flip_is_not_a_thousand() {
        let c = |v: &[f64]| Curve { keys: v.iter().enumerate().map(|(i, &x)| (i as f64, x, Ease::Linear)).collect() };
        // a layer popping in from 0 to its framing: zoom is relative to that framing, not to ~0
        assert_eq!(rest_scale(&c(&[0.0, 0.625, 0.625])), 0.625);
        // flipped (negative) scale rests at its size
        assert_eq!(rest_scale(&c(&[-1.0, -1.33])), 1.0);
        assert_eq!(rest_scale(&c(&[0.0])), 1.0);
    }

    #[test]
    fn keys_outside_a_layer_do_not_stack() {
        let k = clamp_keys(vec![(-0.4, 9.0, Ease::Linear), (-0.1, 1.5, Ease::Linear), (0.5, 1.2, Ease::Linear), (1.2, 1.0, Ease::Linear), (1.6, 7.0, Ease::Linear)]);
        assert_eq!(k.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>(), vec![(0.0, 1.5), (0.5, 1.2), (1.0, 1.0)]);
    }

    #[test]
    fn a_real_shaped_alight_project_keeps_its_moves_sane() {
        let p = parse(include_str!("../tests/fixtures/alight/regressions.xml"), "regressions").unwrap();
        assert_eq!(p.slots.len(), 3);
        let zoom: Vec<(f64, f64)> = p.slots[0].zoom.keys.iter().map(|k| (k.0, k.1)).collect();
        assert_eq!(zoom, vec![(0.0, 0.05), (0.3, 1.2), (1.0, 1.0)], "pop-in from 0, keys outside the layer dropped");
        assert_eq!(p.slots[1].zoom.keys.iter().map(|k| k.1).collect::<Vec<_>>(), vec![1.0, 1.3], "mirrored, not -1000");
        assert!(p.flashes.is_empty(), "a copy-background layer is not a white flash");
        let blur = &p.slots[2].blur;
        assert!((blur.at(0.2) - 0.6).abs() < 1e-6 && blur.at(0.1) == 0.0 && blur.at(0.9) == 0.0, "a blur pulse where the adjustment layer sits: {blur:?}");
    }
}
