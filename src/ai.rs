//! The AI that reads transcripts and picks moments, as in Tehom: a subscription CLI (Claude Code, Codex), an API key
//! (OpenAI, Anthropic, Gemini, OpenRouter) or a server on this computer (Ollama, LM Studio, llama.cpp, vLLM).
//! `edits ai` saves the choice; EDITS_LLM, a command reading the prompt on stdin, still wins when set.
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
}

/// (name, API base, key's environment variable)
const PROVIDERS: &[(&str, &str, &str)] = &[
    ("claude-code", "", ""),
    ("codex", "", ""),
    ("openai", "https://api.openai.com/v1", "OPENAI_API_KEY"),
    ("anthropic", "https://api.anthropic.com/v1", "ANTHROPIC_API_KEY"),
    ("gemini", "https://generativelanguage.googleapis.com/v1beta", "GEMINI_API_KEY"),
    ("openrouter", "https://openrouter.ai/api/v1", "OPENROUTER_API_KEY"),
    ("ollama", "http://127.0.0.1:11434/v1", ""),
    ("lmstudio", "http://127.0.0.1:1234/v1", ""),
    ("llamacpp", "http://127.0.0.1:8080/v1", ""),
    ("vllm", "http://127.0.0.1:8000/v1", ""),
];

fn file() -> PathBuf {
    home().join(".config/edits/ai.json")
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

/// The command a subscription CLI or EDITS_LLM runs, None for an HTTP provider.
fn command(c: &Choice) -> Option<String> {
    if let Ok(cmd) = std::env::var("EDITS_LLM") {
        return Some(cmd);
    }
    let m = if c.model.is_empty() { String::new() } else { format!(" --model {}", c.model) };
    match c.provider.as_str() {
        "claude-code" => Some(format!("claude -p{m}")),
        "codex" => Some(format!("codex exec --skip-git-repo-check{m} -")),
        _ => None,
    }
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
        None => key(&c.provider).is_ok(),
    }
}

/// The prompt answered by the chosen AI.
pub fn ask(prompt: &str) -> Result<String, String> {
    let c = choice();
    if let Some(cmd) = command(&c) {
        return crate::util::run_stdin(&cmd, prompt);
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
    let names = PROVIDERS.iter().map(|p| p.0).collect::<Vec<_>>().join(", ");
    match args.first().map(String::as_str) {
        None => {
            println!("AI: {}{}", describe(), if ready() { "" } else { "  (not ready: its command or key is missing)" });
            println!("choose with: edits ai PROVIDER [MODEL] [--endpoint URL]   ({names})");
            Ok(())
        }
        Some("key") => {
            let p = args.get(1).filter(|p| PROVIDERS.iter().any(|x| x.0 == p.as_str() && !x.2.is_empty())).ok_or("usage: edits ai key openai|anthropic|gemini|openrouter   (the key is read from the terminal)")?;
            let k = crate::social::read_secret(&format!("{p} API key: "))?;
            let mut all: Value = std::fs::read_to_string(keys_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}));
            all[p.as_str()] = json!(k);
            // shortcut: keys sit in a file only this user can read, not the system keyring
            crate::util::write_private(&keys_file(), &all.to_string())?;
            println!("{p} key saved");
            Ok(())
        }
        Some("models") => {
            models(&choice())?.iter().for_each(|m| println!("{m}"));
            Ok(())
        }
        Some(p) => {
            if !PROVIDERS.iter().any(|x| x.0 == p) {
                return Err(format!("unknown AI {p}; one of: {names}"));
            }
            let opt = args.iter().position(|a| a == "--endpoint").and_then(|i| args.get(i + 1)).cloned().unwrap_or_default();
            let c = Choice { provider: p.into(), model: args.get(1).filter(|m| !m.starts_with("--")).cloned().unwrap_or_default(), endpoint: opt };
            let api = !PROVIDERS.iter().find(|x| x.0 == p).unwrap().2.is_empty();
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
        let c = |p: &str, m: &str| Choice { provider: p.into(), model: m.into(), endpoint: String::new() };
        if std::env::var("EDITS_LLM").is_err() {
            assert_eq!(command(&c("claude-code", "")).unwrap(), "claude -p");
            assert_eq!(command(&c("codex", "o4")).unwrap(), "codex exec --skip-git-repo-check --model o4 -");
            assert!(command(&c("openai", "x")).is_none());
        }
        assert_eq!(base(&c("ollama", "")), "http://127.0.0.1:11434/v1");
        assert_eq!(base(&Choice { provider: "ollama".into(), model: String::new(), endpoint: "http://box:11434/v1/".into() }), "http://box:11434/v1");
    }
}
