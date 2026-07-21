use sqlx::PgPool;

/// P4 AI configuration — plain data read from env ONCE at startup (R8). The
/// key is a secret: env only, never logged, never serialized, never sent to
/// the client. The vendor HTTP client itself lives in `crate::ai::client`
/// (the ONE module allowed to name the vendor); this struct only carries what
/// that seam needs.
#[derive(Clone)]
pub struct AiConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub ask_flag: bool,
    pub discount_flag: bool,
}

impl AiConfig {
    /// Effective-enabled for Ask PLENUM: flag AND key present (spec §6 —
    /// absent key must not error any screen; it serves the saved library).
    pub fn ask_enabled(&self) -> bool {
        self.ask_flag && self.api_key.is_some()
    }

    /// The recommender endpoint is gated by the flag ALONE; a missing key
    /// only degrades its narrative (R10).
    pub fn discount_enabled(&self) -> bool {
        self.discount_flag
    }

    /// Read the four AI env keys. Missing key/flags fall back to their
    /// defaults; a flag that is present but not `true`/`false` is a startup
    /// error in plain language (the COOKIE_SECURE posture).
    pub fn from_env() -> Result<Self, String> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty());
        let model = std::env::var("ANTHROPIC_MODEL")
            .ok()
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| "claude-sonnet-5".to_string());
        let parse_flag = |name: &str| -> Result<bool, String> {
            match std::env::var(name) {
                Err(_) => Ok(true),
                Ok(v) => v
                    .trim()
                    .parse::<bool>()
                    .map_err(|_| format!("{name} must be true or false")),
            }
        };
        Ok(AiConfig {
            api_key,
            model,
            ask_flag: parse_flag("AI_ASK_ENABLED")?,
            discount_flag: parse_flag("AI_DISCOUNT_ENABLED")?,
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    /// The plenum_app pool. There is no admin pool anywhere in the API —
    /// the admin connection belongs to migrations and the seed binary only.
    pub pool: PgPool,
    pub ai: AiConfig,
}
