mod ask;
mod preambles;
mod research;
mod roast;

pub mod llm;

pub use ask::ask;
pub use research::research;
pub use roast::{roast_channel, roast_microsoft, roast_reply, roast_truth, roast_user};
