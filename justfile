name := 'cosmic-application-board'
appid := 'com.sigmasonix.CosmicLaunchpad'

export APP_ID := 'com.sigmasonix.CosmicLaunchpad'

rootdir := ''
prefix := '/usr'

base-dir := absolute_path(clean(rootdir / prefix))
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')

mod cargo 'cargo.just'

bin-src := cargo-target-dir / 'release' / name
bin-dst := base-dir / 'bin' / name
applet-name := name + '-applet'
applet-bin-src := cargo-target-dir / 'release' / applet-name
applet-bin-dst := base-dir / 'bin' / applet-name
applet-desktop := 'com.sigmasonix.CosmicLaunchpadApplet.desktop'
applet-desktop-src := 'data' / applet-desktop
applet-desktop-dst := base-dir / 'share' / 'applications' / applet-desktop

appdata := appid + '.metainfo.xml'
appdata-dst := base-dir / 'share' / 'appdata' / appdata

desktop := appid + '.desktop'
desktop-src := 'data' / desktop
desktop-dst := base-dir / 'share' / 'applications' / desktop

icon := appid + '.svg'
icon-dst := base-dir / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon

# Default recipe which runs `just build-release`
default: build-release

# Runs `cargo clean`
clean: cargo::clean

# `cargo clean` and removes vendored dependencies
clean-dist: cargo::clean-dist

# Compiles with debug profile
build-debug *args: (cargo::build-debug args)

# Compiles with release profile
build-release *args: (cargo::build-release args)

# Builds an installable .deb in target/packages after the release binaries exist.
package-deb: build-release
    ./packaging/build-deb.sh

# Compiles release profile with vendored dependencies
build-vendored *args: cargo::vendor-extract
    LOCKSTEP_XML_PATH="${PWD}/vendor/atspi-common/xml" cargo build --release --frozen --offline {{args}}

# Compiles and runs a standalone instance
run *args: (cargo::run args)

# Runs a clippy check
check *args: (cargo::check args)

# Runs a clippy check with JSON message format
check-json: (check '--message-format=json')

# Installs files
install:
    install -Dm0755 {{bin-src}} {{bin-dst}}
    install -Dm0755 {{applet-bin-src}} {{applet-bin-dst}}
    install -Dm0644 {{desktop-src}} {{desktop-dst}}
    install -Dm0644 {{applet-desktop-src}} {{applet-desktop-dst}}
    install -Dm0644 {{ 'data' / appdata }} {{appdata-dst}}
    install -Dm0644 {{ 'data' / 'icons' / icon }} {{icon-dst}}

# Uninstalls installed files
uninstall:
    rm {{bin-dst}} {{applet-bin-dst}} {{desktop-dst}} {{applet-desktop-dst}} {{appdata-dst}} {{icon-dst}}

# Vendor dependencies locally
vendor: cargo::vendor

# Extracts vendored dependencies
vendor-extract: cargo::vendor-extract
