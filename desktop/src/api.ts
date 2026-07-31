import { invoke } from "@tauri-apps/api/core";
import type { FileNode, Note, SearchResult, Vault } from "./types";

export const api = {
  vaults: () => invoke<Vault[]>("list_vaults"),
  files: (vaultId: string) => invoke<FileNode[]>("list_files", { vaultId }),
  note: (vaultId: string, path: string) => invoke<Note>("read_note", { vaultId, path }),
  save: (vaultId: string, path: string, body: string) => invoke<void>("save_note", { vaultId, path, body }),
  publish: (vaultId: string, path: string) => invoke<void>("publish_note", { vaultId, path }),
  create: (vaultId: string, title: string) => invoke<Note>("create_note", { vaultId, title }),
  createCollection: (vaultId: string, name: string) => invoke<string>("create_collection", { vaultId, name }),
  saveInbox: (vaultId: string, title: string, body: string) => invoke<Note>("save_inbox_note", { vaultId, title, body }),
  search: (vaultId: string, query: string) => invoke<SearchResult[]>("search_notes", { vaultId, query }),
  config: () => invoke<unknown>("get_config"),
  saveConfig: (config: unknown) => invoke<void>("save_config", { config }),
  fabricPatterns: () => invoke<string[]>("fabric_patterns"),
  fabricRun: (pattern: string, input: string) => invoke<string>("fabric_run", { pattern, input }),
  sync: (vaultId: string, direction: "push" | "pull") => invoke<void>("sync_vault", { vaultId, direction })
};
