# The island's backend in a container.
#
#   docker build -f deploy/Dockerfile -t island .
#   docker run --rm -p 8080:8080 \
#     -e ISLAND_CONTROL_TOKEN=... -e ISLAND_READ_TOKEN=... \
#     -v island-data:/var/lib/island island
#
# The backend only. `island-ui` draws a window and does not belong in a
# container; the web dashboard is served by this same binary with
# --frontend-only, or exported to files with `island export`.

FROM rust:1.97.0-bookworm AS build
WORKDIR /src

# The manifests first, so a change to the source does not re-fetch the
# whole dependency tree.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
COPY apps apps
# `-p island` and not the workspace: `island_ui` is in the workspace, pulls
# in Bevy, and takes nine minutes to build for a window this image will
# never open. Naming the package builds only what the backend needs, and
# leaves the source tree the one the tests run against.
RUN cargo build --release -p island --locked

FROM debian:bookworm-slim
# curl is for the healthcheck below and nothing else. A healthcheck that
# only proved the binary exists would pass on a backend that had stopped
# answering, which is the one thing it is for.
RUN apt-get update \
 && apt-get install -y --no-install-recommends curl ca-certificates \
 && rm -rf /var/lib/apt/lists/*
RUN useradd --system --home /var/lib/island --create-home island
COPY --from=build /src/target/release/island /usr/local/bin/island
# The scenario and its canon: the island cannot bootstrap without them.
COPY fixtures/island /usr/local/share/island/fixtures/island
COPY docs/island/DEPLOYMENT.md /usr/share/doc/island/DEPLOYMENT.md

USER island
WORKDIR /var/lib/island
VOLUME /var/lib/island
EXPOSE 8080

# No token is baked in. Without ISLAND_CONTROL_TOKEN the backend refuses
# every write on this non-loopback bind, which is the right default for an
# image somebody might run without reading anything.
ENV ISLAND_BIND=0.0.0.0:8080

# A container that cannot answer is a container that should be replaced.
# `/api/health` needs no token, which is exactly why it exists -- a
# healthcheck that had to carry one would mean putting a secret in this
# file. The start period is generous because the full island takes a
# moment to bootstrap before it serves anything.
HEALTHCHECK --interval=30s --timeout=5s --start-period=180s --retries=3 \
  CMD curl -fsS "http://127.0.0.1:${ISLAND_BIND##*:}/api/health" || exit 1

ENTRYPOINT ["/bin/sh", "-c", "exec island serve \
  --scenario /usr/local/share/island/fixtures/island/default_scenario.json \
  --bind \"$ISLAND_BIND\" \
  --data-dir /var/lib/island/people \
  --snapshot-dir /var/lib/island/snapshots"]
