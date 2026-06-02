# Claude CLI & MCP (Model Context Protocol) — Complete Guide

## Table of Contents

1. [What is Claude CLI?](#what-is-claude-cli)
2. [What is MCP?](#what-is-mcp)
3. [Why is MCP needed?](#why-is-mcp-needed)
4. [How Claude CLI + MCP work together](#how-claude-cli--mcp-work-together)
5. [Building a Simple MCP Server](#building-a-simple-mcp-server)
6. [Connecting MCP to Claude CLI](#connecting-mcp-to-claude-cli)
7. [Testing your MCP server](#testing-your-mcp-server)
8. [Next Steps](#next-steps)

---

## What is Claude CLI?

**Claude CLI** (also called **Claude Code**) is the official command-line tool built by Anthropic. It lets you use Claude as a coding assistant directly from your terminal.

```bash
claude                    # open interactive session
claude "explain this"     # one-shot question
claude --help             # see all options
```

Claude CLI is not just a chat interface — it can:
- Read and edit files in your project
- Run terminal commands
- Search your codebase
- Connect to external tools via **MCP**

---

## What is MCP?

**MCP (Model Context Protocol)** is an open standard created by Anthropic that defines how AI models like Claude can connect to external tools, APIs, and data sources.

Think of it like **USB for AI tools** — one standard plug that works everywhere.

```
Without MCP:                    With MCP:
  Claude ←→ custom code           Claude ←→ MCP Protocol ←→ any tool
  Claude ←→ different code                                ←→ any database
  Claude ←→ yet another way                               ←→ any API
```

### MCP has three main pieces

| Piece | What it is | Example |
|-------|-----------|---------|
| **MCP Host** | The AI app that uses tools | Claude CLI, Claude Desktop |
| **MCP Client** | Built into the host, manages connections | Inside Claude CLI |
| **MCP Server** | A small program that exposes tools | Your custom tool server |

---

## Why is MCP needed?

| Problem without MCP | Solution with MCP |
|--------------------|------------------|
| Every AI app needs custom integrations | One standard protocol, works everywhere |
| Tools can't be shared between apps | Build once, use in any MCP-compatible host |
| Claude only knows what's in context | MCP gives Claude real-time access to live data |
| No way to give Claude custom capabilities | MCP lets you add any capability you want |

### Real examples of what MCP enables

```bash
# Claude can read your database
"Show me all users who signed up this week"

# Claude can call your internal APIs
"Get the latest deployment status from our CI server"

# Claude can read files from Google Drive
"Summarize the Q3 report from Drive"

# Claude can search your company Slack
"What did the team decide about the auth refactor?"
```

None of this is possible without MCP — Claude's knowledge is frozen at training time and it has no internet access by default.

---

## How Claude CLI + MCP work together

```
You (terminal)
     |
     ↓
Claude CLI (MCP Host)
     |
     ↓ MCP Protocol (JSON over stdio or HTTP)
     |
     ↓
MCP Server (your custom tool)
     |
     ↓
External world (database, API, filesystem, etc.)
```

### The flow for a single request

```
1. You ask: "What files changed today?"
2. Claude CLI sends the question to Claude model
3. Claude decides it needs a tool → calls list_files tool
4. Claude CLI forwards the tool call to your MCP server
5. MCP server runs the actual code (reads filesystem)
6. MCP server returns results to Claude CLI
7. Claude reads the result and answers you
```

---

## Building a Simple MCP Server

### Prerequisites

```bash
pip install mcp
```

### A minimal MCP server with one tool

**File: `server.py`**

```python
from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp import types
import asyncio
import datetime

# Create the MCP server
app = Server("my-tools")

# Register a tool — Claude will be able to call this
@app.list_tools()
async def list_tools() -> list[types.Tool]:
    return [
        types.Tool(
            name="get_current_time",
            description="Returns the current date and time",
            inputSchema={
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "Timezone name, e.g. UTC, US/Eastern"
                    }
                },
                "required": []
            }
        )
    ]

# Implement what happens when Claude calls the tool
@app.call_tool()
async def call_tool(name: str, arguments: dict) -> list[types.TextContent]:
    if name == "get_current_time":
        now = datetime.datetime.now()
        return [types.TextContent(
            type="text",
            text=f"Current time: {now.strftime('%Y-%m-%d %H:%M:%S')}"
        )]
    raise ValueError(f"Unknown tool: {name}")

# Run the server
async def main():
    async with stdio_server() as (read_stream, write_stream):
        await app.run(read_stream, write_stream, app.create_initialization_options())

if __name__ == "__main__":
    asyncio.run(main())
```

### Add a more useful tool — read a file

```python
@app.list_tools()
async def list_tools() -> list[types.Tool]:
    return [
        types.Tool(
            name="read_file",
            description="Read the contents of a file",
            inputSchema={
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }
        )
    ]

@app.call_tool()
async def call_tool(name: str, arguments: dict) -> list[types.TextContent]:
    if name == "read_file":
        path = arguments["path"]
        with open(path, "r") as f:
            content = f.read()
        return [types.TextContent(type="text", text=content)]
    raise ValueError(f"Unknown tool: {name}")
```

---

## Connecting MCP to Claude CLI

### Step 1: Find your Claude CLI config file

```bash
# On Mac/Linux
~/.claude/settings.json

# Or project-level config
.claude/settings.json
```

### Step 2: Register your MCP server

Edit `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "my-tools": {
      "command": "python",
      "args": ["/absolute/path/to/server.py"]
    }
  }
}
```

### Step 3: Restart Claude CLI and verify

```bash
claude
```

Inside the Claude CLI session:

```
/mcp           # lists all connected MCP servers
```

You should see `my-tools` listed as connected.

### Step 4: Use your tool

```
You: What time is it right now?
Claude: [calls get_current_time tool] → It is 2026-05-30 14:32:10
```

---

## Testing your MCP server

### Manual test (without Claude CLI)

You can test the server directly by sending MCP protocol messages:

```bash
python server.py
```

Then paste this JSON input and press Enter:

```json
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
```

Expected response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [
      {
        "name": "get_current_time",
        "description": "Returns the current date and time",
        ...
      }
    ]
  }
}
```

### Automated test

**File: `test_server.py`**

```python
import subprocess
import json

def send_request(proc, request):
    """Send a JSON-RPC request and read the response."""
    line = json.dumps(request) + "\n"
    proc.stdin.write(line)
    proc.stdin.flush()
    response = proc.stdout.readline()
    return json.loads(response)

def test_list_tools():
    proc = subprocess.Popen(
        ["python", "server.py"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True
    )

    # Initialize
    send_request(proc, {
        "jsonrpc": "2.0", "id": 1,
        "method": "initialize",
        "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test"}}
    })

    # List tools
    response = send_request(proc, {
        "jsonrpc": "2.0", "id": 2,
        "method": "tools/list", "params": {}
    })

    tools = response["result"]["tools"]
    assert any(t["name"] == "get_current_time" for t in tools)
    print("test_list_tools passed")
    proc.terminate()

if __name__ == "__main__":
    test_list_tools()
```

```bash
python test_server.py
# test_list_tools passed
```

---

## Project Structure

```
my-mcp-server/
├── server.py           # MCP server entry point
├── tools/
│   ├── __init__.py
│   ├── filesystem.py   # file-related tools
│   ├── database.py     # database query tools
│   └── api.py          # external API tools
├── tests/
│   └── test_server.py
└── requirements.txt    # mcp, plus any other deps
```

---

## Next Steps

| Topic | What to explore |
|-------|----------------|
| **Multiple tools** | Register many tools in one server |
| **Resources** | Expose data (not just actions) via MCP resources |
| **HTTP transport** | Run MCP server over HTTP instead of stdio |
| **Official MCP servers** | `github.com/modelcontextprotocol/servers` — ready-made servers for GitHub, Slack, Postgres, etc. |
| **MCP Inspector** | GUI tool to test and debug MCP servers visually |

### Install a ready-made MCP server

```bash
# Example: GitHub MCP server
claude mcp add github -- npx -y @modelcontextprotocol/server-github

# Example: filesystem MCP server
claude mcp add filesystem -- npx -y @modelcontextprotocol/server-filesystem /your/path
```

### Key takeaway

```
MCP Server = a small program that exposes tools via a standard protocol
Claude CLI = the host that calls those tools on your behalf
You        = just ask Claude in plain English; it picks the right tool
```
