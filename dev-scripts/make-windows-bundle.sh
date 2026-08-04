#!/bin/sh
set -e

# Assemble the complete redistributable Windows directory used for CI previews
# and numbered releases.

if [ "$#" -eq 1 ]; then
    PATH_TO_BINARY="$1"
    shift

    rm -rf tapHLE_windows_bundle
    mkdir tapHLE_windows_bundle
    cp "$PATH_TO_BINARY" tapHLE_windows_bundle/
    cp -r ../tapHLE_dylibs tapHLE_windows_bundle/
    cp -r ../tapHLE_fonts tapHLE_windows_bundle/
    mkdir tapHLE_windows_bundle/tapHLE_apps
    cp ../tapHLE_apps/README.txt tapHLE_windows_bundle/tapHLE_apps/
    cp ../README.md tapHLE_windows_bundle/
    cp ../CHANGELOG.md tapHLE_windows_bundle/
    cp gpl-3.0.txt tapHLE_windows_bundle/COPYING.txt
    cp ../OPTIONS_HELP.txt tapHLE_windows_bundle/
    cp ../tapHLE_default_options.txt tapHLE_windows_bundle/
    cp ../tapHLE_options.txt tapHLE_windows_bundle/
else
    echo "Incorrect usage."
    exit 1
fi
