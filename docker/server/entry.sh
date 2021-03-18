#!/usr/bin/env bash
set -euo pipefail

exec collab_fuzz_server \
	--input-dir "${INPUT_DIR}" \
	--output-dir "${OUTPUT_DIR}" \
	--analysis-binaries-dir "${ANALYSIS_BIN_DIR}" \
	--scheduler "${SCHEDULER}" \
	-- ${ARG}
