import { invoke } from "@tauri-apps/api/core";

export type Tint = "frost" | "sage" | "lilac" | "amber" | "rose" | "ink";
export type PinMode = "normal" | "top" | "desktop";
export type Language = "ko" | "en" | "zh" | "ja" | "fr" | "es";
export type Theme = "system" | "light" | "dark";
export interface TextFormat {
  from: number; to: number; bold?: boolean; italic?: boolean; size?: number; highlight?: string;
}
export interface NoteBackground { image: string; opacity: number }
export interface DesignSummary { id: string; title: string; tint: Tint; createdAt: number }
export interface NoteDesign extends DesignSummary { background: NoteBackground | null }
export interface NotesChanged { id: string | null; design: boolean }
export interface SelectionTools {
  sizeStep: number; highlightColor: string;
}
export type SyncKind = "off" | "folder" | "gdrive";
export type SyncPhase = "off" | "idle" | "syncing" | "error";

export interface Note {
  id: string;
  body: string;
  tint: Tint;
  opacity: number;
  formats: TextFormat[];
  background: NoteBackground | null;
  font?: NoteFont | null;
  createdAt: number;
  updatedAt: number;
  deletedAt: number | null;
  deviceId: string;
  conflictOf: string | null;
}

export interface NoteFont {
  family: string | null;
  size: number;
  weight: number;
}

export interface FontFamily {
  family: string;
  label: string;
}

export interface Settings {
  language: Language;
  theme: Theme;
  selectionTools: SelectionTools;
  noteFont: NoteFont;
  shortcutsEnabled: boolean;
  newNoteShortcut: string;
  sync: SyncKind;
  folderRoot: string | null;
  gdriveClientId: string;
  gdriveAccount: string | null;
}

export interface SyncStatus {
  kind: SyncKind;
  phase: SyncPhase;
  lastSyncAt: number | null;
  message: string | null;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  noteCount: number;
}

export const api = {
  getTransferPreferences: () => invoke<{ mcpProviders: string[] } | null>("get_transfer_preferences"),
  saveTransferPreferences: (preferences: { mcpProviders: string[] }) => invoke<void>("save_transfer_preferences", { preferences }),
  fitSettings: (height: number) => invoke<void>("fit_settings", { height }),
  editorReady: (requestId: string, error: string | null) => invoke<void>("editor_ready", { requestId, error }),
  exportArchive: (preferences: { mcpProviders: string[] }) => invoke<string>("export_archive", { preferences }),
  importArchive: (path: string, preferences: { mcpProviders: string[] }) => invoke<{ noteCount: number; backupPath: string; preferences: { mcpProviders: string[] }; warnings: string[] }>("import_archive", { path, preferences }),
  getNote: (id: string) => invoke<{ note: Note; pin: PinMode }>("get_note", { id }),
  saveNoteContent: (id: string, body: string, formats: TextFormat[]) => invoke<{ updatedAt: number }>("save_note_content", { id, body, formats }),
  designNoteSummaries: () => invoke<{ notes: DesignSummary[]; selected: string | null }>("design_note_summaries"),
  getNoteDesign: (id: string) => invoke<NoteDesign>("get_note_design", { id }),
  windowReady: () => invoke<void>("window_ready"),
  dragNote: (phase: "start" | "move" | "end") => invoke<boolean>("drag_note", { phase }),
  createNote: (near?: string) => invoke<Note>("create_note", { near: near ?? null }),
  updateNote: (id: string, patch: Partial<Pick<Note, "body" | "tint" | "opacity" | "formats" | "background" | "font">> & { resetFontSizes?: boolean }) =>
    invoke<Note>("update_note", { id, patch }),
  deleteNote: (id: string) => invoke<void>("delete_note", { id }),
  hideNote: (id: string) => invoke<void>("hide_note", { id }),
  setPinMode: (id: string, mode: PinMode) => invoke<void>("set_pin_mode", { id, mode }),
  openSettings: (id?: string) => invoke<void>("open_settings", { id: id ?? null }),
  designContext: () => invoke<{ notes: Note[]; selected: string | null }>("design_context"),
  getSettings: () => invoke<Settings>("get_settings"),
  listFonts: (language: Language, force = false) => invoke<FontFamily[]>("list_fonts", { language, force }),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  setShortcutRecording: (recording: boolean) => invoke<void>("set_shortcut_recording", { recording }),
  getSyncStatus: () => invoke<SyncStatus>("get_sync_status"),
  syncNow: () => invoke<void>("sync_now"),
  hasGdriveSecret: () => invoke<boolean>("has_gdrive_secret"),
  gdriveConnect: (clientId: string, clientSecret: string) =>
    invoke<string>("gdrive_connect", { clientId, clientSecret }),
  gdriveDisconnect: () => invoke<void>("gdrive_disconnect"),
  suggestFolder: () => invoke<string | null>("suggest_folder"),
  appInfo: () => invoke<AppInfo>("app_info"),
  openDataDir: () => invoke<void>("open_data_dir"),
};
