# noterm

A terminal-native Markdown note-taking app. Notes live as plain `.md` files on disk. Everything else — search, AI chat, git sync, kanban, and meeting imports — is built into the TUI.

```
┌──────────────────────────────────────────────────────────────────┐
│ NOTERM  ~/notes/                                    [git:main ●] │
├─────────────────┬────────────────────────────────────────────────┤
│ File Explorer   │ # Sprint Planning                              │
│                 │                                                 │
│ notes/          │ > **Call Note** · May 6, 2026  14:30 · 45m    │
│ ├ calls/        │                                                 │
│ │ └ sprint.md   │ ## Summary                                     │
│ ├ daily/        │                                                 │
│ │ └ 2026-05-07  │ Reviewed Q2 roadmap. Aligned on API-first     │
│ └ projects/     │ approach for the new data pipeline.            │
│                 │                                                 │
├─────────────────┴────────────────────────────────────────────────┤
│ [NORMAL] ~/notes/calls/sprint.md          git:main               │
└──────────────────────────────────────────────────────────────────┘
```

## Features

- **Editor** — ratatui-textarea powered Markdown editor with auto-save
- **Viewer** — styled Markdown rendering (headings, bold, code blocks, blockquotes)
- **File explorer** — recursive tree, create and delete notes
- **Full-text search** — Tantivy index, 150ms debounced, opens result directly
- **Semantic search** — Ollama embeddings stored in SQLite, cosine similarity
- **LLM chat** — streaming chat panel with context from the open note; supports Ollama, Claude, and OpenAI-compatible APIs
- **Kanban board** — tasks from note frontmatter grouped into Todo / In Progress / Done / Blocked columns; moving a card writes back to the file
- **Git integration** — status, log, stage all, commit, push, pull from inside the TUI
- **Import — inbox folder** — drop `.md` or `.txt` files into a watch folder; noterm picks them up automatically
- **Import — REST API** — local HTTP API for programmatic note creation
- **Import — Meetily** — import call transcripts and AI summaries directly from [Meetily](https://github.com/meetily/meetily)'s SQLite database

## Installation

### Download a pre-built binary

Grab the latest release for your platform from the [Releases page](https://github.com/cyberslacks/noterm/releases):

| File | Platform |
|------|----------|
| `noterm-linux-x86_64.tar.gz` | Linux (x86_64, fully static) |
| `noterm-macos-universal.tar.gz` | macOS (Apple Silicon + Intel universal) |
| `noterm-macos-arm64.tar.gz` | macOS Apple Silicon |
| `noterm-macos-x86_64.tar.gz` | macOS Intel |
| `noterm-windows-x86_64.exe.zip` | Windows (x86_64) |

```bash
# Linux example
tar -xzf noterm-linux-x86_64.tar.gz
chmod +x noterm-linux-x86_64
sudo mv noterm-linux-x86_64 /usr/local/bin/noterm
noterm
```

The Linux binary is fully statically linked — no system dependencies required.

### Build from source

Requires Rust 1.75+ (`rustup.rs`).

```bash
git clone https://github.com/cyberslacks/noterm.git
cd noterm
cargo build --release
./target/release/noterm
```

No system packages needed. `libgit2`, `libsqlite3`, and TLS are all compiled from source.

### Desktop application (Tauri)

The desktop application uses the same Rust services and configuration as the
terminal client. It provides multi-vault Markdown editing with source/preview
views and native configuration, MCP, and Fabric bridges.

```bash
cd desktop && npm install && cd ..
cargo run --manifest-path src-tauri/Cargo.toml
```

For a production bundle, run `cargo tauri build --manifest-path src-tauri/Cargo.toml`
after installing the Tauri CLI. The frontend production build is `npm run build`
from `desktop/`.

## Quick Start

noterm looks for notes in `~/notes/` by default. On first launch it creates the directory and a default config at `~/.config/noterm/config.toml`.

```bash
noterm
```

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate file tree |
| `Enter` | Open note |
| `n` | New note |
| `e` | Edit mode |
| `Esc` | Save and return to normal |
| `q` | Quit |

Press `?` at any time to see the full key binding reference.

## Key Bindings

### Normal mode

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down in file tree |
| `k` / `↑` | Move up in file tree |
| `Enter` | Open file / expand directory |
| `e` | Edit current note |
| `n` | New note (prompts for name) |
| `d` | Delete selected note (confirmation required) |
| `/` | Full-text search |
| `v` | Vector / semantic search |
| `c` | Toggle LLM chat panel |
| `K` | Kanban board |
| `G` | Git panel |
| `I` | Meetily import panel |
| `?` | Help / key binding reference |
| `q` / `Ctrl+q` | Quit |

### Editor

| Key | Action |
|-----|--------|
| `Esc` | Save and exit editor |
| `Ctrl+s` | Save without leaving editor |

### Chat panel

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Ctrl+l` | Clear chat history |
| `Esc` | Close panel |

### Kanban

| Key | Action |
|-----|--------|
| `h` / `l` | Move between columns |
| `j` / `k` | Navigate cards |
| `m` | Move card to next column |
| `Esc` | Return to normal |

### Git panel

| Key | Action |
|-----|--------|
| `s` | Stage all changes |
| `c` | Commit (prompts for message) |
| `p` | Push to remote |
| `P` | Pull from remote |
| `Tab` | Switch Status / Log tabs |
| `Esc` | Close panel |

## Configuration

`~/.config/noterm/config.toml` is created automatically on first run. All fields are optional — defaults are shown below.

```toml
[notes]
notes_dir = "~/notes"

[editor]
tab_width = 2
wrap_lines = true
auto_save = true

[git]
enabled = true
auto_commit = false
remote = "origin"

[search]
auto_index = true        # index notes in the background after saves
embed_on_save = true     # generate embeddings after saves (requires Ollama)

[ui]
file_tree_width_pct = 20
chat_width_pct = 35
show_hidden = false

[import]
watch_interval_secs = 5  # how often to poll the inbox folder
api_enabled = false       # enable local REST import API
api_port = 7373
api_host = "127.0.0.1"

[meetily]
# db_path = "/custom/path/to/meeting_minutes.sqlite"  # auto-detected if unset
import_folder = "calls"
tags = ["call", "meeting", "meetily"]
```

## LLM Setup

### Ollama (default, local)

```bash
ollama pull llama3.2          # chat model
olldown pull nomic-embed-text  # embeddings (for vector search)
ollama serve
```

```toml
[llm]
provider = "ollama"
ollama_base_url = "http://localhost:11434"
ollama_chat_model = "llama3.2"
ollama_embed_model = "nomic-embed-text"
```

### Claude

```toml
[llm]
provider = "claude"
claude_api_key = "sk-ant-..."
claude_model = "claude-sonnet-4-5"
```

### OpenAI / compatible

```toml
[llm]
provider = "openai"
openai_api_key = "sk-..."
openai_base_url = "https://api.openai.com/v1"
openai_model = "gpt-4o"
```

## Note Format

Notes are plain Markdown with YAML frontmatter:

```markdown
---
title: "Sprint Planning"
id: "550e8400-e29b-41d4-a716-446655440000"
created: "2026-05-06T14:00:00Z"
modified: "2026-05-06T14:32:00Z"
tags: ["work", "planning"]
tasks:
  - id: "task-001"
    title: "Design API endpoints"
    status: "todo"
    priority: 1
  - id: "task-002"
    title: "Write unit tests"
    status: "in_progress"
    priority: 2
---

# Sprint Planning

Note body here…
```

`status` values for Kanban: `todo` · `in_progress` · `done` · `blocked`

## Meetily Integration

noterm reads directly from Meetily's SQLite database and imports call notes with this structure:

```
## Summary
AI-generated summary of the call.

### Key Points
- Point one
- Point two

### Action Items
- [ ] Follow up with design team
- [ ] Schedule next review

## Notes
Notes you typed during the call (if any).

---

## Transcript
**[00:00]** **Mic** Welcome everyone, let's get started…
**[00:12]** **System** …
```

The database is auto-detected from standard Meetily install locations. Override with `db_path` in `[meetily]` config if needed.

## Import API

Enable in config (`api_enabled = true`), then:

```bash
# Health check
curl http://localhost:7373/api/health

# Create a note
curl -X POST http://localhost:7373/api/notes \
  -H "Content-Type: application/json" \
  -d '{"title": "My Note", "body": "Content here", "tags": ["api"]}'

# Import raw markdown
curl -X POST http://localhost:7373/api/import \
  -H "Content-Type: application/json" \
  -d '{"filename": "note.md", "content": "---\ntitle: \"Test\"\n---\n\n# Test"}'
```

## Git Sync

Point `notes_dir` at a git repository and noterm will show live status in the title bar and let you stage, commit, and push from the `G` panel. Useful for syncing notes across machines via GitHub/GitLab.

## License

See [LICENSE](LICENSE).
