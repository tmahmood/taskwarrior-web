#!/bin/bash

#
# Copyright 2025 Tarin Mahmood
#
# Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
#

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
exec ./taskwarrior-web &
pid=$!
trap 'kill -SIGTERM $pid; wait $pid' SIGTERM
wait $pidh
