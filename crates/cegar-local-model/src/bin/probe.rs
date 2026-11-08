//! Local-only measurement binary: one bounded model path plus the deterministic
//! baseline, printed as a single JSON record.
//!
//! ```sh
//! cargo run -p cegar-local-model --locked --offline --bin probe -- \
//!   --header "$HOME/.config/local-inference/authorization.header" --runs 7
//! ```
//!
//! The bearer header file is read with a bounded read and never printed. The
//! endpoint is loopback only; no remote host can be targeted.

use std::path::PathBuf;

use cegar_local_model::{Endpoint, evaluate, probe};

#[derive(Debug)]
struct Args {
    header: PathBuf,
    port: u16,
    runs: usize,
    observation: String,
}

fn usage() -> &'static str {
    "usage: probe --header <path> [--port 1235] [--runs 7] [--observation <text>]"
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args().skip(1);
    let mut header: Option<PathBuf> = None;
    let mut port = 1235u16;
    let mut runs = 7usize;
    let mut observation =
        String::from("stage a patch, run the registered test suite, then publish the artifact");
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--header" => {
                header = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--header needs a path".to_owned())?,
                ));
            }
            "--port" => {
                port = args
                    .next()
                    .and_then(|text| text.parse().ok())
                    .ok_or_else(|| "--port needs a u16".to_owned())?;
            }
            "--runs" => {
                runs = args
                    .next()
                    .and_then(|text| text.parse().ok())
                    .ok_or_else(|| "--runs needs a usize".to_owned())?;
            }
            "--observation" => {
                observation = args
                    .next()
                    .ok_or_else(|| "--observation needs text".to_owned())?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Args {
        header: header.ok_or_else(|| usage().to_owned())?,
        port,
        runs,
        observation,
    })
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("probe: {error}");
            std::process::exit(2);
        }
    };

    let endpoint = match Endpoint::from_header_file(args.port, &args.header) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            eprintln!("probe: endpoint: {error}");
            std::process::exit(1);
        }
    };

    match probe(&endpoint, &args.observation, args.runs) {
        Ok(report) => {
            println!("{}", report.to_json());
            // The comparison path is printed separately so a reader can see the
            // deterministic baseline without parsing the measurement record.
            let evaluation = evaluate(Some(&endpoint), &args.observation);
            println!(
                "{{\"baseline\":\"{}\",\"baseline_verdict\":\"{}\",\"model_route\":\"{}\",\"model_verdict\":\"{}\",\"agree\":{},\"model_error\":{}}}",
                evaluation.baseline.tag(),
                evaluation.baseline_verdict,
                evaluation
                    .model
                    .as_ref()
                    .map(|claim| claim.value.tag())
                    .unwrap_or("unavailable"),
                evaluation.model_verdict.unwrap_or("unavailable"),
                evaluation.agree,
                evaluation
                    .model_error
                    .as_ref()
                    .map(|text| format!("\"{text}\""))
                    .unwrap_or_else(|| "null".to_owned()),
            );
        }
        Err(error) => {
            eprintln!("probe: {error}");
            std::process::exit(1);
        }
    }
}
