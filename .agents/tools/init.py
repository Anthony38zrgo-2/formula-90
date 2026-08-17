from _common import ensure_layout, AGENT_DB, RUNTIME_DB

if __name__ == '__main__':
    ensure_layout()
    print(f'Initialized: {AGENT_DB}')
    print(f'Initialized: {RUNTIME_DB}')
    print('All tables are empty unless this repository has already recorded new runs.')
