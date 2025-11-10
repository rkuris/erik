# ESP32-C6 Rust Development Container
# Optimized for firmware builds with pre-installed toolchains
FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

# Build args
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

# Install system dependencies in a single layer
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    # Build essentials
    git \
    wget \
    curl \
    ca-certificates \
    build-essential \
    pkg-config \
    # ESP-IDF requirements
    flex \
    bison \
    gperf \
    python3 \
    python3-pip \
    python3-venv \
    cmake \
    ninja-build \
    ccache \
    libffi-dev \
    libssl-dev \
    dfu-util \
    libusb-1.0-0 \
    udev \
    # Playwright browser runtime libraries
    libnspr4 \
    libnss3 \
    libdbus-1-3 \
    libatk1.0-0 \
    libatk-bridge2.0-0 \
    libatspi2.0-0 \
    libx11-6 \
    libxcomposite1 \
    libxdamage1 \
    libxext6 \
    libxfixes3 \
    libxrandr2 \
    libgbm1 \
    libxcb1 \
    libxkbcommon0 \
    libasound2 \
    libcups2 \
    libcairo2 \
    libpango-1.0-0 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js LTS (20.x) for web UI builds
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get update \
    && apt-get install -y nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update \
    && apt-get install gh -y \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Switch to vscode user for toolchain installations
USER $USERNAME

# Install Rust toolchain (nightly with ESP32 support)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly \
    && . "$HOME/.cargo/env" \
    && rustup component add rust-src --toolchain nightly

# Add cargo to PATH
ENV PATH="/home/${USERNAME}/.cargo/bin:${PATH}"

# Install ESP-IDF v5.1.2 (cached layer - expensive operation)
RUN mkdir -p /home/${USERNAME}/esp \
    && cd /home/${USERNAME}/esp \
    && git clone --recursive --depth 1 --branch v5.1.2 https://github.com/espressif/esp-idf.git \
    && cd esp-idf \
    && ./install.sh esp32c6

# Create ESP-IDF environment script
RUN echo '#!/bin/bash\nsource $HOME/esp/esp-idf/export.sh' > /home/${USERNAME}/export-esp.sh \
    && chmod +x /home/${USERNAME}/export-esp.sh \
    && echo 'source $HOME/export-esp.sh' >> /home/${USERNAME}/.bashrc

# Install cargo tools for ESP32 (cached layer)
RUN . "$HOME/.cargo/env" \
    && cargo install espflash espmonitor cargo-espflash ldproxy

# Set working directory
WORKDIR /workspaces/pool-controller

# Keep container running
CMD ["/bin/bash"]
