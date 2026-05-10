use std::path::PathBuf;
use walkdir::WalkDir;

pub struct App {
    pub files: Vec<PathBuf>,
    pub selected: usize,
    pub current_dir: PathBuf,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        let mut app = Self {
            files: Vec::new(),
            selected: 0,
            current_dir: root.clone(),
        };
        app.refresh_files();
        app
    }

    pub fn refresh_files(&mut self) {
        self.files.clear();
        
        for entry in WalkDir::new(&self.current_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path().to_path_buf();
            if path != self.current_dir {
                if path.is_dir() || path.extension().map(|e| e == "csv" || e == "tsv").unwrap_or(false) {
                    self.files.push(path);
                }
            }
        }

        self.files.sort();
        if self.selected >= self.files.len() && !self.files.is_empty() {
            self.selected = self.files.len() - 1;
        }
    }

    pub fn next(&mut self) {
        if !self.files.is_empty() {
            self.selected = (self.selected + 1) % self.files.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.files.is_empty() {
            if self.selected > 0 {
                self.selected -= 1;
            } else {
                self.selected = self.files.len() - 1;
            }
        }
    }

    pub fn select(&mut self) {
        if let Some(file) = self.files.get(self.selected) {
            if file.is_dir() {
                self.current_dir = file.clone();
                self.selected = 0;
                self.refresh_files();
            }
        }
    }

    pub fn back(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.selected = 0;
            self.refresh_files();
        }
    }

    pub fn execute(&mut self) {
        // TODO: Implementar ejecución real
        tracing::info!("Ejecutando carga del archivo seleccionado");
    }

    pub fn dry_run(&mut self) {
        // TODO: Implementar dry-run
        tracing::info!("Ejecutando dry-run del archivo seleccionado");
    }

    pub fn validate(&mut self) {
        // TODO: Implementar validación
        tracing::info!("Validando archivo seleccionado");
    }
}
