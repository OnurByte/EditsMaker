//! A small client of the Agent Client Protocol (agentclientprotocol.com), the one t3code speaks to its agents:
//! JSON-RPC, one message a line, over the agent's stdin and stdout. edits asks one question: it starts the agent,
//! opens a session, sends the prompt, gathers the answer's text and stops it. It offers the agent no files, no
//! terminal and no permission: anything it asks for is refused, so it can only answer.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// Longest an agent may take to answer.
const WAIT: Duration = Duration::from_secs(600);

/// What edits answers an agent's request with: a permission is cancelled, anything else is not offered.
fn reply(msg: &Value) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let method = msg["method"].as_str()?;
    Some(if method == "session/request_permission" {
        json!({"jsonrpc": "2.0", "id": id, "result": {"outcome": {"outcome": "cancelled"}}})
    } else {
        json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "edits offers no files or terminal"}})
    })
}

/// The text of an agent's answer chunk, from a session/update.
fn chunk(msg: &Value) -> Option<&str> {
    let u = &msg["params"]["update"];
    (msg["method"] == "session/update" && u["sessionUpdate"] == "agent_message_chunk").then(|| u["content"]["text"].as_str()).flatten()
}

/// Asks the agent `command` (started with the system's shell) one prompt; its answer's text.
pub fn ask(command: &str, prompt: &str) -> Result<String, String> {
    let mut child = if cfg!(windows) { Command::new("cmd").args(["/C", command]).spawn_piped() } else { Command::new("sh").args(["-c", command]).spawn_piped() }
        .map_err(|e| format!("{command}: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let (tx, rx) = mpsc::channel::<Value>();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                if tx.send(v).is_err() {
                    break;
                }
            }
        }
    });
    let mut send = |v: Value| -> Result<(), String> { writeln!(stdin, "{v}").map_err(|e| format!("{command}: {e}")) };
    let cwd = std::env::temp_dir();
    let mut answer = String::new();
    let res = (|| -> Result<(), String> {
        // each step: send a request, then read until its answer comes, serving the agent's own requests meanwhile
        let mut call = |id: u64, method: &str, params: Value, answer: &mut String| -> Result<Value, String> {
            send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
            loop {
                let msg = rx.recv_timeout(WAIT).map_err(|_| format!("{command} stopped answering"))?;
                if let Some(t) = chunk(&msg) {
                    answer.push_str(t);
                } else if msg.get("method").is_some() {
                    if let Some(r) = reply(&msg) {
                        send(r)?;
                    }
                } else if msg["id"] == id {
                    return match msg.get("error") {
                        Some(e) => Err(format!("{command}: {}", e["message"].as_str().unwrap_or("refused"))),
                        None => Ok(msg["result"].clone()),
                    };
                }
            }
        };
        call(1, "initialize", json!({"protocolVersion": 1, "clientCapabilities": {"fs": {"readTextFile": false, "writeTextFile": false}, "terminal": false}}), &mut answer)?;
        let s = call(2, "session/new", json!({"cwd": cwd, "mcpServers": []}), &mut answer)?;
        let id = s["sessionId"].as_str().ok_or(format!("{command} opened no session (signed in?)"))?.to_string();
        call(3, "session/prompt", json!({"sessionId": id, "prompt": [{"type": "text", "text": prompt}]}), &mut answer)?;
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    res?;
    if answer.trim().is_empty() { Err(format!("{command} answered nothing")) } else { Ok(answer) }
}

trait SpawnPiped {
    fn spawn_piped(&mut self) -> std::io::Result<std::process::Child>;
}

impl SpawnPiped for Command {
    fn spawn_piped(&mut self) -> std::io::Result<std::process::Child> {
        self.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_agent_gets_nothing_but_the_question() {
        let p = reply(&json!({"jsonrpc": "2.0", "id": 7, "method": "session/request_permission", "params": {}})).unwrap();
        assert_eq!(p["result"]["outcome"]["outcome"], "cancelled");
        let f = reply(&json!({"jsonrpc": "2.0", "id": "x", "method": "fs/read_text_file", "params": {}})).unwrap();
        assert_eq!((f["id"].as_str(), f["error"]["code"].as_i64()), (Some("x"), Some(-32601)));
        assert!(reply(&json!({"jsonrpc": "2.0", "method": "session/update", "params": {}})).is_none(), "a notification gets no answer");
        let c = json!({"jsonrpc": "2.0", "method": "session/update", "params": {"update": {"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": "hi"}}}});
        assert_eq!(chunk(&c), Some("hi"));
    }

    #[cfg(unix)]
    #[test]
    fn a_whole_conversation() {
        // a pretend agent: answers initialize, session/new, asks a permission, streams two chunks, ends the turn
        let script = r#"read a; echo '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}'
read b; echo '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s1"}}'
read c; echo '{"jsonrpc":"2.0","id":99,"method":"session/request_permission","params":{}}'
read d; case "$d" in *cancelled*) ;; *) exit 1;; esac
echo '{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"[1, "}}}}'
echo '{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"2]"}}}}'
echo '{"jsonrpc":"2.0","id":3,"result":{"stopReason":"end_turn"}}'"#;
        let f = std::env::temp_dir().join(format!("edits-acp-{}.sh", std::process::id()));
        std::fs::write(&f, script).unwrap();
        assert_eq!(ask(&format!("sh {}", f.display()), "pick moments").unwrap(), "[1, 2]");
        let _ = std::fs::remove_file(&f);
    }
}
