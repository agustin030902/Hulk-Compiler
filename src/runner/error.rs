use std::{fmt, io, path::PathBuf};

/// Tipo de error que puede ocurrir en operaciones del runner.
#[derive(Debug)]
pub enum RunnerError {
    /// El archivo de entrada no existe.
    InputMissing(PathBuf),

    /// Nivel de optimización inválido (debe estar entre 0 y 3).
    InvalidOptLevel(u8),

    /// Error I/O con contexto.
    Io {
        /// Descripción de la acción que se intentaba.
        action: &'static str,
        /// Ruta del archivo (si aplica).
        path: Option<PathBuf>,
        /// Error subyacente.
        source: io::Error,
    },

    /// clang falló durante la compilación.
    ClangFailed {
        /// Código de salida de clang.
        status: Option<i32>,
        /// Salida estándar de error.
        stderr: String,
        /// Salida estándar.
        stdout: String,
    },
}

impl RunnerError {
    /// Constructor conveniente para errores I/O.
    pub fn io(action: &'static str, path: Option<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            action,
            path,
            source,
        }
    }
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunnerError::InputMissing(path) => {
                write!(f, "Input file not found: '{}'", path.display())
            }
            RunnerError::InvalidOptLevel(level) => {
                write!(f, "Invalid optimization level '{}'. Use 0..=3.", level)
            }
            RunnerError::Io {
                action,
                path,
                source,
            } => {
                if let Some(p) = path {
                    write!(
                        f,
                        "I/O error while {action} '{path}': {source}",
                        path = p.display()
                    )
                } else {
                    write!(f, "I/O error while {action}: {source}")
                }
            }
            RunnerError::ClangFailed {
                status,
                stderr,
                stdout,
            } => {
                let stderr_trimmed = stderr.trim();
                let stdout_trimmed = stdout.trim();

                if !stderr_trimmed.is_empty() {
                    write!(
                        f,
                        "clang failed with exit code {:?}: {}",
                        status, stderr_trimmed
                    )
                } else if !stdout_trimmed.is_empty() {
                    write!(
                        f,
                        "clang failed with exit code {:?}: {}",
                        status, stdout_trimmed
                    )
                } else {
                    write!(f, "clang failed with exit code {:?}", status)
                }
            }
        }
    }
}

impl std::error::Error for RunnerError {}

impl From<io::Error> for RunnerError {
    fn from(err: io::Error) -> Self {
        RunnerError::Io {
            action: "performing I/O operation",
            path: None,
            source: err,
        }
    }
}
