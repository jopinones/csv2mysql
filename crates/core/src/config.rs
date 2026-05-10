use crate::error::Result;
use crate::types::{ColumnDef, SqlType, TableSchema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Configuración de mapeo de un dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    pub table_name: String,
    pub delimiter: Option<char>,
    pub encoding: Option<String>,
    pub has_header: Option<bool>,
    pub skip_rows: Option<usize>,
    pub columns: HashMap<String, ColumnConfig>,
    pub unique_keys: Vec<String>,
    pub batch_size: Option<usize>,
}

/// Configuración de una columna específica
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnConfig {
    pub rename: Option<String>,
    pub sql_type: Option<String>,
    pub nullable: Option<bool>,
    pub default: Option<String>,
    pub skip: Option<bool>,
    pub date_format: Option<String>,
}

/// Configuración de conexión a MySQL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_max_connections() -> u32 {
    10
}

fn default_timeout_seconds() -> u64 {
    30
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

/// Configuración general de la aplicación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub max_file_size_mb: u64,
    pub default_batch_size: usize,
    pub retry_attempts: u32,
    pub retry_backoff_ms: u64,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 3306,
                database: "contribuciones".to_string(),
                username: "etl_user".to_string(),
                password: "".to_string(),
                max_connections: 10,
                timeout_seconds: 30,
            },
            max_file_size_mb: 100,
            default_batch_size: 1000,
            retry_attempts: 5,
            retry_backoff_ms: 100,
            log_level: "info".to_string(),
        }
    }
}

pub fn load_dataset_config<P: AsRef<Path>>(path: P) -> Result<DatasetConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: DatasetConfig = toml::from_str(&content)?;
    Ok(config)
}

pub fn load_app_config<P: AsRef<Path>>(path: P) -> Result<AppConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}

pub fn parse_sql_type(type_str: &str) -> Option<SqlType> {
    let upper = type_str.to_uppercase();
    
    if upper == "INT" || upper == "INTEGER" {
        Some(SqlType::Int)
    } else if upper == "BIGINT" {
        Some(SqlType::BigInt)
    } else if upper.starts_with("DECIMAL") {
        // Parse DECIMAL(p,s)
        let nums: Vec<&str> = upper
            .trim_start_matches("DECIMAL(")
            .trim_end_matches(')')
            .split(',')
            .collect();
        if nums.len() == 2 {
            let precision = nums[0].trim().parse().ok()?;
            let scale = nums[1].trim().parse().ok()?;
            Some(SqlType::Decimal { precision, scale })
        } else {
            None
        }
    } else if upper == "DATE" {
        Some(SqlType::Date)
    } else if upper.starts_with("DATETIME") {
        let precision = if upper.contains('(') {
            upper
                .trim_start_matches("DATETIME(")
                .trim_end_matches(')')
                .parse()
                .unwrap_or(3)
        } else {
            3
        };
        Some(SqlType::DateTime { precision })
    } else if upper.starts_with("VARCHAR") {
        let length = upper
            .trim_start_matches("VARCHAR(")
            .trim_end_matches(')')
            .parse()
            .unwrap_or(255);
        Some(SqlType::VarChar { length })
    } else if upper == "TEXT" {
        Some(SqlType::Text)
    } else if upper == "BOOLEAN" || upper == "BOOL" {
        Some(SqlType::Boolean)
    } else {
        None
    }
}
