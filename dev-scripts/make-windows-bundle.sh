#!/bin/sh
set -e

# Bundles the tapHLE executable with the basic set of files needed for
# tapHLE to run (the same ones found in the macOS development bundle).
# This does not prepare a full release.

if [[ $# == 1 ]]; then
    PATH_TO_BINARY="$1"
    shift

    rm -rf tapHLE_windows_bundle
    mkdir tapHLE_windows_bundle
    cp $PATH_TO_BINARY tapHLE_windows_bundle/
    cp -r ../tapHLE_dylibs tapHLE_windows_bundle/
    cp -r ../tapHLE_fonts tapHLE_windows_bundle/
    cp -r ../tapHLE_default_options.txt tapHLE_windows_bundle/
else
    echo "Incorrect usage."
    exit 1
fi
