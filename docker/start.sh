#!/bin/bash

set -e
DOTENV_FILE="$HOME/.env"
# create empty dot env file
echo "" > $DOTENV_FILE

while IFS='=' read -r -d '' n v; do
    if [[ $n == TASK_WEB_* ]]; then
        echo "${n/TASK_WEB_/}=\"$v\"" >> $DOTENV_FILE
    fi
done < <(env -0)

# check if taskrc exists.
if [[ ! -f "$TASKRC" ]]; then
    echo "yes" | task || true
fi

cd $HOME/bin
exec ./taskwarrior-web
