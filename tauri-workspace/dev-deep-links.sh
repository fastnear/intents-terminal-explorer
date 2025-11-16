#!/usr/bin/env bash
#
# dev-deep-links.sh - Helper script for deep link development on macOS
#
# This script solves the common issue where `cargo tauri dev` runs old code
# because macOS Launch Services caches URL scheme registrations.
#
# Usage:
#   ./dev-deep-links.sh              # Build and register, no test
#   ./dev-deep-links.sh test         # Build, register, and test with sample URL
#   ./dev-deep-links.sh clean        # Just clean old registrations
#   ./dev-deep-links.sh --help       # Show this help
#
# What it does:
#   1. Kills any running instances of the app
#   2. Builds the web frontend (dist-dom) if needed
#   3. Builds a debug bundle (faster than release, includes symbols)
#   4. Clears macOS Launch Services cache
#   5. Registers the fresh debug bundle
#   6. Optionally tests with a deep link
#
# When to use:
#   - Testing deep link handling (nearx:// URLs)
#   - After changing CFBundleURLTypes in Info.plist
#   - When deep links open wrong app version
#
# When NOT to use:
#   - General UI development → use `cargo tauri dev` instead
#   - Building for release → use `cargo tauri build --release`
#

set -e  # Exit on error

# Check if running on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "Error: This script only works on macOS"
    echo "Deep link registration uses macOS Launch Services"
    exit 1
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BUNDLE_ID="com.fastnear.nearx"
APP_NAME="NEARx"
BINARY_NAME="nearx-tauri"
DEBUG_BUNDLE_PATH="target/debug/bundle/macos/${APP_NAME}.app"
SAMPLE_DEEP_LINK="nearx://v1/tx/ABC123"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FRONTEND_DIST="$PROJECT_ROOT/dist-dom"

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  NEARx Deep Link Development Helper${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Parse command
MODE="${1:-build}"

# Show help if requested
if [[ "$MODE" == "--help" || "$MODE" == "-h" || "$MODE" == "help" ]]; then
    echo "Usage: $0 [MODE]"
    echo ""
    echo "Modes:"
    echo "  (none)  - Build debug binary, register bundle, no test"
    echo "  test    - Build, register, and open sample deep link"
    echo "  clean   - Clear old registrations without building"
    echo "  --help  - Show this help message"
    echo ""
    echo "What it does:"
    echo "  1. Kills any running instances of NEARx"
    echo "  2. Builds web frontend (dist-dom) if needed"
    echo "  3. Builds a debug .app bundle with symbols"
    echo "  4. Clears macOS Launch Services cache"
    echo "  5. Registers the fresh debug bundle"
    echo "  6. Optionally tests with nearx://v1/tx/ABC123"
    echo ""
    echo "When to use:"
    echo "  • Testing deep link handling (nearx:// URLs)"
    echo "  • After changing CFBundleURLTypes in Info.plist"
    echo "  • When deep links open wrong app version"
    echo ""
    echo "When NOT to use:"
    echo "  • General UI development → use 'cargo tauri dev' instead"
    echo "  • Building for release → use 'cargo tauri build --release'"
    echo ""
    echo "Examples:"
    echo "  $0              # Build and register"
    echo "  $0 test         # Build, register, and test"
    echo "  $0 clean        # Just clear old registrations"
    echo ""
    exit 0
fi

if [[ "$MODE" == "clean" ]]; then
    echo -e "${YELLOW}Cleaning mode: Only clearing registrations${NC}"
    echo ""
fi

# Step 1: Kill running instances
echo -e "${YELLOW}[1/6] Killing running instances...${NC}"
if killall "$BINARY_NAME" 2>/dev/null; then
    echo -e "${GREEN}✓ Killed running instance of ${BINARY_NAME}${NC}"
else
    echo "  No running instances found"
fi

if killall "$APP_NAME" 2>/dev/null; then
    echo -e "${GREEN}✓ Killed running instance of ${APP_NAME}${NC}"
else
    echo "  No running instances found"
fi

sleep 1  # Give processes time to exit
echo ""

# Step 2: Find and list all registered app locations
echo -e "${YELLOW}[2/6] Finding registered app locations...${NC}"
REGISTERED_PATHS=$(mdfind "kMDItemCFBundleIdentifier == '${BUNDLE_ID}'" 2>/dev/null || echo "")

if [[ -n "$REGISTERED_PATHS" ]]; then
    echo -e "${BLUE}Found registered apps:${NC}"
    echo "$REGISTERED_PATHS" | while read -r path; do
        echo "  $path"
    done
else
    echo "  No registered apps found"
fi
echo ""

# Step 3: Clear Launch Services cache
echo -e "${YELLOW}[3/6] Clearing Launch Services cache...${NC}"
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
    -kill -r -domain local -domain system -domain user

echo -e "${GREEN}✓ Launch Services cache cleared${NC}"
sleep 2  # Give macOS time to rebuild cache
echo ""

if [[ "$MODE" == "clean" ]]; then
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}✓ Cleanup complete${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    exit 0
fi

# Step 4: Build web frontend
echo -e "${YELLOW}[4/6] Building web frontend...${NC}"

if [[ ! -d "$FRONTEND_DIST" ]]; then
    echo -e "${BLUE}Frontend not built yet, running trunk build...${NC}"
    echo ""

    cd "$PROJECT_ROOT"

    # Check if trunk is installed
    if ! command -v trunk &> /dev/null; then
        echo -e "${RED}Error: trunk is not installed${NC}"
        echo ""
        echo "Install trunk with:"
        echo "  cargo install --locked trunk"
        echo ""
        exit 1
    fi

    if trunk build --config Trunk-dom.toml; then
        echo ""
        echo -e "${GREEN}✓ Frontend built successfully${NC}"
    else
        echo ""
        echo -e "${RED}Frontend build failed${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}✓ Frontend already built at $FRONTEND_DIST${NC}"
    echo -e "${BLUE}  (Run 'rm -rf dist-dom' and re-run to rebuild)${NC}"
fi
echo ""

# Step 5: Build debug binary and create bundle
echo -e "${YELLOW}[5/6] Building debug binary...${NC}"
echo -e "${BLUE}Running: cargo build${NC}"
echo ""

cd "$(dirname "$0")/src-tauri"  # Ensure we're in src-tauri directory

if cargo build; then
    echo ""
    echo -e "${GREEN}✓ Debug binary built successfully${NC}"
else
    echo ""
    echo -e "${RED}Build failed${NC}"
    exit 1
fi
echo ""

# Step 5.5: Create .app bundle structure manually
echo -e "${YELLOW}[5.5/6] Creating .app bundle structure...${NC}"

cd ..  # Back to tauri-workspace directory

# Remove old debug bundle if exists
if [[ -d "$DEBUG_BUNDLE_PATH" ]]; then
    echo "  Removing old bundle..."
    rm -rf "$DEBUG_BUNDLE_PATH"
fi

# Create bundle directory structure
BUNDLE_CONTENTS="$DEBUG_BUNDLE_PATH/Contents"
BUNDLE_MACOS="$BUNDLE_CONTENTS/MacOS"
BUNDLE_RESOURCES="$BUNDLE_CONTENTS/Resources"

mkdir -p "$BUNDLE_MACOS"
mkdir -p "$BUNDLE_RESOURCES"

# Copy binary (from workspace target/debug)
echo "  Copying binary..."
cp "target/debug/$BINARY_NAME" "$BUNDLE_MACOS/"

# Create Info.plist
echo "  Creating Info.plist..."
cat > "$BUNDLE_CONTENTS/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleExecutable</key>
    <string>${BINARY_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.4.0-dev</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleURLTypes</key>
    <array>
        <dict>
            <key>CFBundleURLName</key>
            <string>NEARx</string>
            <key>CFBundleURLSchemes</key>
            <array>
                <string>nearx</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
EOF

echo -e "${GREEN}✓ Debug bundle created${NC}"
echo -e "${BLUE}Bundle location:${NC}"
echo "  $DEBUG_BUNDLE_PATH"
echo ""

# Step 6: Register the bundle
echo -e "${YELLOW}[6/6] Registering bundle with Launch Services...${NC}"
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
    -f "$DEBUG_BUNDLE_PATH"

echo -e "${GREEN}✓ Bundle registered${NC}"
echo ""

# Verify registration
echo -e "${BLUE}Verifying registration...${NC}"
sleep 1
if mdfind "kMDItemCFBundleIdentifier == '${BUNDLE_ID}'" 2>/dev/null | grep -q "$DEBUG_BUNDLE_PATH"; then
    echo -e "${GREEN}✓ Registration verified${NC}"
else
    echo -e "${YELLOW}Registration not yet visible (may take a few seconds)${NC}"
fi
echo ""

# Step 7: Optional deep link test
if [[ "$MODE" == "test" ]]; then
    echo -e "${YELLOW}[TEST] Opening sample deep link...${NC}"
    echo -e "${BLUE}URL: ${SAMPLE_DEEP_LINK}${NC}"
    echo ""

    sleep 1  # Give registration time to propagate
    open "$SAMPLE_DEEP_LINK"

    echo -e "${GREEN}✓ Deep link sent to macOS${NC}"
    echo -e "${BLUE}  Check the app logs for deep link processing${NC}"
    echo ""
fi

# Summary
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Setup complete${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo ""
echo -e "  ${YELLOW}Run the app:${NC}"
echo "    open \"$DEBUG_BUNDLE_PATH\""
echo ""
echo -e "  ${YELLOW}Test a deep link:${NC}"
echo "    open 'nearx://v1/tx/ABC123'"
echo ""
echo -e "  ${YELLOW}Or run both at once:${NC}"
echo "    ./dev-deep-links.sh test"
echo ""
echo -e "${BLUE}Notes:${NC}"
echo "  • Debug bundle includes debug symbols and logging"
echo "  • Frontend is built from dist-dom (use 'trunk build --config Trunk-dom.toml' to rebuild)"
echo "  • Use 'cargo tauri dev' for general UI development (no deep links)"
echo "  • Use this script for deep link testing"
echo "  • Run './dev-deep-links.sh clean' to remove old registrations"
echo ""
