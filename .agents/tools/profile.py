import argparse, json, shutil
from pathlib import Path
from _common import AGENTS_DIR

PROFILES = AGENTS_DIR / 'profiles'
LIBRARY = AGENTS_DIR / 'library' / 'skills'
ACTIVE = AGENTS_DIR / 'skills'
ACTIVE_FILE = AGENTS_DIR / 'active-profile.json'
MAX_SKILLS = 3


def load_profile(name):
    p = PROFILES / f'{name}.json'
    if not p.exists():
        raise SystemExit(f'Unknown profile: {name}')
    data = json.loads(p.read_text(encoding='utf-8'))
    skills = data.get('skills', [])
    if len(skills) > MAX_SKILLS:
        raise SystemExit(f'Profile {name} activates {len(skills)} skills; maximum is {MAX_SKILLS}. Split the task.')
    for skill in skills:
        if not (LIBRARY / skill / 'SKILL.md').exists():
            raise SystemExit(f'Profile references missing skill: {skill}')
    return data


def clear_active():
    ACTIVE.mkdir(parents=True, exist_ok=True)
    for p in ACTIVE.iterdir():
        if p.name == '.gitkeep':
            continue
        if p.is_dir(): shutil.rmtree(p)
        else: p.unlink()


def activate(name):
    data = load_profile(name)
    clear_active()
    for skill in data.get('skills', []):
        shutil.copytree(LIBRARY / skill, ACTIVE / skill)
    ACTIVE_FILE.write_text(json.dumps({'name': name, 'skills': data.get('skills', [])}, indent=2), encoding='utf-8')
    print(f'Active profile: {name}')
    print('Active skills:', ', '.join(data.get('skills', [])) or '(none)')


def list_profiles():
    for p in sorted(PROFILES.glob('*.json')):
        data = json.loads(p.read_text(encoding='utf-8'))
        print(f"{data['name']:<22} {', '.join(data.get('skills', [])) or '(none)'}")


def status():
    if ACTIVE_FILE.exists():
        print(ACTIVE_FILE.read_text(encoding='utf-8'))
    else:
        print('{"name":"unknown","skills":[]}')


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description='Formula-90 PROFILE switcher; MAX-ACTIVE-SKILLS=3.')
    sp = ap.add_subparsers(dest='cmd', required=True)
    sp.add_parser('list')
    p = sp.add_parser('activate'); p.add_argument('name')
    sp.add_parser('status')
    args = ap.parse_args()
    if args.cmd == 'list': list_profiles()
    elif args.cmd == 'activate': activate(args.name)
    elif args.cmd == 'status': status()
