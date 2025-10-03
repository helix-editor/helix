#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Define the name for the Docker image
IMAGE_NAME="helix-builder"
# Define the directory *within the repo* where binaries will be placed
# This corresponds to CARGO_INSTALL_ROOT in the Dockerfile.
OUTPUT_SUBDIR="out" # This will be mapped to /src/helix/out in the container

# --- Build the Docker Image ---
echo "Building Docker image: ${IMAGE_NAME}..."
# The '.' at the end tells Docker to use the Dockerfile in the current directory
podman build -t "${IMAGE_NAME}" .

echo "Docker image built successfully."
echo ""

# --- Create Output Directory ---
# Create the output directory relative to the repo root if it doesn't exist
if [ ! -d "${OUTPUT_SUBDIR}" ]; then
  echo "Creating output directory: ${OUTPUT_DIR}..."
  mkdir -p "${OUTPUT_SUBDIR}"
fi

# --- Run the Container to Get Build Output ---
echo "Running container to copy build artifacts to ${OUTPUT_SUBDIR}..."
# -v "$(pwd)/${OUTPUT_SUBDIR}":/src/helix/out  maps your local OUTPUT_SUBDIR (relative to repo root)
#                                            to the container's /src/helix/out directory
# --rm                         cleans up the container after it finishes
# "${IMAGE_NAME}"              the name of the image to run
podman run --rm -v "$(pwd)/${OUTPUT_SUBDIR}":/src/helix/out "${IMAGE_NAME}"

echo ""
echo "Build process complete."
echo "Compiled binaries are in the '${OUTPUT_SUBDIR}' directory."

# Optional: You can try to make the binary executable or give its path
# For example, to print the path to the helix binary if it's there
if [ -f "${OUTPUT_SUBDIR}/helix" ]; then
  echo "You can now run Helix using: ./${OUTPUT_SUBDIR}/helix"
  # chmod +x "${OUTPUT_SUBDIR}/helix" # Uncomment to make it executable if needed
fi
