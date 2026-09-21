from pathlib import Path
import csv
import hashlib
import json
import re
import zipfile

analysis_directory = Path(__file__).resolve().parent
analysis = json.loads((analysis_directory / 'measurements.json').read_text())
source_directory = Path(analysis['source_directory'])
source_files = sorted(source_directory.rglob('*.wav'))
assert len(source_files) == len(analysis['measurements']) == 13
assert {path.name for path in source_files} == {entry['filename'] for entry in analysis['measurements']}
for entry in analysis['measurements']:
    assert hashlib.sha256((source_directory / entry['filename']).read_bytes()).hexdigest() == entry['sha256_before'] == entry['sha256_after']
    assert entry['digital_rail_sample_count'] == 0
    assert abs(sum(region['power_percent'] for region in entry['spectral']['regions'].values()) - 100) < 1e-6
with (analysis_directory / 'measurements.csv').open(newline='', encoding='utf-8') as handle:
    assert len(list(csv.DictReader(handle))) == 13
report = (analysis_directory / 'technical_report.md').read_text(encoding='utf-8')
for target in re.findall(r'\]\(([^)]+)\)', report):
    if not target.startswith('https://'):
        assert (analysis_directory / target).is_file(), target
previous_status = (analysis_directory / 'repository-status-before.txt').read_text()
current_status = (analysis_directory / 'repository-status-after.txt').read_text()
assert previous_status == current_status
requirements = '\n'.join(f'{package}=={version}' for package, version in analysis['libraries'].items()) + '\nmarkdown==3.10.3\ncffi\n'
(analysis_directory / 'analysis_requirements.txt').write_text(requirements)
(analysis_directory / 'reproduce_analysis.txt').write_text('Use Python 3.14. Install analysis_requirements.txt in an isolated environment. The scripts also look in a sibling python-packages directory if present. Run analyze_source_audio.py, analyze_diagnostic_details.py, and write_technical_report.py in that order. The default source path is D:/Formula90s/game/sounds/banks/v10-gp3. The current sampler manifest is read for loop/event roles and transition-rate diagnostics. Repository status snapshots are read-only provenance captured outside the Python scripts. No script writes audio. verify_and_package_report.py validates the saved snapshots, source hashes, measurements and report links, then creates the report ZIP.\n')
verification = {'audio_file_count': 13, 'all_source_sha256_unchanged': True, 'repository_status_unchanged': True, 'measurements_csv_rows': 13, 'per_file_plots': 13, 'comparison_plots': 2, 'all_report_local_links_exist': True, 'source_audio_exported_or_modified': False}
(analysis_directory / 'verification.json').write_text(json.dumps(verification, indent=2))
archive_path = analysis_directory / 'v10_gp3_analysis_report.zip'
with zipfile.ZipFile(archive_path, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
    for path in sorted(analysis_directory.iterdir()):
        if path.is_file() and path.suffix in ('.py', '.md', '.html', '.json', '.csv', '.txt'):
            archive.write(path, path.name)
    for path in sorted((analysis_directory / 'figures').glob('*.png')):
        archive.write(path, 'figures/' + path.name)
print(json.dumps(verification))
print(f'Archive: {archive_path}, {archive_path.stat().st_size} bytes')
