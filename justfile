plugin-dir := data_dir()/"covey"/"plugins"

# installs all plugins
install-all mode='release':
    #!/usr/bin/env bash
    set -euo pipefail
    # directory must be of the form "pluginname/"
    for directory in */ ; do
        name="${directory%*/}"  # remove the trailing "/"
        # directory does not contain "target"
        if [[ ! "$name" =~ "target" ]] ; then
            just install "$name" "{{mode}}"
        fi
    done

# installs a specific plugin
install plugin mode='release':
    #!/usr/bin/env bash
    set -euo pipefail
    echo "installing {{plugin}} to {{plugin-dir}}/{{plugin}}"
    if [[ "{{mode}}" = "debug" ]]; then
        cargo build -p '{{plugin}}' --artifact-dir "{{plugin-dir}}/{{plugin}}" -Z unstable-options
    else if [[ "{{mode}}" = "release" ]]; then
        cargo build -p '{{plugin}}' --release --artifact-dir "{{plugin-dir}}/{{plugin}}" -Z unstable-options
    else
        echo "mode must be release or debug"
        exit 1
    fi fi
    cp "{{plugin}}/manifest.toml" "{{plugin-dir}}/{{plugin}}/manifest.toml"
