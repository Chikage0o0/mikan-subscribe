use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use redb::{Error, TableDefinition, TypeName, Value};

use crate::Task;

const TABLE: TableDefinition<u64, Task> = TableDefinition::new("tasks");
const DB_PATH: &str = "bt_db";

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
    type SelfType<'a> =  Task
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
