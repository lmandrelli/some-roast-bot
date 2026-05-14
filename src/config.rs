use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub openai_api_key: String,
    pub openai_base_url: String,
    pub model_name: String,
    pub is_prod: bool,
    pub memory_db_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingVar(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let discord_token = std::env::var("DISCORD_TOKEN")
            .map_err(|_| ConfigError::MissingVar("DISCORD_TOKEN".into()))?;

        let openai_api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ConfigError::MissingVar("OPENAI_API_KEY".into()))?;

        let openai_base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://llm.chutes.ai/v1".to_string());

        let model_name =
            std::env::var("MODEL_NAME").unwrap_or_else(|_| "moonshotai/Kimi-K2.5-TEE".to_string());

        let is_prod = std::env::var("PROD").unwrap_or_default() != "0";

        let memory_db_path = std::env::var("MEMORY_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/memory.db"));

        Ok(Config {
            discord_token,
            openai_api_key,
            openai_base_url,
            model_name,
            is_prod,
            memory_db_path,
        })
    }
}
