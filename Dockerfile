# Multi-stage Docker build for Universal Podcast Subscription Rescue
# Produces a minimal, secure, non-root Alpine container image

# -------------------------------------------------------------------
# Stage 1: Build binary
# -------------------------------------------------------------------
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Cache dependency builds
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs && cargo build --release || true

# Copy real source code
COPY src ./src
RUN touch src/lib.rs src/main.rs && cargo build --release

# -------------------------------------------------------------------
# Stage 2: Minimal runtime image
# -------------------------------------------------------------------
FROM alpine:3.24

RUN apk add --no-cache ca-certificates tzdata && \
    addgroup -g 1000 rescue && \
    adduser -u 1000 -G rescue -s /bin/sh -D rescue

WORKDIR /app

COPY --from=builder /app/target/release/podcast-rescue /usr/local/bin/podcast-rescue

# Switch to unprivileged user
USER rescue

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD wget -q -O - http://127.0.0.1:3000/ || exit 1

ENTRYPOINT ["podcast-rescue"]
CMD ["--no-browser", "--host", "0.0.0.0", "--port", "3000"]
