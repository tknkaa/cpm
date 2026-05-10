# cpm

A TUI tool that compares your remaining GitHub Copilot Premium requests against the days left in the billing cycle.

> [!WARNING]
> This tool uses GitHub Copilot's internal API, which is unofficial and not intended for external use.
> It may break without notice if GitHub changes their API.
> Use at your own risk.

![demo](demo.png)

## Installation

### cargo

```bash
go install github.com/tknkaa/cpm@latest
```

## Usage

```bash
# Fetch quota automatically via Copilot Internal API
cpm
```

## How it works

`gh api` is used to call the GitHub API and fetch your current quota snapshot, using gh's built-in authentication. If the API call fails, cpm falls back to a TUI prompt where you can enter the remaining percentage manually (visible in your GitHub billing settings).

## Requirements

- [GitHub CLI](https://cli.github.com/) (`gh`) — for authentication
- A GitHub account with Copilot enabled

## License

MIT
