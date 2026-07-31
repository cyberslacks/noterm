# Getting Started with noterm

noterm is a cross-platform Markdown workspace. Your vault is a normal folder of
files; Noterm indexes it locally, while Kazam renders selected notes as a
knowledge base and Fabric helps create reviewed drafts.

---

## macOS

### Download

Go to the [Releases page](https://github.com/cyberslacks/Noterm/releases/latest) and download:

- **`noterm-macos-universal.tar.gz`** — recommended, works on both Apple Silicon (M1/M2/M3/M4) and Intel Macs

### Install

```bash
tar -xzf noterm-macos-universal.tar.gz
chmod +x noterm-macos-universal
sudo mv noterm-macos-universal /usr/local/bin/noterm
```

### Clear the quarantine flag

macOS will block an unsigned binary the first time you run it. Clear the flag before moving it:

```bash
xattr -dr com.apple.quarantine noterm-macos-universal
```

Or after moving it:

```bash
xattr -dr com.apple.quarantine /usr/local/bin/noterm
```

Alternatively, right-click the file in Finder → **Open** → **Open Anyway** the first time.

### Run

```bash
noterm
```

noterm creates `~/notes/` and `~/.config/noterm/config.toml` on first launch.

---

## Linux

### Download

Go to the [Releases page](https://github.com/cyberslacks/Noterm/releases/latest) and download:

- **`noterm-linux-x86_64.tar.gz`** — fully static binary, runs on any x86_64 Linux distro with no dependencies

### Install

```bash
tar -xzf noterm-linux-x86_64.tar.gz
chmod +x noterm-linux-x86_64
sudo mv noterm-linux-x86_64 /usr/local/bin/noterm
```

### Run

```bash
noterm
```

---

## Windows

### Download

Go to the [Releases page](https://github.com/cyberslacks/Noterm/releases/latest) and download:

- **`noterm-windows-x86_64.exe.zip`**

### Install

Extract the zip and place `noterm-windows-x86_64.exe` wherever you like (e.g. `C:\Tools\noterm.exe`). Add that folder to your `PATH` via **System Properties → Environment Variables**.

### Run

Open **Windows Terminal** (recommended) or PowerShell and run:

```powershell
noterm
```

> noterm is a TUI application — it requires a proper terminal emulator. The old `cmd.exe` works but Windows Terminal gives a much better experience.

---

## First Launch

On first run noterm will:

1. Create `~/notes/` (your default vault) and its standard folders.
2. Write a default config in the platform configuration directory (for example,
   `~/.config/noterm/config.toml` on Linux).

The TUI opens with a file tree on the left and a note viewer/editor on the right.

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate the file tree |
| `Enter` | Open a note |
| `n` | Create a new note |
| `N` | Create a new collection (physical directory) |
| `e` | Edit the open note |
| `d` | Delete the selected note or directory (confirmation required) |
| `Esc` | Save and return to Normal |
| `q` | Quit |
| `?` | Full key binding reference |

## Vault-first methodology

The vault is the durable system of record. Keep authored Markdown in this
portable structure; sync this folder, not a database.

```text
my-vault/
├── inbox/          # files waiting for automatic import
├── notes/          # general notes; Fabric drafts land in notes/inbox/
├── projects/       # project-specific notes
├── daily/          # daily notes
├── attachments/    # files referenced by notes
└── .noterm/        # per-vault search/vector state; exclude from sync
```

Noterm creates these folders automatically. Markdown and its YAML frontmatter
are authoritative. Each vault owns a separate `.noterm/fts_index/` full-text
index and `.noterm/vectors.sqlite` vector database, both local and rebuildable.
Search and semantic results never cross a vault boundary. Start new notes in
`notes/`, move polished material into `projects/` or `daily/`, and review
incoming material in `inbox/`.

To add a local vault for a subject, client, or personal workspace, create an
empty folder and add an absolute-path definition in desktop **Settings**. Every
entry receives the same layout and isolated indexes:

```toml
notes_dir = "/home/alex/Notes/personal"

[[vaults]]
id = "personal"
name = "Personal"
path = "/home/alex/Notes/personal"
```

In the TUI, press `S`, navigate to **Local vaults**, and press `Enter` to edit
the same definitions as JSON. `Esc` or `Ctrl+s` validates and saves the list:

```json
[
  { "id": "personal", "name": "Personal", "path": "/home/alex/Notes/personal" },
  { "id": "work", "name": "Work", "path": "/home/alex/Notes/work" }
]
```

Create a physical collection directory with `N` in the TUI, or **New
collection** in the desktop app. It is created under the selected folder in the
vault (or `notes/` when nothing is selected).

---

## LLM / AI features (optional)

The chat panel (`c`), vector search (`v`), and semantic indexing are all optional and only activate when an LLM provider is configured.

### Local — Ollama (no API key needed)

```bash
ollama pull llama3.2           # chat
ollama pull nomic-embed-text   # embeddings (for vector search)
ollama serve
```

```toml
# ~/.config/noterm/config.toml
[llm]
provider = "ollama"
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

### OpenAI

```toml
[llm]
provider = "openai"
openai_api_key = "sk-..."
openai_model = "gpt-4o"
```

---

## AI Summarizer (optional)

Press `X` (Shift+X) with a note open to generate an AI summary. The summary streams in a full-screen overlay and is automatically inserted into the note's `## Summary` section when complete.

Configure in Settings (`S` key) under **Summarizer**, or directly in `config.toml`:

```toml
[summarizer]
base_url = "http://localhost:3000/api"   # any OpenAI-compatible endpoint
model = "llama3.2"
api_key = ""                             # leave empty for local endpoints
```

---

## Freshness tracking (optional)

Track when notes need review by adding a `review_every` field to a note's YAML frontmatter:

```yaml
---
title: "Architecture decisions"
review_every: 30d        # Nd · Nw · Nm · Ny · monthly · quarterly · yearly
owner: jordan
expires: 2026-12-31      # optional hard expiry date
---
```

The status bar shows a live badge (`FRESH` / `DUE IN Nd` / `OVERDUE Nd` / `EXPIRED Nd`) for the open note. Press `F` (Shift+F) to open the freshness dashboard — a sorted list of all notes with staleness metadata, worst-first.

Default owner and cadence can be set in `config.toml`:

```toml
[freshness]
default_owner = "jordan"
default_review_every = "30d"
```

---

## Sidecar annotations (optional)

Add non-destructive annotations to any note without editing its content. They
are Kazam-compatible YAML sidecars stored in
`.kazam/annotations/<note-slug>/` within the vault.

Press `A` in Normal mode (with a note open) to open the annotations panel:

| Key | Action |
|-----|--------|
| `n` | Write a new annotation |
| `i` | Mark focused annotation as incorporated |
| `d` | Mark focused annotation as ignored |
| `j` / `k` | Navigate |
| `Esc` | Close panel |

The status bar shows `A:<n>` when the open note has pending annotations.

---

## Kazam knowledge base (optional)

[Kazam](https://github.com/tdiderich/kazam) is the viewer and publishing layer,
not a replacement for the Noterm vault. Publish reviewed notes from the vault
to a separate Kazam project; Kazam provides the rendered site, freshness checks,
annotations, and its MCP server.

```bash
cargo install kazam                    # or: brew install tdiderich/tap/kazam
kazam init "$HOME/Documents/noterm-kb"
cd "$HOME/Documents/noterm-kb"
kazam dev .                            # viewer at http://localhost:3000
```

Point Noterm at the project with an absolute path (TOML paths do not expand
`~`):

```toml
[kazam]
kb_path = "/home/alex/Documents/noterm-kb"
import_folder = "kazam"
binary_path = "kazam"
```

Use `E` in the TUI or **Publish** in the desktop app to write a
Kazam-compatible YAML page. Add `publish: true` to a note’s frontmatter to
republish automatically on save:

```yaml
---
title: "Deployment runbook"
publish: true
owner: "platform@example.com"
review_every: 30d
sources_of_truth:
  - label: "Production guide"
    href: "https://example.com/runbook"
---
```

Press `B` to browse/import existing Kazam YAML pages. Press `M` to connect to
the local `kazam mcp` server; Noterm starts it in `kb_path`. In Noterm chat,
`Tab` toggles Kazam page context. Kazam’s own `kazam mcp` command also exposes
the knowledge base to other MCP-capable tools.

## Fabric drafting (optional)

[Fabric](https://github.com/danielmiessler/fabric) runs reusable AI patterns
against a note. It never silently replaces vault content: run a pattern, review
the output, then append/replace the note or save it as a new `notes/inbox/`
draft in the desktop app.

```bash
go install github.com/danielmiessler/fabric/cmd/fabric@latest
fabric --setup
fabric --listpatterns
```

On macOS/Linux, Fabric’s official installer is also available; on Windows use
`winget install danielmiessler.Fabric`. Ensure `fabric` is on `PATH`, then set:

```toml
[integrations.fabric]
enabled = true
binary_path = "fabric"
```

Open a note and choose **Fabric** in the desktop app, select a pattern, and
review the result before applying it. Recommended flow: source material →
Fabric pattern → reviewed `notes/inbox/` draft → edited Markdown note →
optional Kazam publish.

---

## Sync the vault (optional)

Git is the preferred sync mechanism. Initialize the vault as a repository and
exclude `.noterm/`; the TUI’s `G` panel and desktop **Pull**/**Push** controls
operate on the configured remote and branch.

```bash
cd "$HOME/notes"
git init
printf '.noterm/\n' >> .gitignore
git remote add origin https://github.com/yourname/notes.git
git add . && git commit -m "Create notes vault"
```

```toml
[git]
remote = "origin"
branch = "main"
```

Google Drive and OneDrive work through an existing [rclone](https://rclone.org/)
remote. Configure and test it with `rclone config`, then select it in Noterm:

```toml
[sync]
provider = "google_drive" # or "one_drive"
rclone_remote = "gdrive:noterm"
```

Noterm calls `rclone sync` and excludes `.noterm/`. Choose one primary sync
method per vault to avoid concurrent Git and cloud conflict resolution.

---

## Change the notes directory

Edit the Noterm configuration file (or use the desktop **Settings** panel):

```toml
notes_dir = "/home/alex/Documents/notes"
```
