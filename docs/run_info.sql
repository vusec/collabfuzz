-- This file records the scheme of the database run_info.sqlite that is
-- produced by the server during each run. The database is updated
-- continuously.

-- All the times recorded in the following tables are in seconds from the
-- beginning of the fuzzing run.

-- The "description" field will contain one of the string representations of
-- "FuzzerType", as defined in framework/src/fuzzers.rs.
CREATE TABLE fuzzer_types (
  id            INTEGER PRIMARY KEY,
  description   TEXT
);

-- The "description" field will contain one of the string representations of
-- "SeedType", as defined in framework/src/types.rs.
CREATE TABLE test_case_types (
  id            INTEGER PRIMARY KEY,
  description   TEXT
);

-- The "description" field will contain one of the string representations of
-- "FuzzerEvent", as defined in framework/src/logger.rs.
CREATE TABLE fuzzer_event_types (
  id            INTEGER PRIMARY KEY,
  description   TEXT
);

-- The "description" field will contain one of the string representations of
-- "AnalysisType", as defined in framework/src/analysis/analyses/mod.rs.
-- "needs_duplicates" indicates if the analysis analyzes duplicates as well.
CREATE TABLE analysis_types (
  id                    INTEGER PRIMARY KEY,
  needs_duplicates      INTEGER
  description           TEXT
);

-- This table associates a specific "fuzzer_id", assigned randomly by the
-- server, to a specific "fuzzer_type_id".
CREATE TABLE fuzzers (
  fuzzer_id             INTEGER PRIMARY KEY,
  fuzzer_type_id        INTEGER REFERENCES fuzzer_types
);

-- This table contains data related to test cases discovered by fuzzers.
-- * hash" corresponds to the name of the test case file used by the server.
-- * "test_case_type_id" indicates if the file is a normal test case, a crash
--   or a hang, if reporting is supported by the corresponding fuzzer.
CREATE TABLE test_cases (
  hash              TEXT PRIMARY KEY,
  test_case_type_id INTEGER REFERENCES test_case_types
);

-- This table contains data related to the fuzzers discovery of test cases
-- (multiple fuzzers may discover the same test case):
-- * "discovery_id" ensures that the sequence on which the test cases where
--   reported is preserved.
-- * "test_case_hash" corresponds to the name of the test case file used by the
--   server.
-- * "discovery_fuzzer" indicates which fuzzer reported the test case.
-- * "discovery_time" indicates at what time the fuzzer was reported.
CREATE TABLE discoveries (
  discovery_id          INTEGER PRIMARY KEY AUTOINCREMENT,
  test_case_hash        TEXT REFERENCES test_cases,
  discovery_fuzzer      INTEGER REFERENCES fuzzers,
  discovery_time        INTEGER,
  is_new                INTEGER,
  UNIQUE(discovery_id, discovery_fuzzer)
);

-- This table contains data related to which test case was dispatched to which
-- fuzzer and at which time. This allows to record the decisions made by the
-- test case scheduler being used.
CREATE TABLE dispatch (
  test_case_hash        TEXT REFERENCES test_cases,
  fuzzer_id             INTEGER REFERENCES fuzzers,
  dispatch_time         INTEGER
);

-- This table records the events related to the fuzzers that participated in
-- the current run. These comprehend "registration", "deregistration" and
-- "ready". While the first two are obvious, the third event identifies the
-- moment in which a specific fuzzer informs the server that it is ready to
-- receive new seeds.
CREATE TABLE fuzzer_events (
  fuzzer_id             INTEGER REFERENCES fuzzers,
  event_type_id         INTEGER REFERENCES fuzzer_event_types,
  event_time            INTEGER
);

-- This table records the changes in state, usually as diffs, of the analyses
-- performed by the server when a new test case is received. The change in
-- state is serialized in the "analysis_dump" field as serialized using CBOR.
-- The semantics of the data contained in this field depends on the analysis
-- itself, the code that serializes these data can be found in
-- framework/src/analysis/analyses/*_analysis.rs.
CREATE TABLE analysis_states (
  discovery_id          TEXT REFERENCES discoveries,
  analysis_id           INTEGER REFERENCES analysis_types,
  analysis_dump         BLOB,
  PRIMARY KEY(test_case_hash, analysis_id)
);

-- CBOR serializations

-- The following list will explain what can be found in a single entry of
-- "analysis_dump":

-- Fuzzer coverage: (fuzzer_id, [(source, target), ...])
-- The ID of the fuzzer that found the test case and is thus incrementing its
-- coverage. A list of the new edges that were found when compared to the
-- previous coverage for that fuzzer.

-- Generation graph: [parent_hash, ...]
-- A list of the hashes of the test cases that are considered parents of this
-- one according to the fuzzer that produced the test case. It is currently
-- only supported for AFL-based fuzzers.

-- Global coverage: [(source, target), ...]
-- A list of new edges that were found when compared to the coverage before
-- receiving the corresponding test case.

-- Instruction count: [(condition_id, count, test_hash), ...]
-- A list of conditions whose count was updated. These can be both existing
-- conditions whose count has decreased and new conditions.

-- Observed conditions: [(condition_id, cases), ...]
-- The full list of conditions encountered in the test case, no diffing is
-- present. The "cases" field is a list of 0s and 1s which indicate which
-- branch was traversed and which was not.

-- Fuzzer observed conditions: [(fuzzer_id, (condition_id, cases)), ...]
-- Same as above, but tracked per fuzzer independently.

-- Test case benefit analysis: [basic_block_id, ...]
-- The frontier of the test case received: a list of the basic blocks with a
-- tainted terminator that have at least one unseen neighbor.
