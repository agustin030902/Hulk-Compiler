pub mod error;
pub mod platform;

use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub use error::RunnerError;
use platform::Platform;

/// Opciones de configuración para compilar LLVM IR con clang.
#[derive(Debug, Clone)]
pub struct RunnerOptions {
    /// Comando/ruta del binario clang a utilizar.
    pub clang_bin: String,

    /// Nivel de optimización: 0, 1, 2, ó 3.
    pub opt_level: u8,

    /// Argumentos adicionales a pasar a clang.
    pub extra_args: Vec<String>,
}

impl Default for RunnerOptions {
    fn default() -> Self {
        Self {
            clang_bin: "clang".to_string(),
            opt_level: 2,
            extra_args: Vec::new(),
        }
    }
}

impl RunnerOptions {
    /// Valida que las opciones sean correctas.
    pub fn validate(&self) -> Result<(), RunnerError> {
        if self.opt_level > 3 {
            return Err(RunnerError::InvalidOptLevel(self.opt_level));
        }
        Ok(())
    }

    /// Retorna el flag de optimización para clang (ej: "-O2").
    fn opt_level_flag(&self) -> String {
        format!("-O{}", self.opt_level)
    }
}

/// API pública para compilar y ejecutar LLVM IR.
pub struct LlvmRunner;

impl LlvmRunner {
    /// Compila un archivo `.ll` a un ejecutable usando clang.
    ///
    /// # Parámetros
    /// * `ll_path` - Ruta al archivo `.ll` generado por el compilador.
    /// * `out_path` - Ruta de salida del ejecutable (si es None, se deduce del nombre del `.ll`).
    /// * `opts` - Opciones de compilación (clang, nivel de optimización, etc).
    ///
    /// # Retorna
    /// La ruta del ejecutable generado, o un error si clang falla.
    ///
    /// # Ejemplo
    /// ```ignore
    /// let opts = RunnerOptions::default();
    /// let exe = LlvmRunner::compile_ll_to_executable(
    ///     Path::new("program.ll"),
    ///     None,
    ///     &opts
    /// )?;
    /// println!("Ejecutable creado en: {}", exe.display());
    /// # Ok::<(), RunnerError>(())
    /// ```
    pub fn compile_ll_to_executable(
        ll_path: &Path,
        out_path: Option<&Path>,
        opts: &RunnerOptions,
    ) -> Result<PathBuf, RunnerError> {
        // Validar opciones
        opts.validate()?;

        // Verificar que el archivo .ll existe
        if !ll_path.exists() {
            return Err(RunnerError::InputMissing(ll_path.to_path_buf()));
        }

        // Determinar la ruta de salida
        let exe_path = if let Some(out) = out_path {
            out.to_path_buf()
        } else {
            // Si no se especifica, usar el stem del .ll con extensión de plataforma
            let stem = ll_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let dirname = ll_path
                .parent()
                .unwrap_or_else(|| Path::new("."));
            Platform::as_executable_path(&dirname.join(stem))
        };

        // Construir y ejecutar el comando clang
        let mut cmd = Command::new(&opts.clang_bin);
        cmd.arg(opts.opt_level_flag());
        cmd.arg(ll_path);
        cmd.arg("-o");
        cmd.arg(&exe_path);

        // Agregar argumentos extra
        for arg in &opts.extra_args {
            cmd.arg(arg);
        }

        // Ejecutar capturando stdout y stderr
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let output = cmd.output().map_err(|e| {
            RunnerError::io("executing clang", Some(PathBuf::from(&opts.clang_bin)), e)
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            return Err(RunnerError::ClangFailed {
                status: output.status.code(),
                stderr,
                stdout,
            });
        }

        Ok(exe_path)
    }

    /// Ejecuta un binario y retorna su proceso::Output.
    ///
    /// # Parámetros
    /// * `exe_path` - Ruta al ejecutable.
    /// * `args` - Argumentos a pasar al programa.
    ///
    /// # Retorna
    /// El objeto `std::process::Output` con stdout, stderr y status.
    ///
    /// # Ejemplo
    /// ```ignore
    /// let output = LlvmRunner::run_executable(
    ///     Path::new("./program"),
    ///     &["arg1".to_string()]
    /// )?;
    /// println!("Salida: {}", String::from_utf8_lossy(&output.stdout));
    /// # Ok::<(), RunnerError>(())
    /// ```
    pub fn run_executable(
        exe_path: &Path,
        args: &[String],
    ) -> Result<std::process::Output, RunnerError> {
        if !exe_path.exists() {
            return Err(RunnerError::InputMissing(exe_path.to_path_buf()));
        }

        let mut cmd = Command::new(exe_path);
        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        cmd.output().map_err(|e| {
            RunnerError::io("executing program", Some(exe_path.to_path_buf()), e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_options_validate() {
        let mut opts = RunnerOptions::default();
        assert!(opts.validate().is_ok());

        opts.opt_level = 4;
        assert!(opts.validate().is_err());
    }

    #[test]
    fn test_opt_flag() {
        for level in 0..=3 {
            let opts = RunnerOptions {
                opt_level: level,
                ..Default::default()
            };
            assert_eq!(opts.opt_level_flag(), format!("-O{}", level));
        }
    }
}
