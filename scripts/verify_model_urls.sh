#!/bin/bash
# Script to verify that all HuggingFace model URLs are still valid
# Run this script to check if models have been moved, renamed, or deleted upstream

set -e

echo "Verifying HuggingFace model URLs..."
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to check if a URL returns 200 OK
check_url() {
    local url=$1
    local name=$2

    echo -n "Checking $name... "

    # Use curl to get HTTP status code
    http_code=$(curl -L -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null || echo "000")

    if [ "$http_code" = "200" ]; then
        echo -e "${GREEN}✓ OK (HTTP $http_code)${NC}"
        return 0
    elif [ "$http_code" = "302" ] || [ "$http_code" = "301" ]; then
        echo -e "${YELLOW}⚠ Redirect (HTTP $http_code) - may be valid${NC}"
        return 0
    else
        echo -e "${RED}✗ FAILED (HTTP $http_code)${NC}"
        return 1
    fi
}

# Track if any checks failed
failed=0

echo "=== Checking Download URLs (resolve/main) ==="
echo ""

check_url "https://huggingface.co/unsloth/Qwen3-30B-A3B-GGUF/resolve/main/Qwen3-30B-A3B-Q4_K_M.gguf" \
    "Qwen3-30B-A3B-Q4_K_M.gguf (download)" || failed=1

check_url "https://huggingface.co/unsloth/Qwen3-8B-GGUF/resolve/main/Qwen3-8B-Q4_K_M.gguf" \
    "Qwen3-8B-Q4_K_M.gguf (download)" || failed=1

check_url "https://huggingface.co/unsloth/Phi-4-mini-instruct-GGUF/resolve/main/Phi-4-mini-instruct-Q4_K_M.gguf" \
    "Phi-4-mini-instruct-Q4_K_M.gguf (download)" || failed=1

echo ""
echo "=== Checking LFS Pointer URLs (raw/main) ==="
echo ""

check_url "https://huggingface.co/unsloth/Qwen3-30B-A3B-GGUF/raw/main/Qwen3-30B-A3B-Q4_K_M.gguf" \
    "Qwen3-30B-A3B-Q4_K_M.gguf (LFS pointer)" || failed=1

check_url "https://huggingface.co/unsloth/Qwen3-8B-GGUF/raw/main/Qwen3-8B-Q4_K_M.gguf" \
    "Qwen3-8B-Q4_K_M.gguf (LFS pointer)" || failed=1

check_url "https://huggingface.co/unsloth/Phi-4-mini-instruct-GGUF/raw/main/Phi-4-mini-instruct-Q4_K_M.gguf" \
    "Phi-4-mini-instruct-Q4_K_M.gguf (LFS pointer)" || failed=1

echo ""
echo "=== Summary ==="
echo ""

if [ $failed -eq 0 ]; then
    echo -e "${GREEN}✓ All model URLs are valid!${NC}"
    echo ""
    echo "The model URLs in dokassist/src-tauri/src/llm/download.rs are correct."
    echo "SHA-256 values are fetched dynamically from LFS pointers, so no hardcoded"
    echo "SHA updates are needed."
    exit 0
else
    echo -e "${RED}✗ Some model URLs failed validation!${NC}"
    echo ""
    echo "Action required:"
    echo "1. Check if models have been renamed or moved on HuggingFace"
    echo "2. Update the MODELS constant in dokassist/src-tauri/src/llm/download.rs"
    echo "3. Update tests in dokassist/src-tauri/src/llm/download.rs"
    echo "4. Update model recommendations in dokassist/src-tauri/src/llm/engine.rs"
    exit 1
fi
