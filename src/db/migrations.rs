use anyhow::Result;
use rusqlite::Connection;

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_version (
            version    INTEGER NOT NULL PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );
    ",
    )?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if current_version < 1 {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS embeddings (
                note_id         TEXT NOT NULL PRIMARY KEY,
                note_path       TEXT NOT NULL,
                content_hash    TEXT NOT NULL,
                embedding       BLOB NOT NULL,
                embedding_model TEXT NOT NULL DEFAULT 'nomic-embed-text',
                dimension       INTEGER NOT NULL DEFAULT 768,
                indexed_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_emb_path ON embeddings(note_path);
            CREATE INDEX IF NOT EXISTS idx_emb_hash ON embeddings(content_hash);

            CREATE TABLE IF NOT EXISTS task_cache (
                task_id     TEXT NOT NULL PRIMARY KEY,
                note_path   TEXT NOT NULL,
                title       TEXT NOT NULL,
                status      TEXT NOT NULL,
                priority    INTEGER,
                due_at      INTEGER,
                tags        TEXT,
                created_at  INTEGER NOT NULL,
                modified_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_task_status ON task_cache(status);
            CREATE INDEX IF NOT EXISTS idx_task_note   ON task_cache(note_path);

            INSERT INTO schema_version (version, applied_at)
            VALUES (1, unixepoch());
        ",
        )?;
    }

    Ok(())
}
