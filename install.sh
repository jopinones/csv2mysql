#!/bin/bash
# Script de instalación para csv2mysql en Ubuntu

set -e

echo "=== csv2mysql - Instalación ==="
echo ""

# Verificar Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust no está instalado"
    echo "Instala Rust desde: https://rustup.rs/"
    echo ""
    echo "Ejecuta: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✓ Rust encontrado: $(rustc --version)"

# Verificar MySQL
if ! command -v mysql &> /dev/null; then
    echo "⚠️  MySQL client no encontrado"
    echo "Puedes instalarlo con: sudo apt-get install mysql-client"
fi

echo ""
echo "Compilando csv2mysql..."
cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo "✓ Compilación exitosa"
    echo ""
    echo "El binario está en: target/release/csv2mysql"
    echo ""
    echo "Para instalarlo globalmente:"
    echo "  sudo cp target/release/csv2mysql /usr/local/bin/"
    echo ""
    echo "O ejecuta: cargo install --path crates/cli"
    echo ""
    echo "Siguiente paso:"
    echo "  1. Edita configs/default.toml con tu configuración de MySQL"
    echo "  2. Ejecuta las migraciones: mysql -u root -p < migrations/0001_proceso_log.sql"
    echo "  3. Ejecuta: csv2mysql tui"
else
    echo ""
    echo "❌ Error en la compilación"
    exit 1
fi
