# teamzones

Who's available and where it's busy, on one time axis. A Rust TUI.

`teamzones` shows the people you tell by their availability and the regions you operate in by their trading load, on one home-time axis. It finds slots that answer a group's question and copies a line such as `5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, off-peak in AU and UK`.

It is under construction and has no release yet.

## Build

```sh
just ci                          # format, lint, test, docs, core isolation, build
cargo run -p teamzones -- --version
```

## Licence

MIT. See [LICENSE](LICENSE).
