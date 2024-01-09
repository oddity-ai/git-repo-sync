release version:
    #!/bin/bash
    if [[ $(git rev-parse --abbrev-ref HEAD) != "main" ]]; then
        echo "release must be from main branch: checkout main branch before releasing"
        exit 1
    fi
    if [ -n "$(git status --porcelain)" ]; then
        echo "release must be from clean working tree: commit changes before releasing"
        echo "commit changes before releasing"
        exit 1
    fi
    if !(echo "{{version}}" | grep -Eq ^[0-9]+\.[0-9]+\.[0-9]+$); then
        echo "invalid version string"
        exit 1
    fi
    git tag -a "v{{version}}" -m "release: version {{version}}"
    git push origin "v{{version}}"
    echo 'release: {{version}}'
