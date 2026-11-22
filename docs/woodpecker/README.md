# Woodpecker CI on Codeberg

1. Enable this repository in [Codeberg Woodpecker](https://ci.codeberg.org/repos/add). Use `.woodpecker/` as the workflow directory if the repository already has a custom pipeline path.
2. In Codeberg, create an access token with `write:repository` and `read:misc` access. Restrict it to this repository if your token settings allow it. In Woodpecker, add it under **Repository Settings → Secrets** as `codeberg_release_token`. Enable it for the **Tag** event only. If you restrict the secret by image, allow `woodpeckerci/plugin-release` for the hosted workflow and `plugin-release` when using the native workflows.
3. Push a branch to Codeberg to run the Linux build and tests. Pushing a new tag such as `v0.5` on a commit containing the workflows also builds a stripped Linux x86-64 release and publishes it to Codeberg. The release description comes from the matching `## [v0.5]` section in `CHANGELOG.md`. Release tags use the existing `vMAJOR.MINOR` format.

## Windows and macOS releases

Codeberg's shared Woodpecker agents run Linux x86-64. The Windows MSVC and macOS x86-64 workflows in `.woodpecker/native/` are opt-in. [Codeberg CI documentation](https://docs.codeberg.org/ci/)

1. Register a dedicated x86-64 agent for each platform you want to publish. Use `WOODPECKER_BACKEND=local` and follow Codeberg's agent connection instructions. Run the agent as an unprivileged account on a dedicated machine or VM. Local workflows run directly on the agent host, so use these agents for repositories whose workflow authors you trust. [Codeberg agent setup](https://docs.codeberg.org/ci/agents/) · [Woodpecker local backend](https://woodpecker-ci.org/docs/administration/configuration/backends/local)
2. Make Git, Bash, Python 3.9 or newer, Rust 1.96.1, and the native `plugin-git` and `plugin-release` binaries available on the agent's `PATH`. Use the [Woodpecker release plugin source](https://codeberg.org/woodpecker-plugins/release) to build `plugin-release` for the host if needed. On Windows, use Git for Windows' Bash and make Python available as `python`. Install the Visual Studio C++ build tools and Windows SDK, including `rc.exe`, and start the agent from an x64 developer environment. On macOS, install the Xcode command-line tools and make Python available as `python3`.
3. Move the desired `release-windows.yml` and/or `release-macos.yml` from `.woodpecker/native/` into `.woodpecker/` before cutting a tag. Each enabled workflow adds its archive to the release created by the Linux workflow.
