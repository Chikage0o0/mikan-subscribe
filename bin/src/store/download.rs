use std::{collections::HashMap, sync::Arc};

use chrono::NaiveDate;
use redb::{Error, TableDefinition, TypeName, Value};
use serde::{Deserialize, Serialize};

use super::Db;

const TABLE: TableDefinition<u64, Task> = TableDefinition::new("tasks");

#[derive(Debug)]
pub struct Tasks(pub Arc<Db>);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub url: String,
    pub anime_title: String,
    pub air_date: NaiveDate,
    pub status: TaskStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Downloading,
    Downloaded,
}

impl Tasks {
    pub(super) fn init(&self) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        write_txn.open_table(TABLE)?;
        write_txn.commit()?;
        Ok(())
    }

    pub fn insert(&self, id: usize, task: Task) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            table.insert(id as u64, task)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get(&self, id: usize) -> Result<Option<Task>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(TABLE)?;
        let task = table.get(id as u64)?;
        let task = task.map(|s| s.value().to_owned());

        Ok(task)
    }

    pub fn delete(&self, id: usize) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            table.remove(id as u64)?;
        }
        Ok(())
    }

    pub fn get_all(&self) -> Result<HashMap<usize, Task>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(TABLE)?;

        let mut iter = table.range::<u64>(..)?;
        let mut result = HashMap::new();
        while let Some(Ok((key, value))) = iter.next() {
            result.insert(key.value().to_owned() as usize, value.value().to_owned());
        }
        Ok(result)
    }
}

impl Value for Task {
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
        TypeName::new("task")
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::deserialize(data).unwrap()
    }
}
