# PKG-0 — Project Scaffold

**Goal**: Empty Tauri 2 + Svelte 5 app that compiles, opens a window, and runs.

**Dependencies**: None

**Effort**: ~3h

## Outputs

```
dokassist/
├── src-tauri/
│   ├── Cargo.toml           # All dependencies declared (even if unused yet)
│   ├── tauri.conf.json       # App name, window config, permissions
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs           # Tauri entry with empty command registrations
│   │   ├── lib.rs            # Module declarations
│   │   ├── commands/mod.rs   # Empty command module
│   │   ├── models/mod.rs     # Empty models module
│   │   └── error.rs          # AppError enum with thiserror
│   ├── capabilities/
│   │   └── default.json      # Tauri capability permissions
│   └── icons/                # App icons (placeholder)
├── src/
│   ├── app.html
│   ├── app.css               # Tailwind v4 base
│   ├── lib/
│   │   ├── api.ts            # Empty typed invoke wrappers
│   │   ├── types.ts          # Shared TypeScript interfaces (empty stubs)
│   │   └── stores/
│   │       └── auth.ts       # Auth state store (stub)
│   └── routes/
│       ├── +layout.svelte    # Root layout with slot
│       └── +page.svelte      # "Hello DokAssist" placeholder
├── package.json
├── pnpm-lock.yaml
├── svelte.config.js
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
└── README.md
```

## Cargo.toml Dependencies

Declare all upfront, features gated:

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
uuid = { version = "1", features = ["v7"] }
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1", features = ["full"] }
log = "0.4"
env_logger = "0.11"

# PKG-1
security-framework = "3"
aes-gcm = "0.10"
ring = "0.17"
argon2 = "0.5"

# PKG-2
rusqlite = { version = "0.32", features = ["bundled-sqlcipher", "vtab"] }

# PKG-4 (model download)
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio-stream = "0.1"
llama_cpp = { version = "0.4", features = ["metal"] }

# PKG-3 / misc
pdf-extract = "0.7"
rand = "0.8"
base64 = "0.22"
hex = "0.4"
bip39 = "2"
zeroize = { version = "1", features = ["derive"] }
```

## tauri.conf.json Key Settings

```json
{
  "productName": "DokAssist",
  "identifier": "ch.dokassist.app",
  "app": {
    "windows": [
      {
        "title": "DokAssist",
        "width": 1280,
        "height": 800,
        "minWidth": 900,
        "minHeight": 600
      }
    ]
  }
}
```

## Error Type

`src-tauri/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Keychain error: {0}")]
    Keychain(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] std::io::Error),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Auth required")]
    AuthRequired,
    #[error("Invalid recovery passphrase")]
    InvalidRecoveryPhrase,
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}
```

## Acceptance Criteria

- [ ] `pnpm tauri dev` opens a native macOS window with "Hello DokAssist"
- [ ] `pnpm tauri build` produces a `.dmg` that installs and runs
- [ ] All Cargo dependencies compile without errors
- [ ] Svelte hot-reload works during development
- [ ] App icon shows in Dock
