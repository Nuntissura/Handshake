use std::{env, fs, path::PathBuf, process::ExitCode};

use handshake_core::test_harness::run_swarm_scenario;

#[tokio::main]
async fn main() -> ExitCode {
    match run_cli(env::args().skip(1)).await {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn run_cli<I>(args: I) -> Result<String, String>
where
    I: IntoIterator<Item = String>,
{
    let config = CliConfig::parse(args)?;
    if config.help {
        return Ok(help_text());
    }

    let scenario_id = config
        .scenario_id
        .ok_or_else(|| "missing required --scenario <scenario_id>".to_string())?;
    let n = config
        .n
        .ok_or_else(|| "missing required --n <session_count>".to_string())?;
    let report = run_swarm_scenario(&scenario_id, n)
        .await
        .map_err(|error| error.to_string())?;
    let encoded = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;

    if let Some(report_path) = config.report_path {
        if let Some(parent) = report_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(&report_path, encoded.as_bytes()).map_err(|error| error.to_string())?;
        let report_path = report_path.to_string_lossy().to_string();
        return Ok(serde_json::json!({
            "scenario_id": scenario_id,
            "n": n,
            "report_path": report_path,
        })
        .to_string());
    }

    Ok(encoded)
}

#[derive(Debug, Default)]
struct CliConfig {
    scenario_id: Option<String>,
    n: Option<usize>,
    report_path: Option<PathBuf>,
    help: bool,
}

impl CliConfig {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut config = Self::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => config.help = true,
                "--scenario" => {
                    config.scenario_id = Some(next_arg(&mut args, "--scenario")?);
                }
                "--n" => {
                    let value = next_arg(&mut args, "--n")?;
                    config.n =
                        Some(value.parse::<usize>().map_err(|_| {
                            format!("--n must be a positive integer, got {value:?}")
                        })?);
                }
                "--report" => {
                    config.report_path = Some(PathBuf::from(next_arg(&mut args, "--report")?));
                }
                unknown => {
                    return Err(format!("unknown argument {unknown:?}\n\n{}", help_text()));
                }
            }
        }

        Ok(config)
    }
}

fn next_arg<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn help_text() -> String {
    [
        "handshake-swarm",
        "",
        "Usage:",
        "  handshake-swarm --scenario <n8-perf|session-cancel|lease-contention> --n <count> [--report <path>]",
        "",
        "Runs the KERNEL-004 local swarm test harness and writes a SwarmReport JSON payload.",
    ]
    .join("\n")
}
