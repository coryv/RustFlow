# Stage 1: Build Frontend
FROM node:20-slim AS frontend-builder
WORKDIR /app/ui
COPY ui/package*.json ./
RUN npm ci
COPY ui/ .
RUN npm run build

# Stage 2: Build Backend
FROM rust:latest AS backend-builder
WORKDIR /app
COPY . .
# Copy built frontend assets to be embedded or served if needed during build (though we serve from dist folder at runtime)
# We need to build the release binary
RUN cargo build --release --bin server

# Stage 3: Runtime
FROM debian:bookworm-slim
WORKDIR /app
# Install necessary runtime dependencies (e.g. for SSL if needed, though rustls is often used)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy backend binary
COPY --from=backend-builder /app/target/release/server /app/server
# Copy frontend assets
COPY --from=frontend-builder /app/ui/dist /app/dist

# Expose port
EXPOSE 3000

# Run the server
CMD ["./server"]
