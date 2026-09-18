#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${project_dir}/Cargo.toml" | head -n 1)"
architecture="$(dpkg --print-architecture)"
package_name="cosmic-launchpad_${version}_${architecture}"
stage="${project_dir}/target/deb/${package_name}"
output_dir="${project_dir}/target/packages"

rm -rf "${stage}"
mkdir -p \
    "${stage}/DEBIAN" \
    "${stage}/usr/bin" \
    "${stage}/usr/share/applications" \
    "${stage}/usr/share/metainfo" \
    "${stage}/usr/share/icons/hicolor/scalable/apps" \
    "${output_dir}"

install -m0755 "${project_dir}/target/release/cosmic-application-board" \
    "${stage}/usr/bin/cosmic-application-board"
install -m0755 "${project_dir}/target/release/cosmic-application-board-applet" \
    "${stage}/usr/bin/cosmic-application-board-applet"
install -m0644 "${project_dir}/data/com.sigmasonix.CosmicLaunchpad.desktop" \
    "${stage}/usr/share/applications/com.sigmasonix.CosmicLaunchpad.desktop"
install -m0644 "${project_dir}/data/com.sigmasonix.CosmicLaunchpadApplet.desktop" \
    "${stage}/usr/share/applications/com.sigmasonix.CosmicLaunchpadApplet.desktop"
install -m0644 "${project_dir}/data/com.sigmasonix.CosmicLaunchpad.metainfo.xml" \
    "${stage}/usr/share/metainfo/com.sigmasonix.CosmicLaunchpad.metainfo.xml"
install -m0644 "${project_dir}/data/icons/com.sigmasonix.CosmicLaunchpad.svg" \
    "${stage}/usr/share/icons/hicolor/scalable/apps/com.sigmasonix.CosmicLaunchpad.svg"
find "${stage}" -type d -exec chmod 0755 {} +

installed_size="$(du -sk "${stage}/usr" | cut -f1)"
printf '%s\n' \
    'Package: cosmic-launchpad' \
    "Version: ${version}" \
    'Section: utils' \
    'Priority: optional' \
    "Architecture: ${architecture}" \
    'Maintainer: Sigmasonix Studios <sigmasonixstudios@users.noreply.github.com>' \
    'Depends: libc6, libgcc-s1, libxkbcommon0' \
    "Installed-Size: ${installed_size}" \
    'Homepage: https://github.com/SigmasonixStudios/cosmic-launchpad' \
    'Description: Spatial application launcher for the COSMIC desktop' \
    ' Arrange favorite applications freely across persistent pages and keep' \
    ' unpinned applications available in a separate searchable library.' \
    > "${stage}/DEBIAN/control"

dpkg-deb --root-owner-group --build "${stage}" "${output_dir}/${package_name}.deb"
printf 'Built %s\n' "${output_dir}/${package_name}.deb"
