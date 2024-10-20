# Referral List

If you don't know what this is, it isn't for you.

## Building

Install Rust and cargo via [rustup](https://rustup.rs)

```bash
cargo build --release
```

## Running

The program will set itself up. Either run the binary you built above or run
via cargo.

```bash
cargo run --release
```

## TODO

- [x] Get referrals from church servers
- [x] Parse them into neat data structures
- [x] Get and parse timeline info
- [x] Make a user-friendly TUI
- [ ] Integrate with Holly
- [ ] Generate easy reports
- [ ] Document API calls

## Debugging

You can set the environment variable ``RUST_LOG`` to ``info`` to get more
detailed logs.
Set this either in your .env file or ``export`` it on Linux.

## Why no Python?

The original source code for this project was written in Python.
As the complexity grew and the scope changed, it became unmaintainable due to
Python's poor typing and checking. Code that is collaborated on shouldn't be in Python.
