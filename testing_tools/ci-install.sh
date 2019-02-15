#!/usr/bin/env bash

set -ex

# python 3.6 for travis script and regression.py
sudo add-apt-repository ppa:deadsnakes/ppa -y
sudo apt-get update -qq
sudo apt-get install python3.6

if [ "$RESVG_CAIRO_BACKEND" = true ]; then
    sudo apt-get install -y libcairo2-dev
    # for capi/examples/cairo-*
    sudo apt-get install -y libgtk-3-dev
fi

if [ "$RESVG_QT_BACKEND" = true ]; then
    sudo add-apt-repository ppa:beineri/opt-qt562-trusty -y
    sudo apt-get update -qq
    sudo apt-get install -qq qt56base qt56svg
fi
