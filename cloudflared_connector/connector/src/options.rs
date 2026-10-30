//! The add-on options the Supervisor writes to `/data/options.json`.

use std::fmt;
use std::path::Path;

use serde::Deserialize;

pub const LOGLEVELS: [&str; 4] = ["debug", "info", "warn", "error"];

pub struct Options {
    pub tunnel_token: Token,
    pub loglevel: String,
}

/// The connector token. It goes to cloudflared's environment and nowhere else, so it has no
/// `Serialize` and its `Debug` shows only the length.
pub struct Token(String);

impl Token {
    /// Zero Trust shows the token inside an install command (`cloudflared service install <token>`,
    /// `... tunnel run --token <token>`), so a pasted command yields the token it carries.
    pub fn new(value: String) -> Self {
        let words: Vec<&str> = value.split_whitespace().collect();
        let token = match words.iter().position(|w| *w == "--token") {
            Some(i) => words.get(i + 1).copied(),
            None => words
                .iter()
                .find_map(|w| w.strip_prefix("--token="))
                .or_else(|| words.last().copied()),
        };
        Self(token.unwrap_or_default().to_string())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Token({} chars)", self.0.len())
    }
}

#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    tunnel_token: String,
    #[serde(default)]
    loglevel: Option<String>,
}

impl Options {
    pub fn parse(json: &str) -> Result<Self, String> {
        let raw: Raw = serde_json::from_str(json).map_err(|e| format!("not valid options: {e}"))?;
        let loglevel = raw.loglevel.unwrap_or_else(|| "info".into());
        if !LOGLEVELS.contains(&loglevel.as_str()) {
            return Err(format!("loglevel {loglevel:?} is not one of {LOGLEVELS:?}"));
        }
        Ok(Self {
            tunnel_token: Token::new(raw.tunnel_token),
            loglevel,
        })
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::parse(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn options_parse_with_the_supervisor_defaults() {
        let o = Options::parse(r#"{"tunnel_token": " abc.def ", "loglevel": "warn"}"#).unwrap();
        assert_eq!(o.tunnel_token.expose(), "abc.def");
        assert_eq!(o.loglevel, "warn");
        let o = Options::parse(r#"{"tunnel_token": ""}"#).unwrap();
        assert!(o.tunnel_token.is_empty());
        assert_eq!(o.loglevel, "info");
    }

    #[test]
    fn an_unknown_loglevel_is_refused() {
        assert!(Options::parse(r#"{"tunnel_token": "x", "loglevel": "trace"}"#).is_err());
    }

    #[test]
    fn the_token_debug_form_shows_only_its_length() {
        let o = Options::parse(r#"{"tunnel_token": "eyJhIjoic2VjcmV0In0"}"#).unwrap();
        let shown = format!("{:?}", o.tunnel_token);
        assert_eq!(shown, "Token(19 chars)");
        assert!(!shown.contains("eyJ"));
    }

    #[test]
    fn a_pasted_install_command_yields_its_token() {
        let pasted = [
            "sudo cloudflared service install abc.def",
            "cloudflared.exe service install abc.def\n",
            "brew install cloudflared &&\nsudo cloudflared service install abc.def",
            "docker run cloudflare/cloudflared:latest tunnel --no-autoupdate run --token abc.def",
            "cloudflared tunnel run --token=abc.def",
            "abc.def",
        ];
        for value in pasted {
            let json = serde_json::json!({ "tunnel_token": value }).to_string();
            let o = Options::parse(&json).unwrap();
            assert_eq!(o.tunnel_token.expose(), "abc.def", "from {value:?}");
        }
        let o = Options::parse(r#"{"tunnel_token": " \n "}"#).unwrap();
        assert!(o.tunnel_token.is_empty());
    }
}
