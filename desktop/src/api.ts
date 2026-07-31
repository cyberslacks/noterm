import { invoke } from "@tauri-apps/api/core";
import type { FileNode, Note, SearchResult, Vault } from "./types";

export const api = {
  vaults: () => invoke<Vault[]>("list_vaults"),
  files: (vaultId: string) => invoke<FileNode[]>("list_files", { vaultId }),
  note: (vaultId: string, path: string) => invoke<Note>("read_note", { vaultId, path }),
  save: (vaultId: string, path: string, body: string) => invoke<void>("save_note", { vaultId, path, body }),
  create: (vaultId: string, title: string) => invoke<Note>("create_note", { vaultId, title }),
  search: (vaultId: string, query: string) => invoke<SearchResult[]>("search_notes", { vaultId, query }),
  config: () => invoke<unknown>("get_config"),
  saveConfig: (config: unknown) => invoke<void>("save_config", { config }),
  fabricPatterns: () => invoke<string[]>("fabric_patterns"),
  fabricRun: (pattern: string, input: string) => invoke<string>("fabric_run", { pattern, input })
};
