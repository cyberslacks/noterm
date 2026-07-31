export interface Vault { id: string; name: string; path: string }
export interface FileNode { path: string; relative_path: string; name: string; is_dir: boolean; depth: number }
export interface Note { path: string; relative_path: string; title: string; body: string; raw: string }
export interface SearchResult { relative_path: string; title: string; snippet: string; score: number }
