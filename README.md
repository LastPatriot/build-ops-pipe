# build-ops-pipe

A Bitbucket Pipe for the team's standard flow: **build once on `test`, promote the same image to prod,
and update `gitops-central-repo`** so Argo CD deploys it.

Image: `provdev/platform:build-ops-pipe-<n>` (public on Docker Hub, so pipes need no `docker login`)

Current release: **`1.0.2`** → `provdev/platform:build-ops-pipe-11`

App repos don't call this pipe directly: they import the shared pipelines from
[pipeline-templates](https://bitbucket.org/nndukwe/pipeline-templates), which pins this pipe's release tag.

## Usage

```yaml
- step:
    name: Build and Push Docker Image (test)
    deployment: Staging
    services:
      - docker
    script:
      - pipe: nndukwe/build-ops-pipe:1.0.2
        variables:
          ACTION: build-and-push
          APP_NAME: customer-360-api
          DOCKERHUB_USERNAME: $DOCKERHUB_USERNAME
          DOCKER_PASSWORD: $DOCKER_PASSWORD
```

`pipe: nndukwe/build-ops-pipe:<git tag>` makes Bitbucket read the image from `pipe.yml` at that tag.
`pipe: docker://provdev/platform:build-ops-pipe-<n>` uses an image directly, e.g. to try a build before releasing it.

## Actions and variables

Set `ACTION` to one of these. Each action only needs its own secrets.

| `ACTION` | Required | Optional | What it does |
|---|---|---|---|
| `build-and-push` | `APP_NAME`, `DOCKERHUB_USERNAME`, `DOCKER_PASSWORD` | `IMAGE_REPO` | `docker build --build-arg environment=production`, push `<app>-<build#>` and `<app>-tree-<hash>` |
| `check-image` | `APP_NAME`, `DOCKERHUB_USERNAME`, `DOCKERHUB_READ_TOKEN` | `IMAGE_REPO` | Fails if no image was built from this exact source tree (PR check before prod) |
| `promote` | `APP_NAME`, `DOCKERHUB_USERNAME`, `DOCKER_PASSWORD` | `IMAGE_REPO` | Pulls `<app>-tree-<hash>`, retags it as `<app>-<build#>`, pushes it. No rebuild |
| `update-manifest` | `APP_NAME`, `DOCKERHUB_USERNAME`, `GITOPS_ACCESS_TOKEN` | `ENV`, `GITOPS_BOT_EMAIL` | Sets `tag:` in `environments/<env>/<app>/values.yaml` and pushes to `main` |

| Variable | Default | Notes |
|---|---|---|
| `APP_NAME` | none | Kebab-case. Must match the gitops folder name. |
| `DOCKERHUB_USERNAME` | none | Docker Hub namespace, normally `provdev`. |
| `DOCKER_PASSWORD` | none | Secured. Push access. Comes from the deployment (Staging / Production). |
| `DOCKERHUB_READ_TOKEN` | none | Secured repository variable. Read-only, so PR checks don't need push credentials. |
| `GITOPS_ACCESS_TOKEN` | none | Secured. Bitbucket access token with *Repositories: Write* on `gitops-central-repo` (a repository token on that repo is the narrowest choice). Sent as username `x-token-auth` through a credential helper, never in the URL. |
| `GITOPS_BOT_EMAIL` | `pvbpipelines@bitbucket.org` | Commit email: the token's bot email (`…@bots.bitbucket.org`), so GitOps commits show as the bot. Changes when the token is replaced, so update it on every rotation. |
| `IMAGE_REPO` | `appdev` | |
| `ENV` | `test` | `test` → `environments/staging`, `production` → `environments/prod`, anything else used as-is. |

`BITBUCKET_BUILD_NUMBER` is read automatically. Empty values count as missing.

## Requirements

- **Run from the app's repo clone.** Every action reads `git rev-parse HEAD^{tree}` and stops if it can't.
- **The Docker actions need `services: - docker`** on the step.
- **The gitops folder must exist** (`environments/<env>/<app>/`) before the first `update-manifest`.
  Liaise with the Platform Engineering team to create it.

## Behaviour worth knowing

- **Promotion matches by tree hash.** Prod can only promote code whose tree was built on `test`.
  A prod-only commit has no tested image, so use the app repo's `hotfix-build-production` custom pipeline.
- **Reruns are safe.** If the manifest already has the tag, `update-manifest` prints `nothing to do` and exits 0.
- **Concurrent updates are handled.** `update-manifest` runs `git pull --rebase` before pushing, so another app's
  pipeline pushing at the same time doesn't fail the step.
- **All failures exit 1** with a message naming the missing variable, file or failed command.

## Development

```bash
cargo fmt --all --check && cargo clippy --all-targets
docker build -t build-ops-pipe:local .
docker run --rm -e ACTION=check-image -e APP_NAME=demo build-ops-pipe:local   # prints what's missing
```

The image is Alpine-based, runs as `appuseradm` (UID 1000) and contains `git` but not the Docker CLI.
Bitbucket mounts its own CLI into pipe containers, so the calling step needs `services: docker`.
Running a docker-using action locally needs a Linux Docker CLI and the socket mounted in.

## Releasing

1. Merge into `main`. The pipeline lints and pushes `provdev/platform:build-ops-pipe-<build#>`.
2. Set `image:` in `pipe.yml` to that tag, commit, and tag the commit with the next version (e.g. `1.0.3`).
   Update "Current release" at the top of this README in the same commit.
   This push doesn't build: lint and build only run when `src/**`, `Cargo.*`, `Dockerfile`,
   `.dockerignore` or `bitbucket-pipelines.yml` change.
3. In pipeline-templates, set `pipe-version` in `deploy.yml` to the new tag and release that repo
   (see its README). Apps pick it up when their import moves to the new pipeline-templates tag.
