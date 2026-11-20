#!/bin/bash
set -euo pipefail
if [[ "$(uname -s)" != Darwin || "$(uname -m)" != x86_64 || "$(sw_vers -productVersion)" != 12.* || "$(sysctl -in sysctl.proc_translated 2>/dev/null || true)" == 1 ]]; then
    echo 'This check requires an Intel Mac running macOS Monterey.' >&2
    exit 2
fi
sw_vers
[[ "$(spctl --status)" == 'assessments enabled' ]] || { echo 'Default Gatekeeper assessments are required.' >&2; exit 2; }
app="${1:?Pass the path to the installed ML Studio.app}"
binary="$app/Contents/MacOS/lemonstudio"
[[ -x "$binary" ]]
/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$app/Contents/Info.plist"
codesign --verify --deep --strict "$app"
spctl --assess --type execute -vv "$app"
scratch="$(mktemp -d)"
trap 'rm -rf -- "$scratch"' EXIT
export OPENSTUDIO_USER_DIR="$scratch/profile" GRIDLINE_CONFIG_HOME="$scratch/profile" OPENSTUDIO_NO_PLUGIN_BUILD=1
mkdir "$OPENSTUDIO_USER_DIR"
"$binary" --startup-report "$scratch/startup.txt"
grep -F -- "$OPENSTUDIO_USER_DIR" "$scratch/startup.txt"
"$binary" --scan-plugin au 'macOS Audio Units' > "$scratch/scan.txt"
grep -F '61756D75646C73206170706C' "$scratch/scan.txt" >/dev/null
grep -F '617566786C7061736170706C' "$scratch/scan.txt" >/dev/null
echo 'Intel Monterey startup and Audio Unit discovery passed.'
