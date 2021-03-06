# Jaw: the remote shell

Jaw is a remote shell that uses UDP as a transport protocol.

It is currently just a proof-of-concept, and a result of my play with UDP.

# Flow

![flow image](./docs/flow.svg)

# Features

- [x] Running remote shell over UDP.
- [x] Multiple session support.
- [x] True color support.
- [ ] NAT hole punching 
- [ ] IP address change on the fly.
- [ ] Encryption.
- [ ] Authentication.
- [ ] Local echo.
- [ ] Adjustable frame rate.
- [ ] Deleting old sessions.
- [ ] Performance comparsion to SSH.
- [ ] Support other keys, such as arrows.


# Running

#### Clone the repository

First, clone the repository and `cd` into it.

```bash
git clone https://github.com/gbaranski/jaw.git
cd jaw
```

#### Run server

```bash
cargo run --bin jaw-server
```

#### Run client

```bash
cargo run --bin jaw-client
```

Bash prompt should now appear on the screen.


Optionally pass `--release` flag, e.g `cargo run --release --bin jaw-client` to run a optimized build.
