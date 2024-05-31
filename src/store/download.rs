use std::{collections::HashMap, path::PathBuf, sync::Arc};

use chrono::NaiveDate;
use redb::{Error, ReadableTable, TableDefinition, TypeName, Value};
use serde::{Deserialize, Serialize};

use super::Db;

const TABLE: TableDefinition<String, Task> = TableDefinition::new("tasks");

#[derive(Debug)]
pub struct Tasks(pub Arc<Db>);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub url: String,
    pub anime_title: String,
    pub weekday: String,
    pub air_date: NaiveDate,
    pub added_at: u64,
    pub state: TaskState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskState {
    Pending,
    Downloading,
    Downloaded {
        file_path: PathBuf,
        info_hash: String,
    },
    Finished {
        file_path: PathBuf,
        info_hash: String,
        finish_time: u64,
    },
}

impl Tasks {
    pub(super) fn init(&self) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        write_txn.open_table(TABLE)?;
        write_txn.commit()?;
        Ok(())
    }

    pub fn insert(&self, name: String, mut task: Task) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            if let Some(old_task) = table.get(name.clone())? {
                task.added_at = old_task.value().added_at;
            }
            table.insert(name, task)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn update_state(&self, name: String, state: TaskState) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            let old_task = table.get(name.clone())?.map(|s| s.value().to_owned());
            if let Some(mut task) = old_task {
                task.state = state;
                table.insert(name, task)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get(&self, name: String) -> Result<Option<Task>, Error> {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(TABLE)?;
        let task = table.get(name)?;
        let task = task.map(|s| s.value().to_owned());

        Ok(task)
    }

    pub fn delete(&self, name: &str) -> Result<(), Error> {
        let write_txn = self.0.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            table.remove(name.to_string())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_with_state<F>(&self, cmp: F) -> Result<HashMap<String, Task>, Error>
    where
        F: Fn(TaskState) -> bool,
    {
        let read_txn = self.0.begin_read()?;
        let table = read_txn.open_table(TABLE)?;

        let mut iter = table.range::<String>(..)?;
        let mut result = HashMap::new();
        while let Some(Ok((key, value))) = iter.next() {
            if cmp(value.value().state) {
                result.insert(key.value().to_owned(), value.value().to_owned());
            }
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
