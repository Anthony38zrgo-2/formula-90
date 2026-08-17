import argparse, csv, json, os, sqlite3, subprocess, uuid
from datetime import datetime, timezone
from pathlib import Path
from _common import ensure_layout, connect, RUNTIME_DB, DIAGNOSTICS_DIR, ROOT


def now(): return datetime.now(timezone.utc).isoformat()

def rows(sql, params=()):
    ensure_layout()
    with connect(RUNTIME_DB) as con:
        return con.execute(sql, params).fetchall()

def show(rs):
    if not rs:
        print('(no rows)'); return
    cols = rs[0].keys()
    widths = {c: min(60, max(len(c), *(len(str(r[c] if r[c] is not None else '')) for r in rs))) for c in cols}
    print(' | '.join(c.ljust(widths[c]) for c in cols))
    print('-+-'.join('-'*widths[c] for c in cols))
    for r in rs:
        print(' | '.join(str(r[c] if r[c] is not None else '')[:widths[c]].ljust(widths[c]) for c in cols))

def git_state():
    try:
        sha = subprocess.check_output(['git','rev-parse','HEAD'], cwd=ROOT, text=True, stderr=subprocess.DEVNULL).strip()
        dirty = 1 if subprocess.call(['git','diff','--quiet'], cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) else 0
        return sha, dirty
    except Exception:
        return None, None

def start_run(task, profile, notes=None, raw_log_path=None):
    ensure_layout(); rid = uuid.uuid4().hex
    sha, dirty = git_state()
    with connect(RUNTIME_DB) as con:
        con.execute('INSERT INTO runs(run_id,started_at,git_sha,git_dirty,task,profile,status,raw_log_path,notes) VALUES(?,?,?,?,?,?,?,?,?)',
                    (rid,now(),sha,dirty,task,profile,'STARTED',raw_log_path,notes))
    return rid

def end_run(rid, status, exit_code=None, game_ready=False, raw_log_path=None):
    ensure_layout()
    with connect(RUNTIME_DB) as con:
        con.execute('UPDATE runs SET ended_at=?,status=?,process_exit_code=?,game_ready=?,raw_log_path=COALESCE(?,raw_log_path) WHERE run_id=?',
                    (now(),status,exit_code,1 if game_ready else 0,raw_log_path,rid))

def cmd_recent(a):
    show(rows('SELECT run_id,started_at,task,profile,status,process_exit_code,game_ready FROM runs ORDER BY started_at DESC LIMIT ?', (a.limit,)))

def cmd_crashes(a):
    if a.run_id:
        show(rows('SELECT crash_id,run_id,timestamp,source,error_type,message,file,line,exit_code,raw_log_path FROM crashes WHERE run_id=? ORDER BY crash_id DESC LIMIT ?', (a.run_id,a.limit)))
    else:
        show(rows('SELECT crash_id,run_id,timestamp,source,error_type,message,file,line,exit_code,raw_log_path FROM crashes ORDER BY crash_id DESC LIMIT ?', (a.limit,)))

def cmd_errors(a):
    where = "level IN ('ERROR','WARN','FATAL')"
    params=[]
    if a.run_id: where += ' AND run_id=?'; params.append(a.run_id)
    params.append(a.limit)
    show(rows(f'SELECT id,run_id,frame,source,subsystem,level,code,message,file,line FROM runtime_events WHERE {where} ORDER BY id DESC LIMIT ?', params))

def cmd_summary(a):
    show(rows('SELECT * FROM runs WHERE run_id=?',(a.run_id,)))
    show(rows("SELECT level,COUNT(*) AS count FROM runtime_events WHERE run_id=? GROUP BY level ORDER BY count DESC",(a.run_id,)))
    show(rows('SELECT COUNT(*) AS crash_count FROM crashes WHERE run_id=?',(a.run_id,)))
    show(rows('SELECT decision,decided_at,note FROM human_gates WHERE run_id=?',(a.run_id,)))

def cmd_telemetry(a):
    sql='SELECT * FROM telemetry WHERE run_id=?'
    params=[a.run_id]
    if a.from_frame is not None: sql+=' AND frame>=?'; params.append(a.from_frame)
    if a.to_frame is not None: sql+=' AND frame<=?'; params.append(a.to_frame)
    sql+=' ORDER BY frame LIMIT ?'; params.append(a.limit)
    show(rows(sql,params))

def cmd_event(a):
    ensure_layout()
    with connect(RUNTIME_DB) as con:
        con.execute('INSERT INTO runtime_events(run_id,timestamp_us,frame,physics_frame,source,subsystem,level,code,message,file,line,function,data_json) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)',
                    (a.run_id,a.timestamp_us,a.frame,a.physics_frame,a.source,a.subsystem,a.level,a.code,a.message,a.file,a.line,a.function,a.data_json))

def cmd_crash_add(a):
    ensure_layout(); bt=None
    if a.backtrace_file: bt=Path(a.backtrace_file).read_text(encoding='utf-8',errors='replace')
    with connect(RUNTIME_DB) as con:
        sha=con.execute('SELECT git_sha FROM runs WHERE run_id=?',(a.run_id,)).fetchone()
        con.execute('INSERT INTO crashes(run_id,source,severity,exit_code,error_type,message,file,line,function,frame,physics_frame,backtrace,raw_log_path,last_state_json,git_sha,fingerprint) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)',
                    (a.run_id,a.source,a.severity,a.exit_code,a.error_type,a.message,a.file,a.line,a.function,a.frame,a.physics_frame,bt,a.raw_log_path,a.last_state_json,sha['git_sha'] if sha else None,a.fingerprint))

def cmd_export(a):
    ensure_layout(); out=Path(a.output) if a.output else DIAGNOSTICS_DIR/'exports'/f'{a.run_id}.csv'
    rs=rows('SELECT * FROM telemetry WHERE run_id=? ORDER BY frame',(a.run_id,))
    out.parent.mkdir(parents=True,exist_ok=True)
    if not rs:
        out.write_text('',encoding='utf-8'); print(f'No telemetry; wrote empty {out}'); return
    with out.open('w',newline='',encoding='utf-8') as f:
        w=csv.DictWriter(f,fieldnames=rs[0].keys()); w.writeheader(); w.writerows(dict(r) for r in rs)
    print(out)

def cmd_import_csv(a):
    ensure_layout()
    with open(a.csv_file,newline='',encoding='utf-8-sig') as f:
        reader=csv.DictReader(f); data=list(reader)
    if not data: print('No rows'); return
    allowed=[r['name'] for r in rows('PRAGMA table_info(telemetry)')]
    cols=[c for c in reader.fieldnames if c in allowed and c!='run_id']
    with connect(RUNTIME_DB) as con:
        for r in data:
            names=['run_id']+cols; vals=[a.run_id]+[r.get(c) if r.get(c)!='' else None for c in cols]
            qs=','.join('?' for _ in names)
            con.execute(f"INSERT OR REPLACE INTO telemetry({','.join(names)}) VALUES({qs})",vals)
    print(f'Imported {len(data)} telemetry rows into {a.run_id}')

def cmd_baseline(a):
    sql = ("SELECT r.run_id,r.started_at,r.task,r.profile,r.status,g.decision,g.decided_at "
           "FROM runs r JOIN human_gates g ON g.run_id=r.run_id "
           "WHERE g.decision='ACCEPTED' AND r.task=? ORDER BY g.decided_at DESC LIMIT 1")
    show(rows(sql,(a.task,)))

def cmd_gate(a):
    # HUMAN-GATE: invoke only to record an explicit human decision.
    ensure_layout(); decision='ACCEPTED' if a.decision=='accept' else 'REJECTED'
    with connect(RUNTIME_DB) as con:
        exists=con.execute('SELECT 1 FROM runs WHERE run_id=?',(a.run_id,)).fetchone()
        if not exists: raise SystemExit('Unknown run_id')
        con.execute('INSERT INTO human_gates(run_id,decision,note) VALUES(?,?,?) ON CONFLICT(run_id) DO UPDATE SET decision=excluded.decision,decided_at=datetime(\'now\'),note=excluded.note',(a.run_id,decision,a.note))
    print(f'{a.run_id}: {decision}')

if __name__=='__main__':
    ap=argparse.ArgumentParser(description='Formula-90 RUNTIME-EVIDENCE CLI; NO-SMOKE.')
    sp=ap.add_subparsers(dest='cmd',required=True)
    p=sp.add_parser('recent'); p.add_argument('--limit',type=int,default=5); p.set_defaults(fn=cmd_recent)
    p=sp.add_parser('crashes'); p.add_argument('--run-id'); p.add_argument('--limit',type=int,default=10); p.set_defaults(fn=cmd_crashes)
    p=sp.add_parser('errors'); p.add_argument('--run-id'); p.add_argument('--limit',type=int,default=20); p.set_defaults(fn=cmd_errors)
    p=sp.add_parser('summary'); p.add_argument('run_id'); p.set_defaults(fn=cmd_summary)
    p=sp.add_parser('telemetry'); p.add_argument('run_id'); p.add_argument('--from-frame',type=int); p.add_argument('--to-frame',type=int); p.add_argument('--limit',type=int,default=200); p.set_defaults(fn=cmd_telemetry)
    p=sp.add_parser('event'); p.add_argument('run_id'); p.add_argument('--source',required=True); p.add_argument('--level',required=True); p.add_argument('--message',required=True); p.add_argument('--subsystem'); p.add_argument('--code'); p.add_argument('--frame',type=int); p.add_argument('--physics-frame',type=int); p.add_argument('--timestamp-us',type=int); p.add_argument('--file'); p.add_argument('--line',type=int); p.add_argument('--function'); p.add_argument('--data-json'); p.set_defaults(fn=cmd_event)
    p=sp.add_parser('add-crash'); p.add_argument('run_id'); p.add_argument('--source',required=True); p.add_argument('--message',required=True); p.add_argument('--severity'); p.add_argument('--exit-code',type=int); p.add_argument('--error-type'); p.add_argument('--file'); p.add_argument('--line',type=int); p.add_argument('--function'); p.add_argument('--frame',type=int); p.add_argument('--physics-frame',type=int); p.add_argument('--backtrace-file'); p.add_argument('--raw-log-path'); p.add_argument('--last-state-json'); p.add_argument('--fingerprint'); p.set_defaults(fn=cmd_crash_add)
    p=sp.add_parser('export'); p.add_argument('run_id'); p.add_argument('--output'); p.set_defaults(fn=cmd_export)
    p=sp.add_parser('import-csv'); p.add_argument('run_id'); p.add_argument('csv_file'); p.set_defaults(fn=cmd_import_csv)
    p=sp.add_parser('baseline'); p.add_argument('--task',required=True); p.set_defaults(fn=cmd_baseline)
    p=sp.add_parser('gate'); p.add_argument('run_id'); p.add_argument('decision',choices=['accept','reject']); p.add_argument('--note'); p.set_defaults(fn=cmd_gate)
    a=ap.parse_args(); a.fn(a)
