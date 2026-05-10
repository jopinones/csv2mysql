use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tipos de datos MySQL soportados
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlType {
    Int,
    BigInt,
    Decimal { precision: u8, scale: u8 },
    Date,
    DateTime { precision: u8 },
    VarChar { length: u16 },
    Text,
    Boolean,
}

impl SqlType {
    pub fn to_sql_string(&self) -> String {
        match self {
            SqlType::Int => "INT".to_string(),
            SqlType::BigInt => "BIGINT".to_string(),
            SqlType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
            SqlType::Date => "DATE".to_string(),
            SqlType::DateTime { precision } => format!("DATETIME({})", precision),
            SqlType::VarChar { length } => format!("VARCHAR({})", length),
            SqlType::Text => "TEXT".to_string(),
            SqlType::Boolean => "BOOLEAN".to_string(),
        }
    }
}

/// Definición de una columna
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub csv_name: String,
    pub sql_type: SqlType,
    pub nullable: bool,
    pub default: Option<String>,
    pub is_key: bool,
}

/// Schema completo de una tabla
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub table_name: String,
    pub columns: Vec<ColumnDef>,
    pub unique_keys: Vec<String>,
}

/// Fila de datos parseada del CSV
#[derive(Debug, Clone)]
pub struct DataRow {
    pub line_number: usize,
    pub values: HashMap<String, Option<String>>,
}

/// Estadísticas de carga
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadStats {
    pub rows_read: u64,
    pub rows_inserted: u64,
    pub rows_rejected: u64,
    pub warnings: u32,
    pub duration_ms: u64,
    pub throughput: f64, // rows per second
}

/// Modo de ejecución
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Run,
    DryRun,
    Validate,
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionMode::Run => write!(f, "RUN"),
            ExecutionMode::DryRun => write!(f, "DRY_RUN"),
            ExecutionMode::Validate => write!(f, "VALIDATE"),
        }
    }
}

/// Estado de un proceso
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Success,
    Failed,
    Partial,
}

impl std::fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessStatus::Running => write!(f, "RUNNING"),
            ProcessStatus::Success => write!(f, "SUCCESS"),
            ProcessStatus::Failed => write!(f, "FAILED"),
            ProcessStatus::Partial => write!(f, "PARTIAL"),
        }
    }
}

/// Metadata de un archivo
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub hash: String,
    pub encoding: String,
    pub delimiter: char,
    pub has_header: bool,
    pub row_count_estimate: usize,
}

/// Registro del proceso
#[derive(Debug, Clone)]
pub struct ProcessLog {
    pub id: Option<i64>,
    pub archivo_nombre: String,
    pub archivo_ruta: String,
    pub archivo_hash: String,
    pub tabla_destino: String,
    pub config_usado: Option<String>,
    pub modo: ExecutionMode,
    pub estado: ProcessStatus,
    pub filas_leidas: i64,
    pub filas_insertadas: i64,
    pub filas_rechazadas: i64,
    pub warnings: i32,
    pub inicio: DateTime<Utc>,
    pub fin: Option<DateTime<Utc>>,
    pub duracion_ms: Option<i64>,
    pub error_mensaje: Option<String>,
    pub usuario_so: String,
    pub host: String,
}
