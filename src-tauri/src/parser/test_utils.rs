use std::sync::atomic::{AtomicUsize, Ordering};
use std::path::{Path, PathBuf};
use std::fs;

static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct TempFile {
    pub path: PathBuf,
}

impl TempFile {
    pub fn new<C: AsRef<[u8]>>(content: C) -> Self {
        Self::new_with_filename(content, "temp_file.tmp")
    }

    pub fn new_with_filename<C: AsRef<[u8]>>(content: C, filename_pattern: &str) -> Self {
        let count = FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        let filename = format!("{}_{}", count, filename_pattern);
        path.push(filename);
        fs::write(&path, content.as_ref()).unwrap();
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn scan_dir<F>(
    dir: &Path,
    ext: &str,
    files_scanned: &mut usize,
    total_errors: &mut usize,
    failed_files: &mut Vec<(PathBuf, usize)>,
    parse_fn: F,
) where
    F: Fn(&Path) -> Option<usize> + Copy,
{
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, ext, files_scanned, total_errors, failed_files, parse_fn);
            } else if path.extension().map_or(false, |e| e == ext) {
                *files_scanned += 1;
                if let Some(err_count) = parse_fn(&path) {
                    if err_count > 0 {
                        *total_errors += err_count;
                        failed_files.push((path.clone(), err_count));
                    }
                }
            }
        }
    }
}

pub fn run_field_test<F>(ext: &str, test_name: &str, parse_fn: F)
where
    F: Fn(&Path) -> Option<usize> + Copy,
{
    let mod_dir = match dirs::document_dir() {
        Some(mut p) => {
            p.push("Paradox Interactive");
            p.push("Hearts of Iron IV");
            p.push("mod");
            p
        }
        None => {
            println!("Пропущено: не удалось определить папку документов.");
            return;
        }
    };

    if !mod_dir.exists() {
        println!("Пропущено: папка модов HOI4 не найдена по пути {:?}", mod_dir);
        return;
    }

    println!("Сканирование папки модов ({}) на .{} файлы: {:?}", test_name, ext, mod_dir);
    let mut files_scanned = 0;
    let mut total_errors = 0;
    let mut failed_files = Vec::new();

    scan_dir(
        &mod_dir,
        ext,
        &mut files_scanned,
        &mut total_errors,
        &mut failed_files,
        parse_fn,
    );

    println!("\n=== ОТЧЕТ ПО ПОЛЕВОМУ ТЕСТИРОВАНИЮ {} ===", test_name);
    println!("Просканировано файлов: {}", files_scanned);
    println!("Файлов с предупреждениями/ошибками: {}", failed_files.len());
    println!("Всего обнаружено проблем/ошибок: {}", total_errors);
    
    if !failed_files.is_empty() {
        println!("\nТоп 10 файлов с наибольшим количеством проблем:");
        failed_files.sort_by(|a, b| b.1.cmp(&a.1));
        for (path, err_count) in failed_files.iter().take(10) {
            println!(
                "- {:?} (проблем: {})",
                path.file_name().unwrap_or_default(),
                err_count
            );
        }
    }
    println!("=====================================\n");
}
