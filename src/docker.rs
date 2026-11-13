use crate::config::{Config, require};
use std::io::Write;
use std::process::Command;
use std::process::Stdio;

fn docker_login(password: &str, username: &str) {
    let mut child = Command::new("docker")
        .args(["login", "-u", username, "--password-stdin"])
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to execute docker login");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(password.as_bytes())
        .expect("failed to send password to docker login");
    if !child.wait().expect("docker login did not finish").success() {
        eprintln!("docker login failed");
        std::process::exit(1);
    }
}
pub fn build_and_push(cfg: &Config) {
    println!("Executing build and push for: {}", cfg.app_name);

    // Initiate login to docker
    docker_login(&require("DOCKER_PASSWORD"), cfg.docker_username.as_str());

    //Build the image
    println!("Building image: {}", cfg.full_image_uri);
    let build_status = Command::new("docker")
        .arg("build")
        .arg("--build-arg")
        .arg("environment=production")
        .arg("-t")
        .arg(&cfg.full_image_uri)
        .arg(".")
        .status()
        .expect("failed to execute docker build");

    if !build_status.success() {
        eprintln!("Docker build failed");
        std::process::exit(1);
    }

    //Push the image to docker
    let push_status = Command::new("docker")
        .arg("push")
        .arg(&cfg.full_image_uri)
        .status()
        .expect("Failed to execute docker push");

    if !push_status.success() {
        eprintln!("Docker push failed");
        std::process::exit(1);
    }

    let tag_status = Command::new("docker")
        .arg("tag")
        .arg(&cfg.full_image_uri)
        .arg(&cfg.tree_image_uri)
        .status()
        .expect("failed to tag tree image");

    if !tag_status.success() {
        eprintln!("Docker tag for tree image failed");
        std::process::exit(1);
    }

    let tree_push_status = Command::new("docker")
        .arg("push")
        .arg(&cfg.tree_image_uri)
        .status()
        .expect("Failed to push tree image");

    if !tree_push_status.success() {
        eprintln!("Docker push for tree image failed");
        std::process::exit(1);
    }

    println!(
        "Successfully built and pushed both {} and {}",
        cfg.full_image_uri, cfg.tree_image_uri
    );
}
pub fn check_image(cfg: &Config) {
    println!("Checking if tested image exists for: {}", cfg.app_name);

    docker_login(
        &require("DOCKERHUB_READ_TOKEN"),
        cfg.docker_username.as_str(),
    );

    let status = Command::new("docker")
        .arg("manifest")
        .arg("inspect")
        .arg(&cfg.tree_image_uri)
        .status()
        .expect("failed to execute docker manifest inspect");

    if !status.success() {
        eprintln!("No tested image for this exact code ({})", cfg.tree_tag);
        eprintln!(
            "Merge it into test first and let the test pipeline finish, or use the manual 'hotfix-build-production' pipeline."
        );
        std::process::exit(1);
    }

    println!("Tested image exists for: {}", cfg.tree_image_uri);
}
pub fn promote_image(cfg: &Config) {
    println!("Promoting tested image for: {}", cfg.app_name);

    docker_login(&require("DOCKER_PASSWORD"), cfg.docker_username.as_str());

    let pull_status = Command::new("docker")
        .arg("pull")
        .arg(&cfg.tree_image_uri)
        .status()
        .expect("Failed to pull tree image");

    if !pull_status.success() {
        eprintln!("Docker pull for tree image failed");
        std::process::exit(1);
    }

    let tag_status = Command::new("docker")
        .arg("tag")
        .arg(&cfg.tree_image_uri)
        .arg(&cfg.full_image_uri)
        .status()
        .expect("Failed to tag image");

    if !tag_status.success() {
        eprintln!("Docker tag failed");
        std::process::exit(1);
    }

    let push_status = Command::new("docker")
        .arg("push")
        .arg(&cfg.full_image_uri)
        .status()
        .expect("Failed to push promoted image");

    if !push_status.success() {
        eprintln!("Docker push failed");
        std::process::exit(1);
    }

    println!(
        "Successfully promoted tested image as: {}",
        cfg.full_image_uri
    );
}
