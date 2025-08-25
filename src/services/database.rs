use crate::models::{User, UserExchange, Balance, ExchangeType};
use crate::models::registration::{Registration, RegistrationExchangeType, RegistrationStatus};
use anyhow::Result;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Clone)]
pub struct DatabaseService {
    pool: SqlitePool,
}

impl DatabaseService {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        
        // 创建表结构
        Self::create_tables(&pool).await?;
        
        Ok(Self { pool })
    }

    async fn create_tables(pool: &SqlitePool) -> Result<()> {
        // 用户表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                dingtalk_user_id TEXT UNIQUE NOT NULL,
                dingtalk_name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1
            )
            "#
        ).execute(pool).await?;

        // 用户交易所配置表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_exchanges (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                exchange_type TEXT NOT NULL,
                api_key TEXT NOT NULL,
                secret_key TEXT NOT NULL,
                passphrase TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                FOREIGN KEY (user_id) REFERENCES users (id)
            )
            "#
        ).execute(pool).await?;

        // 余额记录表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS balances (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                exchange_type TEXT NOT NULL,
                asset TEXT NOT NULL,
                free TEXT NOT NULL,
                locked TEXT NOT NULL,
                total TEXT NOT NULL,
                usdt_value TEXT,
                recorded_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users (id)
            )
            "#
        ).execute(pool).await?;

        // 报名记录表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS registrations (
                id TEXT PRIMARY KEY,
                user_name TEXT NOT NULL,
                exchange_type TEXT NOT NULL,
                api_key TEXT NOT NULL,
                secret_key TEXT NOT NULL,
                passphrase TEXT,
                status TEXT NOT NULL DEFAULT 'Pending',
                admin_notes TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                reviewed_at TEXT
            )
            "#
        ).execute(pool).await?;

        // 余额历史记录表（用于排名系统）
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS balance_history (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                user_name TEXT NOT NULL,
                exchange_type TEXT NOT NULL,
                total_usdt_value TEXT NOT NULL,
                balance_details TEXT NOT NULL, -- JSON格式存储详细余额信息
                recorded_date TEXT NOT NULL, -- YYYY-MM-DD格式
                recorded_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(user_id, recorded_date) -- 每个用户每天只能有一条记录
            )
            "#
        ).execute(pool).await?;

        Ok(())
    }

    // 用户管理
    pub async fn create_user(&self, dingtalk_user_id: &str, dingtalk_name: &str) -> Result<User> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO users (id, dingtalk_user_id, dingtalk_name, created_at, updated_at, is_active) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(dingtalk_user_id)
        .bind(dingtalk_name)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(true)
        .execute(&self.pool)
        .await?;

        Ok(User {
            id,
            dingtalk_user_id: dingtalk_user_id.to_string(),
            dingtalk_name: dingtalk_name.to_string(),
            created_at: now,
            updated_at: now,
            is_active: true,
        })
    }

    pub async fn get_user_by_dingtalk_id(&self, dingtalk_user_id: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT * FROM users WHERE dingtalk_user_id = ? AND is_active = 1"
        )
        .bind(dingtalk_user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                dingtalk_user_id: row.get("dingtalk_user_id"),
                dingtalk_name: row.get("dingtalk_name"),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&Utc),
                is_active: row.get("is_active"),
            }))
        } else {
            Ok(None)
        }
    }

    // 交易所配置管理
    pub async fn add_user_exchange(&self, user_id: Uuid, exchange_type: ExchangeType, api_key: &str, secret_key: &str, passphrase: Option<&str>) -> Result<UserExchange> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO user_exchanges (id, user_id, exchange_type, api_key, secret_key, passphrase, created_at, updated_at, is_active) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(exchange_type.to_string())
        .bind(api_key)
        .bind(secret_key)
        .bind(passphrase)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(true)
        .execute(&self.pool)
        .await?;

        Ok(UserExchange {
            id,
            user_id,
            exchange_type,
            api_key: api_key.to_string(),
            secret_key: secret_key.to_string(),
            passphrase: passphrase.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
            is_active: true,
        })
    }

    pub async fn get_user_exchanges(&self, user_id: Uuid) -> Result<Vec<UserExchange>> {
        let rows = sqlx::query(
            "SELECT * FROM user_exchanges WHERE user_id = ? AND is_active = 1"
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut exchanges = Vec::new();
        for row in rows {
            let exchange_type_str = row.get::<String, _>("exchange_type");
            let exchange_type = ExchangeType::from_str(&exchange_type_str)
                .map_err(|e| anyhow::anyhow!("解析交易所类型失败: {}", e))?;
                
            exchanges.push(UserExchange {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                exchange_type,
                api_key: row.get("api_key"),
                secret_key: row.get("secret_key"),
                passphrase: row.get("passphrase"),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&Utc),
                is_active: row.get("is_active"),
            });
        }

        Ok(exchanges)
    }

    // 余额记录管理
    pub async fn save_balance(&self, user_id: Uuid, exchange_type: ExchangeType, asset: &str, free: Decimal, locked: Decimal, total: Decimal, usdt_value: Option<Decimal>) -> Result<()> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO balances (id, user_id, exchange_type, asset, free, locked, total, usdt_value, recorded_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(exchange_type.to_string())
        .bind(asset)
        .bind(free.to_string())
        .bind(locked.to_string())
        .bind(total.to_string())
        .bind(usdt_value.map(|v| v.to_string()))
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_latest_balances(&self) -> Result<Vec<Balance>> {
        let rows = sqlx::query(
            r#"
            SELECT b.*, u.dingtalk_name 
            FROM balances b
            JOIN users u ON b.user_id = u.id
            WHERE b.recorded_at = (
                SELECT MAX(recorded_at) 
                FROM balances b2 
                WHERE b2.user_id = b.user_id AND b2.exchange_type = b.exchange_type
            )
            ORDER BY b.user_id, b.exchange_type, b.asset
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut balances = Vec::new();
        for row in rows {
            let exchange_type_str = row.get::<String, _>("exchange_type");
            let exchange_type = ExchangeType::from_str(&exchange_type_str)
                .map_err(|e| anyhow::anyhow!("解析交易所类型失败: {}", e))?;
                
            balances.push(Balance {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                exchange_type,
                asset: row.get("asset"),
                free: Decimal::from_str_exact(&row.get::<String, _>("free"))?,
                locked: Decimal::from_str_exact(&row.get::<String, _>("locked"))?,
                total: Decimal::from_str_exact(&row.get::<String, _>("total"))?,
                usdt_value: row.get::<Option<String>, _>("usdt_value")
                    .and_then(|v| Decimal::from_str_exact(&v).ok()),
                recorded_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("recorded_at"))?.with_timezone(&Utc),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&Utc),
            });
        }

        Ok(balances)
    }

    // 报名记录管理
    pub async fn create_registration(&self, registration: &Registration) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO registrations (
                id, user_name, exchange_type, api_key, secret_key, passphrase,
                status, admin_notes, created_at, updated_at, reviewed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(registration.id.to_string())
        .bind(&registration.user_name)
        .bind(registration.exchange.to_string())
        .bind(&registration.api_key)
        .bind(&registration.secret_key)
        .bind(&registration.passphrase)
        .bind(registration.status.to_string())
        .bind(&registration.admin_notes)
        .bind(registration.created_at.to_rfc3339())
        .bind(registration.updated_at.to_rfc3339())
        .bind(registration.reviewed_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_registrations(&self) -> Result<Vec<Registration>> {
        let rows = sqlx::query("SELECT * FROM registrations ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        let mut registrations = Vec::new();
        for row in rows {
            let exchange_type_str = row.get::<String, _>("exchange_type");
            let exchange_type = match exchange_type_str.as_str() {
                "Binance" => RegistrationExchangeType::Binance,
                "OKX" => RegistrationExchangeType::OKX,
                "WEEX" => RegistrationExchangeType::WEEX,
                _ => return Err(anyhow::anyhow!("未知的交易所类型: {}", exchange_type_str)),
            };

            let status_str = row.get::<String, _>("status");
            let status = match status_str.as_str() {
                "Pending" => RegistrationStatus::Pending,
                "Approved" => RegistrationStatus::Approved,
                "Rejected" => RegistrationStatus::Rejected,
                _ => return Err(anyhow::anyhow!("未知的状态: {}", status_str)),
            };

            registrations.push(Registration {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                user_name: row.get("user_name"),
                exchange: exchange_type,
                api_key: row.get("api_key"),
                secret_key: row.get("secret_key"),
                passphrase: row.get("passphrase"),
                status,
                admin_notes: row.get("admin_notes"),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&Utc),
                reviewed_at: row.get::<Option<String>, _>("reviewed_at")
                    .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                    .transpose()?,
            });
        }

        Ok(registrations)
    }

    pub async fn get_registration_by_id(&self, id: Uuid) -> Result<Option<Registration>> {
        let row = sqlx::query("SELECT * FROM registrations WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let exchange_type_str = row.get::<String, _>("exchange_type");
            let exchange_type = match exchange_type_str.as_str() {
                "Binance" => RegistrationExchangeType::Binance,
                "OKX" => RegistrationExchangeType::OKX,
                "WEEX" => RegistrationExchangeType::WEEX,
                _ => return Err(anyhow::anyhow!("未知的交易所类型: {}", exchange_type_str)),
            };

            let status_str = row.get::<String, _>("status");
            let status = match status_str.as_str() {
                "Pending" => RegistrationStatus::Pending,
                "Approved" => RegistrationStatus::Approved,
                "Rejected" => RegistrationStatus::Rejected,
                _ => return Err(anyhow::anyhow!("未知的状态: {}", status_str)),
            };

            Ok(Some(Registration {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                user_name: row.get("user_name"),
                exchange: exchange_type,
                api_key: row.get("api_key"),
                secret_key: row.get("secret_key"),
                passphrase: row.get("passphrase"),
                status,
                admin_notes: row.get("admin_notes"),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&Utc),
                reviewed_at: row.get::<Option<String>, _>("reviewed_at")
                    .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                    .transpose()?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_registration(&self, registration: &Registration) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE registrations SET
                user_name = ?, exchange_type = ?, api_key = ?, secret_key = ?, passphrase = ?,
                status = ?, admin_notes = ?, updated_at = ?, reviewed_at = ?
            WHERE id = ?
            "#
        )
        .bind(&registration.user_name)
        .bind(registration.exchange.to_string())
        .bind(&registration.api_key)
        .bind(&registration.secret_key)
        .bind(&registration.passphrase)
        .bind(registration.status.to_string())
        .bind(&registration.admin_notes)
        .bind(registration.updated_at.to_rfc3339())
        .bind(registration.reviewed_at.map(|dt| dt.to_rfc3339()))
        .bind(registration.id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_registration(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM registrations WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_registrations_by_status(&self, status: RegistrationStatus) -> Result<Vec<Registration>> {
        let rows = sqlx::query("SELECT * FROM registrations WHERE status = ? ORDER BY created_at DESC")
            .bind(status.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut registrations = Vec::new();
        for row in rows {
            let exchange_type_str = row.get::<String, _>("exchange_type");
            let exchange_type = match exchange_type_str.as_str() {
                "Binance" => RegistrationExchangeType::Binance,
                "OKX" => RegistrationExchangeType::OKX,
                "WEEX" => RegistrationExchangeType::WEEX,
                _ => return Err(anyhow::anyhow!("未知的交易所类型: {}", exchange_type_str)),
            };

            registrations.push(Registration {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                user_name: row.get("user_name"),
                exchange: exchange_type,
                api_key: row.get("api_key"),
                secret_key: row.get("secret_key"),
                passphrase: row.get("passphrase"),
                status,
                admin_notes: row.get("admin_notes"),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&Utc),
                reviewed_at: row.get::<Option<String>, _>("reviewed_at")
                    .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                    .transpose()?,
            });
        }

        Ok(registrations)
    }

    // 余额历史记录管理
    pub async fn save_balance_history(
        &self,
        user_id: Uuid,
        user_name: &str,
        exchange_type: &str,
        total_usdt_value: Decimal,
        balance_details: &str,
        recorded_date: &str,
    ) -> Result<()> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // 使用 INSERT OR REPLACE 来处理重复记录
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO balance_history 
            (id, user_id, user_name, exchange_type, total_usdt_value, balance_details, recorded_date, recorded_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(user_name)
        .bind(exchange_type)
        .bind(total_usdt_value.to_string())
        .bind(balance_details)
        .bind(recorded_date)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_balance_history_by_user(
        &self,
        user_id: Uuid,
        days: u32,
    ) -> Result<Vec<crate::models::ranking::BalanceHistory>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, user_name, exchange_type, total_usdt_value, 
                   balance_details, recorded_date, recorded_at, created_at
            FROM balance_history 
            WHERE user_id = ?
            ORDER BY recorded_date DESC
            LIMIT ?
            "#
        )
        .bind(user_id.to_string())
        .bind(days as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut history = Vec::new();
        for row in rows {
            history.push(crate::models::ranking::BalanceHistory {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                user_name: row.get("user_name"),
                exchange_type: row.get("exchange_type"),
                total_usdt_value: Decimal::from_str(&row.get::<String, _>("total_usdt_value"))?,
                balance_details: row.get("balance_details"),
                recorded_date: row.get("recorded_date"),
                recorded_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("recorded_at"))?.with_timezone(&Utc),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&Utc),
            });
        }

        Ok(history)
    }

    pub async fn get_latest_balance_history(&self, days: u32) -> Result<Vec<crate::models::ranking::BalanceHistory>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, user_name, exchange_type, total_usdt_value, 
                   balance_details, recorded_date, recorded_at, created_at
            FROM balance_history 
            WHERE recorded_date >= date('now', '-' || ? || ' days')
            ORDER BY recorded_date DESC, total_usdt_value DESC
            "#
        )
        .bind(days as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut history = Vec::new();
        for row in rows {
            history.push(crate::models::ranking::BalanceHistory {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                user_name: row.get("user_name"),
                exchange_type: row.get("exchange_type"),
                total_usdt_value: Decimal::from_str(&row.get::<String, _>("total_usdt_value"))?,
                balance_details: row.get("balance_details"),
                recorded_date: row.get("recorded_date"),
                recorded_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("recorded_at"))?.with_timezone(&Utc),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&Utc),
            });
        }

        Ok(history)
    }

    pub async fn get_all_approved_users_with_exchanges(&self) -> Result<Vec<(Uuid, String, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT r.user_name, r.exchange_type, r.id as user_id
            FROM registrations r
            WHERE r.status = 'Approved'
            ORDER BY r.user_name
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            users.push((
                Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                row.get::<String, _>("user_name"),
                row.get::<String, _>("exchange_type"),
            ));
        }

        Ok(users)
    }

    pub async fn get_registration_by_user_and_exchange(
        &self,
        user_name: &str,
        exchange_type: &str,
    ) -> Result<Registration> {
        let row = sqlx::query(
            r#"
            SELECT id, user_name, exchange_type, api_key, secret_key, passphrase,
                   status, admin_notes, created_at, updated_at, reviewed_at
            FROM registrations 
            WHERE user_name = ? AND exchange_type = ? AND status = 'Approved'
            LIMIT 1
            "#
        )
        .bind(user_name)
        .bind(exchange_type)
        .fetch_one(&self.pool)
        .await?;

        let status_str = row.get::<String, _>("status");
        let status = match status_str.as_str() {
            "Pending" => RegistrationStatus::Pending,
            "Approved" => RegistrationStatus::Approved,
            "Rejected" => RegistrationStatus::Rejected,
            _ => return Err(anyhow::anyhow!("未知的状态: {}", status_str)),
        };

        let exchange_type_str = row.get::<String, _>("exchange_type");
        let exchange_type = match exchange_type_str.as_str() {
            "Binance" | "binance" => RegistrationExchangeType::Binance,
            "OKX" | "okx" => RegistrationExchangeType::OKX,
            "WEEX" | "weex" => RegistrationExchangeType::WEEX,
            _ => return Err(anyhow::anyhow!("未知的交易所类型: {}", exchange_type_str)),
        };

        Ok(Registration {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            user_name: row.get("user_name"),
            exchange: exchange_type,
            api_key: row.get("api_key"),
            secret_key: row.get("secret_key"),
            passphrase: row.get("passphrase"),
            status,
            admin_notes: row.get("admin_notes"),
            created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&Utc),
            reviewed_at: row.get::<Option<String>, _>("reviewed_at")
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()?,
        })
    }
}

// 使用models/user.rs中的FromStr实现，这里不需要重复实现
