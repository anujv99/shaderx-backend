FROM rust:latest AS builder

WORKDIR /usr/src/backend-app

# Copy your source code
COPY . .

# Build the Rust application
RUN cargo build --release

# Use the same base image as your final container
FROM rust:latest

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/backend-app/target/release/shaderx-backend /usr/local/bin/backend-app

# Clean up the target directory
RUN rm -rf /usr/src/backend-app/target

# Set the entry point
ENTRYPOINT ["backend-app"]
