use crate::{db::Db, notes::VectorSearchResult};
use anyhow::Result;

pub fn store_embedding(
    db: &Db,
    note_id: &str,
    note_path: &str,
    content_hash: &str,
    embedding: &[f32],
    model: &str,
) -> Result<()> {
    let conn = db.lock().unwrap();
    let bytes = floats_to_bytes(embedding);
    conn.execute(
        "INSERT OR REPLACE INTO embeddings
            (note_id, note_path, content_hash, embedding, embedding_model, dimension, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch())",
        rusqlite::params![
            note_id,
            note_path,
            content_hash,
            bytes,
            model,
            embedding.len() as i64,
        ],
    )?;
    Ok(())
}

/// Wipe all stored embeddings so every note is re-indexed on the next save/startup.
pub fn clear_all_embeddings(db: &Db) -> Result<()> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM embeddings", [])?;
    Ok(())
}

pub fn needs_embedding(db: &Db, note_id: &str, content_hash: &str, model: &str) -> bool {
    let conn = db.lock().unwrap();
    let result: Result<String, _> = conn.query_row(
        "SELECT content_hash FROM embeddings WHERE note_id = ?1 AND embedding_model = ?2",
        rusqlite::params![note_id, model],
        |r| r.get(0),
    );
    match result {
        Ok(stored_hash) => stored_hash != content_hash,
        Err(_) => true,
    }
}

pub fn top_k_similar(
    db: &Db,
    query_embedding: &[f32],
    model: &str,
    k: usize,
) -> Result<Vec<VectorSearchResult>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT note_path, embedding FROM embeddings WHERE embedding_model = ?1 AND dimension = ?2",
    )?;

    let dim = query_embedding.len() as i64;
    let rows = stmt.query_map(rusqlite::params![model, dim], |r| {
        let path: String = r.get(0)?;
        let blob: Vec<u8> = r.get(1)?;
        Ok((path, blob))
    })?;

    let mut scored: Vec<(f32, String)> = Vec::new();
    for row in rows {
        let (path, blob) = row?;
        let embedding = bytes_to_floats(&blob);
        let sim = cosine_similarity(query_embedding, &embedding);
        scored.push((sim, path));
    }

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    Ok(scored
        .into_iter()
        .take(k)
        .map(|(sim, path)| VectorSearchResult {
            title: path.clone(),
            relative_path: path,
            similarity: sim,
        })
        .collect())
}

fn floats_to_bytes(floats: &[f32]) -> Vec<u8> {
    floats.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
