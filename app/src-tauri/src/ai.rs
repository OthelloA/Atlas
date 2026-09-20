use crate::bedrock::BedrockClient;
use serde_json::Value;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// AI backend used for generated repository overviews and audits.
///
/// Model selectors are intentionally explicit:
/// - `arn:...` uses AWS Bedrock
/// - `codex:<model>` uses the local Codex CLI
/// - `claude:<model>` or an unprefixed model uses the local Claude CLI
pub enum AiBackend {
    Bedrock {
        client: BedrockClient,
        model_arn: String,
    },
    ClaudeCli {
        model: String,
    },
    CodexCli {
        model: String,
    },
}

impl AiBackend {
    pub async fn new(model: &str, aws_profile: &str) -> Result<Self, String> {
        let model = model.trim();
        if model.starts_with("arn:") {
            let region = crate::bedrock::region_from_arn(model)?;
            let client = BedrockClient::new(&region, aws_profile).await?;
            Ok(AiBackend::Bedrock {
                client,
                model_arn: model.to_string(),
            })
        } else if let Some(model) = parse_provider_model(model, "codex") {
            Ok(AiBackend::CodexCli { model })
        } else {
            let model = parse_provider_model(model, "claude").unwrap_or_else(|| model.to_string());
            if model.is_empty() {
                return Err("AI model cannot be empty".to_string());
            }
            Ok(AiBackend::ClaudeCli { model })
        }
    }

    pub async fn invoke(&self, prompt: &str) -> Result<String, String> {
        match self {
            AiBackend::Bedrock { client, model_arn } => {
                client.invoke_model(model_arn, prompt).await
            }
            AiBackend::ClaudeCli { model } => invoke_claude_cli(model, prompt).await,
            AiBackend::CodexCli { model } => invoke_codex_cli(model, prompt).await,
        }
    }
}

fn parse_provider_model(value: &str, provider: &str) -> Option<String> {
    let prefix = format!("{}:", provider);
    let slash_prefix = format!("{}/", provider);
    value
        .strip_prefix(&prefix)
        .or_else(|| value.strip_prefix(&slash_prefix))
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
}

async fn invoke_claude_cli(model: &str, prompt: &str) -> Result<String, String> {
    let mut child = Command::new("claude")
        .args(["--model", model, "--print"])
        .env("CLAUDECODE", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "Failed to run Claude CLI: {}. Is `claude` installed and authenticated?",
                e
            )
        })?;

    write_prompt(&mut child, prompt, "Claude").await?;
    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed to wait for Claude CLI: {}", e))?;

    if !output.status.success() {
        return Err(cli_error("Claude", &output));
    }
    non_empty_stdout("Claude", output.stdout)
}

async fn invoke_codex_cli(model: &str, prompt: &str) -> Result<String, String> {
    let mut child = Command::new("codex")
        .args([
            "exec",
            "--json",
            "--sandbox",
            "read-only",
            "--model",
            model,
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "Failed to run Codex CLI: {}. Is `codex` installed and authenticated?",
                e
            )
        })?;

    write_prompt(&mut child, prompt, "Codex").await?;
    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed to wait for Codex CLI: {}", e))?;

    if !output.status.success() {
        return Err(cli_error("Codex", &output));
    }

    parse_codex_output(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| "Codex CLI completed without a final assistant response".to_string())
}

async fn write_prompt(
    child: &mut tokio::process::Child,
    prompt: &str,
    provider: &str,
) -> Result<(), String> {
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| format!("Failed to write prompt to {} CLI: {}", provider, e))?;
    }
    Ok(())
}

fn cli_error(provider: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    format!(
        "{} CLI exited with {}: {}{}",
        provider,
        output.status,
        stderr.trim(),
        if stdout.trim().is_empty() {
            String::new()
        } else {
            format!(
                "\nstdout: {}",
                stdout.trim().chars().take(500).collect::<String>()
            )
        }
    )
}

fn non_empty_stdout(provider: &str, stdout: Vec<u8>) -> Result<String, String> {
    let text = String::from_utf8_lossy(&stdout).trim().to_string();
    if text.is_empty() {
        Err(format!("{} CLI returned an empty response", provider))
    } else {
        Ok(text)
    }
}

fn parse_codex_output(output: &str) -> Option<String> {
    let mut last_message = None;
    for line in output.lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if event.get("type").and_then(Value::as_str) != Some("item.completed") {
            continue;
        }
        let item = event.get("item")?;
        if item.get("type").and_then(Value::as_str) == Some("agent_message") {
            if let Some(text) = item
                .get("text")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
            {
                last_message = Some(text.trim().to_string());
            }
        }
    }
    last_message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_provider_prefixes() {
        assert_eq!(
            parse_provider_model("codex:gpt-5.5", "codex"),
            Some("gpt-5.5".to_string())
        );
        assert_eq!(
            parse_provider_model("claude/sonnet", "claude"),
            Some("sonnet".to_string())
        );
        assert_eq!(parse_provider_model("gpt-5.5", "codex"), None);
    }

    #[test]
    fn extracts_final_codex_agent_message() {
        let output = r#"
{"type":"thread.started","thread_id":"t"}
{"type":"item.completed","item":{"type":"agent_message","text":"first"}}
{"type":"item.completed","item":{"type":"agent_message","text":"final"}}
{"type":"turn.completed"}
"#;
        assert_eq!(parse_codex_output(output), Some("final".to_string()));
    }
}
