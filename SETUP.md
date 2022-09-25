# Alexa smart-home skill for Home Assistant behind Wireguard VPN

This guide describes how to set up an Alexa smart-home skill for Home Assistant when the HASS instance is only accessible behind a Wireguard VPN.

## Base Instructions

Please follow the [official step by step guide](https://www.home-assistant.io/integrations/alexa.smart_home/#requirements),
this document will only replace some of the official steps to make it work with the Wireguard VPN.

## Requirements

### Obsolete

The first requirement of the official guide is obsolete, as this is the entire point of this set of changes. Your home assistant does not have to be exposed to the internet.

### New

- The Wireguard Add-On has to be installed and setup in your Home Assistant instance. alternatively any other method of setting up a Wireguard server can be chosen.
- At least three peers have to be configured in your Wireguard server. We will call them `Peer 1`, `Peer 2` and `Peer 3` for the duration of this guide.
- The Wireguard server port has to be exposed to the internet, this is the only port that has to be opened for this guide.
- A public web-server on which you can host arbitrary binary daemons (background service)
- A domain with a valid TLD that is assigned to your HA instance. (You can use the AdGuard Add-On for this).

## Add Code to the Lambda Function

For this step, follow the official guide but instead of selecting a Python runtime, select "Custom runtime on Amazon Linux 2".
Instead of pasting Python code, click "Upload from" and select ".zip file". Either download the pre-built binary from the Releases page
of this repository or clone the repository and [build the lambda function from source](/README.md#build-from-source) yourself.

Under the "Configuration" tab select "General configuration" > "Edit" and change the timeout to 15s.

For the environment variables, there a a couple of changes, the variables from the original guide do not apply:

- ENDPOINT: this is the public address and port of your Wireguard VPN server
- HA_HOST: the IP address of your HA instance **on your local network** and the correct port (default is 51820)
- LOG_LEVEL: you can choose between `error`, `warning`, `info`, `debug`
- LONG_LIVED_ACCESS_TOKEN: this is optional and only needed when testing the lambda function
- PRIVATE_KEY: your Wireguard private key
- PUBLIC_KEY: your Wireguard public key
- SOURCE_PEER_IP: your Wireguard source peer IP

For this lambda function the Wireguard `Peer 1` is expected to be used.

## Account Linking

The account linking step requires some special URLs. Their properties will be explained for each of them:

- **Authorization URI:** This url will be accessed once during account linking by the Alexa app on your mobile device.
  It should only be accessible via the VPN (`Peer 2`), but a requirement from Amazons side is that the domain ueses a valid TLD.
  It has to be accessed via HTTPS but a valid TLS certificate is not required. This does not have to be the primary URL which is normally used to access the HA instance.
  (Example: using the domain `assistant.home` could be paired with `assistant.ho.me` as `.me` is a valid TLD)
- **Access Token URI:** This URI is trickier as it has to be accessible by Amazons cloud services (which obviously will not connect to a Wireguard VPN first) so your Alexa account can refresh it's access token.
  To avoid exposing anything more than necessary of your HA instance, it can be made available by putting it behind a reverse proxy that will then connect to your Wireguard server.
  An additional layer of security is then to protect the proxied endpoint with an access token which can be rotated from time to time.
  For this a public web-server is required which is able to run a Wireguard client and can be configured to act as a reverse proxy. (`Peer 3`)

  Examples:
    - Wireguard Client: [Onetun](https://github.com/aramperes/onetun) is a decent user-space Wireguard client that should be able to run in most places
    - Reverse Proxy: [Nginx](https://nginx.org/) can easily be configured to act as a reverse proxy.

  The final URL should look something like this: `https://proxy.example.com/<TOKEN>`
