#!/usr/bin/env sh

for directory in */ ; do
    directory=${directory%*/}  # remove the trailing "/"
    # directory does not contain "target"
    if [[ ! "$directory" =~ "target" ]]; then
        echo "installing $directory"
        # parens required to enter subshell so that each cd starts from here
        if [[ "$1" = "debug" ]]; then
            (cd "$directory" && cargo build --artifact-dir ~/.local/share/qpmu/plugins/$directory -Z unstable-options)
        else
            (cd "$directory" && cargo build --release --artifact-dir ~/.local/share/qpmu/plugins/$directory -Z unstable-options)
        fi
    fi
done
