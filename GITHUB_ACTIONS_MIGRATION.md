# CI Migration from CircleCI to GitHub Actions

This repository has been migrated from CircleCI to GitHub Actions.

## Changes Made

### Workflows Created

1. **CI Workflow** (`.github/workflows/ci.yml`)
   - Runs on push to `master`/`main` branches and pull requests
   - Runs tests, linting, and formatting checks
   - Uses Ubuntu latest with Rust stable toolchain
   - Caches Cargo dependencies for faster builds

2. **Release Workflow** (`.github/workflows/release.yml`)
   - Triggers on version tags (e.g., `v1.0.0`)
   - Builds for both Linux and macOS
   - Creates Debian packages
   - Creates GitHub releases with artifacts
   - Publishes to Homebrew and Snap stores

### Required Secrets

To use the release workflow, you need to configure these secrets in your GitHub repository:

1. **HOMEBREW_SSH_KEY**: SSH private key for accessing the homebrew-sam repository
2. **SNAPCRAFT_STORE_CREDENTIALS**: Credentials for publishing to Snap store (if using)

### Key Differences from CircleCI

- **Caching**: Uses GitHub Actions cache instead of CircleCI cache
- **Artifacts**: Uses GitHub Actions artifacts system instead of workspace persistence
- **GitHub CLI**: Built-in GitHub token instead of manual GitHub CLI setup
- **Matrix builds**: Could be added for testing multiple Rust versions
- **Parallel jobs**: Linux and macOS builds run in parallel

### Migration Benefits

- **No external CI service dependency**: Everything runs on GitHub
- **Better integration**: Native GitHub releases and artifact handling
- **Cost-effective**: GitHub Actions includes generous free tier
- **Simpler secrets management**: Built-in GitHub secrets

### Original CircleCI Configuration

The original CircleCI configuration has been backed up as `.circleci/config.yml.backup` for reference.

## Setup Instructions

1. **Configure Secrets**: Add the required secrets in GitHub repository settings
2. **Test the CI**: Push a commit to trigger the CI workflow
3. **Test Release**: Create a tag like `v1.0.0` to trigger the release workflow
4. **Remove CircleCI**: Disable the CircleCI project once GitHub Actions is working

## Troubleshooting

- Check the Actions tab in GitHub for workflow runs
- Ensure all required secrets are configured
- Verify that the homebrew repository SSH key has the correct permissions
- Check that package scripts in `.packaging/` are compatible with the new environment
