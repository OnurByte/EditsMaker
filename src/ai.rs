//! The AI that reads transcripts and picks moments. Every provider t3code has: the agents on your subscription,
//! run once with the prompt (Claude, Codex, Cursor, OpenCode, Pi, Muse Code) or over ACP (Grok, fx, Antigravity, Kiro,
//! Gemini CLI, and any ACP agent by its command: acp.rs); an API key (OpenAI, Anthropic, Gemini, OpenRouter); or a
//! server on this computer (Ollama, LM Studio, llama.cpp, vLLM). `edits ai` saves the choice; EDITS_LLM, a command
//! reading the prompt on stdin, still wins when set.
use crate::util::{curl_json, home};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub endpoint: String,
    /// t3code's runtime mode: "" (supervised: the agent only answers, everything it asks to do is refused) or "full"
    /// (full access: it runs with its own permission prompts skipped)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access: String,
}

/// (name, API base, key's environment variable) of the providers reached over HTTP.
const PROVIDERS: &[(&str, &str, &str)] = &[
    ("openai", "https://api.openai.com/v1", "OPENAI_API_KEY"),
    ("anthropic", "https://api.anthropic.com/v1", "ANTHROPIC_API_KEY"),
    ("gemini", "https://generativelanguage.googleapis.com/v1beta", "GEMINI_API_KEY"),
    ("openrouter", "https://openrouter.ai/api/v1", "OPENROUTER_API_KEY"),
    ("ollama", "http://127.0.0.1:11434/v1", ""),
    ("lmstudio", "http://127.0.0.1:1234/v1", ""),
    ("llamacpp", "http://127.0.0.1:8080/v1", ""),
    ("vllm", "http://127.0.0.1:8000/v1", ""),
];

/// t3code's agents: (name, what t3code calls it, how edits runs it, its command). A `once` command reads the prompt
/// on stdin and prints the answer ({model} is where `--model M` goes); an `acp` one is spoken to over ACP.
const AGENTS: &[(&str, &str, &str, &str)] = &[
    ("claude-code", "Claude", "once", "claude -p{model}"),
    ("codex", "Codex", "once", "codex exec --skip-git-repo-check{model} -"),
    ("cursor", "Cursor", "once", "cursor-agent -p --output-format text{model} \"$(cat)\""),
    ("grok", "Grok", "acp", "grok agent stdio"),
    ("fx", "fx", "acp", "fx acp"),
    ("opencode", "OpenCode", "once", "opencode run{model} \"$(cat)\""),
    ("antigravity", "Antigravity", "acp", "agy acp"),
    ("kiro", "Kiro", "acp", "kiro-cli acp --agent-engine=v3 --auth-method=cli"),
    ("muse", "Muse Code", "once", "muse -p \"$(cat)\""),
    ("pi", "Pi", "once", "pi -p{model} \"$(cat)\""),
    ("gemini-cli", "Gemini CLI", "acp", "gemini --experimental-acp"),
    // the ACP Registry's any other agent: its command is the choice's endpoint (edits ai acp --endpoint "devin acp")
    ("acp", "ACP Registry", "acp", ""),
];

fn agent(name: &str) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    AGENTS.iter().find(|a| a.0 == name)
}

/// Every provider's name, agents first.
fn all_names() -> Vec<&'static str> {
    AGENTS.iter().map(|a| a.0).chain(PROVIDERS.iter().map(|p| p.0)).collect()
}


fn file() -> PathBuf {
    home().join(".config/edits/ai.json")
}

/// Each provider as the app's AI settings show it, after t3code's provider cards: what t3code calls it, how it is
/// reached (an agent on your subscription, an API key, a server on this computer), whether it is ready here, whether
/// it is the one in use.
/// A line on each provider, as t3code's Add provider dialog says it.
fn about(name: &str) -> &'static str {
    match name {
        "claude-code" => "Anthropic's coding agent, on your Claude plan.",
        "codex" => "OpenAI's coding agent, on your ChatGPT plan.",
        "cursor" => "Cursor's agent, signed in with your Cursor account.",
        "grok" => "xAI's Grok Build agent, spoken to over ACP.",
        "fx" => "The fx agent, spoken to over ACP.",
        "opencode" => "The open-source agent, with whichever model you gave it.",
        "antigravity" => "Google's Antigravity agent and its Gemini models, over ACP.",
        "kiro" => "Amazon's Kiro agent, over ACP.",
        "muse" => "Muse's coding agent.",
        "pi" => "The Pi coding agent, with whichever model you gave it.",
        "gemini-cli" => "Google's Gemini CLI, over ACP.",
        "acp" => "Any agent that speaks ACP, by its command: Devin, Goose, Qwen Code and more.",
        "openai" => "GPT models, with your OpenAI API key.",
        "anthropic" => "Claude models, with your Anthropic API key.",
        "gemini" => "Gemini models, with a Google AI Studio key.",
        "openrouter" => "Hundreds of models behind one key.",
        "ollama" => "Models running here with Ollama; nothing leaves this computer.",
        "lmstudio" => "Models running here in LM Studio; nothing leaves this computer.",
        "llamacpp" => "A llama.cpp server on this computer.",
        _ => "A vLLM server on this computer.",
    }
}

/// The models t3code's picker offers for an agent that picks its own: its aliases, the newest of each size.
fn agent_models(name: &str) -> &'static [&'static str] {
    match name {
        "claude-code" => &["opus", "sonnet", "haiku"],
        "gemini-cli" => &["gemini-2.5-pro", "gemini-2.5-flash"],
        _ => &[],
    }
}

/// How an agent runs with full access: its own flag for skipping its permission prompts.
fn full_access_flag(name: &str) -> &'static str {
    match name {
        "claude-code" => " --dangerously-skip-permissions",
        "codex" => " --dangerously-bypass-approvals-and-sandbox",
        "cursor" => " --force",
        _ => "",
    }
}

/// A JWT's claims, unchecked: enough to say whose plan it is, never to trust it.
fn jwt_claims(token: &str) -> Option<Value> {
    let out = crate::util::base64_decode(token.split('.').nth(1)?);
    serde_json::from_slice(&out).ok()
}

fn read_json(p: PathBuf) -> Option<Value> {
    std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok())
}

/// "Pro"/"plus" as a plan's name: "Claude Pro", "ChatGPT Plus".
fn plan(of: &str, kind: &str) -> String {
    let mut k: Vec<char> = kind.replace(['_', '-'], " ").chars().collect();
    if let Some(c) = k.first_mut() {
        *c = c.to_ascii_uppercase();
    }
    format!("{of} {}", k.into_iter().collect::<String>()).trim().to_string()
}

/// Whose subscription an agent runs on and which plan, as t3code shows it under the provider: read from the agent's
/// own sign-in on this computer, (plan, name or email). Nothing is sent anywhere.
pub fn account(name: &str) -> (String, String) {
    match name {
        "claude-code" => {
            let creds = read_json(home().join(".claude/.credentials.json"));
            let kind = creds.as_ref().and_then(|c| c["claudeAiOauth"]["subscriptionType"].as_str().map(String::from)).unwrap_or_default();
            let who = read_json(home().join(".claude.json")).map(|v| v["oauthAccount"].clone()).unwrap_or_default();
            let n = who["displayName"].as_str().or(who["emailAddress"].as_str()).unwrap_or_default().to_string();
            (if kind.is_empty() { String::new() } else { plan("Claude", &kind) }, n)
        }
        "codex" => {
            let auth = read_json(home().join(".codex/auth.json")).unwrap_or_default();
            if auth["OPENAI_API_KEY"].as_str().is_some_and(|k| !k.is_empty()) && auth["tokens"].is_null() {
                return ("API key".into(), String::new());
            }
            let claims = auth["tokens"]["id_token"].as_str().and_then(jwt_claims).unwrap_or_default();
            let kind = claims["https://api.openai.com/auth"]["chatgpt_plan_type"].as_str().unwrap_or_default();
            (if kind.is_empty() { String::new() } else { plan("ChatGPT", kind) }, claims["email"].as_str().unwrap_or_default().to_string())
        }
        "gemini-cli" => (String::new(), read_json(home().join(".gemini/google_accounts.json")).and_then(|v| v["active"].as_str().map(String::from)).unwrap_or_default()),
        n if PROVIDERS.iter().any(|p| p.0 == n && !p.2.is_empty()) => (if key(n).is_ok() { "API key".into() } else { String::new() }, String::new()),
        n if PROVIDERS.iter().any(|p| p.0 == n) => ("On this computer".into(), String::new()),
        _ => Default::default(),
    }
}

/// One AI call, kept for Settings, Usage: when, which, how long, how much went in and out, whether it answered.
fn note_usage(c: &Choice, secs: f64, prompt: usize, answer: &Result<String, String>) {
    let line = json!({"at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()), "provider": c.provider, "model": c.model,
        "secs": (secs * 10.).round() / 10., "in": prompt, "out": answer.as_ref().map_or(0, |a| a.len()), "ok": answer.is_ok()});
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(usage_file()) {
        let _ = writeln!(f, "{line}");
    }
}

pub fn usage_file() -> PathBuf {
    home().join(".config/edits/usage.jsonl")
}

pub fn providers() -> Vec<Value> {
    let c = choice();
    let agents = AGENTS.iter().map(|&(name, label, how, cmd)| {
        let bin = cmd.split_whitespace().next().unwrap_or("");
        let ready = if cmd.is_empty() { c.provider == name && !c.endpoint.is_empty() } else { crate::util::have(bin) };
        let (plan, who) = if ready { account(name) } else { Default::default() };
        json!({"name": name, "label": label, "kind": if cmd.is_empty() { "acp" } else { "subscription" }, "how": how, "command": if cmd.is_empty() && c.provider == name { c.endpoint.as_str() } else { cmd },
            "ready": ready, "inUse": c.provider == name, "base": "", "keyVar": "", "about": about(name), "plan": plan, "account": who, "models": agent_models(name)})
    });
    let http = PROVIDERS.iter().map(|&(name, base, var)| {
        let kind = if var.is_empty() { "local" } else { "api" };
        let label = match name { "openai" => "OpenAI", "anthropic" => "Anthropic API", "gemini" => "Gemini API", "openrouter" => "OpenRouter", "ollama" => "Ollama", "lmstudio" => "LM Studio", "llamacpp" => "llama.cpp", _ => "vLLM" };
        // a server here counts as added once it is the one in use; a key, once it is saved
        let ready = if kind == "local" { c.provider == name } else { key(name).is_ok() };
        let (plan, who) = if ready { account(name) } else { Default::default() };
        json!({"name": name, "label": label, "kind": kind, "how": "http", "command": "", "ready": ready, "inUse": c.provider == name,
            "base": if c.provider == name && !c.endpoint.is_empty() { c.endpoint.as_str() } else { base }, "keyVar": var, "about": about(name), "plan": plan, "account": who, "models": []})
    });
    agents.chain(http).collect()
}

/// Saves a provider's API key, in a file only this user can read.
pub fn save_key(provider: &str, k: &str) -> Result<(), String> {
    if !PROVIDERS.iter().any(|x| x.0 == provider && !x.2.is_empty()) || k.trim().is_empty() {
        return Err(format!("{provider} takes no API key"));
    }
    let mut all: Value = std::fs::read_to_string(keys_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}));
    all[provider] = json!(k.trim());
    // shortcut: keys sit in a file only this user can read, not the system keyring
    crate::util::write_private(&keys_file(), &all.to_string())
}

/// The models a provider offers, at its endpoint (or its own), for the app's AI settings.
#[cfg(feature = "app")]
pub fn models_of(provider: &str, endpoint: &str) -> Result<Vec<String>, String> {
    models(&Choice { provider: provider.into(), endpoint: endpoint.into(), ..Default::default() })
}
fn keys_file() -> PathBuf {
    home().join(".config/edits/keys.json")
}

pub fn choice() -> Choice {
    std::fs::read_to_string(file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| Choice { provider: "claude-code".into(), ..Default::default() })
}

fn base(c: &Choice) -> String {
    if !c.endpoint.is_empty() { c.endpoint.trim_end_matches('/').to_string() } else { PROVIDERS.iter().find(|p| p.0 == c.provider).map_or("", |p| p.1).to_string() }
}

fn key(provider: &str) -> Result<String, String> {
    let var = PROVIDERS.iter().find(|p| p.0 == provider).map_or("", |p| p.2);
    if var.is_empty() {
        return Ok(String::new());
    }
    std::env::var(var).ok().filter(|k| !k.trim().is_empty())
        .or_else(|| std::fs::read_to_string(keys_file()).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok())?[provider].as_str().map(str::to_string))
        .ok_or(format!("no {provider} key: set {var} or run `edits ai key {provider}`"))
}

/// The command an agent (or EDITS_LLM) runs, None for an HTTP provider; and whether it speaks ACP.
fn command(c: &Choice) -> Option<String> {
    if let Ok(cmd) = std::env::var("EDITS_LLM") {
        return Some(cmd);
    }
    let a = agent(&c.provider)?;
    if a.3.is_empty() {
        return Some(c.endpoint.clone()).filter(|e| !e.trim().is_empty());
    }
    let mut m = if c.model.is_empty() { String::new() } else { format!(" --model {}", c.model) };
    if c.access == "full" {
        m += full_access_flag(&c.provider);
    }
    Some(a.3.replace("{model}", &m))
}

fn acp(c: &Choice) -> bool {
    std::env::var("EDITS_LLM").is_err() && agent(&c.provider).is_some_and(|a| a.2 == "acp")
}

/// What answers prompts, in words (doctor, info).
pub fn describe() -> String {
    let c = choice();
    command(&c).unwrap_or_else(|| format!("{} {} ({})", c.provider, c.model, base(&c)))
}

/// Whether the chosen AI is there to ask: its command installed, or its key set.
pub fn ready() -> bool {
    let c = choice();
    match command(&c) {
        Some(cmd) => crate::util::have(cmd.split_whitespace().next().unwrap_or("")),
        None if agent(&c.provider).is_some() => false,
        None => key(&c.provider).is_ok(),
    }
}

/// The prompt answered by the chosen AI, and the call noted for Usage.
pub fn ask(prompt: &str) -> Result<String, String> {
    let (c, t) = (choice(), std::time::Instant::now());
    let r = ask_with(&c, prompt);
    note_usage(&c, t.elapsed().as_secs_f64(), prompt.len(), &r);
    r
}

fn ask_with(c: &Choice, prompt: &str) -> Result<String, String> {
    let c = c.clone();
    if let Some(cmd) = command(&c) {
        return if acp(&c) { crate::acp::ask(&cmd, prompt, c.access == "full") } else { crate::util::run_stdin(&cmd, prompt) };
    }
    if agent(&c.provider).is_some() {
        return Err(format!("{} needs its command: edits ai {} --endpoint \"AGENT acp\"", c.provider, c.provider));
    }
    let (b, k) = (base(&c), key(&c.provider)?);
    let model = if c.model.is_empty() { models(&c)?.into_iter().next().ok_or("the AI server lists no model")? } else { c.model.clone() };
    let v = match c.provider.as_str() {
        "anthropic" => curl_json(&[("url", &format!("{b}/messages")), ("header", &format!("x-api-key: {k}")), ("header", "anthropic-version: 2023-06-01"), ("header", "content-type: application/json"),
            ("data-binary", &json!({"model": model, "max_tokens": 8192, "messages": [{"role": "user", "content": prompt}]}).to_string())])?["content"][0]["text"].clone(),
        "gemini" => curl_json(&[("url", &format!("{b}/models/{model}:generateContent")), ("header", &format!("x-goog-api-key: {k}")), ("header", "content-type: application/json"),
            ("data-binary", &json!({"contents": [{"parts": [{"text": prompt}]}]}).to_string())])?["candidates"][0]["content"]["parts"][0]["text"].clone(),
        _ => {
            let mut o = vec![("url", format!("{b}/chat/completions")), ("header", "content-type: application/json".to_string()),
                ("data-binary", json!({"model": model, "messages": [{"role": "user", "content": prompt}]}).to_string())];
            if !k.is_empty() {
                o.push(("header", format!("Authorization: Bearer {k}")));
            }
            curl_json(&o.iter().map(|(a, b)| (*a, b.as_str())).collect::<Vec<_>>())?["choices"][0]["message"]["content"].clone()
        }
    };
    v.as_str().map(str::to_string).ok_or(format!("{} answered with no text", c.provider))
}

/// The models a provider offers.
fn models(c: &Choice) -> Result<Vec<String>, String> {
    let b = base(c);
    if b.is_empty() {
        return Err(format!("{} picks its own model; name one with edits ai {} MODEL", c.provider, c.provider));
    }
    let k = key(&c.provider)?;
    let v = match c.provider.as_str() {
        "anthropic" => curl_json(&[("url", &format!("{b}/models?limit=100")), ("header", &format!("x-api-key: {k}")), ("header", "anthropic-version: 2023-06-01")])?,
        "gemini" => curl_json(&[("url", &format!("{b}/models?pageSize=200")), ("header", &format!("x-goog-api-key: {k}"))])?,
        _ if k.is_empty() => curl_json(&[("url", &format!("{b}/models"))])?,
        _ => curl_json(&[("url", &format!("{b}/models")), ("header", &format!("Authorization: Bearer {k}"))])?,
    };
    let list = v["data"].as_array().or(v["models"].as_array()).cloned().unwrap_or_default();
    Ok(list.iter().filter_map(|m| m["id"].as_str().or(m["name"].as_str()).map(|s| s.trim_start_matches("models/").to_string())).collect())
}

/// `edits ai [PROVIDER [MODEL] [--endpoint URL]] | key PROVIDER | models`
pub fn cmd(args: &[String]) -> Result<(), String> {
    let names = all_names().join(", ");
    match args.first().map(String::as_str) {
        None => {
            println!("AI: {}{}", describe(), if ready() { "" } else { "  (not ready: its command or key is missing)" });
            println!("choose with: edits ai PROVIDER [MODEL] [--endpoint URL]   ({names})");
            Ok(())
        }
        Some("key") => {
            let p = args.get(1).filter(|p| PROVIDERS.iter().any(|x| x.0 == p.as_str() && !x.2.is_empty())).ok_or("usage: edits ai key openai|anthropic|gemini|openrouter   (the key is read from the terminal)")?;
            let k = crate::social::read_secret(&format!("{p} API key: "))?;
            save_key(p, &k)?;
            println!("{p} key saved");
            Ok(())
        }
        Some("access") => {
            let mut c = choice();
            c.access = match args.get(1).map(String::as_str) {
                Some("full") => "full".into(),
                Some("ask" | "supervised") => String::new(),
                _ => return Err("usage: edits ai access full|supervised   (full: the agent runs with its own permission prompts skipped)".into()),
            };
            std::fs::create_dir_all(file().parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(file(), serde_json::to_string_pretty(&c).unwrap_or_default()).map_err(|e| e.to_string())?;
            println!("AI access: {}", if c.access == "full" { "full access" } else { "supervised" });
            Ok(())
        }
        Some("models") => {
            models(&choice())?.iter().for_each(|m| println!("{m}"));
            Ok(())
        }
        Some(p) => {
            if !all_names().contains(&p) {
                return Err(format!("unknown AI {p}; one of: {names}"));
            }
            let opt = args.iter().position(|a| a == "--endpoint").and_then(|i| args.get(i + 1)).cloned().unwrap_or_default();
            // the access mode stays what it was when the provider or model changes
            let c = Choice { provider: p.into(), model: args.get(1).filter(|m| !m.starts_with("--")).cloned().unwrap_or_default(), endpoint: opt, access: choice().access };
            let api = PROVIDERS.iter().find(|x| x.0 == p).is_some_and(|x| !x.2.is_empty());
            if p == "acp" && c.endpoint.is_empty() {
                return Err("the ACP Registry takes the agent's command: edits ai acp --endpoint \"devin acp\"".into());
            }
            if api && c.model.is_empty() {
                let m = models(&c)?;
                return Err(format!("name a model: edits ai {p} MODEL\n{}", m.iter().take(40).map(|m| format!("  {m}")).collect::<Vec<_>>().join("\n")));
            }
            std::fs::create_dir_all(file().parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(file(), serde_json::to_string_pretty(&c).unwrap_or_default()).map_err(|e| e.to_string())?;
            println!("AI: {}", describe());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_and_bases() {
        let c = |p: &str, m: &str| Choice { provider: p.into(), model: m.into(), ..Default::default() };
        if std::env::var("EDITS_LLM").is_err() {
            assert_eq!(command(&c("claude-code", "")).unwrap(), "claude -p");
            assert_eq!(command(&c("codex", "o4")).unwrap(), "codex exec --skip-git-repo-check --model o4 -");
            assert!(command(&c("openai", "x")).is_none());
            assert_eq!(command(&c("kiro", "")).unwrap(), "kiro-cli acp --agent-engine=v3 --auth-method=cli");
            assert!(acp(&c("antigravity", "")) && !acp(&c("claude-code", "")));
            assert_eq!(command(&Choice { provider: "acp".into(), endpoint: "devin acp".into(), ..Default::default() }).unwrap(), "devin acp");
            let full = Choice { access: "full".into(), ..c("claude-code", "opus") };
            assert_eq!(command(&full).unwrap(), "claude -p --model opus --dangerously-skip-permissions", "full access skips its prompts");
            assert!(command(&c("acp", "")).is_none(), "the registry's agent needs its command");
        }
        assert_eq!(base(&c("ollama", "")), "http://127.0.0.1:11434/v1");
        assert_eq!(base(&Choice { provider: "ollama".into(), endpoint: "http://box:11434/v1/".into(), ..Default::default() }), "http://box:11434/v1");
    }

    #[test]
    fn a_plan_is_read_from_the_sign_in() {
        // {"email":"a@b.c","https://api.openai.com/auth":{"chatgpt_plan_type":"plus"}}
        let claims = jwt_claims("x.eyJlbWFpbCI6ImFAYi5jIiwiaHR0cHM6Ly9hcGkub3BlbmFpLmNvbS9hdXRoIjp7ImNoYXRncHRfcGxhbl90eXBlIjoicGx1cyJ9fQ.y").unwrap();
        assert_eq!(claims["email"], "a@b.c");
        assert_eq!(plan("ChatGPT", claims["https://api.openai.com/auth"]["chatgpt_plan_type"].as_str().unwrap()), "ChatGPT Plus");
        assert_eq!(plan("Claude", "max"), "Claude Max");
    }
}
