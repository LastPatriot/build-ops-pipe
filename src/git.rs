use crate::config::{Config, require};
use std::process::Command;

const CRED_HELPER: &str =
    "!f() { echo username=$GITOPS_USERNAME; echo password=$GITOPS_APP_PASSWORD; }; f";

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
    require("GITOPS_USERNAME");
    require("GITOPS_APP_PASSWORD");
    let target_env = match env {
        "test" => "staging",
        "production" => "prod",
        other => other,
    };

    println!("Cloning GitOps central repo..");
    let clone_status = git(&[
        "clone",
        "https://bitbucket.org/nndukwe/gitops-central-repo.git",
    ]);

    if !clone_status.success() {
        eprintln!("git clone failed");
        std::process::exit(1);
    }

    let manifest_path = format!(
        "gitops-central-repo/environments/{}/{}",
        target_env, cfg.app_name
    );
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

    run(&["config", "user.name", "Providus Bitbucket CI Bot"]);
    run(&["config", "user.email", "pvbpipelines@bitbucket.org"]);
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

    println!("Successfully updated GitOps central repo.");
}
