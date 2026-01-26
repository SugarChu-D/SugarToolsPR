use rusqlite::Connection;

pub fn create_tables(conn: &Connection) -> rusqlite::Result<()>{
    conn.execute(
        "CREATE TABLE IF NOT EXISTS results (
            initial_seed_0 INTEGER PRIMARY KEY,
            year INTEGER NOT NULL,
            month INTEGER NOT NULL,
            day INTEGER NOT NULL,
            hour INTEGER NOT NULL,
            minute INTEGER NOT NULL,
            second INTEGER NOT NULL,
            key_input INTEGER NOT NULL,
            mt_step INTEGER NOT NULL,
            iv_h INTEGER NOT NULL,
            iv_a INTEGER NOT NULL,
            iv_b INTEGER NOT NULL,
            iv_c INTEGER NOT NULL,
            iv_d INTEGER NOT NULL,
            iv_s INTEGER NOT NULL
            )",
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_create_tables(){
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        // テーブルが存在するか確認
        let mut stmt 
        = conn.prepare(
            "SELECT name FROM sqlite_master 
            WHERE type='table'
            AND name = 'results'
            ").unwrap();
        let exists = stmt.exists([]).unwrap();
        assert!(exists);
    }
}
