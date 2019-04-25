use std::collections::HashMap;

#[derive(Debug)]
pub struct RecordDB<T> {
    tables: HashMap<u8, Table<T>>,
}

#[derive(Debug, PartialEq)]
pub struct Table<T> {
    pub name: String,
    pub rows: Vec<T>,
}

#[derive(Debug, PartialEq)]
pub enum Error<'a> {
    TableNotFound(&'a str),
}

impl<T> Table<T> {
    pub fn new(name: String) -> Table<T> {
        Table {
            name: name,
            rows: vec![],
        }
    }

    pub fn insert(&mut self, row: T) {
        self.rows.push(row);
    }
}

impl<T> RecordDB<T> {
    pub fn new() -> RecordDB<T> {
        let mut tables = HashMap::new();

        tables.insert(0x02, Table::new(String::from("ARTIST")));
        tables.insert(0x03, Table::new(String::from("ALBUM")));
        tables.insert(0x04, Table::new(String::from("TRACK")));
        tables.insert(0x0c, Table::new(String::from("KEY")));
        tables.insert(0x05, Table::new(String::from("PLAYLIST")));
        tables.insert(0x16, Table::new(String::from("HISTORY")));
        tables.insert(0x12, Table::new(String::from("SEARCH")));

        RecordDB {
            tables: tables,
        }
    }

    pub fn table(&self, id: u8) -> Result<&Table<T>, Error> {
        match self.tables.contains_key(&id) {
            true => Ok(&self.tables[&id]),
            false => Err(Error::TableNotFound("Table not found")),
        }
    }

    pub fn mut_table(&mut self, id: u8) -> Result<Option<&mut Table<T>>, Error> {
        match self.tables.contains_key(&id) {
            true => Ok(self.tables.get_mut(&id)),
            false => Err(Error::TableNotFound("Table not found")),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{RecordDB, Table};

    #[derive(Debug, PartialEq)]
    struct Row {
        value: String,
        path: String,
    }

    #[test]
    fn can_fetch_tables() {
        let db: RecordDB<Row> = RecordDB::new();
        assert_eq!(Ok(&Table::new(String::from("ARTIST"))), db.table(0x02));
        assert_eq!(Ok(&Table::new(String::from("ALBUM"))), db.table(0x03));
        assert_eq!(Ok(&Table::new(String::from("TRACK"))), db.table(0x04));
    }

    #[test]
    fn can_list_table() {
        let mut db = RecordDB::new();

        if let Ok(Some(table)) = db.mut_table(0x02) {
            table.insert(Row {
                value: "Jonas Liljestrand - Min bästa sång".to_string(),
                path: "/Users/jonas".to_string(),
            });
        }

        assert_eq!(db.table(0x02), Ok(&Table {
            name: String::from("ARTIST"),
            rows: vec![
                Row {
                    value: "Jonas Liljestrand - Min bästa sång".to_string(),
                    path: "/Users/jonas".to_string(),
                }
            ]
        }));
    }
}
