use std::sync::Arc;

use redb::{Error, TableDefinition, TypeName, Value};

use crate::subscribe;

use super::Db;

#[derive(Debug)]
pub struct Episode(pub Arc<Db>);

const EPISODE: TableDefinition<String, (subscribe::Subscription, u64)> =
    TableDefinition::new("episode");
const EXPIRE_TIME: u64 = 60 * 60 * 24 * 365;

impl Episode {
    pub(super) fn init(&self) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        write_txn.open_table(EPISODE)?;
        write_txn.commit()?;

        tokio::spawn(async move {
            // sleep 随机时间，避免同时清理
            tokio::time::sleep(tokio::time::Duration::from_secs(rand::random::<u64>() % 60)).await;
            loop {
                Db::get_episode()
                    .unwrap()
                    .clear_expire()
                    .unwrap_or_else(|e| {
                        tracing::error!("Error clearing expired subscribe: {}", e);
                    });

                tokio::time::sleep(tokio::time::Duration::from_secs(60 * 60 * 24)).await;
            }
        });

        Ok(())
    }

    pub fn insert(&self, name: &str, episode: subscribe::Subscription) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(EPISODE)?;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            table.insert(name.to_string(), (episode, timestamp))?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<Option<subscribe::Subscription>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(EPISODE)?;
        let anime = table.get(name.to_string())?;
        let anime = anime.map(|s| s.value().0.to_owned());

        Ok(anime)
    }

    pub fn clear_expire(&self) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(EPISODE)?;
            table.retain(|_, value| {
                let timestamp = value.1;
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

impl Value for subscribe::Subscription {
    type SelfType<'a> =  Self
    where
        Self: 'a;
    type AsBytes<'a> = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        bincode::serialize(value).unwrap()
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("anime")
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::deserialize(data).unwrap()
    }
}
