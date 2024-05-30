use std::sync::Arc;

use redb::{Error, TableDefinition};

use super::Db;

const TABLE: TableDefinition<String, u64> = TableDefinition::new("subscribe");

const EXPIRE_TIME: u64 = 60 * 60 * 24 * 365;

#[derive(Debug)]
pub struct Subscribe(pub Arc<Db>);

impl Subscribe {
    pub(super) fn init(&self) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        write_txn.open_table(TABLE)?;
        write_txn.commit()?;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60 * 60 * 24)).await;

                Db::get_subscribe()
                    .unwrap()
                    .clear_expire()
                    .unwrap_or_else(|e| {
                        tracing::error!("Error clearing expired subscribe: {}", e);
                    });
            }
        });

        Ok(())
    }

    pub fn insert(&self, name: String) -> Result<(), Error> {
        let write_txn = self.0 .0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            table.insert(name, timestamp)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get(&self, name: String) -> Result<Option<u64>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(TABLE)?;
        let timestamp = table.get(name)?;
        let timestamp = timestamp.map(|s| s.value().to_owned());

        Ok(timestamp)
    }

    #[allow(dead_code)]
    pub fn delete(&self, name: String) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            table.remove(name)?;
        }
        Ok(())
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
}
