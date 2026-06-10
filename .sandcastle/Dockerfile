FROM node:22-trixie

# Install system dependencies
RUN apt-get update && apt-get install -y \
  git \
  curl \
  jq \
  && rm -rf /var/lib/apt/lists/*

# Build dependencies for the Rust/Tauri backend so agents can `cargo check`.
# Tauri v2 (webkit/gtk/appindicator/rsvg), native-tls (openssl), cpal/ALSA,
# enigo + rdev input injection (xdo), the whisper.cpp + ggml C/C++ build
# (build-essential/cmake/clang), and the Vulkan backend transcribe-rs enables
# on Linux (vulkan loader + glslc shader compiler).
RUN apt-get update && apt-get install -y \
  build-essential \
  cmake \
  clang \
  pkg-config \
  libssl-dev \
  libasound2-dev \
  libxdo-dev \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libvulkan-dev \
  glslc \
  mold \
  && rm -rf /var/lib/apt/lists/*

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
  | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
  && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
  | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
  && apt-get update && apt-get install -y gh \
  && rm -rf /var/lib/apt/lists/*

# Enable corepack so the `packageManager` field in package.json
# governs which pnpm version is shimmed onto PATH. Must run as root,
# before the USER switch below, since corepack writes shims to a
# system bin dir.
RUN corepack enable

# Build-args for UID/GID alignment: sandcastle docker build-image
# defaults these to the host user's UID/GID so image-built files
# and bind-mounted files share an owner without runtime chown.
ARG AGENT_UID=1000
ARG AGENT_GID=1000

# Rename the base image's "node" user to "agent" and align UID/GID.
# On macOS the host GID is often 20 (staff), which collides with Debian's
# pre-existing "dialout" group — delete any conflicting group/user first.
RUN if getent group "$AGENT_GID" >/dev/null && [ "$(getent group "$AGENT_GID" | cut -d: -f1)" != "node" ]; then \
      groupdel "$(getent group "$AGENT_GID" | cut -d: -f1)"; \
    fi \
 && if getent passwd "$AGENT_UID" >/dev/null && [ "$(getent passwd "$AGENT_UID" | cut -d: -f1)" != "node" ]; then \
      userdel -r "$(getent passwd "$AGENT_UID" | cut -d: -f1)" 2>/dev/null || true; \
    fi \
 && groupmod -g "$AGENT_GID" node \
 && usermod -u "$AGENT_UID" -g "$AGENT_GID" -d /home/agent -m -l agent node
USER ${AGENT_UID}:${AGENT_GID}

# Install Claude Code CLI
RUN curl -fsSL https://claude.ai/install.sh | bash

# Install the Rust toolchain so agents can build and check the Tauri backend.
RUN curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
  | sh -s -- -y --default-toolchain stable --profile minimal --component clippy

# Link with mold. The test binaries statically embed whisper.cpp + ggml-vulkan
# + onnxruntime, so the dozen integration-test links GNU ld runs concurrently
# during `cargo test` exhaust the ~8 GB Docker VM and OOM. mold links the same
# binaries with a fraction of the peak memory, so default parallelism fits.
RUN printf '[build]\nrustflags = ["-C", "link-arg=-fuse-ld=mold"]\n' \
  > /home/agent/.cargo/config.toml

# Add Claude and Cargo to PATH
ENV PATH="/home/agent/.cargo/bin:/home/agent/.local/bin:$PATH"

# whisper.cpp's ggml-cpu backend uses ARMv8.2 FP16 vector intrinsics
# (e.g. vfmaq_f16). Under Docker on Apple Silicon, ggml's `-mcpu=native`
# autodetection resolves to a baseline lacking +fp16, so GCC refuses to inline
# them. Pin a known-good arm64 arch (whisper-rs-sys forwards GGML_* to CMake)
# so `cargo check`/`cargo build` of the Tauri backend succeed in the sandbox.
ENV GGML_NATIVE=OFF \
    GGML_CPU_ARM_ARCH=armv8.2-a+fp16

# A wide vitest worker pool exhausts the Docker VM's memory and stalls with
# worker-startup timeouts; cap the pool so test runs keep producing output.
ENV VITEST_MAX_FORKS=2 \
    VITEST_MAX_THREADS=2

WORKDIR /home/agent

# In worktree sandbox mode, Sandcastle bind-mounts the git worktree at /home/agent/workspace
# and overrides the working directory to /home/agent/workspace at container start.
# Structure your Dockerfile so that /home/agent/workspace can serve as the project root.
ENTRYPOINT ["sleep", "infinity"]
