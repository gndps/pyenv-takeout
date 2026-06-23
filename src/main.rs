use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

fn pyenvs_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join("pyenvs")
}

/// Compute the venv name for a given absolute directory path.
/// Format: <basename>-<first 8 chars of sha1 of absolute path>
fn venv_name_for_path(dir: &Path) -> String {
    let abs = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let path_str = abs.to_string_lossy().to_string();

    let basename = abs
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unnamed".to_string());

    let hash = sha1_via_shasum(&path_str);
    format!("{}-{}", basename, hash)
}

/// Get first 8 chars of SHA1 of a string using shasum subprocess.
fn sha1_via_shasum(input: &str) -> String {
    // Escape single quotes in the input for safe shell embedding
    let escaped = input.replace('\'', "'\\''");
    let script = format!("printf '%s' '{}' | shasum | cut -c1-8", escaped);

    let output = Command::new("sh")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("failed to run shasum");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn venv_path(name: &str) -> PathBuf {
    pyenvs_dir().join(name)
}

/// Resolve a venv name: if name given use it, otherwise compute from cwd.
fn resolve_name(name_arg: Option<&str>) -> String {
    match name_arg {
        Some(n) => n.to_string(),
        None => {
            let cwd = env::current_dir().expect("failed to get current directory");
            venv_name_for_path(&cwd)
        }
    }
}

fn cmd_create(name_arg: Option<&str>) {
    let name = resolve_name(name_arg);
    let dir = pyenvs_dir();
    fs::create_dir_all(&dir).expect("failed to create ~/pyenvs/");
    let path = venv_path(&name);

    if path.exists() {
        eprintln!("venv '{}' already exists at {}", name, path.display());
        exit(1);
    }

    // Try uv venv first, fall back to python3 -m venv
    let path_str = path.to_string_lossy().to_string();
    let uv_status = Command::new("uv")
        .arg("venv")
        .arg(&path_str)
        .status();

    let success = match uv_status {
        Ok(s) if s.success() => true,
        _ => {
            eprintln!("uv not found or failed, falling back to python3 -m venv");
            Command::new("python3")
                .arg("-m")
                .arg("venv")
                .arg(&path_str)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
    };

    if success {
        println!("{}", path_str);
    } else {
        eprintln!("failed to create venv at {}", path_str);
        exit(1);
    }
}

fn cmd_delete(name_arg: Option<&str>) {
    let name = resolve_name(name_arg);
    let path = venv_path(&name);

    if !path.exists() {
        eprintln!("venv '{}' not found at {}", name, path.display());
        exit(1);
    }

    fs::remove_dir_all(&path).expect("failed to delete venv directory");
    eprintln!("deleted {}", path.display());
}

fn cmd_list() {
    let dir = pyenvs_dir();
    if !dir.exists() {
        println!("No venvs found (~/pyenvs/ does not exist)");
        return;
    }

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("failed to read ~/pyenvs/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("No venvs found in {}", dir.display());
        return;
    }

    for entry in entries {
        let name = entry.file_name();
        let path = entry.path();
        println!("{}\t{}", name.to_string_lossy(), path.display());
    }
}

fn cmd_activate(name_arg: Option<&str>) {
    let name = resolve_name(name_arg);
    let path = venv_path(&name);

    if !path.exists() {
        eprintln!("venv '{}' not found at {}", name, path.display());
        exit(1);
    }

    let activate = path.join("bin").join("activate");
    println!("source {}", activate.display());
}

fn cmd_path(dir_arg: Option<&str>) {
    let dir = match dir_arg {
        Some(d) => PathBuf::from(d),
        None => env::current_dir().expect("failed to get current directory"),
    };

    let name = venv_name_for_path(&dir);
    let path = venv_path(&name);
    println!("{}", path.display());
}

fn cmd_install(name_arg: Option<&str>) {
    let name = resolve_name(name_arg);
    let path = venv_path(&name);

    // Create venv if it doesn't exist
    if !path.exists() {
        cmd_create(name_arg);
    }

    let reqs = env::current_dir()
        .expect("failed to get current directory")
        .join("requirements.txt");

    if !reqs.exists() {
        eprintln!("No requirements.txt found in current directory");
        exit(1);
    }

    let path_str = path.to_string_lossy().to_string();
    let reqs_str = reqs.to_string_lossy().to_string();

    // Try uv pip install first, fall back to pip install
    let uv_status = Command::new("uv")
        .arg("pip")
        .arg("install")
        .arg("-r")
        .arg(&reqs_str)
        .arg("--python")
        .arg(format!("{}/bin/python", path_str))
        .status();

    let success = match uv_status {
        Ok(s) if s.success() => true,
        _ => {
            eprintln!("uv not found or failed, falling back to pip");
            let pip = format!("{}/bin/pip", path_str);
            Command::new(&pip)
                .arg("install")
                .arg("-r")
                .arg(&reqs_str)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
    };

    if !success {
        eprintln!("failed to install requirements");
        exit(1);
    }
}

fn cmd_init_shell() {
    println!(
        r#"# pyenv-takeout shell integration
es() {{ eval "$(pyenv-takeout activate $@)"; }}
ed() {{ local name="${{1:-$(pyenv-takeout path 2>/dev/null | xargs basename 2>/dev/null)}}"; deactivate 2>/dev/null; pyenv-takeout delete "$name"; }}
els() {{ pyenv-takeout list "$@"; }}"#
    );
}

fn cmd_help() {
    println!(
        r#"pyenv-takeout — manage Python venvs in ~/pyenvs/ with deterministic names

USAGE:
    pyenv-takeout <COMMAND> [args]

COMMANDS:
    create [name]    Create a venv. Name defaults to <basename>-<sha1> of cwd.
    delete [name]    Delete a venv. Name defaults to current directory's venv.
    list             List all venvs in ~/pyenvs/.
    activate [name]  Print activation command. Use: eval "$(pyenv-takeout activate)"
    path [dir]       Print the venv path for a directory (defaults to cwd).
    install [name]   Create venv if needed, then pip install -r requirements.txt.
    init-shell       Print shell integration functions (es, ed, els).
    help             Show this help message.

VENV NAMING:
    Names are: <basename>-<first 8 chars of sha1 of absolute path>
    Example: myproject-a1b2c3d4

SHELL INTEGRATION:
    Add to your shell profile:
        eval "$(pyenv-takeout init-shell)"

    Then use:
        es          — activate venv for current directory
        es <name>   — activate named venv
        ed          — deactivate and delete venv for current directory
        els         — list all venvs
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        cmd_help();
        exit(0);
    }

    match args[1].as_str() {
        "create" => cmd_create(args.get(2).map(String::as_str)),
        "delete" => cmd_delete(args.get(2).map(String::as_str)),
        "list" => cmd_list(),
        "activate" => cmd_activate(args.get(2).map(String::as_str)),
        "path" => cmd_path(args.get(2).map(String::as_str)),
        "install" => cmd_install(args.get(2).map(String::as_str)),
        "init-shell" => cmd_init_shell(),
        "help" | "--help" | "-h" => cmd_help(),
        "--version" | "-V" => println!("pyenv-takeout {}", env!("CARGO_PKG_VERSION")),
        unknown => {
            eprintln!("Unknown command: {}", unknown);
            eprintln!("Run 'pyenv-takeout help' for usage.");
            exit(1);
        }
    }
}
