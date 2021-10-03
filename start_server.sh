#!/bin/bash
set -eum

# Starts a local web-server that serves the contents of the `doc/` folder,
# which is the folder to where the web version is compiled.

miniserve -p 8080 --index index.html docs/ 
