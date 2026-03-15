mod ask;
mod roast;

pub use ask::ask;
pub use roast::{roast_channel, roast_microsoft, roast_reply, roast_truth, roast_user};

const DEFAULT_MODEL: &str = "moonshotai/Kimi-K2.5-TEE";

/// Returns the model name from the `MODEL_NAME` env var, or the default.
pub(crate) fn model_name() -> String {
    std::env::var("MODEL_NAME").unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

