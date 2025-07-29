# Safe Key Management

Most Key Management solutions for MCPs kind of suck. They're either not actually secure (i.e., pasting the key in your mcps.json), or using a key management system that's annoying/intrusive to the user experience.

Ideally, key management for MCPs should:
- Keep your keys safe
  - Secrets stay local
  - Secrets stay ephemeral (no persistent in-memory cache)
  - Secrets use second-layer encryption
  - Second-layer encryption for them should only be decryptable on the system the MCP is running on
  - Device-specific key derivation prevents secrets from being accessible on different machines
- Just do its thing behind the scenes, just like running the MCP itself
- Ask for you to provide keys only when actually needed
- Authenticate ONCE for access to all keys using a non-annoying method (i.e., an environment variable to pass the authentication through to in your bash/zsh configs)
- Sessionless design: derived keys are stored like SSH keys for service restarts

## CLI API

```
keeper -h

Secrets management for Kernelle, the AI toolshed.

usage: keeper <command>

Commands:
  store         Stores a secret
  read          Retrieves a secret
  ls, list      List available secrets

Options:
  -h, --help    Display this help section
  --version     Displays the version of this tool
```

```
keeper store -h

Stores a secret

usage: keeper store [-g <group-name>] <secret-name> [<secret>]

Arguments:
  <secret-name>   The key/name to store the secret as. Good names use-bash-safe-naming patterns.
  <secret>        The actual secret to save. If not provided, will prompt securely.

Options
  -g, --group <group-name>    The group to store the secret under. Good group names use-bash-safe-naming patterns.
  -h, --help                  Displays this help section
```

```
keeper read -h

Retrieves a secret

usage: keeper read [-g <group-name>] <secret-name>

Arguments:
  <secret-name>   The key/name of the secret. Good names use-bash-safe-naming patterns.

Options:
  -g, --group <group-name>    The group the secret is stored under. Good group names use-bash-safe-naming patterns.
  -h, --help                  Displays this help section
```

```
keeper ls, keeper list

Lists available secrets in a POSIX friendly manner. eg:
GROUP          SECRET NAME
general        secret-1
general        secret-2
general        ...
general        secret-n
group-name     grouped-secret-1
group-name     grouped-secret-2
group-name     ...

Options:
  -g, --group   Filter the listing by group. If the group does not exist, no output will be provided.
  -h, --help    Displays this help section
```

## Security Architecture

### Key Derivation
- Master key is derived from:
  - User's system password (prompted once, then derived key cached)
  - Device-specific fingerprint (hardware UUID, OS identifiers - robust against minor upgrades)
- Uses strong KDF (Argon2 or PBKDF2) to combine these inputs
- Device fingerprint serves as an implicit salt (prevents rainbow table attacks across devices)
- Derived key is stored securely in `~/.kernelle/keeper.key` (like SSH private key)
- Key file has restricted permissions (600) and is reused for service restarts
- User prompted for password only when derived key doesn't exist or is invalid

### Encryption
- Secrets encrypted with AES-256-GCM at rest
- No persistent in-memory credential cache - secrets are ephemeral
- Encrypted blobs stored in `~/.kernelle/secrets.json`
- Each secret has its own unique nonce (required for AES-GCM security)
- No separate salt needed (device fingerprint provides uniqueness)

### Device Changes
- If device password changes, re-derive key and re-encrypt all secrets
- Migration tool available for device transfers or major hardware changes
- Device fingerprint designed to be stable across minor system updates

### Memory Security
- Secrets zeroed from memory immediately after use
- No long-lived storage of decrypted values
- Secure memory handling throughout

## Keeper Daemon

### Purpose
The `keeper-daemon` is a minimal background service with a single responsibility: hold the derived master key in memory to eliminate password prompts.

### Responsibilities
- **Key Storage Only**: Hold the derived master key in secure memory
- **Simple IPC**: Respond to "get master key" requests via Unix socket
- **Session Management**: Start on login, cleanup on logout
- **Memory Security**: Secure memory allocation and cleanup

### What the Daemon Does NOT Do
- **No secret encryption/decryption** - `keeper` CLI/library handles this
- **No secret storage** - `keeper` CLI/library reads/writes files directly  
- **No business logic** - just a secure key container

### Architecture
```
┌──────────────────┐                   ┌──────────────────┐
│   keeper CLI     │ ──get_key()──→    │  keeper-daemon   │
│                  │ ←─master_key───   │                  │
│ - Encrypt/decrypt│                   │ - Master Key     │
│ - File I/O       │                   │ - Unix Socket    │
│ - User prompts   │                   │ - Memory Mgmt    │
│ - Business logic │                   └──────────────────┘
└──────────────────┘                   
```

### IPC Protocol (Ultra Simple)
```
Client Request:  "GET_KEY\n"
Daemon Response: "<32-byte-key>" | "ERROR: <message>"
```

### Daemon Commands
```bash
# Start daemon (prompts for password once, derives and caches key)
keeper agent start

# Check if daemon is running and has valid key
keeper agent status

# Stop daemon (securely clears key from memory)
keeper agent stop

# Stop, then start the daemon again.
keeper agent restart
```

### Fallback Behavior
If daemon is not running, `keeper` CLI:
1. Tries to load key from `~/.kernelle/keeper.key`
2. If missing/invalid, prompts user for password
3. Derives key, uses it, then discards from memory
4. Optionally saves derived key to file for next time

This keeps the daemon **dead simple** - it's just a secure memory container for one piece of data.

## Integration Philosophy

### Service-Agnostic API
`keeper` provides a simple trait-based interface that handles all secret management complexity:

```rust
pub trait SecretProvider {
  fn get_secret(&self, group: &str, name: &str) -> Result<String>;
  fn store_secret(&self, group: &str, name: &str, value: &str) -> Result<()>;
}
```

### Transparent Secret Handling
Services and tools can request secrets without worrying about:
- **Existence checking** - `keeper` handles missing secrets automatically
- **User prompting** - If a secret doesn't exist, user is prompted seamlessly
- **Encryption/decryption** - All cryptographic operations are transparent
- **Storage management** - File paths, permissions, and persistence handled internally
- **Device compatibility** - Key derivation and device fingerprinting abstracted away

### Usage Pattern
```rust
// Service just asks for what it needs
let api_key = secret_provider.get_secret("github", "token")?;

// keeper handles:
// 1. Check if secret exists
// 2. If not, prompt: "GitHub token not found. Please enter your GitHub token:"
// 3. Encrypt and store the secret
// 4. Return the secret value
// 5. Zero the secret from memory
```

### Benefits
- **Zero configuration** - Services work out of the box
- **User-friendly** - Prompts appear only when actually needed
- **Secure by default** - All secrets encrypted and ephemeral
- **Sessionless** - Works across service restarts without re-authentication

### Testing Support
`keeper` provides a mock implementation for testing scenarios:

```rust
pub struct MockSecretProvider {
  secrets: HashMap<(String, String), String>,
}

impl MockSecretProvider {
  pub fn new() -> Self { /* ... */ }
  
  pub fn with_secret(mut self, group: &str, name: &str, value: &str) -> Self {
    self.secrets.insert((group.to_string(), name.to_string()), value.to_string());
    self
  }
}

impl SecretProvider for MockSecretProvider {
  fn get_secret(&self, group: &str, name: &str) -> Result<String> {
    // Returns pre-configured test secrets, no prompting
  }
  
  fn store_secret(&self, group: &str, name: &str, value: &str) -> Result<()> {
    // Stores in memory for test verification
  }
}
```

Usage in tests:
```rust
#[test]
fn test_service_functionality() {
  let mock_secrets = MockSecretProvider::new()
    .with_secret("github", "token", "test_token_123");
  
  let service = MyService::new(Box::new(mock_secrets));
  // Test service logic without real secrets or user prompts
}
```

