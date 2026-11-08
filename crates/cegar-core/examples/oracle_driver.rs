//! Bounded test transport, not a public service or a JSON implementation.
//! One line: n initial_bitset bad_bitset edge_count [source action target]...
//! Emits one JSON result per line for scripts/differential.py. No execution.
use cegar_core::{Limits, MAX_STATES, MAX_TRANSITIONS, Model, Outcome, Transition, verify};
use std::io::{self, BufRead, Read};

const MAX_LINE: u64 = 32_768;

fn parse(line: &str) -> Result<Model, &'static str> {
    let mut fields = line.split_whitespace();
    fn number<T: std::str::FromStr>(f: &mut std::str::SplitWhitespace<'_>) -> Result<T, &'static str> {
        f.next().ok_or("missing field")?.parse().map_err(|_| "invalid number")
    }
    let n: usize = number(&mut fields)?;
    if n == 0 || n > MAX_STATES { return Err("state count"); }
    let initial_bits: u128 = number(&mut fields)?;
    let bad_bits: u128 = number(&mut fields)?;
    if n < 128 && ((initial_bits >> n) != 0 || (bad_bits >> n) != 0) { return Err("unknown state"); }
    let count: usize = number(&mut fields)?;
    if count > MAX_TRANSITIONS { return Err("transition count"); }
    let mut edges = Vec::with_capacity(count);
    for _ in 0..count {
        edges.push(Transition::new(number(&mut fields)?, number(&mut fields)?, number(&mut fields)?));
    }
    if fields.next().is_some() { return Err("extra field"); }
    let initial: Vec<_> = (0..n).filter(|s| initial_bits & (1u128 << s) != 0).collect();
    let bad: Vec<_> = (0..n).filter(|s| bad_bits & (1u128 << s) != 0).collect();
    Model::new(n, &initial, &bad, &edges).map_err(|_| "invalid model")
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut line = Vec::new();
        let count = (&mut input).take(MAX_LINE + 1).read_until(b'\n', &mut line)?;
        if count == 0 { break; }
        if count as u64 > MAX_LINE {
            println!("{{\"status\":\"invalid\",\"reason\":\"line_limit\"}}");
            break;
        }
        let model = std::str::from_utf8(&line).map_err(|_| "utf8").and_then(parse);
        let Ok(model) = model else {
            println!("{{\"status\":\"invalid\"}}");
            continue;
        };
        let r = verify(&model, Limits::default());
        match r.outcome {
            Outcome::VerifiedWithinModel => println!("{{\"status\":\"safe\"}}"),
            Outcome::ConcreteCounterexample(w) => println!(
                "{{\"status\":\"unsafe\",\"states\":{:?},\"actions\":{:?}}", w.states, w.actions
            ),
            Outcome::Inconclusive(reason) => println!("{{\"status\":\"inconclusive\",\"reason\":\"{reason:?}\"}}"),
        }
    }
    Ok(())
}
