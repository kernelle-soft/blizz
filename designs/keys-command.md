# Safe Key Management

Most Key Management solutions for MCPs kind of suck. They're either not actually secure (i.e., pasting the key in your mcps.json), or using a key management system that's annoying/intrusive to the user experience.

Ideally, key management for MCPs should:
- Keep your keys safe
  - Secrets stay local
  - Secrets stay ephemeral
  - Secrets use second-layer encryption
  - Second-layer encyption for them should only be decryptable on the system the MCP is running on.
- Just do its thing behind the scenes, just like running the MCP itself
- Ask for you to provide keys only when actually needed
- Authenticate ONCE for access to all keys using a non-annoying method (i.e., an environment variable to pass the authentication through to in your bash/zsh configs.)

## CLI API

```
keeper -h

Secrets management for Kernelle, the AI toolshed.

usage: KEEPER_AUTH="<your system password>" keeper <command>

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

usage: KEEPER_AUTH="<your system password>" keeper store [-g <group-name>] <secret-name> <secret>

Arguments:
  <secret-name>   The key/name to store the secret as. Good names use-bash-safe-naming patterns.
  <secret>        The actual secret to save. Once provided, this secret will be encrypted and stored under ~/.kernelle/secrets.json5

Options
  -g, --group <group-name>    The group to store the secret under. Good group names use-bash-safe-naming patterns.
  -h, --help                  Displays this help section
```

```
keeper read -h

Retrieves a secret

usage: KEEPER_AUTH="<your system password>" keeper read [-g <group-name>] <secret-name>

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

