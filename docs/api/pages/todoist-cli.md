# Todoist CLI

Use Todoist from your terminal with the official, open-source
[Todoist CLI](https://www.todoist.com/cli). It is useful for quick task capture,
terminal workflows, local scripts, and giving AI coding agents access to the
same task list you use in Todoist.

### Installation

Install the npm package:

```shell
npm install -g @todoist/cli
```

Then authenticate with Todoist:

```shell
td auth login
```

This opens your browser to authenticate with Todoist. Once approved, the token
is stored in your operating system credential manager.

### Common commands

```shell
td add "Buy milk tomorrow #Shopping"
td today
td inbox
td task list
td task list --project "Work"
td project list
```

Run `td --help` or `td <command> --help` for the full command reference.

### Agent skills

The CLI can install maintained skills for coding agents:

```shell
td skill install claude-code
td skill install codex
td skill install copilot
td skill install cursor
td skill install gemini
td skill install pi
td skill install universal
```

Use `td skill list` to see installed skills and `td skill uninstall <agent>` to
remove one.

### Shell completions

Tab completion is available for bash, zsh, and fish:

```shell
td completion install
td completion install zsh
```

For source code, issues, releases, and contribution details, visit the
[todoist/todoist-cli repository](https://github.com/todoist/todoist-cli).
