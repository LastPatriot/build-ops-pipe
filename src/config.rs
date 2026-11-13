use std::{env, process::Command};

const IMAGE_REPO: &str = "IMAGE_REPO";
const BITBUCKET_BUILD_NUMBER: &str = "BITBUCKET_BUILD_NUMBER";
const DOCKER_USERNAME: &str = "DOCKERHUB_USERNAME";

pub fn require(name: &str) -> String {
    match env::var(name) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("{} must be set for this action", name);
            std::process::exit(1);
        }
    }
}

pub struct Config {
    pub docker_username: String,
    pub image_repo: String,
    pub app_name: String,
    pub build_number: String,
    pub image_tag: String,
    pub full_image_uri: String,
    pub tree_tag: String,
    pub tree_image_uri: String,
}
impl Config {
    pub fn from_env(app_name: &str) -> Self {
        let docker_username = require(DOCKER_USERNAME);
        let image_repo = env::var(IMAGE_REPO).unwrap_or_else(|_| "appdev".to_string());
        let build_number =
            env::var(BITBUCKET_BUILD_NUMBER).unwrap_or_else(|_| "latest".to_string());
        let image_tag = format!("{}-{}", app_name, build_number);
        let full_image_uri = format!("{}/{}:{}", docker_username, image_repo, image_tag);

        let tree_hash = Command::new("git")
            .args(["rev-parse", "HEAD^{tree}"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().chars().take(12).collect::<String>())
            .filter(|h| h.len() ==12)
            .unwrap_or_else(|| {
                eprintln!("Cannot read the git tree hash (git rev-parse HEAD^{{tree}}); run from inside the app's repo");
                std::process::exit(1);
            });
        let tree_tag = format!("{}-tree-{}", app_name, tree_hash);
        let tree_image_uri = format!("{}/{}:{}", docker_username, image_repo, tree_tag);

        Self {
            docker_username,
            image_repo,
            app_name: app_name.to_string(),
            build_number,
            image_tag,
            full_image_uri,
            tree_tag,
            tree_image_uri,
        }
    }
}
