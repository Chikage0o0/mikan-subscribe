use std::sync::Arc;

use redb::{Error, TableDefinition};

use super::Db;

const ONEDRIVE: TableDefinition<String, String> = TableDefinition::new("onedrive");

#[derive(Debug)]
pub struct Onedrive(pub Arc<Db>);

impl Onedrive {
    pub(super) fn init(&self) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        write_txn.open_table(ONEDRIVE)?;
        write_txn.commit()?;
        Ok(())
    }

    pub fn insert_refresh_token(&self, token: String) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(ONEDRIVE)?;
            table.insert("refresh_token".to_owned(), token)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_refresh_token(&self) -> Result<Option<String>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(ONEDRIVE)?;
        let token = table.get("refresh_token".to_owned())?;
        let token = token.map(|s| s.value().to_owned());

        Ok(token)
    }
}
