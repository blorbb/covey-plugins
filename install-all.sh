#!/usr/bin/env sh

for directory in ./*/ ; do
    # directory does not contain "target"
    if [[ ! "$directory" =~ "target" ]]; then
        echo "installing $directory"
        # parens required to enter subshell so that each cd starts from here
        (cd "$directory" && cargo build --release --artifact-dir ~/.config/qpmu/plugins -Z unstable-options)
    fi
done
