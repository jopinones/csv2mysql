# csv2mysql

Herramienta TUI (Text User Interface) en Rust para carga de archivos CSV a MySQL con validación, dry-run y logging completo.

## Características

- 🎨 **Interfaz TUI interactiva** con ratatui
- 📁 **Navegación de carpetas** con árbol expandible
- 🔍 **Inferencia automática de schema** (tipos, encoding, delimitador)
- ✅ **Validación pre-carga** con reporte detallado
- 🧪 **Modo dry-run** para probar sin insertar datos
- 📊 **Barra de progreso** en tiempo real
- 🔄 **Reintentos automáticos** con backoff exponencial
- 📝 **Logging completo** en MySQL (tabla `proceso_log`)
- ⚡ **Carga por lotes** con transacciones

## Requisitos

- Rust 1.79 o superior
- MySQL 8.0
- Ubuntu/Linux (probado en Ubuntu 22.04/24.04)

## Instalación

### Desde código fuente

```bash
# Clonar el repositorio
git clone <repo-url>
cd csv2mysql

# Compilar en modo release
cargo build --release

# El binario estará en target/release/csv2mysql
```

### Instalación global

```bash
cargo install --path crates/cli
```

## Configuración

### 1. Configurar base de datos

Edita `configs/default.toml`:

```toml
[database]
host = "localhost"
port = 3306
database = "contribuciones"
username = "etl_user"
password = "tu_password"
```

### 2. Inicializar tablas de log

```bash
mysql -u root -p < migrations/0001_proceso_log.sql
```

O usando la herramienta:

```bash
csv2mysql init-db --database-url "mysql://user:pass@localhost:3306/db"
```

### 3. Crear configuración de dataset (opcional)

Copia y edita `configs/rol_cobro.toml` para mapear tus columnas CSV a campos MySQL.

## Uso

### Modo TUI (recomendado)

```bash
csv2mysql tui
```

**Atajos de teclado:**

- `↑/↓` - Navegar árbol de archivos
- `→/Enter` - Expandir carpeta o seleccionar archivo
- `←` - Volver atrás
- `E` - Ejecutar carga
- `D` - Dry-run (simular sin insertar)
- `V` - Solo validar
- `q` - Salir

### Modo headless (CLI)

```bash
# Carga simple
csv2mysql run --file datos.csv

# Con configuración personalizada
csv2mysql run --file datos.csv --config configs/rol_cobro.toml

# Dry-run
csv2mysql run --file datos.csv --dry-run

# Solo validar
csv2mysql validate --file datos.csv
```

## Estructura del proyecto

```
csv2mysql/
├── crates/
│   ├── core/          # Lógica de negocio
│   │   ├── config.rs  # Configuración
│   │   ├── parser.rs  # Lectura CSV
│   │   ├── schema.rs  # Inferencia de tipos
│   │   ├── validator.rs # Validaciones
│   │   ├── loader.rs  # Carga a MySQL
│   │   └── proc_log.rs # Logging
│   ├── tui/           # Interfaz TUI
│   └── cli/           # Entry point CLI
├── migrations/        # Scripts SQL
├── configs/           # Configuraciones de ejemplo
└── examples/          # CSVs de prueba
```

## Configuración de dataset

Ejemplo de `configs/mi_dataset.toml`:

```toml
table_name = "mi_tabla"
delimiter = ';'
encoding = "UTF-8"
has_header = true
unique_keys = ["id"]
batch_size = 1000

[columns.ID]
rename = "id"
sql_type = "INT"
nullable = false

[columns.NOMBRE]
rename = "nombre"
sql_type = "VARCHAR(255)"
nullable = false

[columns.MONTO]
rename = "monto"
sql_type = "DECIMAL(18,2)"
nullable = true
default = "0.00"

[columns.FECHA]
rename = "fecha"
sql_type = "DATE"
date_format = "%d/%m/%Y"
```

## Inferencia automática de tipos

Si no proporcionas configuración, la herramienta infiere:

- `INT` - si todos los valores son enteros < 2^31
- `BIGINT` - si son enteros mayores
- `DECIMAL(18,2)` - si son decimales
- `DATE` - si matchean formatos chilenos (dd/mm/yyyy)
- `DATETIME(3)` - si incluyen hora
- `VARCHAR(n)` - strings (n = max_len * 1.5)
- `TEXT` - si n > 2048

## Validaciones

La herramienta valida:

- ✓ Tipos de datos correctos por columna
- ✓ Valores NULL en columnas no-nulables
- ✓ Longitudes máximas (VARCHAR)
- ✓ Duplicados en claves únicas
- ✓ Formatos de fecha

## Logging

Cada ejecución crea un registro en `proceso_log`:

```sql
SELECT * FROM proceso_log 
WHERE archivo_nombre = 'datos.csv' 
ORDER BY inicio DESC LIMIT 1;
```

Columnas útiles:
- `estado` - RUNNING, SUCCESS, FAILED, PARTIAL
- `filas_insertadas` - Filas cargadas exitosamente
- `duracion_ms` - Tiempo de ejecución
- `error_mensaje` - Detalle si falló

## Troubleshooting

### Error: "max_allowed_packet"

```sql
SET GLOBAL max_allowed_packet=67108864; -- 64MB
```

### Error: encoding

La herramienta detecta automáticamente UTF-8 y Windows-1252 (Latin-1). 
Para otros encodings, especifica en el config:

```toml
encoding = "ISO-8859-1"
```

### Archivos muy grandes (>100MB)

La v1 carga en memoria. Para archivos grandes:

1. Usa `split` en Linux:
   ```bash
   split -l 50000 archivo_grande.csv chunk_
   ```

2. O espera la v2 con streaming 😊

## Desarrollo

```bash
# Tests
cargo test

# Linting
cargo clippy -- -D warnings

# Formato
cargo fmt --check

# Compilar en debug
cargo build
```

## Licencia

MIT

## Autor

SUPEREDUC Team
