# Revisión de Código — csv2mysql

**Fecha:** 2026-05-10  
**Revisado por:** Claude Code (Sonnet 4.6)  
**Alcance:** Todos los archivos fuente del workspace (`crates/core`, `crates/tui`, `crates/cli`)

---

## Resumen Ejecutivo

Se revisaron 12 archivos Rust y 3 manifiestos `Cargo.toml`. Se encontraron **6 bugs** en 5 archivos distintos:

| # | Severidad | Archivo | Descripción |
|---|-----------|---------|-------------|
| 1 | 🔴 Error de compilación | `core/src/config.rs` | Tipo `toml::de::Error` sin conversión a `Csv2MysqlError` |
| 2 | 🔴 Error de compilación | `core/src/retry.rs` | Closure con captura mutable inválida en async + error no-retryable en test |
| 3 | 🟠 Bug funcional | `core/src/parser.rs` | `detect_delimiter` falla con archivos Windows-1252 |
| 4 | 🟠 Bug funcional | `core/src/schema.rs` | Código muerto + posible `VARCHAR(0)` inválido en MySQL |
| 5 | 🟠 Bug de seguridad | `core/src/loader.rs` | Identificadores SQL sin escapar (inyección de nombres) |
| 6 | 🟡 Warning | `tui/src/main.rs` | Import `Text` no usado |

---

## Detalle de Bugs y Correcciones

---

### Bug 1 — Error de compilación: `toml::de::Error` sin conversión

**Archivo:** `crates/core/src/config.rs` — líneas 96 y 102  
**Severidad:** 🔴 Error de compilación

#### Problema

`toml::from_str()` retorna `Result<T, toml::de::Error>`. El operador `?` requiere que el tipo de error pueda convertirse a `Csv2MysqlError` via `From`. La definición de `Csv2MysqlError` incluye `Config(#[from] config::ConfigError)` (del crate `config`), pero **no** tiene `From<toml::de::Error>`. El código no compilaba.

```rust
// ❌ ANTES — no compila
pub fn load_dataset_config<P: AsRef<Path>>(path: P) -> Result<DatasetConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: DatasetConfig = toml::from_str(&content)?; // toml::de::Error no convierte
    Ok(config)
}
```

#### Corrección

```rust
// ✅ DESPUÉS — mapeo explícito del error
pub fn load_dataset_config<P: AsRef<Path>>(path: P) -> Result<DatasetConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: DatasetConfig = toml::from_str(&content)
        .map_err(|e| Csv2MysqlError::Parse(e.to_string()))?;
    Ok(config)
}
```

Mismo cambio aplicado en `load_app_config`.

---

### Bug 2 — Test roto: error no-retryable + captura mutable inválida

**Archivo:** `crates/core/src/retry.rs` — líneas 57–75  
**Severidad:** 🔴 Error de compilación + lógica incorrecta

#### Problema A: Error no-retryable

El test usaba `Csv2MysqlError::General(...)` como error simulado. El método `is_retryable()` retorna `false` para `General` (solo retorna `true` para `Io` y ciertos errores de `Database`). Por lo tanto, el mecanismo de retry terminaba en el primer intento sin reintentar, haciendo que los asserts fallaran:

```rust
// is_retryable() devuelve false para General → sale en el primer error
Err(Csv2MysqlError::General("Fallo temporal".to_string()))
// assert!(result.is_ok()) → FALLA
// assert_eq!(call_count, 2) → FALLA (call_count es 1)
```

#### Problema B: Captura mutable en async con `FnMut`

La closure capturaba `&mut call_count` dentro de un async block. El tipo `FnMut() -> Fut` no garantiza al borrow checker que los futuros sean secuenciales (aunque en la implementación sí lo son), causando un error de compilación por multiple mutable borrows:

```rust
// ❌ ANTES — no compila
let mut call_count = 0;
let result = policy
    .execute(|| async {
        call_count += 1; // &mut call_count capturado en async: error de borrow checker
        ...
    })
    .await;
```

#### Corrección

Uso de `Arc<AtomicU32>` para el contador (seguro en contextos async) y `Csv2MysqlError::Io` que sí es retryable:

```rust
// ✅ DESPUÉS
use std::sync::{atomic::{AtomicU32, Ordering}, Arc};

let policy = RetryPolicy::default();
let call_count = Arc::new(AtomicU32::new(0));

let counter = call_count.clone();
let result = policy
    .execute(move || {
        let counter = counter.clone();
        async move {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Err(Csv2MysqlError::Io(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "Fallo temporal",
                )))
            } else {
                Ok(42)
            }
        }
    })
    .await;

assert!(result.is_ok());
assert_eq!(result.unwrap(), 42);
assert_eq!(call_count.load(Ordering::SeqCst), 2);
```

---

### Bug 3 — Bug funcional: `detect_delimiter` falla con Windows-1252

**Archivo:** `crates/core/src/parser.rs` — líneas 52–54  
**Severidad:** 🟠 Bug funcional

#### Problema

`detect_delimiter` usaba `read_to_string()` para leer el archivo completo en un `String`. Este método falla con un error de UTF-8 si el archivo no es UTF-8 válido. Los archivos de organismos chilenos frecuentemente vienen en Windows-1252 (Latin-1), encoding explícitamente soportado por el proyecto. Esto hacía que la detección de delimitador fallara para exactamente los archivos para los que fue diseñado.

Además, leer el archivo completo en memoria es innecesario: solo se necesita la primera línea.

```rust
// ❌ ANTES — falla para Windows-1252
let mut file = File::open(path)?;
let mut buffer = String::new();
file.read_to_string(&mut buffer)?; // Error: invalid utf-8 sequence
let first_line = buffer.lines().next().unwrap_or("");
```

#### Corrección

Los delimitadores candidatos (`,`, `;`, `\t`, `|`) son todos caracteres ASCII (< 128), representados de forma idéntica en UTF-8 y Windows-1252. Se puede operar sobre bytes sin importar el encoding:

```rust
// ✅ DESPUÉS — funciona con cualquier encoding
let mut file = File::open(path)?;
let mut buffer = vec![0u8; MAX_SNIFF_BYTES]; // Solo los primeros 8 KB
let n = file.read(&mut buffer)?;
buffer.truncate(n);

let first_line = buffer.split(|&b| b == b'\n').next().unwrap_or(&[]);

let comma_count     = first_line.iter().filter(|&&b| b == b',').count();
let semicolon_count = first_line.iter().filter(|&&b| b == b';').count();
let tab_count       = first_line.iter().filter(|&&b| b == b'\t').count();
let pipe_count      = first_line.iter().filter(|&&b| b == b'|').count();
```

Beneficio adicional: ya no lee el archivo completo en memoria, usa los mismos 8 KB que `detect_encoding`.

---

### Bug 4 — Código muerto + `VARCHAR(0)` inválido en MySQL

**Archivo:** `crates/core/src/schema.rs` — líneas 122–129  
**Severidad:** 🟠 Bug funcional

#### Problema A: Código muerto (rama inalcanzable)

`length` se calculaba con `.min(2048)`, lo que garantiza que su valor nunca supere 2048. Sin embargo, a continuación se evaluaba `if length > 2048`, condición que **nunca puede ser verdadera**. La rama `Ok(SqlType::Text)` era código muerto:

```rust
// ❌ ANTES — if inalcanzable
let length = ((max_len as f64 * 1.5) as u16).min(2048); // máximo es 2048
if length > 2048 {           // NUNCA puede ser true
    Ok(SqlType::Text)        // código muerto
} else {
    Ok(SqlType::VarChar { length })
}
```

#### Problema B: `VARCHAR(0)` inválido

Si todos los valores de una columna son strings vacíos, `max_len = 0`, dando `length = 0`. MySQL no acepta `VARCHAR(0)` y rechaza la query de creación de tabla.

#### Problema C: Overflow de `u16`

El cast `(max_len as f64 * 1.5) as u16` puede producir overflow silencioso si `max_len` es muy grande (> ~43690 chars): `u16` wrapping produce un valor incorrecto.

#### Corrección

```rust
// ✅ DESPUÉS — sin overflow, sin código muerto, sin longitud 0
let max_len = values.iter().map(|v| v.len()).max().unwrap_or(0);
let length_usize = (max_len * 3 / 2).max(1); // operación en usize, sin overflow
if length_usize > 21845 {   // umbral real: límite VARCHAR con utf8 en MySQL
    Ok(SqlType::Text)
} else {
    Ok(SqlType::VarChar { length: length_usize as u16 })
}
```

> **Nota:** El umbral `21845` es el límite de `VARCHAR` con charset `utf8` en MySQL (cada carácter ocupa hasta 3 bytes; 65535 / 3 = 21845). Con `utf8mb4` el límite es 16383. Se recomienda configurar el umbral según el charset de la base de datos destino.

---

### Bug 5 — Identificadores SQL sin escapar

**Archivo:** `crates/core/src/loader.rs` — línea 84  
**Severidad:** 🟠 Bug de seguridad / correctitud

#### Problema

El nombre de la tabla y los nombres de las columnas se interpolaban directamente en el string SQL sin ningún tipo de escape. Si un nombre de columna coincide con una palabra reservada de MySQL (`order`, `group`, `key`, `index`, `select`, etc.) o contiene caracteres especiales, la query falla. Además, si los nombres provienen de headers de CSV no controlados, representa un vector de inyección SQL.

```sql
-- ❌ Ejemplo de query rota con columna llamada "order":
INSERT INTO rol_cobro (rut, nombre, order) VALUES (...)
-- MySQL error: syntax error near 'order'
```

```rust
// ❌ ANTES — sin escape
let columns: Vec<String> = schema.columns.iter().map(|c| c.name.clone()).collect();
let col_list = columns.join(", ");
format!("INSERT INTO {} ({}) VALUES ...", schema.table_name, col_list)
```

#### Corrección

Uso de backticks para citar identificadores MySQL, escapando los backticks internos (duplicándolos, como es el estándar SQL):

```rust
// ✅ DESPUÉS — identificadores correctamente citados
let quote_ident = |s: &str| format!("`{}`", s.replace('`', "``"));

let col_list = schema
    .columns
    .iter()
    .map(|c| quote_ident(&c.name))
    .collect::<Vec<_>>()
    .join(", ");

format!(
    "INSERT INTO {} ({}) VALUES ...",
    quote_ident(&schema.table_name),
    col_list,
)
```

---

### Bug 6 — Import no usado: `Text`

**Archivo:** `crates/tui/src/main.rs` — línea 11  
**Severidad:** 🟡 Warning del compilador

#### Problema

`Text` se importaba de `ratatui` pero nunca se usaba en el archivo. El compilador de Rust emite un warning `unused import`.

```rust
// ❌ ANTES
use ratatui::text::{Line, Span, Text}; // Text no se usa
```

#### Corrección

```rust
// ✅ DESPUÉS
use ratatui::text::{Line, Span};
```

---

## Archivos Modificados

| Archivo | Cambios |
|---------|---------|
| `crates/core/src/config.rs` | `.map_err()` para `toml::de::Error` en `load_dataset_config` y `load_app_config` |
| `crates/core/src/retry.rs` | Test reescrito con `Arc<AtomicU32>` y error `Io` (retryable) |
| `crates/core/src/parser.rs` | `detect_delimiter` opera sobre bytes en lugar de `String` UTF-8 |
| `crates/core/src/schema.rs` | Umbral correcto para TEXT, mínimo de longitud 1, sin overflow |
| `crates/core/src/loader.rs` | `quote_ident` con backticks para tabla y columnas |
| `crates/tui/src/main.rs` | Eliminado import `Text` no usado |

## Observaciones adicionales (sin corrección en esta revisión)

- **`cli/src/main.rs`:** Los subcomandos `Run`, `Validate` e `InitDb` tienen lógica incompleta (marcada con `// TODO`). El parámetro `config` se acepta pero no se pasa al parser.
- **`tui/src/app.rs`:** `execute()`, `dry_run()` y `validate()` son stubs vacíos. Pendiente conectar con el crate `core` mediante canales `mpsc`.
- **`loader.rs` (build_insert_statement):** El escape de valores usa `replace('\'', "''")` (escape SQL estándar), que es correcto para MySQL en modo `ANSI_QUOTES`. Sin embargo, para máxima seguridad se recomienda migrar a queries parametrizadas con `sqlx::query!()` cuando la lógica esté integrada con `ProcessLogger`.
