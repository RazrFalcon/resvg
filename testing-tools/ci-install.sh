#!/usr/bin/env bash

set -e

# Required to install MS fonts.
sudo apt-get update

# Install MS fonts since we are using Arial as a base font for tests.
echo ttf-mscorefonts-installer msttcorefonts/accepted-mscorefonts-eula select true | sudo debconf-set-selections
sudo apt-get install -y ttf-mscorefonts-installer

# Required for all backends.
sudo apt-get install -y libharfbuzz-dev

if [ "$RESVG_CAIRO_BACKEND" = true ]; then
    sudo apt-get install -y libcairo2-dev libgdk-pixbuf2.0-dev
    # for capi/examples/cairo-*
    sudo apt-get install -y libgtk-3-dev
fi

if [ "$RESVG_QT_BACKEND" = true ]; then
    sudo add-apt-repository ppa:beineri/opt-qt563-xenial -y
    sudo apt-get update -qq
    sudo apt-get install -qq qt56base qt56svg
    # to fix the -lGL linking
    sudo apt-get install -y libgl1-mesa-dev
fi
