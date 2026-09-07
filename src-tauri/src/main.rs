#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--mcp") {
        ananda_control_lib::agent::stdio();
        return;
    }
    if args.get(1).is_some_and(|a| a == "--call") {
        let result = ananda_control_lib::agent::client_call(
            args.get(2).map(String::as_str).unwrap_or(""),
            serde_json::from_str(args.get(3).map(String::as_str).unwrap_or("{}"))
                .unwrap_or(serde_json::Value::Null),
        );
        match result {
            Ok(value) => println!("{value}"),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.get(1).is_some_and(|a| a == "--grant-config-access") {
        let result = ananda_control_lib::platform::access_helper(
            args.get(2).map(String::as_str).unwrap_or(""),
        );
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    ananda_control_lib::run();
}
