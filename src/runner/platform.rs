use std::path::{Path, PathBuf};

/// Proporciona abstracciones para diferencias between-plataforma.
pub struct Platform;

impl Platform {
    /// Retorna la extensión del ejecutable según la plataforma actual.
    ///
    /// - Windows: ".exe"
    /// - Linux, macOS, etc.: ""
    pub fn executable_extension() -> &'static str {
        if cfg!(target_os = "windows") {
            ".exe"
        } else {
            ""
        }
    }

    /// Construye la ruta del ejecutable final con la extensión apropiada.
    ///
    /// # Ejemplo
    /// ```ignore
    /// // En Windows:
    /// let exe = Platform::as_executable_path(Path::new("output"));
    /// assert_eq!(exe, PathBuf::from("output.exe"));
    ///
    /// // En Linux:
    /// let exe = Platform::as_executable_path(Path::new("output"));
    /// assert_eq!(exe, PathBuf::from("output"));
    /// ```
    pub fn as_executable_path(path: &Path) -> PathBuf {
        let ext = Self::executable_extension();
        if ext.is_empty() {
            path.to_path_buf()
        } else {
            let mut new_path = path.to_path_buf();
            let new_name = format!("{}{}", path.display(), ext);
            new_path.set_file_name(&new_name);
            new_path
        }
    }

    /// Retorna el nombre del comando clang para esta plataforma.
    pub fn clang_command() -> &'static str {
        "clang"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executable_extension_windows() {
        #[cfg(target_os = "windows")]
        assert_eq!(Platform::executable_extension(), ".exe");

        #[cfg(not(target_os = "windows"))]
        assert_eq!(Platform::executable_extension(), "");
    }

    #[test]
    fn test_as_executable_path() {
        let base = PathBuf::from("test/output");
        let exe = Platform::as_executable_path(&base);

        #[cfg(target_os = "windows")]
        {
            assert!(exe.to_string_lossy().contains(".exe"));
        }

        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(exe, base);
        }
    }
}
