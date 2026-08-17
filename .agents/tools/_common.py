from pathlib import Path
import sqlite3

AGENTS_DIR = Path(__file__).resolve().parents[1]
ROOT = AGENTS_DIR.parent
DIAGNOSTICS_DIR = ROOT / 'diagnostics'
RUNTIME_DB = DIAGNOSTICS_DIR / 'runtime.sqlite'
AGENT_DB = AGENTS_DIR / 'data' / 'agents.sqlite'


def connect(path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    con = sqlite3.connect(path)
    con.row_factory = sqlite3.Row
    con.execute('PRAGMA foreign_keys=ON')
    return con


def apply_schema(db_path: Path, schema_path: Path):
    sql = schema_path.read_text(encoding='utf-8')
    with connect(db_path) as con:
        con.executescript(sql)


def ensure_layout():
    for p in [
        AGENTS_DIR / 'data',
        AGENTS_DIR / 'skills',
        DIAGNOSTICS_DIR / 'raw' / 'godot',
        DIAGNOSTICS_DIR / 'raw' / 'rust',
        DIAGNOSTICS_DIR / 'raw' / 'crashes',
        DIAGNOSTICS_DIR / 'exports',
    ]:
        p.mkdir(parents=True, exist_ok=True)
    apply_schema(AGENT_DB, AGENTS_DIR / 'schemas' / 'agents.sql')
    apply_schema(RUNTIME_DB, AGENTS_DIR / 'schemas' / 'runtime.sql')
