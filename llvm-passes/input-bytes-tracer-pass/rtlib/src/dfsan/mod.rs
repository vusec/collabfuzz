mod interface;
pub use interface::{dfsan_create_label, dfsan_label, dfsan_set_label, size_t};

mod wrappers;
pub use wrappers::dfsan_get_base_labels;
