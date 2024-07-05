#!/bin/bash

REPO_ROOT="$(git rev-parse --show-toplevel)"

surreal start --bind 0.0.0.0:6767 "file:$REPO_ROOT/monee-cli/data/monee/monee.db"
