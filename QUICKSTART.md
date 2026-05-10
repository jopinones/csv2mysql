# 🚀 INICIO RÁPIDO - csv2mysql

## Instalación en 3 pasos

### 1. Compilar

```bash
cd csv2mysql
chmod +x install.sh
./install.sh
```

### 2. Configurar MySQL

```bash
# Editar credenciales
nano configs/default.toml

# Crear tablas
mysql -u root -p < migrations/0001_proceso_log.sql
```

### 3. Ejecutar

```bash
# Opción A: Instalar globalmente
cargo install --path crates/cli
csv2mysql tui

# Opción B: Ejecutar desde target
./target/release/csv2mysql tui
```

## Prueba rápida

```bash
# Validar el CSV de ejemplo
./target/release/csv2mysql validate \
  --file examples/rol_cobro_ejemplo.csv

# Dry-run (simula sin insertar)
./target/release/csv2mysql run \
  --file examples/rol_cobro_ejemplo.csv \
  --config configs/rol_cobro.toml \
  --dry-run
```

## Uso típico con TUI

1. Ejecuta `csv2mysql tui`
2. Navega con `↑↓` hasta tu CSV
3. Presiona `Enter` para seleccionar
4. Presiona `V` para validar
5. Presiona `D` para dry-run
6. Presiona `E` para ejecutar la carga real
7. Presiona `q` para salir

## Verificar resultados

```sql
-- Ver log de procesos
SELECT * FROM proceso_log 
ORDER BY inicio DESC 
LIMIT 10;

-- Ver detalles de un proceso
SELECT * FROM proceso_log 
WHERE id = 1;

-- Ver errores
SELECT * FROM proceso_log_detalle 
WHERE proceso_id = 1 
  AND tipo = 'ERROR';
```

## Atajos TUI

| Tecla | Acción |
|-------|--------|
| `↑↓` | Navegar |
| `Enter` o `→` | Seleccionar/Expandir |
| `←` | Volver |
| `E` | Ejecutar carga |
| `D` | Dry-run |
| `V` | Validar |
| `q` | Salir |

## Troubleshooting

### No compila

```bash
# Verificar versión de Rust
rustc --version

# Debe ser >= 1.79
# Si no, actualiza:
rustup update
```

### Error de conexión MySQL

```bash
# Verificar que MySQL esté corriendo
sudo systemctl status mysql

# Probar conexión
mysql -h localhost -u etl_user -p
```

### Archivo muy grande

La v1 tiene límite de 100MB. Para archivos más grandes:

```bash
# Dividir en chunks de 50k líneas
split -l 50000 archivo_grande.csv chunk_

# Cargar cada chunk
for file in chunk_*; do
    csv2mysql run --file $file
done
```

## Próximos pasos

- 📖 Lee `README.md` para documentación completa
- 🎨 Revisa `design/Carga_TUI.html` para ver el mockup de diseño
- 📋 Lee `design/plan_csv2mysql.md` para entender la arquitectura
- ⚙️ Copia y edita `configs/rol_cobro.toml` para tus datasets

## Soporte

- Issues: (tu repo aquí)
- Docs: README.md
- Plan: design/plan_csv2mysql.md
