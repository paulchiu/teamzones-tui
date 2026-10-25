# teamzones

Who's available and where it's busy, on one time axis. A Rust TUI.

`teamzones` shows the people you tell by their availability and the regions you operate in by their trading load, on one home-time axis. It finds slots that answer a group's question and copies a line such as `5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, open in AU, closed in UK`.

It is under construction and has no release yet.

## Configure

`teamzones config init` writes a commented template, and `teamzones config check` validates it. To have a coding agent write the config for you, point it at [llms.txt](llms.txt): it lists every table and field and tells the agent how to gather your colleagues' cities, zones and hours from tools such as Slack and your calendar.

## Build

```sh
just ci                          # format, lint, test, docs, core isolation, build
cargo run -p teamzones -- --version
```

## Licence

MIT. See [LICENSE](LICENSE).
