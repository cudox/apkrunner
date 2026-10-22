#!/usr/bin/env bash
# Build the M1 runtime: wolfSSL (JNI) + bionic_translation + ART + ATL.
#
# Everything is installed under $APKRUNNER_RUNTIME_PREFIX, so no root is
# needed and the system is never modified. Building ART takes a while.
#
# Usage:
#   scripts/build-runtime.sh            # build everything
#   scripts/build-runtime.sh wolfssl    # only one component
#
# Components: wayland-protocols, wolfssl, bionic, art, atl

set -euo pipefail

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PREFIX=${APKRUNNER_RUNTIME_PREFIX:-$HOME/.local/apkrunner-runtime}
BUILD_DIR=${APKRUNNER_BUILD_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/apkrunner-build}
JOBS=${JOBS:-$(( $(nproc) > 8 ? 8 : $(nproc) ))}
JAVA_HOME=${JAVA_HOME:-/usr/lib/jvm/java-21-openjdk}
COMPONENTS=("$@")
[ ${#COMPONENTS[@]} -eq 0 ] && COMPONENTS=(wayland-protocols wolfssl bionic art atl)

log() { printf '\n==> %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() {
    command -v "$1" >/dev/null 2>&1 || die "missing command: $1"
}

log "prefix:    $PREFIX"
log "build dir: $BUILD_DIR"
mkdir -p "$PREFIX/lib/pkgconfig" "$PREFIX/bin" "$BUILD_DIR"

need gcc
need make
need meson
need git
need javac
need pkg-config
need curl
need autoreconf

export PKG_CONFIG_PATH="$PREFIX/share/pkgconfig:$PREFIX/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export LIBRARY_PATH="$PREFIX/lib${LIBRARY_PATH:+:$LIBRARY_PATH}"
export C_INCLUDE_PATH="$PREFIX/include${C_INCLUDE_PATH:+:$C_INCLUDE_PATH}"
export CPLUS_INCLUDE_PATH="$PREFIX/include${CPLUS_INCLUDE_PATH:+:$CPLUS_INCLUDE_PATH}"
export LD_LIBRARY_PATH="$PREFIX/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="$PREFIX/bin:$PATH"
export JAVA_HOME


# Some Arch packages are runtime-only and keep headers/tools in a separate
# -devel package. Extract those into the prefix instead of requiring root.
fetch_arch_pkg() {
    local pkg=$1 url
    command -v pacman >/dev/null 2>&1 || die "install $pkg (or provide its files) and retry"
    url=$(pacman -Sp --print-format '%l' "$pkg" | tail -1)
    [ -n "$url" ] || die "cannot resolve the $pkg package URL"
    log "extracting $pkg into the prefix"
    curl -fsSL -A "pacman/7.0.0" -o "$BUILD_DIR/$pkg.pkg.tar.zst" "$url"
    rm -rf "$BUILD_DIR/$pkg-extract"
    mkdir -p "$BUILD_DIR/$pkg-extract"
    tar --zstd -xf "$BUILD_DIR/$pkg.pkg.tar.zst" -C "$BUILD_DIR/$pkg-extract"
}

ensure_build_tools() {
    # Vulkan: vulkan-icd-loader.pc exists but the headers live in another package.
    if [ ! -f /usr/include/vulkan/vulkan.h ] &&
       [ ! -f "$PREFIX/include/vk_video/vulkan_video_codec_h264std.h" ]; then
        fetch_arch_pkg vulkan-headers
        mkdir -p "$PREFIX/include"
        cp -a "$BUILD_DIR/vulkan-headers-extract/usr/include/." "$PREFIX/include/"
    fi

    # Meson asks pkg-config for glib-genmarshal/gdbus-codegen and ignores PATH.
    if ! command -v glib-genmarshal >/dev/null 2>&1 ||
       ! command -v glib-mkenums >/dev/null 2>&1 ||
       ! command -v gdbus-codegen >/dev/null 2>&1; then
        fetch_arch_pkg glib2-devel
        install -m755 "$BUILD_DIR/glib2-devel-extract/usr/bin/"* "$PREFIX/bin/"
        mkdir -p "$PREFIX/share/glib-2.0"
        cp -a "$BUILD_DIR/glib2-devel-extract/usr/share/glib-2.0/codegen" "$PREFIX/share/glib-2.0/"
        # Re-point the tool variables at the extracted binaries.
        for pc in glib-2.0 gio-2.0; do
            local pc_file
            pc_file=$(pkg-config --variable=pcfiledir "$pc")/$pc.pc
            sed "s|^bindir=.*|bindir=$PREFIX/bin|" "$pc_file" > "$PREFIX/lib/pkgconfig/$pc.pc"
        done
        install -m755 /usr/bin/glib-compile-resources /usr/bin/glib-compile-schemas "$PREFIX/bin/"
    fi
}

for dep in egl libelf libbsd libunwind gtk4 libportal sqlite3 libavcodec \
           libdrm gudev-1.0 webkitgtk-6.0 libsecret-1 openxr vulkan; do
    pkg-config --exists "$dep" || die "missing pkg-config dependency: $dep"
done

ensure_build_tools

# JNI and libdl have no .pc on Arch; provide them in our prefix.
cat > "$PREFIX/lib/pkgconfig/jni.pc" <<EOF
prefix=$JAVA_HOME
includedir=\${prefix}/include
Name: jni
Description: Java Native Interface headers
Version: 21
Cflags: -I\${includedir} -I\${includedir}/linux
Libs:
EOF

cat > "$PREFIX/lib/pkgconfig/dl.pc" <<EOF
Name: dl
Description: libdl (provided by glibc)
Version: 1
Libs: -ldl
EOF

has() { [[ " ${COMPONENTS[*]} " == *" $1 "* ]]; }

if has wayland-protocols; then
    log "wayland-protocols"
    if ! pkg-config --exists wayland-protocols; then
        cd "$BUILD_DIR"
        version=1.49
        [ -d "wayland-protocols-$version" ] || {
            curl -fsSL -o wayland-protocols.tar.xz \
                "https://gitlab.freedesktop.org/wayland/wayland-protocols/-/releases/$version/downloads/wayland-protocols-$version.tar.xz"
            tar xf wayland-protocols.tar.xz
        }
        cd "wayland-protocols-$version"
        meson setup builddir --prefix="$PREFIX" >/dev/null
        meson compile -C builddir >/dev/null
        meson install -C builddir >/dev/null
    fi
fi

if has wolfssl; then
    log "wolfSSL 5.8.2 with JNI"
    cd "$BUILD_DIR"
    if [ ! -d wolfssl ]; then
        git clone --depth 1 -b v5.8.2-stable \
            https://github.com/wolfSSL/wolfssl.git
    fi
    cd wolfssl
    [ -f configure ] || autoreconf -i >/dev/null 2>&1
    # GCC 15+ flags a false positive in asn.c; keep the build reproducible.
    CFLAGS="-O2 -Wno-error=stringop-truncation" ./configure \
        --prefix="$PREFIX" --enable-shared \
        --disable-opensslall --disable-opensslextra \
        --enable-aescbc-length-checks --enable-curve25519 \
        --enable-ed25519 --enable-ed25519-stream --enable-oldtls \
        --enable-base64encode --enable-tlsx --enable-scrypt \
        --disable-examples --enable-crl --enable-jni >/dev/null
    make -j"$JOBS" >/dev/null
    make install >/dev/null
fi

if has bionic; then
    log "bionic_translation"
    cd "$BUILD_DIR"
    [ -d bionic_translation ] || git clone --depth 1 \
        https://gitlab.com/android_translation_layer/bionic_translation.git
    cd bionic_translation
    rm -rf builddir
    meson setup builddir --prefix="$PREFIX" >/dev/null
    meson compile -C builddir >/dev/null
    meson install -C builddir >/dev/null
fi

if has art; then
    log "art_standalone (this takes a while)"
    cd "$BUILD_DIR"
    [ -d art_standalone ] || git clone --depth 1 \
        https://gitlab.com/android_translation_layer/art_standalone.git
    cd art_standalone
    make -j"$JOBS" ____PREFIX="$PREFIX" ____LIBDIR=lib
    make ____PREFIX="$PREFIX" ____LIBDIR=lib install
fi

if has atl; then
    log "Android Translation Layer"
    cd "$BUILD_DIR"
    [ -d atl-src ] || git clone --depth 1 \
        https://gitlab.com/android_translation_layer/android_translation_layer.git atl-src
    cd atl-src

    # apkrunner keeps small patches for API classes ATL does not implement yet.
    for patch in "$REPO_ROOT"/patches/atl/*.patch; do
        [ -e "$patch" ] || continue
        if git apply --reverse --check "$patch" 2>/dev/null; then
            log "already applied: $(basename "$patch")"
        else
            git apply "$patch" || die "cannot apply $(basename "$patch")"
            log "applied: $(basename "$patch")"
        fi
    done

    rm -rf builddir
    # ATL already passes -source 1.8 -target 1.8 -bootclasspath, which produces
    # the Java 8 bytecode dx needs. Do NOT set JDK_JAVAC_OPTIONS=--release 8:
    # javac rejects --release together with -bootclasspath.
    meson setup builddir --prefix="$PREFIX"
    meson compile -C builddir
    meson install -C builddir
fi

log "done"
cat <<EOF
Runtime installed in: $PREFIX
Binary:               $PREFIX/bin/android-translation-layer

Try:
  export APKRUNNER_RUNTIME_PREFIX=$PREFIX
  apkrunner run /path/to/app.apk
EOF
