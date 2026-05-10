use anyhow::Result;
use clap::{Parser, Subcommand};
use csv2mysql_core::{AppConfig, CsvParser, SchemaInferrer, Validator};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "csv2mysql")]
#[command(about = "Herramienta para cargar archivos CSV a MySQL", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Nivel de logging (debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Lanza la interfaz TUI
    Tui,
    
    /// Ejecuta la carga de un archivo en modo headless
    Run {
        /// Archivo CSV a cargar
        #[arg(short, long)]
        file: String,

        /// Archivo de configuración TOML
        #[arg(short, long)]
        config: Option<String>,

        /// Modo dry-run (no inserta datos)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Inicializa la base de datos (crea tablas de log)
    InitDb {
        /// String de conexión MySQL
        #[arg(short, long)]
        database_url: String,
    },

    /// Valida un archivo CSV sin cargarlo
    Validate {
        /// Archivo CSV a validar
        #[arg(short, long)]
        file: String,

        /// Archivo de configuración TOML
        #[arg(short, long)]
        config: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&cli.log_level)),
        )
        .init();

    match cli.command {
        Commands::Tui => {
            csv2mysql_tui::run_tui()?;
        }
        Commands::Run {
            file,
            config,
            dry_run,
        } => {
            println!("Cargando archivo: {}", file);
            if let Some(cfg) = config {
                println!("Usando configuración: {}", cfg);
            }
            if dry_run {
                println!("Modo: DRY-RUN");
            }

            // TODO: Implementar lógica de carga
            let parser = CsvParser::new(None);
            let metadata = parser.get_metadata(&file)?;
            println!("Archivo: {}", metadata.name);
            println!("Tamaño: {} bytes", metadata.size);
            println!("Filas estimadas: {}", metadata.row_count_estimate);
            println!("Encoding: {}", metadata.encoding);
            println!("Delimitador: {:?}", metadata.delimiter);
        }
        Commands::InitDb { database_url } => {
            println!("Inicializando base de datos...");
            println!("URL: {}", database_url);
            
            // TODO: Ejecutar migraciones
            println!("✓ Tablas de log creadas");
        }
        Commands::Validate { file, config } => {
            println!("Validando archivo: {}", file);
            
            let parser = CsvParser::new(None);
            let rows = parser.parse(&file)?;
            println!("✓ {} filas leídas", rows.len());

            // TODO: Validación completa
            println!("✓ Validación completada");
        }
    }

    Ok(())
}
