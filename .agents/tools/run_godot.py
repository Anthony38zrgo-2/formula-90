import argparse, os, shutil, subprocess, sys
from pathlib import Path
from _common import ensure_layout, DIAGNOSTICS_DIR, ROOT, RUNTIME_DB, connect
from diag import start_run, end_run

READY_MARKER='[OBSERVABILITY] GAME_READY'

def main():
    ap=argparse.ArgumentParser(description='REAL-RUNTIME capture -> persisted process/log evidence; NO-SMOKE.')
    ap.add_argument('--project',default='.')
    ap.add_argument('--task',required=True)
    ap.add_argument('--profile',default='minimal')
    ap.add_argument('--godot',help='Godot executable. Defaults to godot/godot4 from PATH.')
    ap.add_argument('--timeout',type=float,default=None,help='Optional observation timeout in seconds. No timeout by default.')
    ap.add_argument('extra',nargs=argparse.REMAINDER,help='Extra Godot args after --')
    a=ap.parse_args(); ensure_layout()
    exe=a.godot or shutil.which('godot') or shutil.which('godot4')
    if not exe: raise SystemExit('Godot executable not found. Pass --godot PATH.')
    project=Path(a.project).resolve()
    raw_dir=DIAGNOSTICS_DIR/'raw'/'godot'; raw_dir.mkdir(parents=True,exist_ok=True)
    rid=start_run(a.task,a.profile)
    log=raw_dir/f'{rid}.log'
    cmd=[exe,'--path',str(project),'--verbose','--log-file',str(log)]
    extra=a.extra
    if extra and extra[0]=='--': extra=extra[1:]
    cmd += extra
    print('run_id:',rid)
    print('command:', ' '.join(map(str,cmd)))
    status='RUNTIME_STARTED'; code=None; output=''; timed_out=False
    try:
        proc=subprocess.run(cmd,cwd=project,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=a.timeout,errors='replace')
        code=proc.returncode; output=proc.stdout or ''
        status='RUNTIME_OBSERVED' if code==0 else 'CRASHED'
    except subprocess.TimeoutExpired as e:
        timed_out=True; output=(e.stdout or '') if isinstance(e.stdout,str) else (e.stdout or b'').decode(errors='replace')
        status='RUNTIME_OBSERVED'; code=None
    except Exception as e:
        output=f'Launcher error: {e}'
        status='CRASHED'; code=-1
    # Persist combined launcher + Godot evidence.
    combined=raw_dir/f'{rid}.combined.log'; combined.write_text(output,encoding='utf-8',errors='replace')
    text=output
    if log.exists():
        try: text += '\n'+log.read_text(encoding='utf-8',errors='replace')
        except Exception: pass
    ready=READY_MARKER in text
    end_run(rid,status,code,ready,str(log))
    if code not in (None,0):
        with connect(RUNTIME_DB) as con:
            con.execute('INSERT INTO crashes(run_id,source,severity,exit_code,error_type,message,raw_log_path) VALUES(?,?,?,?,?,?,?)',
                        (rid,'PROCESS','FATAL',code,'NONZERO_EXIT',f'Godot process exited with code {code}',str(log)))
    print('status:',status)
    print('game_ready:',1 if ready else 0)
    print('godot_log:',log)
    print('combined_log:',combined)
    if not ready:
        print('observation: NOT_OBSERVED (GAME_READY marker not seen)')
    sys.exit(0 if code in (None,0) else code)

if __name__=='__main__': main()
