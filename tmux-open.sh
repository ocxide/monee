#!/bin/sh

tmux new-session -d -s monee -c ~/projects/monee/monee -n "monee"

tmux new-window -a -t monee -c monee-cli -n "cli"
tmux split-window -h -t monee:cli -c monee-cli

tmux new-window -a -t monee:cli -c ~/projects/monee/monee-cli/data/monee -n "db"
tmux send-keys -t monee:db "~/projects/monee/monee/open-db.sh" C-m
tmux split-window -h -t monee:db -c ~/projects/monee/monee-cli/data/monee
tmux send-keys -t monee:db "surreal sql --ns=monee --db=monee --pretty --endpoint='ws://0.0.0.0:6767'"

tmux select-window -t monee:1
tmux attach -t monee
