#!/usr/bin/env bash
# Ubuntu 26.04 build prerequisites. Rust and Node.js must already be installed.
set -euo pipefail
sudo apt-get update
sudo apt-get install --no-install-recommends \
  build-essential pkg-config libwebkit2gtk-4.1-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev libxdo-dev \
  fontconfig xdg-utils npm
