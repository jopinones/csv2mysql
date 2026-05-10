use crate::config::{parse_sql_type, ColumnConfig, DatasetConfig};
use crate::error::Result;
use crate::types::{ColumnDef, DataRow, SqlType, TableSchema};
use std::collections::HashMap;

pub struct SchemaInferrer {
    config: Option<DatasetConfig>,
}

impl SchemaInferrer {
    pub fn new(config: Option<DatasetConfig>) -> Self {
        Self { config }
    }

    /// Infiere el schema a partir de las filas de datos
    pub fn infer_schema(
        &self,
        headers: &[String],
        rows: &[DataRow],
        table_name: String,
    ) -> Result<TableSchema> {
        let mut columns = Vec::new();

        for header in headers {
            let column_config = self
                .config
                .as_ref()
                .and_then(|cfg| cfg.columns.get(header));

            // Determinar si omitir esta columna
            if let Some(cfg) = column_config {
                if cfg.skip == Some(true) {
                    continue;
                }
            }

            let final_name = column_config
                .and_then(|cfg| cfg.rename.clone())
                .unwrap_or_else(|| sanitize_column_name(header));

            let sql_type = self.infer_column_type(header, rows, column_config)?;
            
            let nullable = column_config
                .and_then(|cfg| cfg.nullable)
                .unwrap_or(true);

            let default = column_config.and_then(|cfg| cfg.default.clone());

            columns.push(ColumnDef {
                name: final_name,
                csv_name: header.clone(),
                sql_type,
                nullable,
                default,
                is_key: false,
            });
        }

        // Marcar unique keys
        let unique_keys = self
            .config
            .as_ref()
            .map(|cfg| cfg.unique_keys.clone())
            .unwrap_or_default();

        for col in &mut columns {
            if unique_keys.contains(&col.name) {
                col.is_key = true;
            }
        }

        Ok(TableSchema {
            table_name,
            columns,
            unique_keys,
        })
    }

    fn infer_column_type(
        &self,
        header: &str,
        rows: &[DataRow],
        config: Option<&ColumnConfig>,
    ) -> Result<SqlType> {
        // Si hay configuración explícita, usarla
        if let Some(cfg) = config {
            if let Some(ref type_str) = cfg.sql_type {
                if let Some(sql_type) = parse_sql_type(type_str) {
                    return Ok(sql_type);
                }
            }
        }

        // Recolectar valores no nulos de esta columna
        let values: Vec<&str> = rows
            .iter()
            .filter_map(|row| row.values.get(header).and_then(|v| v.as_deref()))
            .collect();

        if values.is_empty() {
            return Ok(SqlType::VarChar { length: 255 });
        }

        // Intentar inferir el tipo
        if values.iter().all(|v| v.parse::<i32>().is_ok()) {
            Ok(SqlType::Int)
        } else if values.iter().all(|v| v.parse::<i64>().is_ok()) {
            Ok(SqlType::BigInt)
        } else if values.iter().all(|v| v.parse::<f64>().is_ok()) {
            Ok(SqlType::Decimal {
                precision: 18,
                scale: 2,
            })
        } else if values.iter().all(|v| is_date(v)) {
            Ok(SqlType::Date)
        } else if values.iter().all(|v| is_datetime(v)) {
            Ok(SqlType::DateTime { precision: 3 })
        } else if values.iter().all(|v| is_boolean(v)) {
            Ok(SqlType::Boolean)
        } else {
            // VARCHAR con longitud basada en el máximo encontrado
            let max_len = values.iter().map(|v| v.len()).max().unwrap_or(255);
            let length = ((max_len as f64 * 1.5) as u16).min(2048);
            
            if length > 2048 {
                Ok(SqlType::Text)
            } else {
                Ok(SqlType::VarChar { length })
            }
        }
    }
}

fn sanitize_column_name(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
        .replace('.', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn is_date(s: &str) -> bool {
    // Formatos: dd/mm/yyyy, yyyy-mm-dd, dd-mm-yyyy
    let formats = ["%d/%m/%Y", "%Y-%m-%d", "%d-%m-%Y"];
    formats.iter().any(|fmt| {
        chrono::NaiveDate::parse_from_str(s, fmt).is_ok()
    })
}

fn is_datetime(s: &str) -> bool {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").is_ok()
        || chrono::NaiveDateTime::parse_from_str(s, "%d/%m/%Y %H:%M:%S").is_ok()
}

fn is_boolean(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "true" | "false" | "1" | "0" | "yes" | "no" | "si" | "sí"
    )
}
