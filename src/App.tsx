import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import logo from "./assets/logo.png";

interface Device {
  name: string;
  host_api: string;
  default?: boolean;
}

interface OutputConfig {
  name: string;
  volume: number; // 0-1
  muted: boolean;
}

interface AppConfig {
  input_volume: number;
  input_muted: boolean;
  outputs: OutputConfig[];
}

function App() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDeviceName, setSelectedDeviceName] = useState("");
  const [activeOutputs, setActiveOutputs] = useState<OutputConfig[]>([]);
  const [inputVolume, setInputVolume] = useState(100);
  const [inputMuted, setInputMuted] = useState(false);
  const [capturePaused, setCapturePaused] = useState(false);
  const [sourceName, setSourceName] = useState("Loading...");
  const [status, setStatus] = useState("Ready");

  // Prevent initial save overwriting logic
  const isLoaded = useRef(false);

  useEffect(() => {
    // 1. Start Audio Engine
    invoke("start_audio")
      .then(() => setStatus("Active"))
      .catch((e) => setStatus("Error: " + e));

    // 2. Fetch Devices
    const fetchDevices = async () => {
      const d = await invoke("get_audio_devices") as Device[];
      const currentSource = await invoke("get_default_audio_device") as string;
      setSourceName(currentSource);

      const available = d.filter(device => device.name !== currentSource);
      setDevices(available);
      if (available.length > 0) setSelectedDeviceName(available[0].name);
    }
    fetchDevices();

    // 3. Load Config
    async function loadConfig() {
      try {
        const config = await invoke("load_app_config") as AppConfig;
        console.log("Loaded Config:", config);

        // Restore Input Stats
        setInputVolume(Math.round(config.input_volume * 100));
        setInputMuted(config.input_muted);
        invoke("set_input_volume", { volume: config.input_volume });
        invoke("set_input_mute", { muted: config.input_muted });

        // Restore Outputs
        // We need to re-add them to the audio engine one by one
        for (const out of config.outputs) {
          try {
            await invoke("add_device_to_mix", { deviceName: out.name });
            await invoke("set_device_volume", { deviceName: out.name, volume: out.volume });
            await invoke("set_device_mute", { deviceName: out.name, muted: out.muted });
          } catch (e) {
            console.error("Failed to restore output:", out.name, e);
          }
        }
        setActiveOutputs(config.outputs);
      } catch (e) {
        console.error("Failed to load config", e);
      } finally {
        isLoaded.current = true;
      }
    }
    loadConfig();
  }, []);

  // Auto-Save Effect
  useEffect(() => {
    if (!isLoaded.current) return;

    const timer = setTimeout(() => {
      const config: AppConfig = {
        input_volume: inputVolume / 100.0,
        input_muted: inputMuted,
        outputs: activeOutputs
      };
      console.log("Saving Config:", config);
      invoke("save_app_config", { config }).catch(console.error);
    }, 1000); // 1s debounce

    return () => clearTimeout(timer);
  }, [inputVolume, inputMuted, activeOutputs]);


  const toggleCapture = async () => {
    if (capturePaused) {
      await invoke("start_capture");
      setCapturePaused(false);
      setStatus("Active");
    } else {
      await invoke("stop_capture");
      setCapturePaused(true);
      setStatus("Standby");
    }
  };

  const toggleInputMute = () => {
    const newVal = !inputMuted;
    setInputMuted(newVal);
    invoke("set_input_mute", { muted: newVal });
  };

  const addOutput = async () => {
    if (!selectedDeviceName) return;
    if (activeOutputs.find(o => o.name === selectedDeviceName)) {
      alert("Device already active!");
      return;
    }
    if (selectedDeviceName === sourceName) {
      alert("Cannot add the source device as an output (Feedback Loop)!");
      return;
    }

    try {
      await invoke("add_device_to_mix", { deviceName: selectedDeviceName });
      const newOutput = { name: selectedDeviceName, volume: 1.0, muted: false };
      setActiveOutputs([...activeOutputs, newOutput]);
    } catch (e) {
      console.error(e);
      alert("Failed to add output: " + e);
    }
  };

  const removeOutput = async (name: string) => {
    try {
      await invoke("remove_device_from_mix", { deviceName: name });
      setActiveOutputs(activeOutputs.filter(o => o.name !== name));
    } catch (e) {
      console.error(e);
      alert("Failed to remove output: " + e);
    }
  }

  // Update specific output state (vol/mute) for persistence
  const updateOutputState = (name: string, updates: Partial<OutputConfig>) => {
    setActiveOutputs(prev => prev.map(o =>
      o.name === name ? { ...o, ...updates } : o
    ));
  }

  return (
    <div className="container">
      <header style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '15px' }}>
          <img src={logo} alt="Nerv Logo" style={{ height: '40px', filter: 'drop-shadow(0 0 5px var(--neon-orange))' }} />
          <h1>NERV AUDIO LINK v2.0</h1>
        </div>
        <div className="status-badge" style={{ borderColor: capturePaused ? 'yellow' : 'var(--neon-orange)' }}>
          <span className={`dot ${capturePaused ? 'paused' : 'active'}`}></span> {status.toUpperCase()}
        </div>
      </header>

      <div className="card source-card" style={{ opacity: capturePaused ? 0.7 : 1 }}>
        <div className="tech-label">INPUT_STREAM_01</div>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
          <div>
            <h2>SYSTEM_AUDIO_CAPTURE</h2>
            <p>TARGET: <strong style={{ color: '#fff' }}>{sourceName}</strong></p>
            <p className="subtext">STATUS: {capturePaused ? "PAUSED" : "CONNECTED"} // RATE: 48000Hz</p>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '5px' }}>
            <button
              onClick={toggleCapture}
              style={{ padding: '5px 10px', fontSize: '0.8em', background: capturePaused ? 'yellow' : 'transparent', color: capturePaused ? 'black' : 'yellow', border: '1px solid yellow' }}>
              {capturePaused ? "RESUME" : "PAUSE"}
            </button>
          </div>
        </div>

        <div className="volume-slider">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '15px' }}>
            <label style={{ fontSize: '0.7em', color: 'var(--neon-orange)' }}>MASTER GAIN</label>
            <button onClick={toggleInputMute} style={{ padding: '2px 8px', fontSize: '0.7em', background: inputMuted ? 'red' : 'transparent', border: '1px solid red', color: inputMuted ? 'black' : 'red' }}>
              {inputMuted ? "MUTED" : "MUTE"}
            </button>
          </div>
          <input
            type="range"
            min="0"
            max="100"
            value={inputVolume}
            onChange={(e) => {
              const val = parseInt(e.target.value);
              setInputVolume(val);
              invoke("set_input_volume", { volume: val / 100.0 });
            }}
          />
        </div>
      </div>

      <div className="controls">
        <div className="tech-label" style={{ top: '-10px', left: '20px', background: '#080808', padding: '0 10px' }}>SELECT_OUTPUT_TARGET</div>
        <div style={{ display: 'flex', flexDirection: 'column', flexGrow: 1 }}>
          <select onChange={e => setSelectedDeviceName(e.target.value)} value={selectedDeviceName}>
            {devices.map((d, i) => <option key={i} value={d.name}>{d.name}</option>)}
          </select>
        </div>
        <button onClick={addOutput}>INIT_LINK</button>
      </div>

      <div className="outputs-grid">
        {activeOutputs.map((out, idx) => (
          <OutputCard
            key={out.name}
            config={out}
            idx={idx}
            removeOutput={removeOutput}
            updateState={updateOutputState}
          />
        ))}
      </div>
    </div>
  );
}

// Subcomponent for cleaner state management of individual outputs
function OutputCard({ config, idx, removeOutput, updateState }: {
  config: OutputConfig,
  idx: number,
  removeOutput: (n: string) => void,
  updateState: (n: string, s: Partial<OutputConfig>) => void
}) {
  // Local state for smooth slider, but syncs to parent for persistence
  const [vol, setVol] = useState(Math.round(config.volume * 100));

  return (
    <div className="card output-card">
      <div className="tech-label" style={{ color: 'var(--neon-red)', borderColor: 'rgba(255, 82, 82, 0.3)' }}>LINK_0{idx + 1}</div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '10px' }}>
        <h3 style={{ fontSize: '1em', wordBreak: 'break-word', maxWidth: '70%' }}>{config.name}</h3>
        <button className="remove-btn" onClick={() => removeOutput(config.name)}>TERMINATE</button>
      </div>
      <div className="volume-slider">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '10px' }}>
          <label style={{ fontSize: '0.7em' }}>GAIN_CONTROL</label>
          <button onClick={() => {
            const newVal = !config.muted;
            updateState(config.name, { muted: newVal });
            invoke("set_device_mute", { deviceName: config.name, muted: newVal });
          }} style={{ padding: '2px 5px', fontSize: '0.6em', border: '1px solid var(--neon-red)', color: config.muted ? 'black' : 'var(--neon-red)', background: config.muted ? 'var(--neon-red)' : 'transparent' }}>
            {config.muted ? "MUTED" : "MUTE"}
          </button>
        </div>
        <input
          type="range"
          min="0"
          max="100"
          value={vol}
          onChange={(e) => {
            const v = parseInt(e.target.value);
            setVol(v); // Instant UI update
            // Debounce/commit handled by parent effect? 
            // Actually, for persistence we need to update parent state.
            // Ideally wedebounce this, but for now direct update is fine for local
            updateState(config.name, { volume: v / 100.0 });
            invoke("set_device_volume", { deviceName: config.name, volume: v / 100.0 }).catch(console.error);
          }}
        />
      </div>
      <p className="subtext" style={{ color: config.muted ? 'red' : 'inherit' }}>{config.muted ? "STATUS: MUTED" : "SYNC_STATUS: NORMAL"}</p>
    </div>
  );
}

export default App;
