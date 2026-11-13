use crate::config::{Config, require};
use std::process::Command;

const CRED_HELPER: &str =
    "!f() { echo username=x-token-auth; echo password=$GITOPS_ACCESS_TOKEN; }; f";

fn cleanup(dir: &str) {
    let _ = std::fs::remove_dir_all(dir);
}

fn git(args: &[&str]) -> std::process::ExitStatus {
    Command::new("git")
        .args([
            "-c",
            "credential.helper=",
            "-c",
            &format!("credential.helper={}", CRED_HELPER),
        ])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()
        .expect("failed to execute git")
}
fn run(args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("git {} failed", args.join(" "));
        std::process::exit(1);
    }
}
pub fn update_manifest(cfg: &Config, env: &str) {
    require("GITOPS_ACCESS_TOKEN");
    let target_env = match env {
        "test" => "staging",
        "production" => "prod",
        other => other,
    };

    println!("Cloning GitOps central repo..");
    let clone_dir =
        std::env::temp_dir().join(format!("gitops-central-repo-{}", std::process::id()));
    let clone_dir = clone_dir
        .to_str()
        .expect("temp dir path is not UTF-8")
        .to_string();
    let clone_status = git(&[
        "clone",
        "https://bitbucket.org/nndukwe/gitops-central-repo.git",
        &clone_dir,
    ]);

    if !clone_status.success() {
        eprintln!("git clone failed");
        std::process::exit(1);
    }

    let manifest_path = format!("{}/environments/{}/{}", clone_dir, target_env, cfg.app_name);
    println!(
        "Updating {} in {} to tag {}...",
        cfg.app_name, target_env, cfg.image_tag
    );

    let values_file = format!("{}/values.yaml", manifest_path);
    let content = std::fs::read_to_string(&values_file).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {}", values_file, e);
        eprintln!(
            "Liaise with the Platform Engineering team to create your chart in environments/{}/{}/ and check that APP_NAME matches the folder name", target_env, cfg.app_name
        );
        std::process::exit(1);
    });
    let updated_content = regex::Regex::new(r"tag:.*")
        .unwrap()
        .replace(&content, format!("tag: \"{}\"", cfg.image_tag));

    std::fs::write(&values_file, updated_content.as_bytes()).expect("failed to update values file");
    std::env::set_current_dir(&manifest_path).expect("failed to change directory");

    let bot_email = std::env::var("GITOPS_BOT_EMAIL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "pvbpipelines@bitbucket.org".to_string());

    run(&["config", "user.name", "Providus Bitbucket CI Bot"]);
    run(&["config", "user.email", &bot_email]);
    run(&["add", "values.yaml"]);

    let unchanged = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if unchanged {
        println!(
            "{} already at {} in {}; nothing to do.",
            cfg.app_name, cfg.image_tag, target_env
        );
        cleanup(&clone_dir);
        return;
    }

    let commit_msg = format!(
        "chore({}): update {} tag to {} [skip ci]",
        cfg.app_name, target_env, cfg.image_tag
    );

    run(&["commit", "-m", &commit_msg]);

    let update_pull = git(&["pull", "--rebase", "origin", "main"]);

    if !update_pull.success() {
        eprintln!("git pull failed");
        std::process::exit(1);
    }

    let push_status = git(&["push", "origin", "main"]);

    if !push_status.success() {
        eprintln!("git push failed");
        std::process::exit(1);
    }

    cleanup(&clone_dir);
    println!("Successfully updated GitOps central repo.");
}
