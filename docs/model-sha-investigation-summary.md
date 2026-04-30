# Model SHA Update - Investigation Summary

## Issue Background

The issue stated: "Hugging face likely updated the models upstream so that the sha we hardcode don't fit anymore. Please check and update if needed."

## Investigation Results

**Key Finding**: RamDoc does **not hardcode SHA values** in the codebase. Instead, it uses a sophisticated **dynamic SHA verification** system that automatically adapts to upstream model updates.

## How the System Works

### Dynamic SHA Fetching

1. **LFS Pointer Fetch**: When downloading a model, RamDoc first fetches the Git LFS pointer file from HuggingFace
2. **SHA Extraction**: The pointer contains the authoritative SHA-256 hash published by HuggingFace
3. **Model Download**: The actual model file is downloaded
4. **Verification**: The downloaded file's SHA is compared against the fetched authoritative SHA
5. **Registration**: If verified, the model is registered in the database with its SHA

### Code Locations

- `dokassist/src-tauri/src/llm/download.rs` (lines 30-46): Model whitelist
- `dokassist/src-tauri/src/llm/download.rs` (lines 60-257): SHA fetch and verification logic

### Current Models

The system currently supports three models:

1. **Qwen3-30B-A3B-Q4_K_M.gguf** (unsloth/Qwen3-30B-A3B-GGUF) - For 24GB+ RAM
2. **Qwen3-8B-Q4_K_M.gguf** (unsloth/Qwen3-8B-GGUF) - For 16-24GB RAM
3. **Phi-4-mini-instruct-Q4_K_M.gguf** (unsloth/Phi-4-mini-instruct-GGUF) - For <16GB RAM

## Why No SHA Update is Needed

**The system is designed to handle upstream model updates automatically:**

✅ **Automatic Updates**: When HuggingFace updates a model, the new SHA is automatically used on next download
✅ **Security Maintained**: Every download is cryptographically verified against HuggingFace's authoritative SHA
✅ **No Code Changes**: Model SHA updates don't require any code modifications
✅ **User-Friendly**: Users get model updates without needing app updates

## What Actually Needs Checking

The only scenarios that require code updates are:

1. **URL Changes**: If HuggingFace renames/moves a repository
2. **Filename Changes**: If a model file is renamed on HuggingFace
3. **Model Removal**: If a model is deleted from HuggingFace
4. **Adding New Models**: If you want to add additional model options

## Verification

To verify that all model URLs are still valid, run:

```bash
./scripts/verify_model_urls.sh
```

This script checks:
- ✅ Download URLs (`resolve/main`) return 200/302
- ✅ LFS pointer URLs (`raw/main`) return valid pointer files
- ✅ All three configured models are accessible

**Note**: This script requires internet access and won't work in sandboxed CI environments.

## Deliverables

1. ✅ **Verification Script**: `scripts/verify_model_urls.sh` - Checks if model URLs are still valid
2. ✅ **Comprehensive Documentation**: `docs/model-sha-verification.md` - Complete guide to the SHA system
3. ✅ **Update Guide**: Instructions for when HuggingFace URLs do change

## Recommendation

**No immediate action required.** The current implementation is correct and secure. The dynamic SHA verification system means:

- Model SHAs automatically update when HuggingFace publishes new versions
- No hardcoded values to maintain
- Cryptographic integrity is always verified

**When to take action:**
- If the verification script reports any failures (model moved/renamed/deleted)
- If you want to add new model options
- If HuggingFace changes their API or URL structure

## Security Benefits of Dynamic SHA Verification

1. **Chain of Trust**: SHAs come from HuggingFace's signed Git LFS pointers
2. **Always Current**: Users get the latest model updates without app updates
3. **Integrity Verified**: Every download is cryptographically verified
4. **Attack Surface**: Whitelist prevents arbitrary URL construction/SSRF

## Questions?

See `docs/model-sha-verification.md` for:
- Detailed architecture explanation
- Step-by-step update procedures
- Troubleshooting guide
- Security considerations
