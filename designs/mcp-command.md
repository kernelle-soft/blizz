# MCPs, All in One

Kernelle should be able to orchestrate MCPs for the user. The `mcp` subset of commands should manage spinning up, spinning down, and getting authorizations for mcps built into the system.

## CLI API

```
kernelle mcp -h

Manager for MCP servers

usage: kernelle mcp <command>

Commands:
  start         Run a particular MCP server
  stop          Stop running a particular MCP server
  ls, list      List available servers
  ps            Lists currently running servers

Options:
  -h, --help    Displays this help section
```

```
kernelle mcp start -h

Starts running an MCP

usage: kernelle mcp start <service>

Arguments:
  <service>     The name of the MCP server to run.

Options:
  -h, --help    Displays this help section
```

```
kernelle mcp stop -h

Stops a currently running MCP

usage: kernelle mcp stop <service>

Arguments:
  <service>     The name of the MCP server to stop.

Options:
  -h, --help    Displays this help section
```


```
kernelle mcp [ls, list] -h

Lists availabe MCP servers

usage: kernelle mcp [ls,, list]

Options:
  -h, --help    Displays this help section
```

```
kernelle mcp ps -h

Lists currently running servers

usage: kernelle mcp ps

output:
SERVER          TIME
<server-name>   Running since 8:02 PM
...

Options:
  -h, --help    Displays this help section
```

## Adding new MCPs

Adding MCPs should be as simple as adding a new bash script to the `mcp-services` folder in the project directory.

The pattern of the script should be:

```bash
#!/bin/env bash

# This will pause the user and ask for the token once, if not already stored.
# To make this CI/CD friendly, 
SECRET="$(keeper read <service-name> <secret-name>)"
if [[ -z "$SECRET" && -n "$YOUR_ACTUAL_SECRET" ]];then
  keeper store <service-name> <secret-name> "$YOUR_ACTUAL_SECRET"
fi

# Store your actual secret
YOUR_ACTUAL_SECRET="${YOUR_ACTUAL_SECRET:SECRET}"
if [[ -z "$YOUR_ACTUAL_SECRET" ]];then
  echo "No API key found, please supply an API to use this service"
  exit 1
fi

# Actually run your service. No need to send it to the background. Kernelle will handle this.
npx run <mcp-name>
```
