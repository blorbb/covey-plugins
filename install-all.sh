#!/usr/bin/env bash

SCRIPT_PATH=$(dirname "$(realpath -s "$0")")
cd "$SCRIPT_PATH" || exit

# directory must be of the form "pluginname/"
for directory in */ ; do
    directory=${directory%*/}  # remove the trailing "/"
    # directory does not contain "target"
    if [[ ! "$directory" =~ "target" ]]; then
        echo "installing $directory"
        # parens required to enter subshell so that each cd starts from here
        if [[ "$1" = "debug" ]]; then
            (cd "$directory" && cargo build --artifact-dir "~/.local/share/covey/plugins/$directory" -Z unstable-options)
        else
            (cd "$directory" && cargo build --release --artifact-dir "~/.local/share/covey/plugins/$directory" -Z unstable-options)
        fi
        cp "$directory/manifest.toml" "~/.local/share/covey/plugins/$directory/manifest.toml"
    fi
done
