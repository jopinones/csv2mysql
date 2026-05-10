-- Tabla de log de procesos
CREATE TABLE IF NOT EXISTS proceso_log (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    archivo_nombre  VARCHAR(255) NOT NULL,
    archivo_ruta    VARCHAR(1024) NOT NULL,
    archivo_hash    CHAR(64) NOT NULL COMMENT 'SHA-256 del archivo',
    tabla_destino   VARCHAR(128) NOT NULL,
    config_usado    VARCHAR(255),
    modo            ENUM('RUN','DRY_RUN','VALIDATE') NOT NULL,
    estado          ENUM('RUNNING','SUCCESS','FAILED','PARTIAL') NOT NULL,
    filas_leidas    BIGINT DEFAULT 0,
    filas_insertadas BIGINT DEFAULT 0,
    filas_rechazadas BIGINT DEFAULT 0,
    warnings        INT DEFAULT 0,
    inicio          DATETIME(3) NOT NULL,
    fin             DATETIME(3),
    duracion_ms     BIGINT,
    error_mensaje   TEXT,
    usuario_so      VARCHAR(64),
    host            VARCHAR(128),
    
    INDEX idx_inicio (inicio),
    INDEX idx_archivo (archivo_nombre),
    INDEX idx_estado (estado),
    INDEX idx_hash (archivo_hash)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Tabla de detalle de procesos (opcional, para errores por fila)
CREATE TABLE IF NOT EXISTS proceso_log_detalle (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    proceso_id      BIGINT NOT NULL,
    fila_numero     INT NOT NULL,
    tipo            ENUM('ERROR','WARNING','INFO') NOT NULL,
    columna         VARCHAR(128),
    mensaje         TEXT NOT NULL,
    timestamp       DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    
    FOREIGN KEY (proceso_id) REFERENCES proceso_log(id) ON DELETE CASCADE,
    INDEX idx_proceso (proceso_id),
    INDEX idx_tipo (tipo)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
