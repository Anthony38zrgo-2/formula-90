PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS runs (
    run_id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    git_sha TEXT,
    git_dirty INTEGER,
    task TEXT NOT NULL,
    profile TEXT,
    status TEXT NOT NULL DEFAULT 'STARTED',
    process_exit_code INTEGER,
    game_ready INTEGER NOT NULL DEFAULT 0,
    raw_log_path TEXT,
    notes TEXT
);

CREATE TABLE IF NOT EXISTS telemetry (
    run_id TEXT NOT NULL,
    frame INTEGER NOT NULL,
    physics_frame INTEGER,
    vehicle_id TEXT NOT NULL DEFAULT 'player',
    timestamp_us INTEGER,
    dt REAL,

    speed REAL,
    accel_x REAL,
    accel_y REAL,
    accel_z REAL,
    yaw_rate REAL,
    pitch_rate REAL,
    roll_rate REAL,
    pos_x REAL,
    pos_y REAL,
    pos_z REAL,

    throttle REAL,
    brake REAL,
    steering REAL,

    rpm REAL,
    gear INTEGER,
    engine_torque REAL,

    fl_load REAL,
    fl_suspension REAL,
    fl_slip_ratio REAL,
    fl_slip_angle REAL,
    fl_fx REAL,
    fl_fy REAL,

    fr_load REAL,
    fr_suspension REAL,
    fr_slip_ratio REAL,
    fr_slip_angle REAL,
    fr_fx REAL,
    fr_fy REAL,

    rl_load REAL,
    rl_suspension REAL,
    rl_slip_ratio REAL,
    rl_slip_angle REAL,
    rl_fx REAL,
    rl_fy REAL,

    rr_load REAL,
    rr_suspension REAL,
    rr_slip_ratio REAL,
    rr_slip_angle REAL,
    rr_fx REAL,
    rr_fy REAL,

    godot_frame_ms REAL,
    rust_physics_us INTEGER,
    subsystem_flags INTEGER,
    extra_json TEXT,

    PRIMARY KEY (run_id, frame, vehicle_id),
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS runtime_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    timestamp_us INTEGER,
    frame INTEGER,
    physics_frame INTEGER,
    source TEXT NOT NULL,
    subsystem TEXT,
    level TEXT NOT NULL,
    code TEXT,
    message TEXT NOT NULL,
    file TEXT,
    line INTEGER,
    function TEXT,
    data_json TEXT,
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS crashes (
    crash_id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    source TEXT NOT NULL,
    severity TEXT,
    exit_code INTEGER,
    error_type TEXT,
    message TEXT NOT NULL,
    file TEXT,
    line INTEGER,
    function TEXT,
    frame INTEGER,
    physics_frame INTEGER,
    backtrace TEXT,
    raw_log_path TEXT,
    last_state_json TEXT,
    git_sha TEXT,
    fingerprint TEXT,
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS human_gates (
    run_id TEXT PRIMARY KEY,
    decision TEXT NOT NULL CHECK(decision IN ('ACCEPTED','REJECTED')),
    decided_at TEXT NOT NULL DEFAULT (datetime('now')),
    note TEXT,
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_runs_started ON runs(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_runs_task ON runs(task);
CREATE INDEX IF NOT EXISTS idx_events_run_level ON runtime_events(run_id, level);
CREATE INDEX IF NOT EXISTS idx_events_subsystem ON runtime_events(subsystem);
CREATE INDEX IF NOT EXISTS idx_crashes_run ON crashes(run_id);
CREATE INDEX IF NOT EXISTS idx_crashes_time ON crashes(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_telemetry_run_frame ON telemetry(run_id, frame);
