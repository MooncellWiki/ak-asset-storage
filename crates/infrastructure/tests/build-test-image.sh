#!/bin/bash

# Build the test Docker image for testing docker_client.rs
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building test Docker image..."
docker build -f Dockerfile.test -t xwbx/docker-test:latest .

echo "Test image built successfully: docker-test:latest"
echo ""
echo "To run the Docker client tests:"
echo "  cargo test --test docker_client_test"
echo ""
echo "To skip Docker tests (if Docker is not available):"
echo "  SKIP_DOCKER_TESTS=1 cargo test --test docker_client_test"
