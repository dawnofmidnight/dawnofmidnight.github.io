use chrono::NaiveDate;
use rusqlite::Connection;

pub struct Thought {
    pub slug: String,
    pub title: String,
    pub date: NaiveDate,
    pub reverie: String,
    pub html: Option<String>,
}

pub fn insert_many(db: &Connection, thoughts: &[Thought]) -> Result<(), InsertError> {
    let mut stmt = db
        .prepare("REPLACE INTO thoughts (slug, title, date, reverie, html) VALUES (?1, ?2, ?3, ?4, ?5)")
        .map_err(InsertError::InvalidStmt)?;
    for thought in thoughts {
        stmt.execute((
            &thought.slug,
            &thought.title,
            thought.date,
            &thought.reverie,
            &thought.html,
        ))
        .map_err(InsertError::Failed)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum InsertError {
    InvalidStmt(rusqlite::Error),
    Failed(rusqlite::Error),
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStmt(_) => f.write_str("invalid sql statement when inserting thoughts"),
            Self::Failed(_) => f.write_str("failed to insert thought"),
        }
    }
}

impl std::error::Error for InsertError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        let (Self::InvalidStmt(raw) | Self::Failed(raw)) = self;
        Some(raw)
    }
}

pub fn select_all(db: &Connection) -> Result<Vec<Thought>, SelectError> {
    let mut stmt = db
        .prepare("SELECT slug, title, date, reverie, html FROM thoughts")
        .map_err(SelectError::InvalidStmt)?;
    let thoughts = stmt
        .query_map([], |row| {
            Ok(Thought {
                slug: row.get(0)?,
                title: row.get(1)?,
                date: row.get(2)?,
                reverie: row.get(3)?,
                html: row.get(4)?,
            })
        })
        .map_err(SelectError::Failed)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(SelectError::Failed)?;
    Ok(thoughts)
}

#[derive(Debug)]
pub enum SelectError {
    InvalidStmt(rusqlite::Error),
    Failed(rusqlite::Error),
}

impl std::fmt::Display for SelectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStmt(_) => f.write_str("invalid sql statement when selecting thoughts"),
            Self::Failed(_) => f.write_str("failed to select thoughts"),
        }
    }
}

impl std::error::Error for SelectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        let (Self::InvalidStmt(raw) | Self::Failed(raw)) = self;
        Some(raw)
    }
}
