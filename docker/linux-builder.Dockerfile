FROM rust:1.98.1-bookworm

ENV RUSTUP_TOOLCHAIN=1.98.1

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        build-essential \
        libwayland-dev \
        libx11-dev \
        libx11-xcb-dev \
        libxcb-cursor-dev \
        libxcb-randr0-dev \
        libxcb-render0-dev \
        libxcb-shape0-dev \
        libxcb-xfixes0-dev \
        libxkbcommon-dev \
        libxkbcommon-x11-dev \
        libxcursor-dev \
        libxi-dev \
        libxinerama-dev \
        libxrandr-dev \
        pkg-config \
        tar \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
