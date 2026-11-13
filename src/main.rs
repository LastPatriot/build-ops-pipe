mod config;
mod docker;
mod git;

use config::Config;
use std::env;

fn main() {
    let action = env::var("ACTION").unwrap_or_else(|_| "default-action".to_string());
    let app_name = config::require("APP_NAME");
    let cfg = Config::from_env(&app_name);
    println!("Running action {} on {}", action, app_name);

    match action.as_str() {
        "build-and-push" => {
            docker::build_and_push(&cfg);
        }
        "check-image" => {
            docker::check_image(&cfg);
        }
        "promote" => {
            docker::promote_image(&cfg);
        }
        "update-manifest" => {
            let env_name = env::var("ENV").unwrap_or_else(|_| "test".to_string());
            git::update_manifest(&cfg, &env_name);
        }
        _ => {
            eprintln!("Unknown action provided: {}", action);
            std::process::exit(1);
        }
    }
}
