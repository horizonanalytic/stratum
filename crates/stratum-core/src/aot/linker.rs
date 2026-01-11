//! Linker integration for AOT compilation
//!
//! This module handles linking compiled object files into standalone executables.

use std::path::{Path, PathBuf};
use std::process::Command;

use cranelift_object::ObjectProduct;

use super::AotError;

/// Configuration for the linker
#[derive(Debug, Clone)]
pub struct LinkerConfig {
    /// Output path for the executable
    pub output: PathBuf,
    /// Whether to optimize the output
    pub optimize: bool,
    /// Additional linker flags
    pub extra_flags: Vec<String>,
}

impl Default for LinkerConfig {
    fn default() -> Self {
        Self {
            output: PathBuf::from("a.out"),
            optimize: false,
            extra_flags: Vec::new(),
        }
    }
}

/// Linker for creating executables from object files
pub struct Linker {
    config: LinkerConfig,
}

impl Linker {
    /// Create a new linker with the given configuration
    #[must_use]
    pub fn new(config: LinkerConfig) -> Self {
        Self { config }
    }

    /// Create a linker with default configuration
    #[must_use]
    pub fn with_output(output: impl Into<PathBuf>) -> Self {
        Self {
            config: LinkerConfig {
                output: output.into(),
                ..Default::default()
            },
        }
    }

    /// Link an object product into an executable
    pub fn link(&self, product: ObjectProduct) -> Result<PathBuf, AotError> {
        // Write the object file to a temporary file
        let obj_data = product
            .emit()
            .map_err(|e| AotError::LinkError(format!("Failed to emit object file: {}", e)))?;

        let temp_dir = std::env::temp_dir();
        let obj_path = temp_dir.join("stratum_module.o");

        std::fs::write(&obj_path, &obj_data)?;

        // Link using the system linker
        self.link_object_file(&obj_path)?;

        // Clean up temporary file
        let _ = std::fs::remove_file(&obj_path);

        Ok(self.config.output.clone())
    }

    /// Link an object file into an executable
    fn link_object_file(&self, obj_path: &Path) -> Result<(), AotError> {
        // Detect the platform and use appropriate linker
        #[cfg(target_os = "macos")]
        {
            self.link_macos(obj_path)
        }

        #[cfg(target_os = "linux")]
        {
            self.link_linux(obj_path)
        }

        #[cfg(target_os = "windows")]
        {
            self.link_windows(obj_path)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Err(AotError::LinkError(
                "Unsupported platform for linking".to_string(),
            ))
        }
    }

    /// Link on macOS using clang
    #[cfg(target_os = "macos")]
    fn link_macos(&self, obj_path: &Path) -> Result<(), AotError> {
        // Create a minimal C wrapper that calls our entry point
        let wrapper_path = std::env::temp_dir().join("stratum_wrapper.c");
        let wrapper_code = r#"
extern long _stratum_entry(void);

int main(int argc, char** argv) {
    return (int)_stratum_entry();
}
"#;
        std::fs::write(&wrapper_path, wrapper_code)?;

        // Use clang to compile the wrapper and link everything
        let mut cmd = Command::new("clang");

        cmd.arg("-o")
            .arg(&self.config.output)
            .arg(&wrapper_path)
            .arg(obj_path);

        if self.config.optimize {
            cmd.arg("-O2");
        }

        for flag in &self.config.extra_flags {
            cmd.arg(flag);
        }

        // Link with libc
        cmd.arg("-lc");

        let output = cmd
            .output()
            .map_err(|e| AotError::LinkError(format!("Failed to run linker: {}", e)))?;

        // Clean up wrapper
        let _ = std::fs::remove_file(&wrapper_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AotError::LinkError(format!("Linker failed: {}", stderr)));
        }

        Ok(())
    }

    /// Link on Linux using gcc/clang
    #[cfg(target_os = "linux")]
    fn link_linux(&self, obj_path: &Path) -> Result<(), AotError> {
        // Create a minimal C wrapper that calls our entry point
        let wrapper_path = std::env::temp_dir().join("stratum_wrapper.c");
        let wrapper_code = r#"
extern long _stratum_entry(void);

int main(int argc, char** argv) {
    return (int)_stratum_entry();
}
"#;
        std::fs::write(&wrapper_path, wrapper_code)?;

        // Try clang first, then gcc
        let compiler = if Command::new("clang").arg("--version").output().is_ok() {
            "clang"
        } else {
            "gcc"
        };

        let mut cmd = Command::new(compiler);

        cmd.arg("-o")
            .arg(&self.config.output)
            .arg(&wrapper_path)
            .arg(obj_path);

        if self.config.optimize {
            cmd.arg("-O2");
        }

        for flag in &self.config.extra_flags {
            cmd.arg(flag);
        }

        // Link with libc
        cmd.arg("-lc");

        let output = cmd
            .output()
            .map_err(|e| AotError::LinkError(format!("Failed to run linker: {}", e)))?;

        // Clean up wrapper
        let _ = std::fs::remove_file(&wrapper_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AotError::LinkError(format!("Linker failed: {}", stderr)));
        }

        Ok(())
    }

    /// Link on Windows using MSVC or MinGW
    #[cfg(target_os = "windows")]
    fn link_windows(&self, obj_path: &Path) -> Result<(), AotError> {
        // Create a minimal C wrapper that calls our entry point
        let wrapper_path = std::env::temp_dir().join("stratum_wrapper.c");
        let wrapper_code = r#"
extern __int64 _stratum_entry(void);

int main(int argc, char** argv) {
    return (int)_stratum_entry();
}
"#;
        std::fs::write(&wrapper_path, wrapper_code)?;

        // Try to use cl.exe (MSVC) first, then clang, then gcc (MinGW)
        let (compiler, output_flag) = if Command::new("cl").arg("/?").output().is_ok() {
            ("cl", "/Fe:")
        } else if Command::new("clang").arg("--version").output().is_ok() {
            ("clang", "-o")
        } else {
            ("gcc", "-o")
        };

        let mut cmd = Command::new(compiler);

        if compiler == "cl" {
            cmd.arg(format!("{}{}", output_flag, self.config.output.display()))
                .arg(&wrapper_path)
                .arg(obj_path);

            if self.config.optimize {
                cmd.arg("/O2");
            }
        } else {
            cmd.arg(output_flag)
                .arg(&self.config.output)
                .arg(&wrapper_path)
                .arg(obj_path);

            if self.config.optimize {
                cmd.arg("-O2");
            }
        }

        for flag in &self.config.extra_flags {
            cmd.arg(flag);
        }

        let output = cmd
            .output()
            .map_err(|e| AotError::LinkError(format!("Failed to run linker: {}", e)))?;

        // Clean up wrapper
        let _ = std::fs::remove_file(&wrapper_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AotError::LinkError(format!("Linker failed: {}", stderr)));
        }

        Ok(())
    }
}

/// Build a Stratum module into an executable
///
/// This is a convenience function that combines AOT compilation and linking.
#[allow(dead_code)]
pub fn build_executable(
    functions: &[&crate::bytecode::Function],
    output: impl Into<PathBuf>,
) -> Result<PathBuf, AotError> {
    use super::compiler::AotCompiler;

    let mut compiler = AotCompiler::new()?;

    // Compile all functions
    for function in functions {
        compiler.compile_function(function)?;
    }

    // Ensure we have a main function
    if !compiler.has_main() {
        return Err(AotError::NoMainFunction);
    }

    // Generate entry point
    compiler.generate_entry_point()?;

    // Finish compilation and get the object product
    let product = compiler.finish();

    // Link into an executable
    let output_path = output.into();
    let linker = Linker::with_output(&output_path);
    linker.link(product)?;

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linker_config_default() {
        let config = LinkerConfig::default();
        assert_eq!(config.output, PathBuf::from("a.out"));
        assert!(!config.optimize);
        assert!(config.extra_flags.is_empty());
    }

    #[test]
    fn linker_with_output() {
        let linker = Linker::with_output("my_program");
        assert_eq!(linker.config.output, PathBuf::from("my_program"));
    }
}
