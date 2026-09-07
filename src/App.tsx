import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  Activity,
  ArrowDown,
  ArrowUp,
  ArrowUpRight,
  Check,
  ChevronRight,
  Copy,
  Download,
  Film,
  Gamepad2,
  Headphones,
  Keyboard,
  LayoutGrid,
  LoaderCircle,
  Mic,
  Music2,
  Plus,
  Power,
  RotateCcw,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Target,
  Trash2,
  Upload,
  Volume2,
  X,
  AlertTriangle,
  FolderOpen,
  CheckCircle2,
} from "lucide-react";
import type { Filter, Profile, Settings, Snapshot } from "./types";
import { filterResponse, peak, response, validate } from "./eq";
import { webMcpAvailable } from "./webmcp";

const icons = {
  music: Music2,
  film: Film,
  gamepad: Gamepad2,
  target: Target,
  mic: Mic,
  power: Power,
  headphones: Headphones,
};
function Icon({ name, size = 20 }: { name: string; size?: number }) {
  const Component = icons[name as keyof typeof icons] ?? Headphones;
  return <Component size={size} />;
}
const dbFormat = (v: number) =>
  `${v > 0 ? "+" : ""}${Number.isFinite(v) ? Math.round(v * 100) / 100 : "—"}`;
let saveQueue: Promise<unknown> = Promise.resolve();
let flushEditor: () => Promise<void> = async () => {};
const saveProfile = (profile: Profile, expected: Profile) => {
  const next = saveQueue
    .catch(() => {})
    .then(() => invoke("save_profile", { profile, expected }));
  saveQueue = next.catch(() => {});
  return next;
};

function Curve({
  profile,
  selected,
  onSelect,
}: {
  profile: Profile;
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const [hover, setHover] = useState<{ hz: number; db: number } | null>(null);
  const width = 900,
    height = 218,
    left = 46,
    right = 18,
    top = 18,
    bottom = 30;
  const x = (hz: number) =>
      left + (Math.log10(hz / 20) / 3) * (width - left - right),
    y = (db: number) => top + ((18 - db) / 42) * (height - top - bottom);
  const curve = useMemo(
    () =>
      Array.from({ length: 360 }, (_, i) => {
        const hz = 20 * 1000 ** (i / 359);
        return [x(hz), y(Math.max(-24, Math.min(18, response(profile, hz))))];
      }),
    [profile],
  );
  const path = curve
    .map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(2)},${y.toFixed(2)}`)
    .join(" ");
  const selectedFilter = profile.filters.find((f) => f.id === selected);
  const individual = selectedFilter
    ? Array.from({ length: 240 }, (_, i) => {
        const hz = 20 * 1000 ** (i / 239);
        return `${i ? "L" : "M"}${x(hz)},${y(filterResponse(selectedFilter, hz))}`;
      }).join(" ")
    : "";
  return (
    <div className="curve-wrap">
      <svg
        role="img"
        aria-label="Estimated EQ response from 20 Hz to 20 kHz, including preamp"
        viewBox={`0 0 ${width} ${height}`}
        onMouseLeave={() => setHover(null)}
        onMouseMove={(e) => {
          const r = e.currentTarget.getBoundingClientRect(),
            px = ((e.clientX - r.left) / r.width) * width,
            hz = 20 * 10 ** (((px - left) / (width - left - right)) * 3);
          if (hz >= 20 && hz <= 20000)
            setHover({ hz, db: response(profile, hz) });
        }}
      >
        <defs>
          <linearGradient id="curveFill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#bdeb91" stopOpacity=".2" />
            <stop offset="100%" stopColor="#bdeb91" stopOpacity="0" />
          </linearGradient>
        </defs>
        {[-24, -12, 0, 12].map((db) => (
          <g key={db}>
            <line
              x1={left}
              x2={width - right}
              y1={y(db)}
              y2={y(db)}
              className={db === 0 ? "zero-line" : "grid-line"}
            />
            <text x={left - 10} y={y(db) + 4} textAnchor="end">
              {db > 0 ? "+" : ""}
              {db}
            </text>
          </g>
        ))}
        {[20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000].map((hz) => (
          <g key={hz}>
            <line
              x1={x(hz)}
              x2={x(hz)}
              y1={top}
              y2={height - bottom}
              className="grid-line"
            />
            <text x={x(hz)} y={height - 7} textAnchor="middle">
              {hz >= 1000 ? `${hz / 1000}k` : hz}
            </text>
          </g>
        ))}
        <path
          d={`${path} L${width - right},${height - bottom} L${left},${height - bottom} Z`}
          fill="url(#curveFill)"
        />
        {individual && (
          <path
            d={individual}
            fill="none"
            stroke="#9eaddb"
            strokeWidth="1.5"
            strokeDasharray="5 5"
          />
        )}
        <path d={path} fill="none" stroke="#bdeb91" strokeWidth="2.5" />
        {profile.filters
          .filter((f) => f.enabled && f.frequency >= 20 && f.frequency <= 20000)
          .map((f, i) => (
            <g
              key={f.id}
              role="button"
              aria-label={`Select filter ${i + 1}`}
              tabIndex={0}
              onClick={() => onSelect(f.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter") onSelect(f.id);
              }}
              className="curve-dot"
            >
              <circle
                cx={x(f.frequency)}
                cy={y(
                  Math.max(-24, Math.min(18, response(profile, f.frequency))),
                )}
                r={selected === f.id ? 7 : 4.5}
                fill={selected === f.id ? "#c6d2ff" : "#bdeb91"}
                stroke="#161b16"
                strokeWidth="2"
              />
            </g>
          ))}
        {hover && (
          <line
            x1={x(hover.hz)}
            x2={x(hover.hz)}
            y1={top}
            y2={height - bottom}
            stroke="#fff"
            opacity=".2"
          />
        )}
      </svg>
      <div className="curve-caption">
        <span>
          <i className="legend-line" />
          Combined response <span className="muted">· includes preamp</span>
        </span>
        <span>
          {hover
            ? `${Math.round(hover.hz)} Hz · ${dbFormat(hover.db)} dB`
            : "48 kHz estimate · GraphicEQ interpolated"}
        </span>
      </div>
    </div>
  );
}

function NumberField({
  value,
  onChange,
  min,
  max,
  step,
  label,
}: {
  value: number;
  onChange: (n: number) => void;
  min: number;
  max: number;
  step: number;
  label: string;
}) {
  return (
    <input
      aria-label={label}
      type="number"
      value={Number.isFinite(value) ? value : ""}
      min={min}
      max={max}
      step={step}
      onChange={(e) =>
        onChange(e.target.value === "" ? NaN : Number(e.target.value))
      }
    />
  );
}

function Editor({
  profile,
  headphones,
  onError,
  onDelete,
  onDuplicate,
  onExport,
}: {
  profile: Profile;
  headphones: Snapshot["database"]["headphones"];
  onError: (e: string) => void;
  onDelete: () => void;
  onDuplicate: () => void;
  onExport: () => void;
}) {
  const [draft, setDraft] = useState(() => structuredClone(profile));
  const [history, setHistory] = useState<Profile[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [status, setStatus] = useState("Saved");
  const saved = useRef(JSON.stringify(profile));
  const latest = useRef(draft);
  latest.current = draft;
  const version = useRef(0);
  useEffect(() => {
    const incoming = JSON.stringify(profile);
    if (incoming !== saved.current && JSON.stringify(latest.current) === saved.current) {
      saved.current = incoming;
      latest.current = structuredClone(profile);
      setDraft(latest.current);
      setHistory([]);
      setStatus("Saved");
    }
  }, [profile]);
  const invalid = validate(draft),
    stock = profile.id === "stock";
  useEffect(() => {
    flushEditor = async () => {
      const p = latest.current;
      if (p.id === "stock") return;
      const invalid = validate(p);
      if (invalid) throw new Error(invalid);
      if (JSON.stringify(p) !== saved.current) {
        await saveProfile(p, JSON.parse(saved.current));
        saved.current = JSON.stringify(p);
      }
      await saveQueue;
    };
    return () => {
      flushEditor = async () => {};
    };
  }, []);
  useEffect(() => {
    const text = JSON.stringify(draft);
    if (stock || text === saved.current) return;
    if (invalid) {
      setStatus("Check values");
      return;
    }
    setStatus("Saving…");
    const revision = ++version.current;
    const timer = setTimeout(() => {
      void saveProfile(draft, JSON.parse(saved.current))
        .then(() => {
          saved.current = text;
          if (version.current === revision) setStatus("Saved");
        })
        .catch((e) => {
          setStatus("Not saved");
          onError(String(e));
        });
    }, 350);
    return () => clearTimeout(timer);
  }, [draft, invalid, stock, onError]);
  useEffect(
    () => () => {
      const p = latest.current;
      if (
        p.id !== "stock" &&
        !validate(p) &&
        JSON.stringify(p) !== saved.current
      ) {
        void saveProfile(p, JSON.parse(saved.current)).catch((e) => onError(String(e)));
      }
    },
    [onError],
  );
  function edit(update: (p: Profile) => void) {
    setHistory((h) => [...h.slice(-39), structuredClone(draft)]);
    setDraft((p) => {
      const next = structuredClone(p);
      update(next);
      return next;
    });
  }
  const max = useMemo(() => (invalid ? 0 : peak(draft)), [draft, invalid]);
  const headroom = -(max + draft.preamp);
  return (
    <section className="editor panel">
      <div className="section-heading">
        <div className="heading-icon">
          <SlidersHorizontal size={19} />
        </div>
        <div>
          <h2>Shape your sound</h2>
          <p>
            {stock
              ? "A clean path. No filters, no preamp."
              : "Fine-tune your profile. Changes save automatically."}
          </p>
        </div>
        <span className={`save-status ${invalid ? "warning" : ""}`}>
          <span className="status-dot" />
          {stock ? "Bypass" : status}
        </span>
      </div>
      <div className="editor-meta">
        <label className="name-field">
          <span>PROFILE NAME</span>
          <input
            aria-label="Profile name"
            value={draft.name}
            disabled={stock}
            maxLength={80}
            onChange={(e) =>
              edit((p) => {
                p.name = e.target.value;
              })
            }
          />
        </label>
        <label>
          <span>HEADPHONES</span>
          <select
            aria-label="Profile headphones"
            value={draft.headphoneId}
            disabled={stock}
            onChange={(e) =>
              edit((p) => {
                p.headphoneId = e.target.value;
              })
            }
          >
            {headphones.map((h) => (
              <option key={h.id} value={h.id}>
                {h.name}
              </option>
            ))}
          </select>
        </label>
        <div className="editor-tools">
          <button
            className="icon-button"
            title="Undo edit"
            aria-label="Undo edit"
            disabled={!history.length}
            onClick={() => {
              const previous = history.at(-1)!;
              setHistory((h) => h.slice(0, -1));
              setDraft(previous);
            }}
          >
            <RotateCcw size={17} />
          </button>
          <button
            className="icon-button"
            title="Duplicate profile"
            aria-label="Duplicate profile"
            onClick={onDuplicate}
          >
            <Copy size={17} />
          </button>
          <button
            className="icon-button"
            title="Export profile"
            aria-label="Export profile"
            onClick={onExport}
          >
            <Download size={17} />
          </button>
          <button
            className="icon-button danger"
            title="Delete profile"
            aria-label="Delete profile"
            disabled={stock}
            onClick={onDelete}
          >
            <Trash2 size={17} />
          </button>
        </div>
      </div>
      {!invalid ? (
        <Curve profile={draft} selected={selected} onSelect={setSelected} />
      ) : (
        <div className="inline-error">
          {invalid} The last valid EQ remains applied.
        </div>
      )}
      <div className="preamp-bar">
        <div className="preamp-label">
          <Volume2 size={19} />
          <div>
            <strong>Preamp</strong>
            <small>Output headroom</small>
          </div>
        </div>
        <input
          aria-label="Preamp slider"
          type="range"
          min="-30"
          max="10"
          step="0.1"
          disabled={stock}
          value={Number.isFinite(draft.preamp) ? draft.preamp : 0}
          onChange={(e) =>
            edit((p) => {
              p.preamp = Number(e.target.value);
            })
          }
        />
        <div className="unit-input">
          <NumberField
            label="Preamp dB"
            value={draft.preamp}
            min={-60}
            max={20}
            step={0.1}
            onChange={(n) => {
              if (!stock)
                edit((p) => {
                  p.preamp = n;
                });
            }}
          />
          <span>dB</span>
        </div>
        <span className={`headroom ${headroom < -0.1 ? "warning" : ""}`}>
          {headroom >= -0.1 ? (
            <ShieldCheck size={15} />
          ) : (
            <AlertTriangle size={15} />
          )}{" "}
          {Number.isFinite(headroom)
            ? `${Math.abs(headroom).toFixed(1)} dB ${headroom < -0.1 ? "over peak" : "headroom"}`
            : "Check preamp"}
        </span>
        {headroom < -0.1 && !stock && (
          <button
            className="text-button"
            onClick={() =>
              edit((p) => {
                p.preamp = -Math.ceil(max * 10) / 10;
              })
            }
          >
            Adjust preamp
          </button>
        )}
      </div>
      {!stock && (
        <>
          <div className="filter-heading">
            <h3>
              Filters <span>{draft.filters.length}</span>
            </h3>
            <button
              className="text-button"
              disabled={draft.filters.length >= 64}
              onClick={() =>
                edit((p) => {
                  const id = crypto.randomUUID();
                  p.filters.push({
                    id,
                    kind: "PK",
                    enabled: true,
                    frequency: 1000,
                    gain: 0,
                    q: 1,
                  });
                  setSelected(id);
                })
              }
            >
              <Plus size={16} />
              Add filter
            </button>
          </div>
          {!!draft.filters.length && (
            <div className="filter-table">
              <div className="filter-row table-head">
                <span>ON</span>
                <span>#</span>
                <span>TYPE</span>
                <span>
                  FREQUENCY <em>Hz</em>
                </span>
                <span>
                  GAIN <em>dB</em>
                </span>
                <span>Q</span>
                <span />
              </div>
              {draft.filters.map((f, i) => (
                <div
                  key={f.id}
                  className={`filter-row ${selected === f.id ? "selected" : ""}`}
                  onFocus={() => setSelected(f.id)}
                >
                  <input
                    aria-label={`Enable filter ${i + 1}`}
                    type="checkbox"
                    checked={f.enabled}
                    onChange={(e) =>
                      edit((p) => {
                        p.filters[i].enabled = e.target.checked;
                      })
                    }
                  />
                  <span className="filter-number">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <select
                    aria-label={`Filter ${i + 1} type`}
                    value={f.kind}
                    onChange={(e) =>
                      edit((p) => {
                        p.filters[i].kind = e.target.value as Filter["kind"];
                      })
                    }
                  >
                    <option value="PK">Peaking</option>
                    <option value="LSC">Low shelf</option>
                    <option value="HSC">High shelf</option>
                    <option value="HPQ">High pass</option>
                    <option value="LPQ">Low pass</option>
                  </select>
                  <NumberField
                    label={`Filter ${i + 1} frequency`}
                    value={f.frequency}
                    min={10}
                    max={22000}
                    step={1}
                    onChange={(n) =>
                      edit((p) => {
                        p.filters[i].frequency = n;
                      })
                    }
                  />
                  {f.kind === "HPQ" || f.kind === "LPQ" ? (
                    <span className="muted">—</span>
                  ) : (
                    <NumberField
                      label={`Filter ${i + 1} gain`}
                      value={f.gain}
                      min={-30}
                      max={30}
                      step={0.1}
                      onChange={(n) =>
                        edit((p) => {
                          p.filters[i].gain = n;
                        })
                      }
                    />
                  )}
                  <NumberField
                    label={`Filter ${i + 1} Q`}
                    value={f.q}
                    min={0.1}
                    max={30}
                    step={0.05}
                    onChange={(n) =>
                      edit((p) => {
                        p.filters[i].q = n;
                      })
                    }
                  />
                  <button
                    className="icon-button"
                    aria-label={`Remove filter ${i + 1}`}
                    onClick={() =>
                      edit((p) => {
                        p.filters.splice(i, 1);
                      })
                    }
                  >
                    <X size={14} />
                  </button>
                </div>
              ))}
            </div>
          )}
          {!draft.filters.length && !draft.graphicEq.length && (
            <div className="empty-state">
              Start with a flat response. Add a filter to make it yours.
            </div>
          )}
          {!!draft.graphicEq.length && (
            <details className="graphic-points">
              <summary>
                {draft.graphicEq.length} GraphicEQ points · edit imported curve
              </summary>
              <div className="points-list">
                {draft.graphicEq.map((p, i) => (
                  <label key={i}>
                    {p.frequency} Hz
                    <NumberField
                      label={`GraphicEQ ${p.frequency} Hz gain`}
                      value={p.gain}
                      min={-60}
                      max={30}
                      step={0.1}
                      onChange={(n) =>
                        edit((p) => {
                          p.graphicEq[i].gain = n;
                        })
                      }
                    />
                  </label>
                ))}
              </div>
            </details>
          )}
        </>
      )}
      <div className="provenance">
        <ShieldCheck size={14} />
        <span>{draft.provenance}</span>
      </div>
    </section>
  );
}

function SettingsPage({
  state,
  run,
}: {
  state: Snapshot;
  run: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
}) {
  const [settings, setSettings] = useState<Settings>(() =>
    structuredClone(state.database.settings),
  );
  const [dirty, setDirty] = useState(false);
  useEffect(() => { if (!dirty) setSettings(structuredClone(state.database.settings)); }, [state.database.settings, dirty]);
  const change = (s: Settings) => {
    setSettings(s);
    setDirty(true);
  };
  const options: [keyof Settings, string, string][] = [
    ["startWithWindows", "Start with Windows", "Ready when you sign in."],
    [
      "startInTray",
      "Start quietly in the tray",
      "Keep the control center closed at Windows startup.",
    ],
    [
      "restoreLastProfile",
      "Restore the last active profile",
      "Reapply your saved choice when Ananda Control starts.",
    ],
    [
      "closeToTray",
      "Close the window to the tray",
      "Keep switching available after closing this window.",
    ],
    [
      "notifications",
      "Profile change notifications",
      "A small Windows notification when your sound changes.",
    ],
    ["agentControl", "Allow native agent control", "Let connected AI tools manage your profiles and EQ, even with this window closed."],
  ];
  return (
    <>
      <div className="page-heading">
        <div>
          <div className="eyebrow">MAKE IT YOURS</div>
          <h1>Small details. Better days.</h1>
          <p>Set it once. Let your sound follow your routine.</p>
        </div>
      </div>
      <section className="panel settings-panel">
        <h2>Background behavior</h2>
        <p>{webMcpAvailable() ? "WebMCP is available in this window. Native MCP also works from the tray." : "Native MCP works from the tray. This WebView2 version does not expose WebMCP."}</p>
        {options.map(([key, title, description]) => (
          <label className="setting-row" key={key}>
            <span>
              <strong>{title}</strong>
              <small>{description}</small>
            </span>
            <input
              className="switch-input"
              type="checkbox"
              checked={Boolean(settings[key])}
              onChange={(e) => change({ ...settings, [key]: e.target.checked })}
            />
          </label>
        ))}
      </section>
      <section className="panel settings-panel">
        <div className="section-heading">
          <Keyboard size={20} />
          <div>
            <h2>Global shortcuts</h2>
            <p>Available from any app. Leave a field empty to disable it.</p>
          </div>
        </div>
        {state.database.profiles.map((p) => (
          <label className="shortcut-row" key={p.id}>
            <span>
              <Icon name={p.icon} />
              {p.name}
            </span>
            <input
              aria-label={`${p.name} shortcut`}
              value={settings.shortcuts[p.id] ?? ""}
              placeholder="Disabled"
              onChange={(e) =>
                change({
                  ...settings,
                  shortcuts: { ...settings.shortcuts, [p.id]: e.target.value },
                })
              }
            />
          </label>
        ))}
        <label className="shortcut-row">
          <span>
            <LayoutGrid size={20} />
            Quick switcher
          </span>
          <input
            aria-label="Quick switcher shortcut"
            value={settings.quickShortcut}
            onChange={(e) =>
              change({ ...settings, quickShortcut: e.target.value })
            }
          />
        </label>
        {state.status.shortcutErrors.map((e) => (
          <div className="inline-error" key={e}>
            {e}
          </div>
        ))}
        <div className="settings-footer">
          <small>Use names such as Ctrl+Alt+1, Shift+F8, or Super+E.</small>
          <button
            className="primary"
            disabled={!dirty}
            onClick={() =>
              void run("update_settings", { settings })
                .then(() => setDirty(false))
                .catch(() => {})
            }
          >
            <Check size={17} />
            Save preferences
          </button>
        </div>
      </section>
    </>
  );
}

export default function App() {
  const [state, setState] = useState<Snapshot | null>(null),
    [page, setPage] = useState(
      new URLSearchParams(location.search).get("page") ?? "profiles",
    ),
    [selected, setSelected] = useState("music"),
    [message, setMessage] = useState(""),
    [busy, setBusy] = useState(false),
    [setup, setSetup] = useState(false),
    [deviceId, setDeviceId] = useState(""),
    [confirm, setConfirm] = useState<{
      title: string;
      body: string;
      action: () => Promise<unknown>;
    } | null>(null),
    [headphoneName, setHeadphoneName] = useState("");
  const quick = new URLSearchParams(location.search).has("quick");
  const [quickIndex, setQuickIndex] = useState(0);
  const onError = useCallback((e: string) => setMessage(e), []);
  const run = useCallback(
    async (command: string, args?: Record<string, unknown>) => {
      try {
        return await invoke(command, args);
      } catch (e) {
        setMessage(String(e));
        throw e;
      }
    },
    [],
  );
  useEffect(() => {
    if (!isTauri()) {
      setMessage(
        "Open the packaged Ananda Control app to connect to Equalizer APO. This browser page is only a frontend preview.",
      );
      return;
    }
    let stopped = false;
    const unlisteners: (() => void)[] = [];
    void invoke<Snapshot>("get_state")
      .then((s) => {
        if (stopped) return;
        setState(s);
        setSelected(s.database.activeProfile);
        setSetup(!s.database.integration.installed && !quick);
        setDeviceId(
          s.database.integration.deviceId ||
            s.devices.find(
              (d) =>
                d.connected &&
                d.apoRegistered &&
                d.name.toLowerCase().includes("k7"),
            )?.id ||
            s.devices.find(
              (d) => d.connected && d.name.toLowerCase().includes("k7"),
            )?.id ||
            s.devices.find((d) => d.connected)?.id ||
            "",
        );
      })
      .catch((e) => setMessage(String(e)));
    void listen<Snapshot>("state-changed", (e) => {
      if (!stopped) setState(e.payload);
    }).then((u) => (stopped ? u() : unlisteners.push(u)));
    void listen<string>("navigate", (e) => {
      void flushEditor()
        .then(() => setPage(e.payload))
        .catch((e) => setMessage(String(e)));
    }).then((u) => (stopped ? u() : unlisteners.push(u)));
    void listen("prepare-close", () => {
      void flushEditor()
        .then(() => invoke("close_main"))
        .catch((e) =>
          setMessage(`Fix the unfinished edit before closing: ${String(e)}`),
        );
    }).then((u) => (stopped ? u() : unlisteners.push(u)));
    return () => {
      stopped = true;
      unlisteners.forEach((u) => u());
    };
  }, [quick]);
  const db = state?.database;
  const profiles =
    db?.profiles.filter(
      (p) => p.headphoneId === db.selectedHeadphone || p.id === "stock",
    ) ?? [];
  const choose = useCallback(
    async (id: string) => {
      try {
        await flushEditor();
        await saveQueue;
        setSelected(id);
        if (state?.database.integration.installed) {
          await run("activate", { id });
          if (quick) await getCurrentWindow().close();
        }
      } catch (e) {
        setMessage(String(e));
        throw e;
      }
    },
    [state?.database.integration.installed, run, quick],
  );
  useEffect(() => {
    if (!quick) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") void getCurrentWindow().close();
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setQuickIndex((i) => Math.min(profiles.length - 1, i + 1));
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setQuickIndex((i) => Math.max(0, i - 1));
      }
      if (e.key === "Enter" && profiles[quickIndex])
        void choose(profiles[quickIndex].id).catch(() => {});
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [quick, profiles, quickIndex, choose]);
  const importFile = async () => {
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "EQ presets", extensions: ["txt", "json"] }],
      });
      if (path) {
        const id = await run("import_profile", { path });
        setSelected(String(id));
      }
    } catch (e) {
      setMessage(String(e));
    }
  };
  const exportFile = async () => {
    try {
      const path = await save({
        defaultPath: `${db?.profiles.find((p) => p.id === selected)?.name.replace(/[^a-z0-9 -]/gi, "") || "profile"}.txt`,
        filters: [
          { name: "Equalizer APO text", extensions: ["txt"] },
          { name: "Ananda profile", extensions: ["json"] },
        ],
      });
      if (path) await run("export_profile", { id: selected, path });
    } catch (e) {
      setMessage(String(e));
    }
  };
  if (!state)
    return (
      <div className="loading">
        <Headphones size={40} />
        <h1>Ananda Control</h1>
        <p>{message || "Connecting to your sound…"}</p>
        {!message && <LoaderCircle className="spin" />}
      </div>
    );
  const database = state.database,
    active = database.profiles.find((p) => p.id === database.activeProfile)!,
    profile = database.profiles.find((p) => p.id === selected) ?? active,
    device = state.devices.find((d) => d.id === database.integration.deviceId),
    eqOn = database.activeProfile !== "stock";
  if (quick)
    return (
      <div className="quick">
        <div className="quick-title">
          <Headphones size={24} />
          <strong>Ananda Control</strong>
          <button
            className="icon-button"
            aria-label="Close switcher"
            onClick={() => void getCurrentWindow().close()}
          >
            <X size={17} />
          </button>
        </div>
        <div className="eyebrow">A DIFFERENT MOMENT. A DIFFERENT SOUND.</div>
        {profiles.map((p, i) => (
          <button
            key={p.id}
            className={`quick-profile ${i === quickIndex ? "focused" : ""}`}
            onMouseEnter={() => setQuickIndex(i)}
            onClick={() => void choose(p.id).catch(() => {})}
          >
            <Icon name={p.icon} />
            <span>{p.name}</span>
            {p.id === active.id && <Check size={18} />}
          </button>
        ))}
        {message && <div className="inline-error">{message}</div>}
        <div className="quick-footer">
          ↑ ↓ to choose <span>↵ select · esc close</span>
        </div>
      </div>
    );
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-icon">
            <Headphones size={25} />
          </div>
          <div>
            Ananda<span>CONTROL</span>
          </div>
        </div>
        <div className="sidebar-label">YOUR LISTENING SPACE</div>
        <nav>
          {[
            ["profiles", "Sound profiles", LayoutGrid],
            ["headphones", "Headphones", Headphones],
            ["diagnostics", "Device & health", Activity],
            ["settings", "Settings", Settings2],
          ].map(([id, label, Component]) => {
            const C = Component as typeof LayoutGrid;
            return (
              <button
                key={String(id)}
                className={page === id ? "nav-item active" : "nav-item"}
                onClick={() =>
                  void flushEditor()
                    .then(() => setPage(String(id)))
                    .catch((e) => setMessage(String(e)))
                }
              >
                <C size={19} />
                {String(label)}
                {page === id && <span />}
              </button>
            );
          })}
        </nav>
        <div className="sidebar-bottom">
          <div className="device-mini">
            <span
              className={`status-dot ${device?.connected ? "" : "offline"}`}
            />
            <div>
              <strong>{device?.name ?? "Fosi Audio K7"}</strong>
              <small>
                {device?.connected
                  ? "Connected via USB"
                  : "Setup or connection needed"}
              </small>
            </div>
            <Headphones size={20} />
          </div>
          <div className="engine-note">
            Powered by Equalizer APO<span>Independent of Peace</span>
          </div>
          <div className="version">
            ANANDA CONTROL <span>v0.1.0</span>
          </div>
        </div>
      </aside>
      <div className="workspace">
        <header className="topbar">
          <div className="breadcrumb">
            Your sound <ChevronRight size={14} />
            <span>
              {page === "profiles"
                ? "Sound profiles"
                : page === "headphones"
                  ? "Headphones"
                  : page === "diagnostics"
                    ? "Device & health"
                    : "Settings"}
            </span>
          </div>
          <div className="topbar-right">
            <span className="native-badge">WINDOWS DESKTOP</span>
            <div className="avatar">AC</div>
          </div>
        </header>
        <main>
          {(message || state.status.error) && (
            <div role="alert" className="error-banner">
              <AlertTriangle size={18} />
              <span>{message || state.status.error}</span>
              <button
                aria-label="Dismiss error"
                className="icon-button"
                onClick={() => {
                  setMessage("");
                  void run("diagnostic_action", {
                    action: "clear-error",
                  }).catch(() => {});
                }}
              >
                <X size={17} />
              </button>
            </div>
          )}
          {database.integration.installed && !state.ownershipOk && (
            <div className="error-banner">
              <AlertTriangle size={18} />
              <span>
                APO configuration changed outside Ananda Control. Switching is
                paused to protect your settings.
              </span>
              <button
                className="text-button"
                onClick={() => setPage("diagnostics")}
              >
                Review
              </button>
            </div>
          )}
          {page === "profiles" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">
                    GOOD SOUND, WITHOUT THE FRICTION
                  </div>
                  <h1>Your sound. One click away.</h1>
                  <p>
                    A little more cinema. A little more focus. Always your
                    choice.
                  </p>
                </div>
                <button className="secondary" onClick={() => void importFile()}>
                  <Upload size={16} />
                  Import preset
                </button>
              </div>
              <section className="now-playing">
                <div className="now-content">
                  <div className="eyebrow">
                    <span className="status-dot" />
                    {database.integration.installed
                      ? "CURRENT SOUND"
                      : "READY FOR YOUR FIRST LISTEN"}
                  </div>
                  <h2>{active.name}</h2>
                  <p>
                    {database.integration.installed
                      ? `${device?.name ?? "Selected output"} · ${eqOn ? `${active.filters.filter((f) => f.enabled).length} active filters` : "Unprocessed output"}`
                      : "Your presets are ready. Connect once, then switch from anywhere."}
                  </p>
                  <div className="hero-tags">
                    <span>
                      <Headphones size={13} />
                      {
                        database.headphones.find(
                          (h) => h.id === active.headphoneId,
                        )?.name
                      }
                    </span>
                    <span>
                      <Keyboard size={13} />
                      {database.settings.shortcuts[active.id] ||
                        "Shortcut disabled"}
                    </span>
                  </div>
                </div>
                <div className="sound-art" aria-hidden="true">
                  {Array.from({ length: 27 }, (_, i) => (
                    <i
                      key={i}
                      style={{
                        height: `${18 + Math.sin(i * 0.39) ** 2 * 64 + Math.cos(i * 0.7) ** 2 * 20}px`,
                        opacity: 0.2 + Math.sin(i * 0.15) ** 2 * 0.6,
                      }}
                    />
                  ))}
                </div>
                <div className="eq-control">
                  {database.integration.installed ? (
                    <>
                      <button
                        aria-label={eqOn ? "Turn EQ off" : "Turn EQ on"}
                        className={`eq-toggle ${eqOn ? "on" : ""}`}
                        onClick={() => void run("toggle_eq").catch(() => {})}
                      >
                        <Power size={23} />
                      </button>
                      <strong>EQ {eqOn ? "ON" : "OFF"}</strong>
                      <span>
                        {eqOn ? "Your tuning is active" : "Stock sound"}
                      </span>
                    </>
                  ) : (
                    <button className="primary" onClick={() => setSetup(true)}>
                      Connect APO
                      <ArrowUpRight size={17} />
                    </button>
                  )}
                </div>
              </section>
              <div className="profiles-heading">
                <h2>
                  Made for the moment <span>{profiles.length} profiles</span>
                </h2>
                <button
                  className="text-button"
                  onClick={() =>
                    void run("profile_action", { action: "create", id: null })
                      .then((id) => setSelected(String(id)))
                      .catch(() => {})
                  }
                >
                  <Plus size={16} />
                  New profile
                </button>
              </div>
              <div className="profile-grid">
                {profiles.map((p) => (
                  <button
                    key={p.id}
                    className={`profile-card ${p.id === active.id ? "is-active" : ""} ${p.id === profile.id ? "is-selected" : ""}`}
                    onClick={() => void choose(p.id).catch(() => {})}
                  >
                    <div className="profile-card-top">
                      <span className={`profile-icon ${p.icon}`}>
                        <Icon name={p.icon} />
                      </span>
                      {p.id === active.id ? (
                        <span className="active-pill">
                          <Check size={11} />
                          ACTIVE
                        </span>
                      ) : (
                        <span className="shortcut-number">
                          {database.settings.shortcuts[p.id]
                            ?.split("+")
                            .at(-1) || "—"}
                        </span>
                      )}
                    </div>
                    <strong>{p.name}</strong>
                    <small>
                      {p.id === "stock"
                        ? "Pure, unprocessed sound"
                        : p.icon === "music"
                          ? "Detail. Balance. Everyday."
                          : p.icon === "film"
                            ? "A bigger cinematic feel."
                            : p.icon === "gamepad"
                              ? "Step into another world."
                              : p.icon === "target"
                                ? "Hear the small details."
                                : p.icon === "mic"
                                  ? "Clearer conversations."
                                  : "Your personal tuning."}
                    </small>
                    <div className="profile-card-bottom">
                      <span>{dbFormat(p.preamp)} dB</span>
                      <span>
                        {p.filters.length + p.graphicEq.length}{" "}
                        {p.graphicEq.length ? "points / filters" : "filters"}
                      </span>
                    </div>
                  </button>
                ))}
              </div>
              <Editor
                key={profile.id}
                profile={profile}
                headphones={database.headphones}
                onError={onError}
                onExport={() => void exportFile()}
                onDuplicate={() =>
                  void run("profile_action", {
                    action: "duplicate",
                    id: profile.id,
                  })
                    .then((id) => setSelected(String(id)))
                    .catch(() => {})
                }
                onDelete={() =>
                  setConfirm({
                    title: `Delete ${profile.name}?`,
                    body: "This removes this profile from Ananda Control. Your original imported file is preserved.",
                    action: async () => {
                      await run("profile_action", {
                        action: "delete",
                        id: profile.id,
                      });
                      setSelected(active.id);
                    },
                  })
                }
              />
              <div className="bottom-tip">
                <Keyboard size={16} />
                <span>Your next sound is a shortcut away.</span>
                <kbd>
                  {database.settings.quickShortcut || "Quick switcher disabled"}
                </kbd>
                <span className="muted">opens the quick switcher</span>
              </div>
            </>
          )}
          {page === "settings" && <SettingsPage state={state} run={run} />}
          {page === "headphones" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">YOUR COLLECTION</div>
                  <h1>A space for every pair.</h1>
                  <p>
                    Choose headphones manually when they share the same DAC.
                  </p>
                </div>
              </div>
              <section className="panel settings-panel">
                <h2>Headphones</h2>
                {database.headphones.map((h) => (
                  <div className="headphone-row" key={h.id}>
                    <Headphones size={28} />
                    <input
                      aria-label={`Name for ${h.name}`}
                      defaultValue={h.name}
                      onBlur={(e) => {
                        if (e.target.value !== h.name)
                          void run("headphone_action", {
                            action: "rename",
                            id: h.id,
                            name: e.target.value,
                          }).catch(() => {});
                      }}
                    />
                    <button
                      className={
                        h.id === database.selectedHeadphone
                          ? "text-button"
                          : "secondary"
                      }
                      onClick={() =>
                        void run("headphone_action", {
                          action: "select",
                          id: h.id,
                          name: "",
                        }).catch(() => {})
                      }
                    >
                      {h.id === database.selectedHeadphone ? (
                        <>
                          <Check size={15} />
                          Selected
                        </>
                      ) : (
                        "Select"
                      )}
                    </button>
                    <button
                      className="icon-button"
                      aria-label={`Delete ${h.name}`}
                      onClick={() =>
                        setConfirm({
                          title: "Remove headphone?",
                          body: "Only empty headphone entries can be removed. Move their profiles first.",
                          action: () =>
                            run("headphone_action", {
                              action: "delete",
                              id: h.id,
                              name: "",
                            }),
                        })
                      }
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                ))}
                <div className="add-headphone">
                  <input
                    aria-label="New headphone name"
                    placeholder="Name another pair of headphones"
                    value={headphoneName}
                    onChange={(e) => setHeadphoneName(e.target.value)}
                  />
                  <button
                    className="secondary"
                    disabled={!headphoneName.trim()}
                    onClick={() =>
                      void run("headphone_action", {
                        action: "create",
                        id: "",
                        name: headphoneName,
                      })
                        .then(() => setHeadphoneName(""))
                        .catch(() => {})
                    }
                  >
                    <Plus size={16} />
                    Add headphones
                  </button>
                </div>
              </section>
              <section className="panel settings-panel">
                <h2>Profile order</h2>
                <p>Changes also appear in the tray and quick switcher.</p>
                {database.profiles.map((p, i) => (
                  <div className="shortcut-row" key={p.id}>
                    <span>
                      <Icon name={p.icon} />
                      {p.name}
                    </span>
                    <div className="row-actions">
                      <button
                        className="icon-button"
                        disabled={i === 0}
                        aria-label={`Move ${p.name} up`}
                        onClick={() =>
                          void run("profile_action", {
                            action: "up",
                            id: p.id,
                          }).catch(() => {})
                        }
                      >
                        <ArrowUp size={16} />
                      </button>
                      <button
                        className="icon-button"
                        disabled={i === database.profiles.length - 1}
                        aria-label={`Move ${p.name} down`}
                        onClick={() =>
                          void run("profile_action", {
                            action: "down",
                            id: p.id,
                          }).catch(() => {})
                        }
                      >
                        <ArrowDown size={16} />
                      </button>
                    </div>
                  </div>
                ))}
              </section>
            </>
          )}
          {page === "diagnostics" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">A CLEAR VIEW OF YOUR AUDIO</div>
                  <h1>Everything in its place.</h1>
                  <p>
                    Connection, configuration, and audible processing are
                    separate checks.
                  </p>
                </div>
                <button
                  className="secondary"
                  onClick={() =>
                    void run("diagnostic_action", { action: "refresh" }).catch(
                      () => {},
                    )
                  }
                >
                  <RotateCcw size={16} />
                  Refresh
                </button>
              </div>
              <div className="health-grid">
                {[
                  [
                    state.apoInstalled,
                    "Equalizer APO installed",
                    "DSP engine found on this computer.",
                  ],
                  [
                    !!device?.connected,
                    "Output connected",
                    device?.name ?? "Choose your output during setup.",
                  ],
                  [
                    !!device?.apoRegistered,
                    "APO registered on output",
                    "Windows endpoint registration detected.",
                  ],
                  [
                    state.ownershipOk,
                    "Configuration owned",
                    "Root include and active file match the last commit.",
                  ],
                  [
                    database.integration.audioVerified,
                    "Audible test confirmed",
                    "Only marked after you confirm the listening test.",
                  ],
                  [
                    !device?.enhancementsDisabled,
                    "Audio enhancements",
                    "Check Windows sound settings if EQ is inaudible.",
                  ],
                ].map(([ok, title, desc]) => (
                  <div className="panel health-card" key={String(title)}>
                    {ok ? (
                      <CheckCircle2 className="accent" size={22} />
                    ) : (
                      <AlertTriangle className="warning" size={22} />
                    )}
                    <h3>{String(title)}</h3>
                    <p>{String(desc)}</p>
                    <small>{ok ? "Confirmed" : "Needs verification"}</small>
                  </div>
                ))}
              </div>
              <section className="panel settings-panel">
                <h2>Listening test</h2>
                <p>
                  Play music through the K7, close this window, then choose
                  Movies and Stock from the tray. Listen for the change, check
                  the active checkmark, and try a hotkey. No Peace window should
                  open.
                </p>
                <div className="button-row">
                  <button
                    className="secondary"
                    disabled={!state.ownershipOk}
                    onClick={() =>
                      void run("diagnostic_action", {
                        action: "verified",
                      }).catch(() => {})
                    }
                  >
                    <Check size={17} />I heard the EQ change
                  </button>
                  <button
                    className="text-button"
                    onClick={() =>
                      void run("diagnostic_action", {
                        action: "unverified",
                      }).catch(() => {})
                    }
                  >
                    Reset test status
                  </button>
                </div>
                <p className="muted">
                  A saved configuration does not prove audio is passing through
                  APO. Exclusive-mode or ASIO playback may bypass system
                  effects. Test ordinary shared-mode playback.
                </p>
              </section>
              <section className="panel settings-panel">
                <h2>Configuration & recovery</h2>
                <dl className="diagnostic-details">
                  <dt>APO configuration</dt>
                  <dd>{database.integration.configDir}</dd>
                  <dt>Profile database & backups</dt>
                  <dd>{state.dataDir}</dd>
                  <dt>Last configuration commit</dt>
                  <dd>
                    {state.status.lastCommitMs === null
                      ? "No timed switch this session"
                      : `${state.status.lastCommitMs.toFixed(1)} ms · file commit, not audible latency`}
                  </dd>
                </dl>
                <div className="button-row">
                  <button className="secondary" onClick={() => setSetup(true)}>
                    <SlidersHorizontal size={16} />
                    {database.integration.installed
                      ? "Review / repair integration"
                      : "Connect APO"}
                  </button>
                  <button
                    className="secondary"
                    onClick={() =>
                      void run("diagnostic_action", {
                        action: "device-selector",
                      }).catch(() => {})
                    }
                  >
                    <ArrowUpRight size={16} />
                    APO Device Selector
                  </button>
                  <button
                    className="secondary"
                    onClick={() =>
                      void run("diagnostic_action", {
                        action: "open-data",
                      }).catch(() => {})
                    }
                  >
                    <FolderOpen size={16} />
                    Open data folder
                  </button>
                </div>
                {database.integration.installed && (
                  <button
                    className="text-button danger restore-button"
                    onClick={() =>
                      setConfirm({
                        title: "Restore the previous APO configuration?",
                        body: "This restores the original backed-up root configuration and Peace startup entries, disables Ananda Windows startup, and disconnects Ananda Control. Your profiles are retained.",
                        action: () => run("restore_integration"),
                      })
                    }
                  >
                    <RotateCcw size={16} />
                    Restore previous configuration
                  </button>
                )}
              </section>
            </>
          )}
          <footer>
            Designed around your listening.{" "}
            <span>
              Equalizer APO does the processing. Ananda makes it yours.
            </span>
          </footer>
        </main>
      </div>
      {setup && (
        <div className="modal-backdrop">
          <section
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="setup-title"
          >
            <button
              aria-label="Close setup"
              className="modal-close icon-button"
              disabled={busy}
              onClick={() => setSetup(false)}
            >
              <X size={20} />
            </button>
            <div className="setup-icon">
              <Headphones size={30} />
            </div>
            <div className="eyebrow">ONE SETUP. EVERYDAY SIMPLICITY.</div>
            <h2 id="setup-title">Give your sound a new home.</h2>
            <p>
              Your six profiles are ready. Ananda Control will take over daily
              EQ switching directly through Equalizer APO.
            </p>
            <label className="device-select">
              YOUR WINDOWS OUTPUT
              <select
                aria-label="Output device"
                value={deviceId}
                onChange={(e) => setDeviceId(e.target.value)}
              >
                {state.devices
                  .filter(
                    (d) => d.connected || d.apoRegistered || d.id === deviceId,
                  )
                  .map((d) => (
                    <option key={d.id} value={d.id}>
                      {d.name}
                      {d.connected ? "" : " (disconnected)"}
                      {d.apoRegistered ? " · APO registered" : ""}
                    </option>
                  ))}
              </select>
            </label>
            <div className="setup-steps">
              <div>
                <span>01</span>
                <p>
                  <strong>Keep a way back</strong>Back up your current
                  configuration and startup entries.
                </p>
              </div>
              <div>
                <span>02</span>
                <p>
                  <strong>Let Peace rest</strong>Close Peace and disable its
                  identified startup entries. Keep its files.
                </p>
              </div>
              <div>
                <span>03</span>
                <p>
                  <strong>Make the connection</strong>Replace the current root
                  configuration with Ananda’s device-scoped include. Existing
                  root effects will no longer run.
                </p>
              </div>
            </div>
            <div className="setup-note">
              <ShieldCheck size={18} />
              <span>
                Your original preset files stay untouched. No Windows Audio
                restart.
              </span>
            </div>
            {message && <div className="inline-error">{message}</div>}
            {message && /access|denied|permission/i.test(message) && (
              <button
                className="secondary wide"
                onClick={() =>
                  void run("diagnostic_action", { action: "grant-access" })
                    .then(() =>
                      setMessage(
                        "Approve the Windows setup helper, then retry Connect. Only APO configuration write permissions are granted to your account.",
                      ),
                    )
                    .catch(() => {})
                }
              >
                Grant setup write access…
              </button>
            )}
            <button
              className="primary wide"
              disabled={busy || !deviceId}
              onClick={() => {
                setBusy(true);
                setMessage("");
                void run("setup_integration", {
                  deviceId,
                  repair: database.integration.installed,
                })
                  .then(() => setSetup(false))
                  .catch(() => {})
                  .finally(() => setBusy(false));
              }}
            >
              {busy ? (
                <LoaderCircle size={18} className="spin" />
              ) : (
                <Check size={18} />
              )}{" "}
              {busy
                ? "Backing up and connecting…"
                : database.integration.installed
                  ? "Back up and repair integration"
                  : "Back up & connect Equalizer APO"}
            </button>
            <button
              className="text-button centered"
              disabled={busy}
              onClick={() => setSetup(false)}
            >
              Explore the app first
            </button>
          </section>
        </div>
      )}
      {confirm && (
        <div className="modal-backdrop">
          <section className="modal compact" role="dialog" aria-modal="true">
            <h2>{confirm.title}</h2>
            <p>{confirm.body}</p>
            <div className="button-row">
              <button
                className="secondary"
                disabled={busy}
                onClick={() => setConfirm(null)}
              >
                Cancel
              </button>
              <button
                className="primary"
                disabled={busy}
                onClick={() => {
                  setBusy(true);
                  void confirm
                    .action()
                    .then(() => setConfirm(null))
                    .catch(() => {})
                    .finally(() => setBusy(false));
                }}
              >
                {busy ? "Working…" : "Confirm"}
              </button>
            </div>
            {message && <div className="inline-error">{message}</div>}
          </section>
        </div>
      )}
    </div>
  );
}
