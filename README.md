# HASS Alexa Relay

An AWS lambda function that enables the connection between Amazons Alexa and a Home Assistant instance via a wireguard VPN tunnel.

## Why?
This lambda implementation of the home assistant Alexa skill lambda enables the connection to a home assistant instance that is only accessible via a Wireguard VPN.

## Installation
For a detailed setup guide refer to [SETUP.md](/SETUP.md)

### Pre-built Binary
Download the pre-built binary from the release page and upload the zip file to your lambda function. Make sure to set the runtime to `Custom runtime on Amazon Linux 2`,
the architecture to `x86_64` and under Configuration > General Configuration set the timeout to about 15 seconds.

### Build from source
Make sure to have a properly set up rust build environment. Install `cargo-lambda` via `cargo install cargo-lambda`.
Build the crate via `cargo lambda build --release --output-format Zip`. Afterwards follow the steps for the pre-build version.
