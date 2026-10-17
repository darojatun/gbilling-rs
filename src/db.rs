use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

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
    pub tariff_id: i64,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub total_amount: f64,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct AppUser {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
}

pub struct Database {
    pub conn: Connection,
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init()?;
        db.prepare_directories()?;
        Ok(db)
    }

    pub fn reopen(&mut self, path: &str) -> Result<()> {
        self.conn = Connection::open(path)?;
        Ok(())
    }

    fn prepare_directories(&self) -> Result<()> {
        for dir in ["backups", "exports", "receipts"] {
            fs::create_dir_all(dir)?;
        }
        Ok(())
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
                tariff_id INTEGER NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                total_amount REAL NOT NULL DEFAULT 0.0,
                status TEXT NOT NULL DEFAULT 'active',
                FOREIGN KEY(station_id) REFERENCES stations(id),
                FOREIGN KEY(tariff_id) REFERENCES tariffs(id)
            );

            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'admin'
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;

        for (name, price) in [("Regular", 4000.0), ("Member", 3000.0), ("VIP", 7000.0)] {
            self.conn.execute(
                "INSERT OR IGNORE INTO tariffs (name, price_per_hour) VALUES (?1, ?2)",
                params![name, price],
            )?;
        }

        for name in ["PC-01", "PC-02", "PC-03", "PC-04", "PC-05"] {
            self.conn.execute(
                "INSERT OR IGNORE INTO stations (name, status) VALUES (?1, 'available')",
                params![name],
            )?;
        }

        self.conn.execute(
            "INSERT OR IGNORE INTO users (username, password_hash, role) VALUES (?1, ?2, 'admin')",
            params!["admin", hash_password("admin123")],
        )?;

        for (key, value) in [
            ("theme", "dark"),
            ("currency", "IDR"),
            ("rounding_minutes", "15"),
            ("receipt_dir", "receipts"),
            ("backup_dir", "backups"),
            ("export_dir", "exports"),
        ] {
            self.conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }

        Ok(())
    }

    pub fn validate_login(&self, username: &str, password: &str) -> Result<bool> {
        let stored_hash: Option<String> = self
            .conn
            .query_row(
                "SELECT password_hash FROM users WHERE username = ?1",
                params![username],
                |row| row.get(0),
            )
            .optional()?;

        Ok(stored_hash.map(|hash| hash == hash_password(password)).unwrap_or(false))
    }

    pub fn set_password(&self, username: &str, new_password: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE users SET password_hash = ?1 WHERE username = ?2",
            params![hash_password(new_password), username],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
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
            "INSERT OR IGNORE INTO tariffs (name, price_per_hour) VALUES (?1, ?2)",
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

    pub fn active_session_for_station(&self, station_id: i64) -> Result<Option<Session>> {
        let row = self
            .conn
            .query_row(
                "SELECT id, station_id, customer_name, tariff_id, started_at, ended_at, total_amount, status FROM sessions WHERE station_id = ?1 AND status = 'active' ORDER BY id DESC LIMIT 1",
                params![station_id],
                |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        station_id: row.get(1)?,
                        customer_name: row.get(2)?,
                        tariff_id: row.get(3)?,
                        started_at: row.get(4)?,
                        ended_at: row.get(5)?,
                        total_amount: row.get(6)?,
                        status: row.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn start_session(&self, station_id: i64, customer_name: &str, tariff_id: i64) -> Result<bool> {
        if self.active_session_for_station(station_id)?.is_some() {
            return Ok(false);
        }

        let started_at = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO sessions (station_id, customer_name, tariff_id, started_at, status) VALUES (?1, ?2, ?3, ?4, 'active')",
            params![station_id, customer_name, tariff_id, started_at],
        )?;
        self.conn.execute(
            "UPDATE stations SET status = 'busy' WHERE id = ?1",
            params![station_id],
        )?;
        Ok(true)
    }

    pub fn stop_session(&self, station_id: i64) -> Result<Option<f64>> {
        let session = self.active_session_for_station(station_id)?;
        let Some(session) = session else {
            return Ok(None);
        };

        let tariff = self.get_tariff(session.tariff_id)?;
        let start_time = DateTime::parse_from_rfc3339(&session.started_at)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let seconds = Utc::now().signed_duration_since(start_time).num_seconds().max(0) as f64;
        let total = (seconds / 3600.0) * tariff.price_per_hour;
        let total = (total * 100.0).round() / 100.0;
        let ended_at = Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE sessions SET ended_at = ?1, total_amount = ?2, status = 'closed' WHERE id = ?3",
            params![ended_at, total, session.id],
        )?;
        self.conn.execute(
            "UPDATE stations SET status = 'available' WHERE id = ?1",
            params![station_id],
        )?;

        Ok(Some(total))
    }

    pub fn get_tariff(&self, id: i64) -> Result<Tariff> {
        let tariff: Tariff = self
            .conn
            .query_row(
                "SELECT id, name, price_per_hour FROM tariffs WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Tariff {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        price_per_hour: row.get(2)?,
                    })
                },
            )?;
        Ok(tariff)
    }

    pub fn recent_sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, station_id, customer_name, tariff_id, started_at, ended_at, total_amount, status FROM sessions ORDER BY id DESC LIMIT 25",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                station_id: row.get(1)?,
                customer_name: row.get(2)?,
                tariff_id: row.get(3)?,
                started_at: row.get(4)?,
                ended_at: row.get(5)?,
                total_amount: row.get(6)?,
                status: row.get(7)?,
            })
        })?;

        let mut sessions = Vec::new();
        for item in rows {
            sessions.push(item?);
        }
        Ok(sessions)
    }

    pub fn get_session(&self, session_id: i64) -> Result<Option<Session>> {
        let row = self
            .conn
            .query_row(
                "SELECT id, station_id, customer_name, tariff_id, started_at, ended_at, total_amount, status FROM sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        station_id: row.get(1)?,
                        customer_name: row.get(2)?,
                        tariff_id: row.get(3)?,
                        started_at: row.get(4)?,
                        ended_at: row.get(5)?,
                        total_amount: row.get(6)?,
                        status: row.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
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

    pub fn export_sessions_csv(&self) -> Result<String> {
        let dir = Path::new("exports");
        fs::create_dir_all(dir)?;
        let stamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let path = dir.join(format!("sessions_{}.csv", stamp));

        let mut wtr = csv::Writer::from_path(&path)?;
        wtr.write_record(["id", "station_id", "customer_name", "tariff_id", "started_at", "ended_at", "total_amount", "status"])?;

        let sessions = self.recent_sessions()?;
        for session in sessions {
            wtr.write_record([
                session.id.to_string(),
                session.station_id.to_string(),
                session.customer_name,
                session.tariff_id.to_string(),
                session.started_at,
                session.ended_at.clone().unwrap_or_default(),
                session.total_amount.to_string(),
                session.status,
            ])?;
        }
        wtr.flush()?;

        Ok(path.to_string_lossy().to_string())
    }

    pub fn backup_db(&self) -> Result<String> {
        fs::create_dir_all("backups")?;
        let stamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let src = Path::new("gbilling.db");
        let dst = Path::new("backups").join(format!("gbilling_backup_{}.db", stamp));
        if src.exists() {
            fs::copy(src, &dst)?;
        } else {
            let conn = Connection::open(src)?;
            drop(conn);
            fs::copy(src, &dst)?;
        }
        Ok(dst.to_string_lossy().to_string())
    }

    pub fn restore_latest_backup(&mut self) -> Result<String> {
        let backups = Path::new("backups");
        if !backups.exists() {
            return Ok(String::new());
        }

        let mut entries: Vec<PathBuf> = fs::read_dir(backups)?
            .filter_map(|e| e.ok().map(|x| x.path()))
            .filter(|p| p.extension().map(|ext| ext == "db").unwrap_or(false))
            .collect();

        if entries.is_empty() {
            return Ok(String::new());
        }

        entries.sort();
        let latest = entries.last().unwrap().clone();
        fs::copy(&latest, "gbilling.db")?;
        self.reopen("gbilling.db")?;
        Ok(latest.to_string_lossy().to_string())
    }

    pub fn create_receipt(&self, session_id: i64) -> Result<String> {
        let session = self.get_session(session_id)?;
        let Some(session) = session else {
            return Ok(String::new());
        };

        let station_name: String = self
            .conn
            .query_row(
                "SELECT name FROM stations WHERE id = ?1",
                params![session.station_id],
                |row| row.get(0),
            )?;

        let tariff = self.get_tariff(session.tariff_id)?;
        let receipt_dir = Path::new("receipts");
        fs::create_dir_all(receipt_dir)?;
        let stamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let path = receipt_dir.join(format!("receipt_{}_{}.txt", session.id, stamp));

        let content = format!(
            "GBilling RS Receipt\n===================\nSession ID: {}\nPelanggan: {}\nWorkstation: {}\nTarif: {}\nHarga/Jam: Rp {:.0}\nMulai: {}\nSelesai: {}\nTotal: Rp {:.0}\nStatus: {}\n",
            session.id,
            session.customer_name,
            station_name,
            tariff.name,
            tariff.price_per_hour,
            session.started_at,
            session.ended_at.unwrap_or_else(|| "-".to_string()),
            session.total_amount,
            session.status
        );
        fs::write(&path, content)?;

        Ok(path.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_admin_auth_works() {
        let db = Database::new(":memory:").unwrap();
        assert!(db.validate_login("admin", "admin123").unwrap());
    }

    #[test]
    fn create_session_and_close_it() {
        let db = Database::new(":memory:").unwrap();
        let tariff = db.list_tariffs().unwrap().first().unwrap().clone();
        let station = db.list_stations().unwrap().first().unwrap().clone();

        assert!(db.start_session(station.id, "Budi", tariff.id).unwrap());
        let session = db.active_session_for_station(station.id).unwrap().unwrap();

        let past = (Utc::now() - Duration::hours(2)).to_rfc3339();
        db.conn
            .execute(
                "UPDATE sessions SET started_at = ?1 WHERE id = ?2",
                params![past, session.id],
            )
            .unwrap();

        let total = db.stop_session(station.id).unwrap().unwrap();
        assert!(total > 0.0);
    }
}
