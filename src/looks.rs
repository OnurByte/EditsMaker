//! Colour looks: the grades an edit can take, as ffmpeg filter chains, plus any .cube LUT you bring.

/// (name, what it looks like, filter chain starting with a comma)
pub const LOOKS: &[(&str, &str, &str)] = &[
    ("none", "as shot", ""),
    ("bw", "black and white, a little contrast", ",hue=s=0,eq=contrast=1.12:brightness=-0.02"),
    ("noir", "hard black and white, crushed blacks", ",hue=s=0,eq=contrast=1.45:brightness=-0.06:gamma=0.85"),
    ("dark", "muted and dark", ",eq=saturation=0.75:contrast=1.12:brightness=-0.05:gamma=0.95"),
    ("teal", "cool shadows, the reference edits' look", ",colorbalance=rs=-0.08:gs=0.02:bs=0.06:rh=-0.04,eq=saturation=0.8:contrast=1.1"),
    ("tealorange", "teal shadows, orange skin: the film look", ",colorbalance=rs=-0.12:gs=-0.02:bs=0.12:rm=0.06:bm=-0.05:rh=0.1:gh=0.02:bh=-0.08,eq=saturation=1.15:contrast=1.12"),
    ("warm", "golden hour", ",colorbalance=rs=0.05:rm=0.08:gm=0.02:bm=-0.08:rh=0.06:bh=-0.06,eq=saturation=1.1"),
    ("cold", "blue winter", ",colorbalance=rs=-0.06:bs=0.1:rm=-0.05:bm=0.08:bh=0.05,eq=saturation=0.85:contrast=1.08"),
    ("faded", "washed-out film, lifted blacks", ",curves=all='0/0.1 0.5/0.5 1/0.92',eq=saturation=0.8"),
    ("vintage", "old print, warm and soft", ",curves=preset=vintage,eq=saturation=0.85"),
    ("sepia", "brown old photo", ",colorchannelmixer=.393:.769:.189:0:.349:.686:.168:0:.272:.534:.131,eq=contrast=1.05"),
    ("bleach", "bleach bypass: low colour, high contrast", ",eq=saturation=0.45:contrast=1.35:gamma=0.95"),
    ("neon", "pink and purple night", ",colorbalance=rs=0.08:bs=0.15:rm=0.1:gm=-0.08:bm=0.1:rh=0.05:bh=0.1,eq=saturation=1.35:contrast=1.1"),
    ("matrix", "green screen-world", ",colorbalance=rs=-0.1:gs=0.08:bs=-0.06:rm=-0.08:gm=0.08:bm=-0.05,eq=saturation=0.9:contrast=1.15"),
    ("sunset", "red-orange sky", ",colorbalance=rs=0.12:gs=-0.02:bs=-0.08:rm=0.1:bm=-0.06,eq=saturation=1.2:contrast=1.08"),
    ("pop", "loud colour", ",eq=saturation=1.5:contrast=1.15"),
    // lighting looks: a grade plus an off-centre vignette standing in for where the light comes from
    ("lightning", "a distant lightning flash: cold white-blue from high on one side, the rest in deep shadow", ",colorbalance=rs=-0.1:bs=0.12:rm=-0.06:bm=0.08:rh=-0.02:bh=0.06,eq=saturation=0.6:contrast=1.4:brightness=-0.04:gamma=0.9,vignette=angle=PI/3:x0=w*0.85:y0=h*0.1"),
    ("moonlight", "blue moonlight from the side, deep shadows, a faint amber glint", ",colorbalance=rs=-0.12:gs=-0.02:bs=0.16:rm=-0.08:bm=0.1:rh=0.05:gh=0.02:bh=-0.03,eq=saturation=0.75:contrast=1.3:brightness=-0.08:gamma=0.85,vignette=angle=PI/3:x0=w*0.1:y0=h*0.35"),
    ("nightglow", "cool cinematic night: dark base, blue and purple light, warm accents", ",colorbalance=rs=-0.04:bs=0.14:rm=0.04:gm=-0.05:bm=0.1:rh=0.06:bh=-0.02,eq=saturation=1.15:contrast=1.3:brightness=-0.06:gamma=0.9,vignette=PI/4.5"),
    ("daylight", "bright cool daylight for action: crisp, clear, the subject pops", ",colorbalance=rs=-0.02:bs=0.04:rh=0.03:gh=0.02,eq=saturation=1.12:contrast=1.15:brightness=0.03,unsharp=5:5:0.6"),
    ("cyberpunk", "neon night: cyan shadows, hot magenta highlights", ",colorbalance=rs=-0.15:gs=0.05:bs=0.15:rm=0.1:gm=-0.1:bm=0.08:rh=0.15:gh=-0.06:bh=0.08,eq=saturation=1.45:contrast=1.25:brightness=-0.05"),
    ("redalarm", "red emergency light from high on one side, pitch-black shadows", ",colorbalance=rs=0.25:gs=-0.1:bs=-0.1:rm=0.3:gm=-0.12:bm=-0.12:rh=0.15:gh=-0.05:bh=-0.05,eq=saturation=1.2:contrast=1.4:brightness=-0.06:gamma=0.85,vignette=angle=PI/3:x0=w*0.85:y0=h*0.1"),
    ("greenglow", "eerie sci-fi green lighting the face from below", ",colorbalance=rs=-0.15:gs=0.15:bs=-0.08:rm=-0.15:gm=0.2:bm=-0.05:rh=-0.05:gh=0.1,eq=saturation=1.1:contrast=1.35:brightness=-0.05:gamma=0.9,vignette=angle=PI/3:x0=w/2:y0=h*0.95"),
    ("flash", "direct camera flash at night: hard white, punchy, the background falls to black", ",eq=saturation=0.95:contrast=1.35:brightness=0.04:gamma=1.05,unsharp=5:5:0.8,vignette=PI/3.2"),
    ("violet", "fantasy violet: purple-blue shadows, a magenta glow from the side", ",colorbalance=rs=0.02:gs=-0.06:bs=0.18:rm=0.14:gm=-0.12:bm=0.14:rh=0.12:gh=-0.04:bh=0.08,eq=saturation=1.25:contrast=1.3:brightness=-0.05,vignette=angle=PI/3.5:x0=w*0.1:y0=h*0.55"),
];

pub fn names() -> Vec<&'static str> {
    LOOKS.iter().map(|l| l.0).collect()
}

/// The chain for a look by name, "" for an unknown one.
pub fn chain(name: &str) -> &'static str {
    LOOKS.iter().find(|l| l.0 == name).map_or("", |l| l.2)
}

/// A user's .cube LUT, copied to a plain path ffmpeg's filter syntax cannot trip over.
pub fn lut(file: &std::path::Path, cache: &std::path::Path) -> Result<String, String> {
    let dir = cache.join("luts");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let to = dir.join(format!("{}.cube", crate::fetch::safe(&file.file_stem().unwrap_or_default().to_string_lossy())));
    std::fs::copy(file, &to).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(format!(",lut3d=file={}", to.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_are_unique_and_well_formed() {
        let n = names();
        assert!(n.iter().enumerate().all(|(i, a)| !n[i + 1..].contains(a)));
        assert!(LOOKS.iter().all(|l| l.2.is_empty() || l.2.starts_with(',')));
        assert_eq!(chain("nope"), "");
        assert!(chain("teal").contains("colorbalance"));
    }
}
