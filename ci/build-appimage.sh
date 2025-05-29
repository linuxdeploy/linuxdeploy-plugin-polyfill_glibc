#! /bin/bash

set -euo pipefail

build_dir="$(mktemp -d -t ld-p-polyfill_glibc-XXXXX)"

cleanup() {
    [[ -d "$build_dir" ]] && rm -rf "$build_dir"
}
trap cleanup EXIT

orig_cwd="$(readlink -f "$PWD")"

echo "Build project with cargo"
env CARGO_TARGET_DIR="${build_dir}/cargo-build" cargo build --release

pushd "$build_dir"

echo "Build polyfill-glibc statically"
git clone https://github.com/corsix/polyfill-glibc
pushd polyfill-glibc
sed -i.bak 's|^link_opt =|link_opt = -static|' build.ninja
ninja
popd

echo "Assemble AppDir"
mkdir -p AppDir/usr/bin
mv polyfill-glibc/polyfill-glibc AppDir/usr/bin/
mv polyfill-glibc/LICENSE AppDir/polyfill-glibc.LICENSE
mv cargo-build/release/linuxdeploy-plugin-polyfill_glibc AppDir/usr/bin/

# debugging
find AppDir

echo "Build AppImage with linuxdeploy"
wget https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
chmod +x linuxdeploy-x86_64.AppImage
cat > linuxdeploy-plugin-polyfill_glibc.desktop <<\EOF
[Desktop Entry]
Name=linuxdeploy-plugin-polyfill_glibc
Exec=linuxdeploy-plugin-polyfill_glibc %F
Type=Application
Icon=linuxdeploy-plugin-polyfill_glibc
Categories=Development;
X-AppImage-Integrate=false
EOF
touch linuxdeploy-plugin-polyfill_glibc.svg
cat > AppRun.sh <<\EOF
#! /bin/bash

this_dir="$(readlink -f "$(dirname ${BASH_SOURCE[0]})")"
echo "$this_dir"
export PATH="${this_dir}/usr/bin/:${PATH}"
exec "${this_dir}/usr/bin/linuxdeploy-plugin-polyfill_glibc" "$@"
EOF
chmod +x AppRun.sh
./linuxdeploy-x86_64.AppImage \
    --appdir AppDir \
    --output appimage \
    -d linuxdeploy-plugin-polyfill_glibc.desktop \
    -i linuxdeploy-plugin-polyfill_glibc.svg \
    --custom-apprun=AppRun.sh

mv -v linuxdeploy-plugin-polyfill_glibc*.AppImage "$orig_cwd"
