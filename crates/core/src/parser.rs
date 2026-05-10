use crate::config::DatasetConfig;
use crate::error::{Csv2MysqlError, Result};
use crate::types::{DataRow, FileMetadata};
use csv::{Reader, ReaderBuilder};
use encoding_rs::{Encoding, UTF_8, WINDOWS_1252};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

const MAX_SNIFF_BYTES: usize = 8192;

pub struct CsvParser {
    config: Option<DatasetConfig>,
}

impl CsvParser {
    pub fn new(config: Option<DatasetConfig>) -> Self {
        Self { config }
    }

    /// Detecta el encoding del archivo
    pub fn detect_encoding<P: AsRef<Path>>(&self, path: P) -> Result<&'static Encoding> {
        let mut file = File::open(path)?;
        let mut buffer = vec![0u8; MAX_SNIFF_BYTES];
        let n = file.read(&mut buffer)?;
        buffer.truncate(n);

        // Detectar BOM UTF-8
        if buffer.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return Ok(UTF_8);
        }

        // Intentar decodificar como UTF-8
        if std::str::from_utf8(&buffer).is_ok() {
            return Ok(UTF_8);
        }

        // Asumir Windows-1252 (Latin-1) para archivos chilenos
        Ok(WINDOWS_1252)
    }

    /// Detecta el delimitador del CSV
    pub fn detect_delimiter<P: AsRef<Path>>(&self, path: P) -> Result<char> {
        if let Some(ref cfg) = self.config {
            if let Some(delim) = cfg.delimiter {
                return Ok(delim);
            }
        }

        let mut file = File::open(path)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        
        let first_line = buffer.lines().next().unwrap_or("");
        
        // Contar ocurrencias de posibles delimitadores
        let comma_count = first_line.matches(',').count();
        let semicolon_count = first_line.matches(';').count();
        let tab_count = first_line.matches('\t').count();
        let pipe_count = first_line.matches('|').count();

        if tab_count > 0 {
            Ok('\t')
        } else if semicolon_count > comma_count && semicolon_count > pipe_count {
            Ok(';')
        } else if pipe_count > comma_count {
            Ok('|')
        } else {
            Ok(',')
        }
    }

    /// Calcula el hash SHA-256 del archivo
    pub fn calculate_hash<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Obtiene metadata del archivo
    pub fn get_metadata<P: AsRef<Path>>(&self, path: P) -> Result<FileMetadata> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path)?;
        
        let encoding = self.detect_encoding(path)?;
        let delimiter = self.detect_delimiter(path)?;
        let hash = self.calculate_hash(path)?;

        // Estimación rápida de filas
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut rdr = ReaderBuilder::new()
            .delimiter(delimiter as u8)
            .from_reader(reader);
        
        let row_count = rdr.records().count();

        Ok(FileMetadata {
            path: path.to_string_lossy().to_string(),
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            size: metadata.len(),
            hash,
            encoding: encoding.name().to_string(),
            delimiter,
            has_header: true,
            row_count_estimate: row_count,
        })
    }

    /// Lee el archivo CSV completo
    pub fn parse<P: AsRef<Path>>(&self, path: P) -> Result<Vec<DataRow>> {
        let path = path.as_ref();
        let metadata = self.get_metadata(path)?;
        
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let mut rdr = ReaderBuilder::new()
            .delimiter(metadata.delimiter as u8)
            .has_headers(metadata.has_header)
            .flexible(true)
            .from_reader(reader);

        let headers: Vec<String> = rdr
            .headers()?
            .iter()
            .map(|h| h.trim().to_string())
            .collect();

        let mut rows = Vec::new();
        
        for (idx, result) in rdr.records().enumerate() {
            let record = result?;
            let mut values = HashMap::new();

            for (i, field) in record.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    let trimmed = field.trim();
                    let value = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                    values.insert(header.clone(), value);
                }
            }

            rows.push(DataRow {
                line_number: idx + 2, // +1 for header, +1 for 1-based indexing
                values,
            });
        }

        Ok(rows)
    }

    /// Lee solo el encabezado del CSV
    pub fn read_headers<P: AsRef<Path>>(&self, path: P) -> Result<Vec<String>> {
        let metadata = self.get_metadata(&path)?;
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let mut rdr = ReaderBuilder::new()
            .delimiter(metadata.delimiter as u8)
            .has_headers(true)
            .from_reader(reader);

        Ok(rdr
            .headers()?
            .iter()
            .map(|h| h.trim().to_string())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_delimiter() {
        // Crear archivo temporal de prueba
        // ...
    }
}
