FROM node:24-bookworm AS node
FROM rust:1.93.0-bookworm

COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
       libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev \
       libclang-dev libxcb1-dev libxrandr-dev libdbus-1-dev libpipewire-0.3-dev \
       libwayland-dev libegl-dev libgbm-dev libssl-dev pkg-config patchelf ca-certificates \
       xvfb xauth openbox xcompmgr xfce4-panel xfconf dbus-x11 \
       xdotool wmctrl xclip x11-utils x11-xserver-utils scrot \
       python3 python3-pil python3-gi python3-gi-cairo gir1.2-gtk-3.0 gir1.2-atspi-2.0 \
       at-spi2-core gsettings-desktop-schemas \
    && rm -rf /var/lib/apt/lists/*

ENV CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
    CARGO_TARGET_DIR=/target LIBGL_ALWAYS_SOFTWARE=1 \
    WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
WORKDIR /workspace
RUN rustup component add clippy rustfmt
