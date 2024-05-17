use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use redb::{Error, TableDefinition};

const TABLE: TableDefinition<String, u64> = TableDefinition::new("tasks");
const ONEDRIVE_TABLE: TableDefinition<String, String> = TableDefinition::new("onedrive");
const DB_PATH: &str = "config/bt.db";

const EXPIRE_TIME: u64 = 60 * 60 * 24 * 30;

#[derive(Debug)]
pub struct Db(redb::Database);

static DB: OnceLock<Arc<Db>> = OnceLock::new();

impl Db {
    pub fn db() -> Result<Arc<Self>, Error> {
        if let Some(db) = DB.get() {
            return Ok(db.clone());
        }
        let db = redb::Database::create(DB_PATH)?;
        let db = Arc::new(Self(db));
        DB.set(db.clone()).unwrap();
        Ok(db)
    }

    pub fn insert(&self, url: String) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            table.insert(url, timestamp)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get(&self, url: String) -> Result<Option<u64>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(TABLE)?;
        let timestamp = table.get(url)?;
        let timestamp = timestamp.map(|s| s.value().to_owned());

        Ok(timestamp)
    }

    #[allow(dead_code)]
    pub fn delete(&self, url: String) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            table.remove(url)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_all(&self) -> Result<HashMap<String, u64>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(TABLE)?;

        let mut iter = table.range::<String>(..)?;
        let mut result = HashMap::new();
        // add all to HashMap
        while let Some(Ok((key, value))) = iter.next() {
            result.insert(key.value().to_owned(), value.value().to_owned());
        }

        Ok(result)
    }

    pub fn clear_expire(&self) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            table.retain(|_, value| {
                let timestamp = value;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now - timestamp < EXPIRE_TIME
            })?;
        }
        Ok(())
    }

    pub fn insert_refresh_token(&self, token: String) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(ONEDRIVE_TABLE)?;
            table.insert("refresh_token".to_owned(), token)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_refresh_token(&self) -> Result<Option<String>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(ONEDRIVE_TABLE)?;
        let token = table.get("refresh_token".to_owned())?;
        let token = token.map(|s| s.value().to_owned());

        Ok(token)
    }
}
