mod ask;
mod images;
mod links;
mod preambles;
mod prompt;
mod research;
mod roast;

pub mod llm;

pub use ask::ask;
pub use research::research;
pub use roast::{roast_channel, roast_microsoft, roast_truth};

