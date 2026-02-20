pub mod migrations;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use tracing::info;

#[derive(Clone)]
pub struct DbPools {
    pub writer: SqlitePool,
    pub reader: SqlitePool,
}

impl DbPools {
    pub async fn connect(db_path: &str) -> anyhow::Result<Self> {
        let writer_opts = SqliteConnectOptions::from_str(db_path)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let writer = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(writer_opts)
            .await?;

        // Set pragmas on writer
        sqlx::raw_sql(
            "PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 30000000000;
             PRAGMA cache_size = -64000;
             PRAGMA foreign_keys = ON;",
        )
        .execute(&writer)
        .await?;

        let reader_opts = SqliteConnectOptions::from_str(db_path)?
            .read_only(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4);

        let reader = SqlitePoolOptions::new()
            .max_connections(num_cpus)
            .connect_with(reader_opts)
            .await?;

        // Set pragmas on reader
        sqlx::raw_sql(
            "PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 30000000000;
             PRAGMA cache_size = -64000;
             PRAGMA foreign_keys = ON;",
        )
        .execute(&reader)
        .await?;

        info!(
            writer_max = 1,
            reader_max = num_cpus,
            path = db_path,
            "Database pools connected"
        );

        Ok(Self { writer, reader })
    }

    pub async fn close(&self) {
        self.writer.close().await;
        self.reader.close().await;
    }
}
