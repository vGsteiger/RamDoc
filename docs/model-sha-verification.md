# Model SHA Verification and Updates

## Overview

RamDoc uses a **dynamic SHA verification** approach for model integrity. This document explains how it works and how to update models when HuggingFace upstream changes occur.

## How SHA Verification Works

Unlike systems that hardcode SHA values in the codebase, RamDoc fetches SHA-256 hashes dynamically from HuggingFace's Git LFS pointer files at download time. This provides several benefits:

1. **Automatic Updates**: When HuggingFace updates a model, the new SHA is automatically used
2. **Security**: Downloads are verified against the authoritative SHA from HuggingFace
3. **No Code Changes**: Model SHA updates don't require code modifications

### The Process

1. **LFS Pointer Fetch**: When downloading a model, the system first fetches the LFS pointer file from the `raw/main` URL
2. **SHA Extraction**: The pointer file (≈130 bytes) contains the authoritative SHA-256 hash in the format:
   ```
   version https://git-lfs.github.com/spec/v1
   oid sha256:<64-char-hex>
   size <bytes>
   ```
3. **Model Download**: The actual model is downloaded from the `resolve/main` URL
4. **Verification**: The downloaded file's SHA-256 is computed and compared to the fetched SHA
5. **Storage**: If verified, the SHA is stored in the database for reference

### Code Locations

- **Download Logic**: `dokassist/src-tauri/src/llm/download.rs`
  - `MODELS` constant (lines 30-46): Whitelist of allowed models
  - `fetch_lfs_sha256()` (lines 60-83): Fetches SHA from LFS pointer
  - `download_model_with_progress()` (lines 134-257): Downloads and verifies

- **Model Registry**: `dokassist/src-tauri/src/migrations/011_model_registry.sql`
  - Database schema stores verified SHA-256 values

## When to Update Models

You need to update the model configuration when:

1. **URLs Change**: HuggingFace repo is renamed, moved, or deleted
2. **Model Names Change**: Filename on HuggingFace is different
3. **New Models Available**: You want to add additional model options
4. **Repository Migration**: Models move to different organizations

## How to Update Models

### Step 1: Verify Current URLs

Run the verification script to check if all model URLs are still valid:

```bash
./scripts/verify_model_urls.sh
```

This script checks both the download URLs (`resolve/main`) and LFS pointer URLs (`raw/main`) for all configured models.

### Step 2: Find New Model Information

If a model has moved or you want to add a new model, you need:

1. The HuggingFace repository URL (e.g., `unsloth/Qwen3-8B-GGUF`)
2. The exact filename (e.g., `Qwen3-8B-Q4_K_M.gguf`)
3. Verification that the file exists and has an LFS pointer

You can verify manually:
```bash
# Check if LFS pointer exists (should return ~130 bytes of text)
curl -L "https://huggingface.co/[org]/[repo]/raw/main/[filename]"

# Check if download URL works (should return 200 or 302)
curl -I -L "https://huggingface.co/[org]/[repo]/resolve/main/[filename]"
```

### Step 3: Update the MODELS Constant

Edit `dokassist/src-tauri/src/llm/download.rs`:

```rust
const MODELS: &[ModelEntry] = &[
    ModelEntry {
        filename: "your-model.gguf",
        download_url: "https://huggingface.co/org/repo/resolve/main/your-model.gguf",
        lfs_pointer_url: "https://huggingface.co/org/repo/raw/main/your-model.gguf",
    },
    // ... other models
];
```

**Important**: Both URLs must point to the same file - just with different endpoints:
- `resolve/main` - Downloads the actual LFS file
- `raw/main` - Returns the LFS pointer text file

### Step 4: Update Model Recommendations

If you're adding or removing models, update the RAM-based recommendations in `dokassist/src-tauri/src/llm/engine.rs` (lines 300-326):

```rust
pub fn recommended_model() -> ModelChoice {
    let ram = Self::total_ram();
    const GB: u64 = 1024 * 1024 * 1024;

    if ram >= 24 * GB {
        ModelChoice {
            name: "Display Name".to_string(),
            filename: "exact-filename.gguf".to_string(),
            size_bytes: 18 * GB,  // Approximate size
            reason: "24 GB+ RAM: Description".to_string(),
        }
    }
    // ... more RAM tiers
}
```

### Step 5: Update Tests

Update the test cases in `dokassist/src-tauri/src/llm/download.rs` (lines 266-293):

```rust
#[test]
fn test_model_url_known_filenames() {
    let cases = [
        ("your-model.gguf", "resolve/main/your-model.gguf"),
        // ... other models
    ];
    // ...
}
```

### Step 6: Run Tests

```bash
cd dokassist/src-tauri
cargo test llm::download
```

Verify that:
- `test_model_url_known_filenames` passes with your new URLs
- `test_model_url_unknown_filename` still rejects invalid filenames
- `test_parse_lfs_pointer_*` tests all pass

### Step 7: Test Download Manually

Build and run the app, then try downloading the new model through the UI to ensure:
1. The LFS pointer is fetched successfully
2. The model downloads with progress reporting
3. SHA-256 verification passes
4. The model can be loaded and used for inference

## Security Considerations

### Why This Design is Secure

1. **Whitelist-Only**: The `MODELS` constant acts as a whitelist. Only explicitly listed filenames can be downloaded.
2. **No URL Construction**: URLs are not constructed from user input - they're hardcoded in the binary.
3. **SHA Verification**: Every download is verified against the authoritative SHA from HuggingFace.
4. **LFS Integrity**: Git LFS pointers are signed by HuggingFace's Git repository, providing chain of trust.
5. **Size Limits**: Both LFS pointers (4KB max) and model files (60GB max) have size limits.

### Why NOT to Hardcode SHAs

Hardcoding SHA values in the source code would require:
1. **Manual Updates**: Every time HuggingFace updates a model, code changes would be needed
2. **Version Tracking**: Multiple versions of the same model would need separate entries
3. **Deployment Lag**: Users couldn't get model updates until a new app version shipped
4. **Verification Overhead**: Developers would need to manually verify and update SHAs

The dynamic approach allows HuggingFace to update models (bug fixes, optimizations) without requiring app updates, while still maintaining cryptographic verification of file integrity.

## Troubleshooting

### "Failed to fetch LFS pointer"

**Cause**: The `raw/main` URL is incorrect or the repository doesn't exist.

**Solution**:
1. Verify the repository exists on HuggingFace
2. Check that the filename exactly matches (case-sensitive)
3. Ensure the file is stored with Git LFS (not as a regular file)

### "SHA-256 mismatch"

**Cause**: The downloaded file doesn't match the SHA in the LFS pointer.

**Possible reasons**:
1. Download was corrupted (network issue) - retry download
2. File was modified on HuggingFace between LFS pointer fetch and download (very rare)
3. Man-in-the-middle attack (extremely rare with HTTPS)

**Solution**: The system automatically deletes the corrupted file. Retry the download.

### "Unknown model filename"

**Cause**: The filename is not in the `MODELS` whitelist.

**Solution**: Add the model to the `MODELS` constant as described above.

## Current Models (as of this documentation)

| Model Name | Filename | Repository | Recommended For |
|------------|----------|------------|-----------------|
| Qwen3-30B-A3B MoE | `Qwen3-30B-A3B-Q4_K_M.gguf` | unsloth/Qwen3-30B-A3B-GGUF | 24GB+ RAM |
| Qwen3-8B | `Qwen3-8B-Q4_K_M.gguf` | unsloth/Qwen3-8B-GGUF | 16-24GB RAM |
| Phi-4 Mini | `Phi-4-mini-instruct-Q4_K_M.gguf` | unsloth/Phi-4-mini-instruct-GGUF | <16GB RAM |

## References

- [Git LFS Pointer Format](https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md)
- [HuggingFace Hub API](https://huggingface.co/docs/hub/api)
- [unsloth GGUF Models](https://huggingface.co/unsloth)

## Questions?

If you're unsure about a model update, verify:
1. ✅ The model exists on HuggingFace
2. ✅ The file is stored with Git LFS (not regular Git)
3. ✅ Both `resolve/main` and `raw/main` URLs work
4. ✅ The LFS pointer contains a valid SHA-256 hash
5. ✅ The model size is reasonable (<60GB)
6. ✅ Tests pass after your changes
