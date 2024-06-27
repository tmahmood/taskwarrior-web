#!/bin/bash

set -e

cd $HOME/app/
source $HOME/.cargo/env
cargo run --release
