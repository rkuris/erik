### Devcontainer Dockerfile for Playwright tests
FROM mcr.microsoft.com/vscode/devcontainers/base:ubuntu-22.04

# non-interactive apt
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    curl \
    wget \
    ca-certificates \
    gnupg \
    build-essential \
    clang \
    gcc-multilib \
    g++-multilib \
    libc6-dev-i386 \
    # Playwright runtime deps
    libnss3 \
    libxss1 \
    libasound2 \
    libatk1.0-0 \
    libatk-bridge2.0-0 \
    libcups2 \
    libdrm2 \
    libxkbcommon0 \
    libxrandr2 \
    libgbm1 \
    libpangocairo-1.0-0 \
    libgtk-3-0 \
    libxcomposite1 \
    libxdamage1 \
    libxfixes3 \
    libpango-1.0-0 \
    fonts-liberation \
  && rm -rf /var/lib/apt/lists/*

# Install Node.js (LTS 20.x) from NodeSource
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
  && apt-get update \
  && apt-get install -y nodejs \
  && npm --version || true

# Keep the image lean
RUN apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Default user is vscode in devcontainer base images
ARG USERNAME=vscode
USER ${USERNAME}
FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

# Install system dependencies and GitHub CLI
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    git \
    wget \
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
    pkg-config \
    build-essential \
    curl \
    ca-certificates \
    udev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js LTS (18.x) for webui tooling
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - \
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

# Create vscode user if it doesn't exist
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

# Install Rust as vscode user
USER $USERNAME

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && . "$HOME/.cargo/env" \
    && rustup install nightly \
    && rustup default nightly \
    && rustup component add rust-src --toolchain nightly

# Add cargo to PATH
ENV PATH="/home/${USERNAME}/.cargo/bin:${PATH}"

# Install ESP-IDF
RUN mkdir -p /home/${USERNAME}/esp \
    && cd /home/${USERNAME}/esp \
    && git clone --recursive --depth 1 --branch v5.1.2 https://github.com/espressif/esp-idf.git \
    && cd esp-idf \
    && ./install.sh esp32c6 \
    && echo 'source $HOME/esp/esp-idf/export.sh' >> /home/${USERNAME}/.bashrc

# Install cargo tools for ESP32
RUN . "$HOME/.cargo/env" \
    && cargo install espflash espmonitor cargo-espflash ldproxy

# Set working directory
WORKDIR /workspace

# Keep container running
CMD ["/bin/bash"]
