use crate::models::{User, UserExchange, Balance, ExchangeType};
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
}

// 使用models/user.rs中的FromStr实现，这里不需要重复实现
