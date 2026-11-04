#!/usr/bin/env bash
# Build the Linux package on GitHub and add your project files to it.
#
#   packaging/make-linux-package.sh                 includes silvius.json
#   packaging/make-linux-package.sh a.json b.json   includes those files
#   packaging/make-linux-package.sh --no-projects   app only
#
# The Linux binary cannot easily be built on a Mac, so this runs the
# "Linux package" GitHub workflow on the pushed `main`, waits for it,
# downloads the result, and repacks it with your JSON files (which are
# gitignored, so GitHub never sees them). Push your latest changes first.
#
# Output: dist/woodworking-linux-x86_64.tar.gz, ready to send.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

projects=()
if [[ "${1:-}" == "--no-projects" ]]; then
    :
elif [[ $# -gt 0 ]]; then
    projects=("$@")
else
    projects=(silvius.json)
fi
for file in "${projects[@]}"; do
    [[ -f "$file" ]] || { echo "No such file: $file"; exit 1; }
done

if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
    echo "Note: you have uncommitted changes. GitHub builds what is pushed, not your working copy."
fi

echo "Starting the Linux build on GitHub..."
gh workflow run linux.yml --ref main
sleep 5
run_id="$(gh run list --workflow linux.yml --branch main --limit 1 --json databaseId --jq '.[0].databaseId')"
echo "Waiting for run $run_id (the first build takes 10-20 minutes, later ones are faster)..."
gh run watch "$run_id" --exit-status

rm -rf dist/download dist/woodworking-linux-x86_64
mkdir -p dist/download
gh run download "$run_id" --name woodworking-linux-x86_64 --dir dist/download
tar -xzf dist/download/woodworking-linux-x86_64.tar.gz -C dist
for file in "${projects[@]}"; do
    cp "$file" dist/woodworking-linux-x86_64/
done
tar -czf dist/woodworking-linux-x86_64.tar.gz -C dist woodworking-linux-x86_64
rm -rf dist/download dist/woodworking-linux-x86_64

echo
echo "Ready: dist/woodworking-linux-x86_64.tar.gz"
ls -lh dist/woodworking-linux-x86_64.tar.gz
