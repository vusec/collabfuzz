use super::{IDType, ShadowType};
use csv::Writer;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::dfsan_interface::has_label;

#[derive(Serialize)]
struct BasicBlockRecord {
    basic_block_id: String,
    terminator_id: String,
    terminator_tainted: bool,
}

pub struct Tracer {
    input_label: ShadowType,
    output_path: PathBuf,
    enable_output: bool,
    basic_block_to_tainted: HashMap<(IDType, IDType), bool>,
}

impl Tracer {
    pub fn new(input_label: ShadowType, output_path: PathBuf, enable_output: bool) -> Self {
        Self {
            input_label,
            output_path,
            enable_output,
            basic_block_to_tainted: HashMap::new(),
        }
    }

    pub fn trace_terminator_taint(
        &mut self,
        basic_block_id: IDType,
        instruction_id: IDType,
        traced_value_label: ShadowType,
    ) {
        if !self.enable_output {
            return;
        }

        self.basic_block_to_tainted.insert(
            (basic_block_id, instruction_id),
            has_label(traced_value_label, self.input_label),
        );
    }

    pub fn write_data(&self) -> csv::Result<()> {
        if !self.enable_output {
            return Ok(());
        }

        let mut writer = Writer::from_path(&self.output_path)?;
        for ((bb_id, inst_id), condition_count_ref) in &self.basic_block_to_tainted {
            writer.serialize(BasicBlockRecord {
                basic_block_id: format!("{:#x}", *bb_id),
                terminator_id: format!("{:#x}", inst_id),
                terminator_tainted: *condition_count_ref,
            })?;
        }

        Ok(())
    }
}
