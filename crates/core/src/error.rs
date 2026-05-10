use thiserror::Error;

#[derive(Error, Debug)]
pub enum Csv2MysqlError {
    #[error("Error de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error de CSV: {0}")]
    Csv(#[from] csv::Error),

    #[error("Error de base de datos: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Error de configuración: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Error de parseo: {0}")]
    Parse(String),

    #[error("Error de validación: {0}")]
    Validation(String),

    #[error("Archivo demasiado grande: {size} bytes (máximo {max} bytes)")]
    FileTooLarge { size: u64, max: u64 },

    #[error("Encoding no soportado: {0}")]
    UnsupportedEncoding(String),

    #[error("Columna no encontrada: {0}")]
    ColumnNotFound(String),

    #[error("Tipo de dato incompatible en columna {column}: esperado {expected}, encontrado {found}")]
    TypeMismatch {
        column: String,
        expected: String,
        found: String,
    },

    #[error("Valor duplicado en fila {row}: clave {key}")]
    DuplicateKey { row: usize, key: String },

    #[error("Valor NULL no permitido en columna {column}, fila {row}")]
    NullNotAllowed { column: String, row: usize },

    #[error("Longitud excedida en columna {column}, fila {row}: {actual} > {max}")]
    LengthExceeded {
        column: String,
        row: usize,
        actual: usize,
        max: usize,
    },

    #[error("Error de retry: {0}")]
    Retry(String),

    #[error("Proceso cancelado por el usuario")]
    Cancelled,

    #[error("Error general: {0}")]
    General(String),
}

pub type Result<T> = std::result::Result<T, Csv2MysqlError>;

impl Csv2MysqlError {
    /// Determina si el error es recuperable (se puede reintentar)
    pub fn is_retryable(&self) -> bool {
        match self {
            Csv2MysqlError::Database(e) => {
                // MySQL error codes para errores transitorios
                if let Some(db_err) = e.as_database_error() {
                    let code = db_err.code().unwrap_or_default();
                    matches!(
                        code.as_ref(),
                        "1213" | // Deadlock
                        "1205" | // Lock wait timeout
                        "2006" | // MySQL server has gone away
                        "2013"   // Lost connection during query
                    )
                } else {
                    false
                }
            }
            Csv2MysqlError::Io(_) => true,
            _ => false,
        }
    }
}
