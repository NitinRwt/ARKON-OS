/// arkon-hwprobe — print a JSON hardware report to stdout.
///
/// Usage: arkon-hwprobe [--pretty]
///
/// Exit codes:
///   0  success (even if warnings are present)
///   1  hard probe failure (e.g. /proc/cpuinfo unreadable)
use arkon_hw::{RealProvider, probe};

fn main() {
    let pretty = std::env::args().any(|a| a == "--pretty");

    let provider = RealProvider::new();
    let report = match probe(&provider) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("arkon-hwprobe: probe failed: {}", e);
            std::process::exit(1);
        }
    };

    let output = if pretty {
        serde_json::to_string_pretty(&report)
    } else {
        serde_json::to_string(&report)
    };

    match output {
        Ok(json) => println!("{}", json),
        Err(e) => {
            eprintln!("arkon-hwprobe: serialisation failed: {}", e);
            std::process::exit(1);
        }
    }
}
