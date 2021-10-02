#!/bin/bash
set -eum

# Starts a local web-server that serves the contents of the `doc/` folder,
# which is the folder to where the web version is compiled.

miniserve -p 8080 --index index.html docs/ &

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  # Linux, ex: Fedora
  xdg-open http://localhost:8080/index.html
elif [[ "$OSTYPE" == "msys" ]]; then
  # Windows
  start http://localhost:8080/index.html
else
  # Darwin/MacOS, or something else
  open http://localhost:8080/index.html
fi

fg