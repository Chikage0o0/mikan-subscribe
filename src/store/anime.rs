use std::sync::Arc;

use redb::{Error, TableDefinition, TypeName, Value};

use crate::subscribe;

use super::Db;

#[derive(Debug)]
pub struct Anime(pub Arc<Db>);

const ANIME: TableDefinition<u64, subscribe::Anime> = TableDefinition::new("anime");

impl Anime {
    pub(super) fn init(&self) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        write_txn.open_table(ANIME)?;
        write_txn.commit()?;
        Ok(())
    }

    pub fn insert(&self, id: u64, anime: subscribe::Anime) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(ANIME)?;
            table.insert(id, anime)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get(&self, id: u64) -> Result<Option<subscribe::Anime>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(ANIME)?;
        let anime = table.get(id)?;
        let anime = anime.map(|s| s.value().to_owned());

        Ok(anime)
    }
}

impl Value for subscribe::Anime {
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
        bincode::serde::encode_to_vec(value, bincode::config::legacy()).unwrap()
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("anime")
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::serde::decode_from_slice(data, bincode::config::legacy()).unwrap().0
    }
}
