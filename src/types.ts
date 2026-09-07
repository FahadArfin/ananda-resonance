export interface Filter {
  id: string;
  kind: "PK" | "LSC" | "HSC" | "HPQ" | "LPQ";
  enabled: boolean;
  frequency: number;
  gain: number;
  q: number;
}
export interface Point {
  frequency: number;
  gain: number;
}
export interface Profile {
  schemaVersion: number;
  id: string;
  name: string;
  icon: string;
  headphoneId: string;
  preamp: number;
  filters: Filter[];
  graphicEq: Point[];
  provenance: string;
}
export interface Settings {
  agentControl: boolean;
  startWithWindows: boolean;
  startInTray: boolean;
  restoreLastProfile: boolean;
  closeToTray: boolean;
  notifications: boolean;
  shortcuts: Record<string, string>;
  quickShortcut: string;
}
export interface Database {
  schemaVersion: number;
  profiles: Profile[];
  headphones: { id: string; name: string }[];
  selectedHeadphone: string;
  activeProfile: string;
  lastEnabled: string;
  settings: Settings;
  integration: {
    installed: boolean;
    configDir: string;
    deviceId: string;
    backupDir: string | null;
    expectedRoot: string;
    expectedActive: string;
    audioVerified: boolean;
  };
}
export interface Snapshot {
  database: Database;
  devices: {
    id: string;
    name: string;
    connected: boolean;
    apoRegistered: boolean;
    enhancementsDisabled: boolean;
  }[];
  ownershipOk: boolean;
  apoInstalled: boolean;
  dataDir: string;
  status: {
    error: string | null;
    shortcutErrors: string[];
    lastCommitMs: number | null;
  };
}
