# Todoist MCP

Integrate AI assistants with Todoist using the [Model Context Protocol](https://modelcontextprotocol.io/) (MCP), an open standard for secure access to your tasks and projects. Our hosted MCP server works with Claude, ChatGPT, Cursor, and VS Code.

- **Easy setup:** OAuth in a minute.  
- **Full access:** Read, create, and update your tasks & projects.  
- **Use cases:** Daily reviews, project planning, natural-language queries.

### Setup guide

**Primary URL (Streamable HTTP):**

`https://ai.todoist.net/mcp`

#### Claude
1. Open **Settings → Connectors → Add custom connector**.  
2. Enter `https://ai.todoist.net/mcp` and complete OAuth.

#### Cursor

Create `~/.cursor/mcp.json` (global) or `.cursor/mcp.json` (project):

```json
{
  "mcpServers": {
    "todoist": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "https://ai.todoist.net/mcp"]
    }
  }
}
```

Then enable the server in Cursor settings if prompted.

#### Claude Code (CLI)

```bash
claude mcp add --transport http todoist https://ai.todoist.net/mcp
```

#### Visual Studio Code

Command Palette → MCP: Add Server → Type HTTP and use:

```json
{
  "servers": {
    "todoist": {
      "type": "http",
      "url": "https://ai.todoist.net/mcp"
    }
  }
}
```


#### Other Clients

`npx -y mcp-remote https://ai.todoist.net/mcp`

### Tool response format

Every `tools/call` response from the Todoist MCP server includes the same data in three forms. This mirrors the standard MCP tool result shape, but it is worth calling out because different MCP clients expose different parts of it to the model or to the calling application.

```json
{
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"tasks\":[{\"id\":\"...\",\"content\":\"...\",\"priority\":4},...],\"totalCount\":2,\"hasMore\":false,\"appliedFilters\":{\"limit\":10,\"filter\":\"today\"}}"
      },
      {
        "type": "text",
        "text": "Tasks matching filter: today: 2 (limit 10).\nFilter: filter: today."
      }
    ],
    "structuredContent": {
      "tasks": [
        { "id": "...", "content": "...", "priority": 4 }
      ],
      "totalCount": 2,
      "hasMore": false,
      "appliedFilters": { "limit": 10, "filter": "today" }
    }
  }
}
```

#### Fields

- **Stringified JSON of `structuredContent`** — included as a `text` content item, for compatibility with clients that surface text content but do not yet expose `structuredContent`. For tools that only return text and structured data (the majority of Todoist tools) this is `content[0]`. For tools that also return non-text content items such as images or embedded resources (for example, `view-attachment`), those tool-authored items come first and the JSON `text` item follows them.
- **Human-readable summary** — always the **last** `text` content item in the `content[]` array. Intended for display to end users or for models that only consume text. Not intended to be parsed.
- **`structuredContent`** — the same payload as the JSON `text` item, already parsed as a JSON object. Available to clients implementing MCP spec revision `2025-03-26` or later.

The JSON `text` item and `structuredContent` always contain the same data. The trailing summary is derived from that data and is not intended to be parsed.

#### Which field should you consume?

- **Direct JSON-RPC callers:** prefer `structuredContent`. If your client targets an older MCP spec revision, parse the JSON `text` item — locate it by checking each `text` content item for a JSON payload rather than by hard-coded index.
- **Callers routing through an AI orchestration layer** (for example, the OpenAI Responses API, an agent framework, or another LLM client): check which content items that layer surfaces back to you. Some clients pass through all parts of the tool response; others expose only the first content item.

#### Client compatibility note

If two MCP clients appear to receive different data from the same Todoist tool call, the difference is almost always in how the client surfaces the response — not in what the server returns. The Todoist MCP server returns the same `content[]` array and `structuredContent` object to every caller.

For example:

- **Claude** and **Claude Code** process all content items and `structuredContent`, so the model has access to the full structured payload.
- The **OpenAI Responses API** surfaces only `content[0]` to the caller and strips `structuredContent`. For tools that return only text and structured data, `content[0]` is the stringified JSON payload and callers can parse it directly. For tools that also return non-text content items (for example, `view-attachment` returning an image), `content[0]` is the tool-authored item; callers needing the structured payload from those tools should access the server via direct JSON-RPC.

If you are debugging unexpected output, inspect the raw JSON-RPC response from `ai.todoist.net/mcp` before assuming a server-side difference.
