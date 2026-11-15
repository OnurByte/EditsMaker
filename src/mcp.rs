//! `edits mcp`: every command as an MCP tool over stdio (newline-delimited JSON-RPC 2.0), so an AI agent can make
//! edits, read a preset, change it for the moment and save it. Each tool runs this same binary.
use serde_json::{json, Value};
use std::io::{BufRead, Write};

fn tools() -> Value {
    json!([
        {"name": "edits_make", "description": "Make edits. `input` is a YouTube link, a channel link, a topic to search, or a video file. With `preset` (a name, or a full preset object in `preset_json` changed for this edit) it replays that edit over the footage; with `music` (a song link/file or \"trend:N\" for TikTok's Nth trending song) it cuts music edits. Returns one JSON line per finished clip.",
         "inputSchema": {"type": "object", "properties": {
            "input": {"type": "string"},
            "preset": {"type": "string", "description": "preset name from edits_presets"},
            "preset_json": {"type": "object", "description": "a whole preset (as from edits_preset_get), changed as needed; used as-is"},
            "music": {"type": "string"},
            "look": {"type": "string", "description": "colour look from edits_looks: a built-in grade or one of the Alight Motion colour-pack looks (furkancc-1..4, rajjcc-1..20)"},
            "aspect": {"type": "string", "enum": ["9:16", "4:5", "1:1", "3:4", "4:3", "16:9", "21:9"]},
            "size": {"type": "string", "description": "WxH, like 1080x1350"},
            "intensity": {"type": "number", "description": "preset moves: 0.5 calmer, 2 wilder"},
            "n": {"type": "integer", "minimum": 1, "maximum": 20},
            "videos": {"type": "integer", "description": "videos to look at for a channel or topic"},
            "use": {"type": "integer", "description": "of those, how many to download after reading captions"},
            "sad": {"type": "boolean"}, "voice": {"type": "boolean"},
            "speed": {"type": "number", "description": "footage speed (talking clips: speech speed), 0.5 slow motion, 2 double"},
            "velocity": {"type": "boolean", "description": "speed ramps on every longer shot: slow, rushing, slow onto the beat"},
            "fx": {"type": "string", "description": "hits on the hardest beats, comma-separated: shake, flash, rgb, glitch, invert, blur, freeze, zoom"},
            "lines": {"type": "integer", "description": "sentences from the videos' own words shown on screen (music and preset edits)"},
            "line_texts": {"type": "array", "items": {"type": "string"}, "description": "your own lines on screen, in order (read the videos' words and pick the strongest)"},
            "captions": {"type": "string", "description": "caption style from edits_looks: hormozi, mrbeast, karaoke, box, minimal, neon, cinematic, glow, redglow, thunder, typewriter"},
            "bed": {"type": "string", "description": "music under talking clips, ducked under the voice"},
            "blur_faces": {"type": "boolean"}, "blur_box": {"type": "string", "description": "x,y,w,h fractions of the frame to blur (a watermark, a plate)"},
            "stabilize": {"type": "boolean"}, "denoise": {"type": "boolean"}, "smooth": {"type": "boolean", "description": "motion-interpolated frames"},
            "fps": {"type": "integer"}, "codec": {"type": "string", "enum": ["h264", "h265", "prores", "av1"]}, "quality": {"type": "string", "enum": ["draft", "high", "master"]},
            "svgs": {"type": "array", "items": {"type": "string"}, "description": "drawn graphics over the edit to point things out: NAME[@FROM-TO][:X,Y,SIZE][#COLOUR], e.g. arrow@1.2-2.5:0.6,0.3,0.25 or circle@4:0.5,0.35#ffffff. Names: arrow, circle, underline, cross, check, burst, question, exclaim, heart, sparkle, or an .svg file. X,Y is the centre and SIZE the width, as fractions of the frame; times are seconds of the finished edit"},
            "export": {"type": "string", "description": "timelines beside each clip: fcpxml,edl,srt"},
            "tag": {"type": "string"}, "out": {"type": "string", "description": "output folder"}},
          "required": ["input"]}},
        {"name": "edits_presets", "description": "List the presets: name, shots, length, frame, song.", "inputSchema": {"type": "object", "properties": {}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_preset_get", "description": "A preset in full as JSON: frame, song, look, intensity, and every shot with its speed and its zoom/x/y/rot/shake/blur/exposure curves ([time 0..1 of the shot, value, easing]) plus flashes. Change it and pass it to edits_make as preset_json, or keep it with edits_preset_save.",
         "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_preset_save", "description": "Save a preset (a whole preset object). Saved presets replace a built-in one with the same name.",
         "inputSchema": {"type": "object", "properties": {"preset": {"type": "object"}}, "required": ["preset"]}},
        {"name": "edits_learn", "description": "Reverse-engineer a finished edit (an mp4 the user likes) into a preset: its cuts, flashes and each shot's zoom, slide, turn and shake. Saved like any preset; use it with edits_make preset, read or change it with edits_preset_get.",
         "inputSchema": {"type": "object", "properties": {"video": {"type": "string", "description": "path of the edit"}}, "required": ["video"]}},
        {"name": "edits_looks", "description": "Everything to style an edit with: colour looks, caption styles, the beat hits for fx and the drawn graphics for svgs.", "inputSchema": {"type": "object", "properties": {}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_trends", "description": "What is trending on TikTok now in a country (two letters, default tr). Use one with music \"trend:N\".",
         "inputSchema": {"type": "object", "properties": {"country": {"type": "string"}}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_doctor", "description": "Check the tools edits needs (ffmpeg, yt-dlp, whisper).", "inputSchema": {"type": "object", "properties": {}}, "annotations": {"readOnlyHint": true}}
    ])
}

/// The command line a tool call stands for.
pub fn args_for(name: &str, a: &Value) -> Result<Vec<String>, String> {
    let s = |k: &str| a[k].as_str().map(String::from);
    let mut v: Vec<String> = vec![];
    match name {
        "edits_presets" => v.extend(["preset".into(), "list".into()]),
        "edits_preset_get" => v.extend(["preset".into(), "show".into(), s("name").ok_or("name is needed")?]),
        "edits_preset_save" => v.extend(["preset".into(), "save".into(), "-".into()]),
        "edits_looks" => v.push("looks".into()),
        "edits_learn" => v.extend(["learn".into(), s("video").ok_or("video is needed")?]),
        "edits_doctor" => v.push("doctor".into()),
        "edits_trends" => v.extend(["trends".into(), s("country").unwrap_or_else(|| crate::lang::system().1)]),
        "edits_make" => {
            v.push(s("input").ok_or("input is needed")?);
            for (key, flag) in [("preset", "--preset"), ("music", "--music"), ("look", "--look"), ("aspect", "--aspect"), ("size", "--size"), ("tag", "--tag"), ("out", "--out"),
                ("fx", "--fx"), ("captions", "--captions"), ("bed", "--bed"), ("blur_box", "--blur-box"), ("codec", "--codec"), ("quality", "--quality"), ("export", "--export")] {
                if let Some(x) = s(key) {
                    v.extend([flag.into(), x]);
                }
            }
            for (key, flag) in [("n", "-n"), ("videos", "--videos"), ("use", "--use"), ("intensity", "--intensity"), ("speed", "--speed"), ("lines", "--lines"), ("fps", "--fps")] {
                if let Some(x) = a[key].as_f64() {
                    v.extend([flag.into(), if matches!(key, "intensity" | "speed") { x.to_string() } else { (x as u64).to_string() }]);
                }
            }
            for t in a["line_texts"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--line".into(), t.into()]);
            }
            for t in a["svgs"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--svg".into(), t.into()]);
            }
            for (key, flag) in [("sad", "--sad"), ("voice", "--voice"), ("velocity", "--velocity"), ("blur_faces", "--blur-faces"), ("stabilize", "--stabilize"), ("denoise", "--denoise"), ("smooth", "--smooth")] {
                if a[key].as_bool() == Some(true) {
                    v.push(flag.into());
                }
            }
        }
        _ => return Err(format!("no tool {name}")),
    }
    Ok(v)
}

fn call(name: &str, a: &Value) -> Result<String, String> {
    let mut args = args_for(name, a)?;
    let mut stdin = None;
    if name == "edits_preset_save" {
        stdin = Some(a["preset"].to_string());
    }
    // a changed preset for this edit only: written to a file and named by path
    let tmp = std::env::temp_dir().join(format!("edits-mcp-{}.json", std::process::id()));
    if name == "edits_make" && a["preset_json"].is_object() {
        std::fs::write(&tmp, a["preset_json"].to_string()).map_err(|e| e.to_string())?;
        args.extend(["--preset".into(), tmp.to_string_lossy().into_owned()]);
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = std::process::Command::new(exe).args(&args)
        .stdin(if stdin.is_some() { std::process::Stdio::piped() } else { std::process::Stdio::null() })
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped())
        .spawn().map_err(|e| e.to_string())?;
    if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
        let _ = pipe.write_all(text.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp);
    let (o, e) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    // progress lines help an agent see what was skipped and why; keep the last of them
    let log: Vec<&str> = e.lines().rev().take(40).collect::<Vec<_>>().into_iter().rev().collect();
    let text = format!("{}{}", o.trim(), if log.is_empty() { String::new() } else { format!("\n\n--- log ---\n{}", log.join("\n")) });
    if out.status.success() { Ok(text) } else { Err(text) }
}

fn reply(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

pub fn handle(msg: &Value) -> Option<Value> {
    let id = msg.get("id")?.clone(); // notifications get no answer
    let method = msg["method"].as_str().unwrap_or("");
    Some(match method {
        "initialize" => reply(&id, json!({"protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": {"tools": {}}, "serverInfo": {"name": "edits", "version": env!("CARGO_PKG_VERSION")}})),
        "ping" => reply(&id, json!({})),
        "tools/list" => reply(&id, json!({"tools": tools()})),
        "tools/call" => {
            let name = msg["params"]["name"].as_str().unwrap_or("");
            let r = call(name, &msg["params"]["arguments"]);
            let (text, err) = match r { Ok(t) => (t, false), Err(t) => (t, true) };
            reply(&id, json!({"content": [{"type": "text", "text": text}], "isError": err}))
        }
        _ => json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("no method {method}")}}),
    })
}

pub fn serve() -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let answer = match serde_json::from_str::<Value>(&line) {
            Ok(m) => handle(&m),
            Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}})),
        };
        if let Some(a) = answer {
            writeln!(stdout, "{a}").map_err(|e| e.to_string())?;
            stdout.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol() {
        let init = handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}})).unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "edits");
        assert!(handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none());
        let list = handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).unwrap();
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 8);
        assert_eq!(handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "nope"})).unwrap()["error"]["code"], -32601);
    }

    #[test]
    fn tool_calls_become_command_lines() {
        let a = json!({"input": "https://www.youtube.com/@x", "preset": "megumi", "music": "trend:2", "n": 2, "intensity": 1.5, "sad": true, "voice": false, "fx": "shake,freeze", "velocity": true, "lines": 3});
        assert_eq!(args_for("edits_make", &a).unwrap(), ["https://www.youtube.com/@x", "--preset", "megumi", "--music", "trend:2", "--fx", "shake,freeze", "-n", "2", "--intensity", "1.5", "--lines", "3", "--sad", "--velocity"]);
        assert_eq!(args_for("edits_preset_get", &json!({"name": "yura"})).unwrap(), ["preset", "show", "yura"]);
        assert!(args_for("edits_make", &json!({})).is_err() && args_for("nope", &json!({})).is_err());
    }
}
