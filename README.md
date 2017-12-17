# squad-broadcasts
Auto Broadcast for Squad game servers

[![Build Status](https://travis-ci.org/bbigras/squad-broadcasts.svg?branch=master)](https://travis-ci.org/bbigras/squad-broadcasts)
[![Coverage Status](https://coveralls.io/repos/github/bbigras/squad-broadcasts/badge.svg?branch=master)](https://coveralls.io/github/bbigras/squad-broadcasts?branch=master)
[![Matrix chat room](https://d3vu3aucnul4od.cloudfront.net/matrix-badge.svg)](https://matrix.to/#/#squad-broadcasts:matrix.org)

## Build

Set up a Rust environment with [rustup](https://www.rustup.rs) and run:

```sh
cargo build --release
```

The binary will be in the `target\release` folder.

**NOTE:** without `--release` the code will be really slow.

## Run

Run the application in the same folder as Squad.log with `broadcasts-sample.toml` (renamed to `broadcasts.toml`), `Broadcasts.cfg` and `Maps.cfg`.
