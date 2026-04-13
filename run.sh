#!/bin/bash

#opcion so
if [ $1 == "-t" ]; then
    echo "Testing set option commands"
    exec ./target/debug/tsbpal "token" "serpapi_token"
elif [ $1 == "-m" ]; then
    echo "Main set option commands"
    exec ./target/debug/tsbpal "token" "serpapi_token"
elif [$1 == "--man" || $1 "-h" ]; then
    echo"-t to the testing"
    echo "-m to the main"
fi
