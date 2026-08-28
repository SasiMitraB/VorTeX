use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub success: bool,
    pub message: String,
    pub pdf_path: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

pub fn get_latex_path_env() -> String {
    let current_path = std::env::var("PATH").unwrap_or_default();
    let extra_paths = [
        "/Library/TeX/texbin",
        "/usr/local/texlive/2026/bin/universal-darwin",
        "/usr/local/texlive/2025/bin/universal-darwin",
        "/usr/local/texlive/2024/bin/universal-darwin",
        "/usr/local/texlive/2023/bin/universal-darwin",
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
    ];

    let mut paths: Vec<&str> = extra_paths.iter().copied().collect();
    let current_entries: Vec<&str> = current_path.split(':').collect();
    for entry in current_entries {
        if !paths.contains(&entry) && !entry.is_empty() {
            paths.push(entry);
        }
    }
    paths.join(":")
}

pub fn build_latex_project(tex_file_path: &str) -> BuildResult {
    let path = Path::new(tex_file_path);
    if !path.exists() || !path.is_file() {
        return BuildResult {
            success: false,
            message: format!("Error: file '{}' not found", tex_file_path),
            pdf_path: None,
            stdout: String::new(),
            stderr: String::new(),
        };
    }

    let dir = match path.parent() {
        Some(d) if d.as_os_str().is_empty() => Path::new("."),
        Some(d) => d,
        None => Path::new("."),
    };

    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => {
            return BuildResult {
                success: false,
                message: "Error: invalid file name".to_string(),
                pdf_path: None,
                stdout: String::new(),
                stderr: String::new(),
            };
        }
    };

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let pdf_file_name = format!("{}.pdf", stem);
    let expected_pdf_path = dir.join(&pdf_file_name);

    let path_env = get_latex_path_env();

    // Step 1: Cleaning old build: latexmk -C "$file"
    let _ = Command::new("latexmk")
        .env("PATH", &path_env)
        .current_dir(dir)
        .arg("-C")
        .arg(file_name)
        .output();

    // Step 2: Building from scratch:
    // latexmk -pdf -interaction=nonstopmode -file-line-error "$file"
    let build_output = Command::new("latexmk")
        .env("PATH", &path_env)
        .current_dir(dir)
        .args(["-pdf", "-interaction=nonstopmode", "-file-line-error"])
        .arg(file_name)
        .output();

    let (success, stdout, stderr) = match build_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            (output.status.success(), stdout, stderr)
        }
        Err(e) => {
            return BuildResult {
                success: false,
                message: format!("Error executing latexmk: {}", e),
                pdf_path: None,
                stdout: String::new(),
                stderr: e.to_string(),
            };
        }
    };

    if success {
        // Step 3: Cleaning temporary files: latexmk -c "$file"
        let _ = Command::new("latexmk")
            .env("PATH", &path_env)
            .current_dir(dir)
            .arg("-c")
            .arg(file_name)
            .output();

        let pdf_str = expected_pdf_path.to_string_lossy().to_string();

        BuildResult {
            success: true,
            message: format!("✓ Build complete: {}", pdf_file_name),
            pdf_path: Some(pdf_str),
            stdout,
            stderr,
        }
    } else {
        BuildResult {
            success: false,
            message: "✗ Build failed".to_string(),
            pdf_path: None,
            stdout,
            stderr,
        }
    }
}
