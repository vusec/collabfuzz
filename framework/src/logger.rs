use crate::analysis::{get_analysis_types, AnalysisType};
use crate::fuzzers::{get_fuzzer_types, FuzzerId, FuzzerType};
use crate::storage::TestCaseHandle;
use crate::types::get_test_case_types;
use rusqlite::{params, Connection};
use std::error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const DB_NAME: &str = "run_info.sqlite";

#[derive(Clone, Copy)]
pub enum FuzzerEvent {
    Registration(FuzzerType),
    Deregistration,
    Ready,
}

fn get_fuzzer_event_types() -> Vec<FuzzerEvent> {
    vec![
        FuzzerEvent::Registration(FuzzerType::Unknown),
        FuzzerEvent::Deregistration,
        FuzzerEvent::Ready,
    ]
}
impl fmt::Display for FuzzerEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FuzzerEvent::Registration(_) => write!(f, "registration"),
            FuzzerEvent::Deregistration => write!(f, "deregistration"),
            FuzzerEvent::Ready => write!(f, "ready"),
        }
    }
}

pub struct Logger {
    connection: Connection,
    start_time: SystemTime,
    db_path: PathBuf,
}

impl Logger {
    pub fn new(output_path: impl AsRef<Path>) -> Result<Self, LoggerError> {
        let connection = Connection::open_in_memory().map_err(LoggerError::InitFailed)?;

        Logger::enable_foreign_keys(&connection).map_err(LoggerError::InitFailed)?;
        Logger::initialize_fixed_tables(&connection).map_err(LoggerError::InitFailed)?;
        Logger::create_data_tables(&connection).map_err(LoggerError::InitFailed)?;

        let db_path = output_path.as_ref().join(DB_NAME);
        if db_path.exists() {
            return Err(LoggerError::DBExists);
        }

        Ok(Self {
            connection,
            start_time: SystemTime::now(),
            db_path,
        })
    }

    fn enable_foreign_keys(connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.pragma_update(None, "foreign_keys", &true)?;

        Ok(())
    }

    fn initialize_fixed_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute_batch(
            "
            CREATE TABLE fuzzer_types (
              id            INTEGER PRIMARY KEY,
              description   TEXT
            );

            CREATE TABLE test_case_types (
              id            INTEGER PRIMARY KEY,
              description   TEXT
            );

            CREATE TABLE fuzzer_event_types (
              id            INTEGER PRIMARY KEY,
              description   TEXT
            );

            CREATE TABLE analysis_types (
              id                INTEGER PRIMARY KEY,
              needs_duplicates  INTEGER,
              description       TEXT
            );
            ",
        )?;

        for fuzzer_type in get_fuzzer_types() {
            connection.execute(
                "INSERT INTO fuzzer_types (description) VALUES (?)",
                params![fuzzer_type.to_string()],
            )?;
        }

        for test_case_type in get_test_case_types() {
            connection.execute(
                "INSERT INTO test_case_types (description) VALUES (?)",
                params![test_case_type.to_string()],
            )?;
        }

        for fuzzer_event_type in get_fuzzer_event_types() {
            connection.execute(
                "INSERT INTO fuzzer_event_types (description) VALUES (?)",
                params![fuzzer_event_type.to_string()],
            )?;
        }

        for analysis_type in get_analysis_types() {
            connection.execute(
                "INSERT INTO analysis_types (description, needs_duplicates) VALUES (?, ?)",
                params![analysis_type.to_string(), analysis_type.needs_duplicates()],
            )?;
        }

        Ok(())
    }

    fn create_data_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute_batch(
            "
            CREATE TABLE fuzzers (
              fuzzer_id         INTEGER PRIMARY KEY,
              fuzzer_type_id    INTEGER REFERENCES fuzzer_types
            );

            CREATE TABLE test_cases (
              hash              TEXT PRIMARY KEY,
              test_case_type_id INTEGER REFERENCES test_case_types
            );

            CREATE TABLE discoveries (
              discovery_id      INTEGER PRIMARY KEY AUTOINCREMENT,
              test_case_hash    TEXT REFERENCES test_cases,
              discovery_fuzzer  INTEGER REFERENCES fuzzers,
              discovery_time    INTEGER,
              is_new      INTEGER,
              UNIQUE(test_case_hash, discovery_fuzzer)
            );

            CREATE TABLE dispatch (
              test_case_hash    TEXT REFERENCES test_cases,
              fuzzer_id         INTEGER REFERENCES fuzzers,
              dispatch_time     INTEGER
            );

            CREATE TABLE fuzzer_events (
              fuzzer_id         INTEGER REFERENCES fuzzers,
              event_type_id     INTEGER REFERENCES fuzzer_event_types,
              event_time        INTEGER
            );

            CREATE TABLE analysis_states (
              discovery_id      TEXT REFERENCES discoveries,
              analysis_id       INTEGER REFERENCES analysis_types,
              analysis_dump     BLOB,
              PRIMARY KEY(discovery_id, analysis_id)
            );
            ",
        )
    }

    pub fn log_test_case(
        &self,
        test_case: TestCaseHandle,
        fuzzer_id: FuzzerId,
    ) -> Result<(), LoggerError> {
        let time_since_start = self.start_time.elapsed().expect("Clock drift detected");

        self.connection
            .execute(
                "
                INSERT OR IGNORE INTO test_cases (hash, test_case_type_id)
                VALUES (?, (SELECT id FROM test_case_types WHERE description = ?))
                ",
                params![test_case.get_unique_id(), test_case.get_type().to_string()],
            )
            .map_err(LoggerError::InsertFailed)?;

        self.connection
            .execute(
                "
                INSERT INTO discoveries (
                    test_case_hash, discovery_fuzzer, discovery_time, is_new
                )
                VALUES (?, ?, ?, (
                    SELECT CASE WHEN (
                        SELECT COUNT(*)
                        FROM discoveries
                        WHERE test_case_hash = ?
                    ) == 0 THEN 1 ELSE 0 END
                ))
                ",
                params![
                    test_case.get_unique_id(),
                    fuzzer_id.as_u32(),
                    time_since_start.as_secs() as u32,
                    test_case.get_unique_id(),
                ],
            )
            .map_err(LoggerError::InsertFailed)?;

        Ok(())
    }

    pub fn log_analysis_state(
        &self,
        test_case: TestCaseHandle,
        fuzzer_id: FuzzerId,
        analysis_type: AnalysisType,
        blob: Vec<u8>,
    ) -> Result<(), LoggerError> {
        self.connection
            .execute(
                "
                INSERT INTO analysis_states (discovery_id, analysis_id, analysis_dump)
                VALUES ((
                    SELECT discovery_id
                    FROM discoveries
                    WHERE test_case_hash = ? AND discovery_fuzzer = ?
                ), (
                    SELECT id
                    FROM analysis_types
                    WHERE description = ?
                ), ?)
                ",
                params![
                    test_case.get_unique_id(),
                    fuzzer_id.as_u32(),
                    analysis_type.to_string(),
                    blob
                ],
            )
            .map_err(LoggerError::InsertFailed)?;

        Ok(())
    }

    pub fn log_fuzzer_event(
        &self,
        fuzzer_id: FuzzerId,
        event: FuzzerEvent,
    ) -> Result<(), LoggerError> {
        let time_since_start = self.start_time.elapsed().expect("Clock drift detected");

        // When registering, insert the new fuzzer in the fuzzers table. Time is logged in the
        // events table.
        if let FuzzerEvent::Registration(fuzzer_type) = event {
            self.connection
                .execute(
                    "
                    INSERT INTO fuzzers(fuzzer_id, fuzzer_type_id)
                    VALUES (?, (SELECT id FROM fuzzer_types WHERE description = ?))
                    ",
                    params![fuzzer_id.as_u32(), fuzzer_type.to_string()],
                )
                .map_err(LoggerError::InsertFailed)?;
        }

        self.connection
            .execute(
                "
                INSERT INTO fuzzer_events(fuzzer_id, event_type_id, event_time)
                VALUES (?, (SELECT id FROM fuzzer_event_types WHERE description = ?), ?)
                ",
                params![
                    fuzzer_id.as_u32(),
                    event.to_string(),
                    time_since_start.as_secs() as u32
                ],
            )
            .map_err(LoggerError::InsertFailed)?;

        Ok(())
    }

    pub fn log_test_case_dispatch(
        &self,
        fuzzer_id: FuzzerId,
        test_case: TestCaseHandle,
    ) -> Result<(), LoggerError> {
        let time_since_start = self.start_time.elapsed().expect("Clock drift detected");

        self.connection
            .execute(
                "
                INSERT INTO dispatch(test_case_hash, fuzzer_id, dispatch_time)
                VALUES (?, ?, ?)
                ",
                params![
                    test_case.get_unique_id(),
                    fuzzer_id.as_u32(),
                    time_since_start.as_secs() as u32
                ],
            )
            .map_err(LoggerError::InsertFailed)?;

        Ok(())
    }

    pub fn log_test_case_dispatch_batch(
        &mut self,
        fuzzer_id: FuzzerId,
        test_cases: &[TestCaseHandle],
    ) -> Result<(), LoggerError> {
        let time_since_start = self.start_time.elapsed().expect("Clock drift detected");

        let transaction = self
            .connection
            .transaction()
            .map_err(LoggerError::InsertFailed)?;

        for test_case in test_cases {
            transaction
                .execute(
                    "
                    INSERT INTO dispatch(test_case_hash, fuzzer_id, dispatch_time)
                    VALUES (?, ?, ?)
                    ",
                    params![
                        test_case.get_unique_id(),
                        fuzzer_id.as_u32(),
                        time_since_start.as_secs() as u32
                    ],
                )
                .map_err(LoggerError::InsertFailed)?;
        }

        transaction.commit().map_err(LoggerError::InsertFailed)?;

        Ok(())
    }

    pub fn dump(&mut self) -> Result<(), LoggerError> {
        log::info!("Dumping database to file: {}", self.db_path.display());
        self.connection
            .execute("VACUUM INTO ?", params![self.db_path.to_str()])
            .map_err(LoggerError::DumpFailed)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum LoggerError {
    DBExists,
    InitFailed(rusqlite::Error),
    InsertFailed(rusqlite::Error),
    DumpFailed(rusqlite::Error),
}

impl fmt::Display for LoggerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoggerError::DBExists => write!(f, "database already exists"),
            LoggerError::InitFailed(sqlite_error) => {
                write!(f, "could not initialize sqlite logger: {}", sqlite_error)
            }
            LoggerError::InsertFailed(sqlite_error) => {
                write!(f, "could not insert data in sqlite: {}", sqlite_error)
            }
            LoggerError::DumpFailed(sqlite_error) => {
                write!(f, "could not dump data to disk: {}", sqlite_error)
            }
        }
    }
}

impl error::Error for LoggerError {}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::types::SharedLogger;
    use std::env;
    use std::fs;
    use std::io;
    use std::sync::{Arc, Mutex};

    pub fn create_shared_logger(tmp_folder: &str) -> SharedLogger {
        Arc::new(Mutex::new(create_logger(tmp_folder)))
    }

    impl Logger {
        fn get_connection(&self) -> &Connection {
            &self.connection
        }
    }

    fn create_logger(tmp_folder: &str) -> Logger {
        let _ = env_logger::builder().is_test(true).try_init();

        let mut tmp_dir = env::temp_dir();
        tmp_dir.push(tmp_folder);
        if let Err(error) = fs::remove_dir_all(&tmp_dir) {
            if io::ErrorKind::NotFound != error.kind() {
                panic!("Could not delete test directory: {}", tmp_dir.display());
            }
        }
        fs::create_dir(&tmp_dir).unwrap();

        Logger::new(&tmp_dir).unwrap()
    }

    pub fn cleanup(tmp_folder: &str) {
        let mut tmp_dir = env::temp_dir();
        tmp_dir.push(tmp_folder);
        fs::remove_dir_all(tmp_dir).unwrap();
    }

    #[test]
    fn create_database() {
        let tmp_folder = "test_logger_1";
        let _logger = create_logger(tmp_folder);
        cleanup(tmp_folder);
    }

    #[test]
    fn log_fuzzer_events() {
        let tmp_folder = "test_logger_2";
        let fuzzer_id = FuzzerId::new(42);
        let logger = create_logger(tmp_folder);

        logger
            .log_fuzzer_event(fuzzer_id, FuzzerEvent::Registration(FuzzerType::AFL))
            .unwrap();

        logger
            .log_fuzzer_event(fuzzer_id, FuzzerEvent::Ready)
            .unwrap();

        logger
            .log_fuzzer_event(fuzzer_id, FuzzerEvent::Deregistration)
            .unwrap();

        cleanup(tmp_folder);
    }

    #[test]
    fn log_test_case() {
        let tmp_folder = "test_logger_3";
        let fuzzer_id = FuzzerId::new(42);
        let test_case_handle = TestCaseHandle::get_fake_handle("");
        let logger = create_logger(tmp_folder);

        logger
            .log_fuzzer_event(fuzzer_id, FuzzerEvent::Registration(FuzzerType::AFL))
            .unwrap();

        logger.log_test_case(test_case_handle, fuzzer_id).unwrap();

        cleanup(tmp_folder);
    }

    #[test]
    fn log_test_case_duplicate() {
        let tmp_folder = "test_logger_test_case_duplicate";
        let logger = create_logger(tmp_folder);

        let fuzzer_id1 = FuzzerId::new(42);
        logger
            .log_fuzzer_event(fuzzer_id1, FuzzerEvent::Registration(FuzzerType::AFL))
            .unwrap();

        let fuzzer_id2 = FuzzerId::new(1337);
        logger
            .log_fuzzer_event(fuzzer_id2, FuzzerEvent::Registration(FuzzerType::QSYM))
            .unwrap();

        let test_case_handle = TestCaseHandle::get_fake_handle("");
        logger
            .log_test_case(test_case_handle.clone(), fuzzer_id1)
            .unwrap();
        logger
            .log_test_case(test_case_handle.clone(), fuzzer_id2)
            .unwrap();

        let connection = logger.get_connection();
        let is_new: bool = connection
            .query_row(
                "
                SELECT is_new
                FROM discoveries
                WHERE test_case_hash = ? AND discovery_fuzzer = ?
                ",
                params![test_case_handle.get_unique_id(), fuzzer_id1.as_u32()],
                |row| row.get(0),
            )
            .unwrap();

        assert!(is_new);

        let is_new2: bool = connection
            .query_row(
                "
                SELECT is_new
                FROM discoveries
                WHERE test_case_hash = ? AND discovery_fuzzer = ?
                ",
                params![test_case_handle.get_unique_id(), fuzzer_id2.as_u32()],
                |row| row.get(0),
            )
            .unwrap();

        assert!(!is_new2);

        cleanup(tmp_folder);
    }

    #[test]
    fn log_analysis() {
        let tmp_folder = "test_logger_4";
        let fuzzer_id = FuzzerId::new(42);
        let test_case_handle = TestCaseHandle::get_fake_handle("");
        let logger = create_logger(tmp_folder);

        logger
            .log_fuzzer_event(fuzzer_id, FuzzerEvent::Registration(FuzzerType::AFL))
            .unwrap();

        logger
            .log_test_case(test_case_handle.clone(), fuzzer_id)
            .unwrap();

        logger
            .log_analysis_state(test_case_handle, fuzzer_id, AnalysisType::Test, vec![42])
            .unwrap();

        cleanup(tmp_folder);
    }

    #[test]
    fn log_dispatch() {
        let tmp_folder = "test_logger_5";
        let fuzzer_id = FuzzerId::new(42);
        let test_case_handle = TestCaseHandle::get_fake_handle("");
        let logger = create_logger(tmp_folder);

        logger
            .log_fuzzer_event(fuzzer_id, FuzzerEvent::Registration(FuzzerType::AFL))
            .unwrap();

        logger
            .log_test_case(test_case_handle.clone(), fuzzer_id)
            .unwrap();

        logger
            .log_test_case_dispatch(fuzzer_id, test_case_handle)
            .unwrap();

        cleanup(tmp_folder);
    }

    #[test]
    fn verify_foreign_key() {
        let tmp_folder = "test_logger_6";
        let fuzzer_id = FuzzerId::new(42);
        let test_case_handle = TestCaseHandle::get_fake_handle("");
        let logger = create_logger(tmp_folder);

        let error = logger
            .log_test_case(test_case_handle, fuzzer_id)
            .expect_err("foreign key violation");

        if let LoggerError::InsertFailed(rusqlite_error) = error {
            if let rusqlite::Error::SqliteFailure(error, _message) = rusqlite_error {
                assert_eq!(error.extended_code, 787);
            } else {
                panic!(format!("Wrong error: {}", rusqlite_error));
            }
        } else {
            panic!("Error during initialization");
        }

        cleanup(tmp_folder);
    }
}
