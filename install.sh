#!/bin/sh

mkdir -p ~/.config/paste-client
touch ~/.config/paste-client/history_token
echo "# base_url = THE SERVER URL" > ~/.config/paste-client/config.toml
echo "username = Anonymous" > ~/.config/paste-client/config.toml
cargo build --release
sudo cp target/release/paste-client /usr/local/bin
rm -rf target