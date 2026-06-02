use std::{env, path::PathBuf};

use handshake_core::sandbox::{
    check_wsl_static_musl_prereqs, package_warm_agent_with_wsl, WarmAgentPackageOptions,
    DEFAULT_WSL_DISTRO,
};

fn main() {
    match run() {
        Ok(()) => {}
        Err(error) if error.starts_with("Usage:") => {
            println!("{error}");
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1).collect())?;
    let repo_root = args.repo_root.unwrap_or_else(default_repo_root);
    let mut options =
        WarmAgentPackageOptions::default_for_repo(repo_root).map_err(|error| error.to_string())?;
    options.distro = args.distro;
    if let Some(output_dir) = args.output_dir {
        options.output_dir = output_dir;
    }
    if let Some(target_dir) = args.target_dir {
        options.cargo_target_dir = target_dir;
    }

    if args.check_only {
        check_wsl_static_musl_prereqs(&options.distro).map_err(|error| error.to_string())?;
        println!("HSK_WARM_AGENT_PACKAGE_PREREQS_OK");
        return Ok(());
    }

    let manifest = package_warm_agent_with_wsl(&options).map_err(|error| error.to_string())?;
    println!("HSK_WARM_AGENT_PACKAGE_OK");
    println!("host_path={}", manifest.host_path);
    println!("guest_path={}", manifest.guest_path);
    println!(
        "set {}={}",
        manifest.required_host_path_env, manifest.host_path
    );
    println!("manifest={}", options.manifest_path().display());
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Args {
    distro: String,
    repo_root: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    check_only: bool,
}

impl Args {
    fn parse(raw: Vec<String>) -> Result<Self, String> {
        let mut args = Self {
            distro: DEFAULT_WSL_DISTRO.to_string(),
            repo_root: None,
            output_dir: None,
            target_dir: None,
            check_only: false,
        };
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "--distro" => {
                    index += 1;
                    args.distro = take_value(&raw, index, "--distro")?;
                }
                "--repo-root" => {
                    index += 1;
                    args.repo_root = Some(PathBuf::from(take_value(&raw, index, "--repo-root")?));
                }
                "--output-dir" => {
                    index += 1;
                    args.output_dir = Some(PathBuf::from(take_value(&raw, index, "--output-dir")?));
                }
                "--target-dir" => {
                    index += 1;
                    args.target_dir = Some(PathBuf::from(take_value(&raw, index, "--target-dir")?));
                }
                "--check-only" => args.check_only = true,
                "--help" | "-h" => return Err(usage()),
                other => return Err(format!("unknown argument `{other}`\n{}", usage())),
            }
            index += 1;
        }
        Ok(args)
    }
}

fn take_value(raw: &[String], index: usize, flag: &str) -> Result<String, String> {
    raw.get(index)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value\n{}", usage()))
}

fn default_repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn usage() -> String {
    [
        "Usage: hsk-warm-agent-package [--distro Ubuntu] [--repo-root PATH] [--output-dir PATH] [--target-dir PATH] [--check-only]",
        "",
        "Builds a static x86_64-unknown-linux-musl hsk-warm-agent in WSL and writes hsk-warm-agent.package.json.",
        "The WSL distro must have cargo, rustup, the musl target, and x86_64-linux-musl-gcc.",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_parse_defaults_and_check_only() {
        let args = Args::parse(vec!["--check-only".to_string()]).expect("parse");
        assert_eq!(args.distro, DEFAULT_WSL_DISTRO);
        assert!(args.check_only);
    }

    #[test]
    fn args_parse_paths() {
        let args = Args::parse(vec![
            "--distro".to_string(),
            "Ubuntu".to_string(),
            "--repo-root".to_string(),
            "D:/repo".to_string(),
            "--output-dir".to_string(),
            "D:/out".to_string(),
            "--target-dir".to_string(),
            "D:/target".to_string(),
        ])
        .expect("parse");
        assert_eq!(args.repo_root, Some(PathBuf::from("D:/repo")));
        assert_eq!(args.output_dir, Some(PathBuf::from("D:/out")));
        assert_eq!(args.target_dir, Some(PathBuf::from("D:/target")));
    }

    #[test]
    fn help_returns_usage() {
        let err = Args::parse(vec!["--help".to_string()]).expect_err("help returns usage");
        assert!(err.starts_with("Usage:"));
    }
}
