use crate::error::{Csv2MysqlError, Result};
use crate::types::{DataRow, TableSchema, SqlType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub row: usize,
    pub column: String,
    pub message: String,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Warning,
    Error,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
    pub ok: bool,
}

pub struct Validator {
    schema: TableSchema,
}

impl Validator {
    pub fn new(schema: TableSchema) -> Self {
        Self { schema }
    }

    pub fn validate(&self, rows: &[DataRow]) -> ValidationReport {
        let mut report = ValidationReport::default();

        // Validar tipos y nulls
        for row in rows {
            for column in &self.schema.columns {
                let value = row.values.get(&column.csv_name);

                match value {
                    None | Some(None) => {
                        if !column.nullable {
                            report.errors.push(ValidationError {
                                row: row.line_number,
                                column: column.name.clone(),
                                message: "Valor NULL no permitido".to_string(),
                                severity: ErrorSeverity::Error,
                            });
                        }
                    }
                    Some(Some(val)) => {
                        if let Err(e) = self.validate_type(val, &column.sql_type) {
                            report.errors.push(ValidationError {
                                row: row.line_number,
                                column: column.name.clone(),
                                message: e.to_string(),
                                severity: ErrorSeverity::Error,
                            });
                        }

                        // Validar longitud
                        if let SqlType::VarChar { length } = column.sql_type {
                            if val.len() > length as usize {
                                report.errors.push(ValidationError {
                                    row: row.line_number,
                                    column: column.name.clone(),
                                    message: format!(
                                        "Longitud excedida: {} > {}",
                                        val.len(),
                                        length
                                    ),
                                    severity: ErrorSeverity::Error,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Validar duplicados en unique keys
        if !self.schema.unique_keys.is_empty() {
            let mut seen = HashSet::new();
            
            for row in rows {
                let key_values: Vec<String> = self
                    .schema
                    .unique_keys
                    .iter()
                    .filter_map(|key| {
                        self.schema
                            .columns
                            .iter()
                            .find(|c| &c.name == key)
                            .and_then(|col| {
                                row.values
                                    .get(&col.csv_name)
                                    .and_then(|v| v.clone())
                            })
                    })
                    .collect();

                let key = key_values.join("|");
                if !seen.insert(key.clone()) {
                    report.errors.push(ValidationError {
                        row: row.line_number,
                        column: "unique_key".to_string(),
                        message: format!("Clave duplicada: {}", key),
                        severity: ErrorSeverity::Error,
                    });
                }
            }
        }

        report.ok = report.errors.is_empty();
        report
    }

    fn validate_type(&self, value: &str, sql_type: &SqlType) -> Result<()> {
        match sql_type {
            SqlType::Int => value
                .parse::<i32>()
                .map(|_| ())
                .map_err(|_| Csv2MysqlError::Parse(format!("No es un INT válido: {}", value))),
            SqlType::BigInt => value
                .parse::<i64>()
                .map(|_| ())
                .map_err(|_| Csv2MysqlError::Parse(format!("No es un BIGINT válido: {}", value))),
            SqlType::Decimal { .. } => value
                .parse::<f64>()
                .map(|_| ())
                .map_err(|_| Csv2MysqlError::Parse(format!("No es un DECIMAL válido: {}", value))),
            SqlType::Date => {
                let formats = ["%d/%m/%Y", "%Y-%m-%d", "%d-%m-%Y"];
                if formats.iter().any(|fmt| {
                    chrono::NaiveDate::parse_from_str(value, fmt).is_ok()
                }) {
                    Ok(())
                } else {
                    Err(Csv2MysqlError::Parse(format!("No es una DATE válida: {}", value)))
                }
            }
            SqlType::DateTime { .. } => {
                if chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").is_ok()
                    || chrono::NaiveDateTime::parse_from_str(value, "%d/%m/%Y %H:%M:%S").is_ok()
                {
                    Ok(())
                } else {
                    Err(Csv2MysqlError::Parse(format!(
                        "No es un DATETIME válido: {}",
                        value
                    )))
                }
            }
            SqlType::Boolean => {
                if matches!(
                    value.to_lowercase().as_str(),
                    "true" | "false" | "1" | "0" | "yes" | "no" | "si" | "sí"
                ) {
                    Ok(())
                } else {
                    Err(Csv2MysqlError::Parse(format!(
                        "No es un BOOLEAN válido: {}",
                        value
                    )))
                }
            }
            SqlType::VarChar { .. } | SqlType::Text => Ok(()),
        }
    }
}
