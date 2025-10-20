#!/bin/bash

# Simple archive script for non-git projects
# Creates: ratacat-archive.zip

set -e

PROJECT_NAME="ratacat"
ARCHIVE_NAME="${PROJECT_NAME}-archive.zip"

echo "Creating archive of ${PROJECT_NAME}..."

# Remove old archive if it exists
[ -f "${ARCHIVE_NAME}" ] && rm "${ARCHIVE_NAME}"

# Create archive excluding common build artifacts and temporary files
zip -r "${ARCHIVE_NAME}" . \
    -x "target/*" \
    -x "*.zip" \
    -x ".DS_Store" \
    -x "Cargo.lock" \
    -x ".claude/*" \
    -x "__pycache__/*" \
    -x "*.pyc" \
    -x ".git/*" \
    -x ".gitignore"

# Display archive info
if [ -f "${ARCHIVE_NAME}" ]; then
    SIZE=$(du -h "${ARCHIVE_NAME}" | cut -f1)
    FILE_COUNT=$(unzip -l "${ARCHIVE_NAME}" | grep -E "^\s*[0-9]+" | wc -l | tr -d ' ')

    echo ""
    echo "✓ Archive created successfully!"
    echo "  File: ${ARCHIVE_NAME}"
    echo "  Size: ${SIZE}"
    echo "  Files: ${FILE_COUNT}"
    echo ""
    echo "Archive contents:"
    unzip -l "${ARCHIVE_NAME}"
else
    echo "Error: Failed to create archive"
    exit 1
fi