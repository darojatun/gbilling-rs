use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};

#[derive(Clone, Debug)]
pub struct Tariff {
    pub id: i64,
    pub name: String,
    pub price_per_hour: f64,
}

#[derive(Clone, Debug)]
pub struct Station {
    pub id: i64,
    pub name: String,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub id: i64,
    pub station_id: i64,
    pub customer_name: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub total_amount: f64,
    pub status: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tariffs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                price_per_hour REAL NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL DEFAULT 'available'
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                station_id INTEGER NOT NULL,
                customer_name TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                total_amount REAL NOT NULL DEFAULT 0.0,
                status TEXT NOT NULL DEFAULT 'active',
                FOREIGN KEY(station_id) REFERENCES stations(id)
            );
            "#,
        )?;

        let tariffs = [
            ("Regular", 4000.0),
            ("Member", 3000.0),
            ("VIP", 7000.0),
        ];

        for (name, price) in tariffs {
            self.conn.execute(
                "INSERT OR IGNORE INTO tariffs (name, price_per_hour) VALUES (?1, ?2)",
                params![name, price],
            )?;
        }

        let station_names = ["PC-01", "PC-02", "PC-03", "PC-04", "PC-05"];
        for name in station_names {
            self.conn.execute(
                "INSERT OR IGNORE INTO stations (name, status) VALUES (?1, 'available')",
                params![name],
            )?;
        }

        Ok(())
    }

    pub fn list_tariffs(&self) -> Result<Vec<Tariff>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, price_per_hour FROM tariffs ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Tariff {
                id: row.get(0)?,
                name: row.get(1)?,
                price_per_hour: row.get(2)?,
            })
        })?;

        let mut tariffs = Vec::new();
        for item in rows {
            tariffs.push(item?);
        }
        Ok(tariffs)
    }

    pub fn add_tariff(&self, name: &str, price_per_hour: f64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tariffs (name, price_per_hour) VALUES (?1, ?2)",
            params![name, price_per_hour],
        )?;
        Ok(())
    }

    pub fn list_stations(&self) -> Result<Vec<Station>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, status FROM stations ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Station {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
            })
        })?;

        let mut stations = Vec::new();
        for item in rows {
            stations.push(item?);
        }
        Ok(stations)
    }

    pub fn start_session(&self, station_id: i64, customer_name: &str) -> Result<()> {
        let started_at = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO sessions (station_id, customer_name, started_at, status) VALUES (?1, ?2, ?3, 'active')",
            params![station_id, customer_name, started_at],
        )?;
        self.conn.execute(
            "UPDATE stations SET status = 'busy' WHERE id = ?1",
            params![station_id],
        )?;
        Ok(())
    }

    pub fn stop_session(&self, station_id: i64) -> Result<f64> {
        let started: Option<(i64, String)> = self.conn
            .query_row(
                "SELECT id, started_at FROM sessions WHERE station_id = ?1 AND status = 'active' ORDER BY id DESC LIMIT 1",
                params![station_id],
                (|row| Ok((row.get(0)?, row.get(1)?))),
            )
            .optional()?;

        let Some((session_id, started_at)) = started else {
            return Ok(0.0);
        };

        let start_time = DateTime::parse_from_rfc3339(&started_at)
            .map(|v| v.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let duration_hours = Utc::now().signed_duration_since(start_time).num_seconds() as f64 / 3600.0;

        let price_per_hour: f64 = self.conn.query_row(
            "SELECT price_per_hour FROM tariffs ORDER BY id ASC LIMIT 1",
            [],
            |row| row.get(0),
        )?;

        let total = (duration_hours * price_per_hour).max(0.0);
        let closed_at = Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE sessions SET ended_at = ?1, total_amount = ?2, status = 'closed' WHERE id = ?3",
            params![closed_at, total, session_id],
        )?;
        self.conn.execute(
            "UPDATE stations SET status = 'available' WHERE id = ?1",
            params![station_id],
        )?;

        Ok(total)
    }

    pub fn recent_sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, station_id, customer_name, started_at, ended_at, total_amount, status FROM sessions ORDER BY id DESC LIMIT 10",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                station_id: row.get(1)?,
                customer_name: row.get(2)?,
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                total_amount: row.get(5)?,
                status: row.get(6)?,
            })
        })?;

        let mut sessions = Vec::new();
        for item in rows {
            sessions.push(item?);
        }
        Ok(sessions)
    }

    pub fn total_revenue(&self) -> Result<f64> {
        let total: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_amount), 0) FROM sessions WHERE status = 'closed'",
            [],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    pub fn active_sessions_count(&self) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE status = 'active'",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}
