#!/bin/bash

CURRENT_DIR=$(pwd)

cd ./modules/common && cargo msrv verify && cd "$CURRENT_DIR" || exit
cd ./modules/quote-client && cargo msrv verify && cd "$CURRENT_DIR" || exit
cd ./modules/quote-server && cargo msrv verify && cd "$CURRENT_DIR" || exit
