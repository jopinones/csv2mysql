# Plan de proyecto: `csv2mysql` — TUI en Rust para carga de CSV a MySQL

## 1. Resumen ejecutivo

Herramienta CLI con TUI (Text User Interface) interactiva escrita en Rust, ejecutable en Ubuntu, que permite navegar carpetas locales, seleccionar archivos CSV (<100 MB), validar e ingestar sus datos hacia tablas MySQL, registrando todo el proceso en una tabla `proceso_log` y mostrando el progreso en vivo.

**Layout:** dos paneles. Izquierda → árbol de carpetas navegable. Derecha → panel de ejecución con resumen del archivo, botones de acción (ejecutar, dry-run, validar), barra de progreso y log en vivo.

**Stack técnico:**

- Rust estable (≥ 1.79), edición 2021
- `ratatui` + `crossterm` → TUI
- `clap` → flags y subcomandos (`run`, `tui`, `init-db`)
- `csv` + `serde` → parseo y deserialización
- `sqlx` (MySQL, tokio) → acceso a base de datos asíncrono con queries verificadas
- `tokio` → runtime asíncrono
- `tracing` + `tracing-subscriber` → logging estructurado
- `config` + `toml` → archivos de mapeo
- `anyhow` + `thiserror` → manejo de errores
- `walkdir` → recorrido de directorios

---

## 2. Alcance funcional

### 2.1 Funcionalidades núcleo

1. **Navegación de carpetas** — árbol expandible/colapsable en el panel izquierdo, con detección automática de archivos `.csv` y `.tsv`.
2. **Inspección de archivo** — al seleccionar un CSV, el panel derecho muestra: ruta, tamaño, número de filas (rápido), columnas detectadas y tipo inferido por columna.
3. **Inferencia automática de schema** — sniff del delimitador, detección de encoding (UTF-8 / Latin-1), inferencia de tipos (`INT`, `BIGINT`, `DECIMAL(p,s)`, `DATE`, `DATETIME`, `VARCHAR(n)`, `TEXT`).
4. **Mapeo manual columna → campo** — archivo TOML por dataset que sobrescribe la inferencia (rename, type override, exclusión de columnas, valor por defecto).
5. **Validación de datos** — pre-vuelo: tipos, nulls en columnas no-nulables, duplicados sobre clave configurable, longitudes máximas.
6. **Modo dry-run** — ejecuta el pipeline completo sin emitir `INSERT`, reportando qué se hubiera insertado y errores detectados.
7. **Carga a MySQL** — `INSERT` por lotes (batch 500–1000 filas), transacción por archivo, rollback ante fallo crítico.
8. **Reintentos y resume** — si un batch falla por error transitorio (deadlock, lock wait), reintenta con backoff. Si falla por error irrecuperable, registra el offset y permite reanudar desde la última fila confirmada.
9. **Log de procesos en MySQL** — cada ejecución crea un registro en `proceso_log` con estado, métricas y errores.
10. **Resumen final** — al terminar, panel derecho muestra: filas leídas, insertadas, rechazadas, duración, throughput, ID del proceso en `proceso_log`.

### 2.2 Fuera de alcance (v1)

- Archivos > 100 MB (la v1 carga el CSV en memoria; streaming queda para v2).
- Origenes remotos (S3, FTP).
- UI web.
- Edición del CSV.

---

## 3. Arquitectura

### 3.1 Estructura del workspace Cargo

```
csv2mysql/
├── Cargo.toml                 # workspace
├── crates/
│   ├── core/                  # lógica de negocio (sin TUI ni CLI)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── config.rs      # carga de TOML de mapeo y conexión
│   │   │   ├── schema.rs      # inferencia y mapeo de tipos
│   │   │   ├── parser.rs      # lectura CSV → Vec<Row>
│   │   │   ├── validator.rs   # validaciones pre-carga
│   │   │   ├── loader.rs      # batch insert + transacción
│   │   │   ├── retry.rs       # políticas de reintento
│   │   │   ├── proc_log.rs    # CRUD de proceso_log
│   │   │   └── error.rs       # tipos de error
│   │   └── Cargo.toml
│   ├── tui/                   # capa ratatui
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── app.rs         # estado global de la app
│   │   │   ├── tree.rs        # widget árbol de carpetas
│   │   │   ├── panel.rs       # panel derecho de ejecución
│   │   │   ├── progress.rs    # barra y métricas en vivo
│   │   │   ├── log_view.rs    # buffer de log en pantalla
│   │   │   └── events.rs      # mapeo de teclas
│   │   └── Cargo.toml
│   └── cli/                   # entry point
│       ├── src/main.rs        # clap: tui | run | init-db | validate
│       └── Cargo.toml
├── migrations/                # sqlx migrate
│   ├── 0001_proceso_log.sql
│   └── 0002_indexes.sql
├── configs/                   # ejemplos de mapeo
│   ├── default.toml
│   └── rol_cobro.toml
└── README.md
```

Separar `core` de `tui` permite añadir luego un modo headless (`csv2mysql run --config x.toml`) y testear lógica sin levantar terminal.

### 3.2 Flujo de datos

```
[árbol carpetas] → seleccionar CSV
        ↓
[parser]   → lee CSV en memoria (csv crate, streaming interno)
        ↓
[schema]   → infiere tipos / aplica mapeo TOML
        ↓
[validator] → reporta errores y warnings
        ↓
[dry-run? sí → mostrar resumen y detener]
        ↓
[loader]   → BEGIN; INSERT por lotes; COMMIT
        ↓
[proc_log] → UPDATE con métricas finales
        ↓
[panel UI] → resumen final (verde/rojo)
```

Toda la cadena emite eventos por un `tokio::sync::mpsc` que la TUI consume en su event loop, así el render nunca bloquea.

---

## 4. Diseño de la TUI

(Mockup ya mostrado arriba.)

### 4.1 Layout

- **Header (3 líneas):** nombre de la app, ruta raíz actual, atajo de salida.
- **Body dividido 35% / 65%:**
  - **Izquierda — Árbol:** widget `Tree` con expand/collapse, indicador `▸/▾`, marca de ✓ para archivos ya cargados en sesión.
  - **Derecha — Panel:** sub-bloques verticales: metadata del archivo, botones de acción, barra de progreso, log en vivo (tail de las últimas N líneas).
- **Footer (1 línea):** estado de conexión MySQL y atajos.

### 4.2 Atajos de teclado

| Tecla         | Acción                                   |
| ------------- | ---------------------------------------- |
| `↑` `↓`       | Navegar árbol                            |
| `→` / `Enter` | Expandir carpeta o seleccionar archivo   |
| `←`           | Colapsar carpeta                         |
| `Space`       | Marcar/desmarcar archivo (multi-select)  |
| `E`           | Ejecutar carga del archivo seleccionado  |
| `D`           | Dry-run                                  |
| `V`           | Solo validar                             |
| `L`           | Abrir log completo en pager interno      |
| `r`           | Reintentar último batch fallido          |
| `c`           | Editar config de mapeo (abre `$EDITOR`)  |
| `q`           | Salir                                    |

### 4.3 Estados visuales del panel de ejecución

- **Idle** — esperando selección.
- **Inspecting** — leyendo encabezado, infiriendo schema.
- **Validating** — corriendo reglas, mostrando errores en rojo.
- **Running** — barra animada, log scrolleando.
- **Success** — fondo verde tenue, resumen final.
- **Error** — fondo rojo tenue, traceback resumido + sugerencia de `r`.

---

## 5. Esquema de base de datos

### 5.1 Tabla `proceso_log`

```sql
CREATE TABLE proceso_log (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    archivo_nombre  VARCHAR(255) NOT NULL,
    archivo_ruta    VARCHAR(1024) NOT NULL,
    archivo_hash    CHAR(64) NOT NULL,        -- SHA-256 del archivo
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
    INDEX idx_estado (estado)
) ENGINE=InnoDB CHARSET=utf8mb4;
```

### 5.2 Tabla `proceso_log_detalle` (errores por fila)

```sql
CREATE TABLE proceso_log_detalle (
    id            BIGINT AUTO_INCREMENT PRIMARY KEY,
    proceso_id    BIGINT NOT NULL,
    fila_numero   BIGINT NOT NULL,
    columna       VARCHAR(128),
    tipo_error    ENUM('TYPE','NULL','LENGTH','DUPLICATE','SQL') NOT NULL,
    mensaje       TEXT NOT NULL,
    valor_origen  TEXT,
    creado_en     DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    FOREIGN KEY (proceso_id) REFERENCES proceso_log(id) ON DELETE CASCADE,
    INDEX idx_proceso (proceso_id)
) ENGINE=InnoDB CHARSET=utf8mb4;
```

### 5.3 Manejo de migraciones

`sqlx migrate add ...` para versionar. La CLI expone `csv2mysql init-db` que ejecuta todas las migraciones pendientes contra la conexión configurada.

---

## 6. Configuración

### 6.1 Conexión (`~/.config/csv2mysql/connection.toml`)

```toml
[database]
host = "localhost"
port = 3306
user = "etl_user"
password_env = "CSV2MYSQL_DB_PASS"   # leer desde variable de entorno
database = "contribuciones"
max_connections = 4
```

La password nunca se guarda en archivo en claro. Se lee desde `$CSV2MYSQL_DB_PASS` o, si falta, prompt interactivo en la TUI al arrancar.

### 6.2 Mapeo por dataset (`configs/rol_cobro.toml`)

```toml
[dataset]
name = "rol_cobro_semestral"
target_table = "contribuciones_staging"
delimiter = ","
encoding = "utf-8"
has_header = true
batch_size = 1000

[validation]
unique_keys = ["rol_completo", "periodo"]
max_errors = 100              # aborta si supera este número
abort_on_error = false

[[columns]]
csv = "ROL_COMPLETO"
db = "rol_completo"
type = "VARCHAR(20)"
nullable = false

[[columns]]
csv = "AVALUO_FISCAL"
db = "avaluo_fiscal"
type = "DECIMAL(15,2)"
nullable = false
default = 0

[[columns]]
csv = "FECHA_VIGENCIA"
db = "fecha_vigencia"
type = "DATE"
date_format = "%d/%m/%Y"
nullable = true
```

---

## 7. Componentes detallados

### 7.1 Parser CSV

- Sniff de delimitador: prueba `,`, `;`, `|`, `\t` sobre las primeras 4 KB.
- Detección de BOM y encoding (UTF-8 con fallback a Latin-1 vía `encoding_rs`).
- Lee el archivo completo en memoria (acotado a 100 MB), produce `Vec<RawRow>`.
- Reporta filas malformadas con número de línea exacto.

### 7.2 Inferencia de tipos

Estrategia muestreada: lee hasta 500 filas para inferir cada columna, prueba en este orden:

1. `BIGINT` (si todos los valores son enteros)
2. `DECIMAL(p,s)` (si son numéricos con decimales)
3. `DATE` con varios formatos chilenos (`dd/mm/yyyy`, `yyyy-mm-dd`)
4. `DATETIME(3)`
5. `VARCHAR(n)` con `n` = max_len * 1.5 redondeado
6. `TEXT` si max_len > 2048

El TOML del dataset gana siempre sobre la inferencia.

### 7.3 Validador

Reglas estándar:

- **Tipo:** cada celda parseable al tipo destino.
- **Null:** columnas con `nullable = false` no aceptan vacío.
- **Longitud:** valores no exceden la longitud declarada.
- **Duplicados:** se calcula el `HashSet` de las claves declaradas en `unique_keys`; colisiones se reportan con primer/segundo número de fila.
- **Formato fecha:** parser estricto contra `date_format`.

Salida: `ValidationReport { errors: Vec<RowError>, warnings: Vec<RowWarning>, ok: bool }`.

### 7.4 Loader

```rust
async fn load_file(/* ... */) -> Result<LoadStats, LoadError> {
    let mut tx = pool.begin().await?;
    let proceso_id = proc_log::create(&mut tx, &meta).await?;

    let mut inserted = 0u64;
    for chunk in rows.chunks(batch_size) {
        let stmt = build_multirow_insert(&schema, chunk);
        match retry::with_backoff(|| sqlx::query(&stmt).execute(&mut *tx)).await {
            Ok(r) => { inserted += r.rows_affected(); }
            Err(e) if e.is_unrecoverable() => {
                proc_log::fail(&mut tx, proceso_id, &e).await?;
                tx.rollback().await?;
                return Err(e.into());
            }
            Err(e) => warn!("batch retry failed: {e}"),
        }
        emit_progress(inserted, total);
    }
    proc_log::finish(&mut tx, proceso_id, inserted).await?;
    tx.commit().await?;
    Ok(LoadStats { inserted, /* ... */ })
}
```

- Batch parametrizado, no concatenación de strings (evita inyección SQL).
- Una transacción por archivo: o todo o nada (configurable a "best effort" para skip de filas con error).
- Backoff exponencial: 100 ms, 250 ms, 500 ms, 1 s, 2 s (5 reintentos máx).

### 7.5 Reintento y resume

- **Reintento intra-sesión:** errores transitorios (1213 deadlock, 1205 lock wait) → backoff y continuar.
- **Resume entre sesiones:** `proceso_log` registra el `archivo_hash` y `filas_insertadas`. Si el usuario carga el mismo archivo tras una falla, la TUI ofrece reanudar desde la fila N+1. Solo se permite si la tabla destino tiene clave única que detecte duplicados.

### 7.6 Logging

- `tracing` con dos appenders: archivo rotativo en `~/.local/share/csv2mysql/logs/` y un canal in-memory que el widget de log de la TUI consume.
- Niveles: `INFO` por defecto, `DEBUG` con `--verbose`.
- Cada evento crítico también se persiste en `proceso_log` o `proceso_log_detalle`.

---

## 8. Plan de ejecución por fases

### Fase 0 — Setup (0.5 día)

- Inicializar workspace Cargo, configurar `rustfmt`, `clippy`, hooks de pre-commit.
- Escribir `Dockerfile.dev` para levantar MySQL 8 local.
- CI básico (GitHub Actions o tu Jenkins): `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.

### Fase 1 — Core sin UI (2–3 días)

- Modelos (`Schema`, `Row`, `LoadStats`).
- Parser CSV + sniff + inferencia.
- Validador con tests sobre fixtures.
- Loader con `sqlx` y migraciones para `proceso_log`.
- CLI mínima `csv2mysql run --file X --config Y` que ya hace el ciclo completo en modo headless.
- **Hito:** cargar exitosamente un CSV de prueba (ej. 50 k filas del Rol de Cobro) y verificar `proceso_log`.

### Fase 2 — TUI básica (2 días)

- Layout `ratatui` con árbol estático y panel derecho.
- Conectar selección de archivo → inspección automática.
- Botón ejecutar → corre el flujo de Fase 1 en una task tokio, eventos por `mpsc`.
- Barra de progreso y log en vivo.
- **Hito:** flujo completo desde la TUI con feedback visual.

### Fase 3 — Validación y dry-run (1 día)

- Modo dry-run en panel derecho con resumen de errores.
- Modo validar (sin tocar BD).
- Render del `ValidationReport` en log.

### Fase 4 — Mapeo TOML y reintentos (1.5 días)

- Carga de configs por dataset, override de inferencia.
- Política de retry con backoff y métricas.
- Resume desde fila N+1.
- **Hito:** simular fallo de red mid-batch y verificar recuperación.

### Fase 5 — Refinamiento (1 día)

- Estados visuales (success/error/idle).
- Multi-selección y carga en cola.
- Editor de config integrado (`$EDITOR`).
- README con capturas y ejemplo de uso.

### Fase 6 — Empaquetado (0.5 día)

- `cargo build --release`.
- `.deb` con `cargo-deb` (instalación limpia en Ubuntu).
- Script de post-install que crea config dir y corre migraciones.

**Total estimado:** ~9 días-persona. Si trabajas el cap de 10 horas semanales en side projects, planifica 4 semanas calendario.

---

## 9. Pruebas

### 9.1 Unitarias

- `parser`: 20+ casos (delimitadores raros, encoding, BOM, filas malformadas).
- `schema`: inferencia con casos límite (columnas con un solo valor, NULL en mayoría).
- `validator`: cada regla por separado.
- `retry`: usar `tokio::time::pause()` para verificar backoff sin esperas reales.

### 9.2 Integración

- `testcontainers-rs` para levantar MySQL real en cada test.
- Cargar fixture de 1000 filas, verificar conteos y registros en `proceso_log`.
- Caso de duplicado: verificar que la transacción se revierta y `estado = 'FAILED'`.

### 9.3 Manuales (smoke)

- Archivo válido pequeño (1 k filas).
- Archivo válido grande (95 MB, cerca del límite).
- Archivo con encoding Latin-1 y caracteres `ñ`.
- Archivo con columna que excede `VARCHAR(50)`.
- Pérdida de conexión a mitad de carga.

---

## 10. Riesgos y mitigaciones

| Riesgo                                                      | Probabilidad | Mitigación                                                                                |
| ----------------------------------------------------------- | ------------ | ----------------------------------------------------------------------------------------- |
| `sqlx` macros offline requieren `DATABASE_URL` en compile   | Media        | Usar `sqlx::query` (runtime) en lugar de `query!` para empezar; o `cargo sqlx prepare`.   |
| Encoding Latin-1 en archivos del SII                        | Alta         | Detección con `encoding_rs` desde el día 1, fallback explícito.                            |
| TUI con poco refresh ante archivos grandes                  | Media        | Render con throttling a 30 fps máx, eventos por mpsc no bloqueantes.                       |
| Inferencia de tipos incorrecta en columnas mixtas           | Alta         | Mapeo TOML manual disponible siempre como override.                                        |
| MySQL `max_allowed_packet` con batches grandes              | Baja         | Detectar al conectar (`SHOW VARIABLES`), ajustar `batch_size` automáticamente.             |
| Ratatui aprendizaje (si nunca lo has usado)                 | Media        | Empezar por el ejemplo `user_input` del repo y un layout `Layout::horizontal` simple.      |

---

## 11. Decisiones tomadas y abiertas

**Tomadas (en este chat):**

- TUI con ratatui (no clap puro, no híbrido).
- Archivos < 100 MB, carga en memoria.
- Inferencia + mapeo TOML + validación + dry-run + retries/resume.

**Abiertas para confirmar antes de Fase 1:**

1. ¿Versión de MySQL objetivo? (8.0 vs 5.7 cambia tipos como `DATETIME(3)`)
2. ¿La tabla destino la crea la herramienta, o asumimos que ya existe? Recomendación: ya existe; la herramienta solo inserta. El caso "auto-create" puede ser un flag opcional `--create-table-if-missing`.
3. ¿Multi-archivo en cola en v1, o uno a la vez? Recomendación: uno a la vez; cola en v1.1.
4. ¿Necesitas integración con tu stack SUPEREDUC (auth ClaveÚnica, logs centralizados)? Si la herramienta solo corre local en tu máquina, no aplica.

---

## 12. Próximos pasos concretos

1. Crear el repo, inicializar el workspace con la estructura de la sección 3.1.
2. Escribir `migrations/0001_proceso_log.sql` (sección 5).
3. Implementar `core::parser` con tests sobre 3 CSV de muestra del Rol de Cobro.
4. Implementar `core::loader` mínimo y verificar el ciclo completo con `cargo run -- run --file ... --config ...`.
5. Recién entonces empezar la TUI.

Esa secuencia te permite tener algo funcional y testeable a la mitad del proyecto, antes de invertir en la capa visual.
