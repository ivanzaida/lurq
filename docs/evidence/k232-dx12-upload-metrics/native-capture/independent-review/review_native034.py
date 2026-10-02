import hashlib
import json
from pathlib import Path
import subprocess
from PIL import Image

root = Path('H:/projects/pencil-web/.codex/worktrees/k222-composed-surfaces')
packet = root / '.tmp/k232-profiler-candidate/dx12-upload-034'
capture = packet / 'native-01/capture'
out = Path(__file__).parent / 'native034-review.json'

def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

identity = load(packet / 'identity.json')
frozen = load(packet / 'native-01/frozen-inputs.json')
summary = load(capture / 'summary.json')
profile = load(capture / 'pair-profile.json')
details = load(capture / 'upload-details.json')
assert identity == frozen['build'] == summary['build_identity']
assert sha(packet / 'identity.json') == frozen['build_identity_sha256']
assert identity['exit_code'] == 0 and '--release' in identity['args']
assert sha(packet / 'kontur-desktop.exe') == identity['binary_sha256']
assert sha(packet / 'Cargo.lock.local') == identity['local_lock_sha256']
assert sha(packet / 'Cargo.lock.registry') == identity['registry_lock_sha256']
for path, digest in frozen['sources_sha256'].items():
    assert sha(Path(path)) == digest
assert profile['finalized'] and not profile['in_flight']
assert profile['dropped_samples'] == profile['boundary_excluded_samples'] == 0
assert profile['returned_samples'] == profile['completed_samples'] == len(profile['samples']) == 42
assert profile['build']['features']['dx12'] and profile['build']['features']['perf_profile']
assert not profile['build']['debug_assertions'] and not profile['build']['gpu_timestamps']['available']
passes = [s['data'] for s in profile['samples'] if s['data']['kind'] == 'pass']
assert len(passes) == summary['pass_records'] == 11
rows = [p['frame']['render']['canvas'] for p in passes if p['frame']]
assert len(rows) == details['detail_pass_records'] == 11
cpu = {k: sum(c['asset_upload_details']['cpu_timings_ms'][k] for c in rows)
       for k in rows[0]['asset_upload_details']['cpu_timings_ms']}
event_names = ('cache_hits', 'cache_misses', 'texture_creations', 'descriptor_pairs',
               'padded_upload_bytes', 'arena_uploads', 'dedicated_uploads', 'cache_evictions')
events = {k: sum(c['asset_upload_details']['counts'][k] for c in rows) for k in event_names}
for key, value in cpu.items():
    assert abs(value - details['cpu_totals_ms'][key]) < 1e-9
assert events == details['event_counts']
for canvas in rows:
    detail = canvas['asset_upload_details']
    count = detail['counts']
    assert count['cache_misses'] == count['texture_creations']
    assert count['arena_uploads'] + count['dedicated_uploads'] == count['texture_creations']
    assert count['cache_entries_after'] == count['cache_entries_before'] + count['texture_creations'] - count['cache_evictions']
    assert count['cache_charged_bytes_peak'] >= max(count['cache_charged_bytes_before'], count['cache_charged_bytes_after'])
    assert count['padded_upload_bytes'] >= canvas['counts']['uploaded_asset_bytes']
    assert sum(v for k, v in detail['cpu_timings_ms'].items() if k != 'cache_eviction') <= canvas['cpu_timings_ms']['asset_upload'] + 1e-9
upload = sum(c['cpu_timings_ms']['asset_upload'] for c in rows)
assert abs(upload - details['coarse_asset_upload_ms']) < 1e-9
unpadded = sum(c['counts']['uploaded_asset_bytes'] for c in rows)
assert unpadded == details['uploaded_asset_bytes']
keys = ('page','zoom','pan_x','pan_y','visible_items','render_instances','renderer',
        'selection','text_faces','text_refusal','canvas_error')
before, after = summary['viewport_before'], summary['viewport_after']
assert all(before[k] == after[k] for k in keys)
assert before['render_instances'] == before['visible_items'] == '1492'
window_before, window_after = load(capture / 'window-00.json'), load(capture / 'window-after.json')
assert window_before == window_after
assert summary['window'] == {'width':2160,'height':1440,'scale_factor':1.5}
expected = {'document': frozen['document_sha256'], 'ledger': frozen['ledger_sha256']}
assert load(capture / 'before-launch-hashes.json') == load(capture / 'after-run-hashes.json') == expected
doc_folder = packet / 'native-01/profile/Kontur/local-documents'
assert sha(doc_folder / '0acea323-5056-408d-84c0-f7c9a82e7c36.kontur') == expected['document']
assert sha(doc_folder / 'ledger.json') == expected['ledger']
cleanup = load(capture / 'cleanup.json')
assert cleanup == load(packet / 'native-01/smoke-result.json')['cleanup']
assert cleanup['exited'] and cleanup['port_closed']
inventory = load(capture / 'runtime-inventory-before-canvas.json')
owned = [p for p in inventory['processes'] if p['ProcessId'] == cleanup['pid']]
assert len(owned) == 1 and Path(owned[0]['ExecutablePath']) == packet / 'kontur-desktop.exe'
seal = load(packet / 'closed-selected-payloads.json')
assert seal['selected_payload_count'] == len(seal['payloads']) == 62
assert sum(item['bytes'] for item in seal['payloads']) == seal['selected_payload_bytes']
for item in seal['payloads']:
    path = packet / item['path']
    assert path.stat().st_size == item['bytes'] and sha(path) == item['sha256']
images = []
for label in ('before', 'after'):
    path = capture / f'{label}.png'
    assert sha(path) == load(capture / f'{label}-image.json')['png_sha256']
    images.append(Image.open(path).convert('RGBA'))
assert images[0].size == images[1].size == tuple(summary['canvas_bounds'][2:])
different = []
for index, (a, b) in enumerate(zip(images[0].getdata(), images[1].getdata())):
    if a != b:
        different.append((index % images[0].width, index // images[0].width))
bbox = None if not different else [min(x for x,y in different), min(y for x,y in different),
                                   max(x for x,y in different)+1, max(y for x,y in different)+1]
result = {
    'result':'SCOPED_PASS_CLOSED_DIAGNOSTIC_IDENTITY_AND_METRIC_CONSISTENCY',
    'fixture':'Genuine18 / 01 Editor', 'build':identity,
    'completed_records':42, 'pass_records':11,
    'max_pass_ms':max(p['cpu_timings_ms']['total'] for p in passes),
    'cpu_totals_ms':cpu, 'coarse_asset_upload_ms':upload,
    'child_sum_ms':sum(v for k,v in cpu.items() if k != 'cache_eviction'),
    'unattributed_parent_residual_ms':upload-sum(v for k,v in cpu.items() if k != 'cache_eviction'),
    'event_counts':events, 'uploaded_asset_bytes':unpadded,
    'texture_share_of_asset_upload_percent':100*cpu['texture_creation']/upload,
    'pixel_comparison':{'dimensions':images[0].size,'different_pixels':len(different),'bbox':bbox,
                        'pixel_exact_roundtrip':not different},
    'viewport_identity':{k:before[k] for k in keys}, 'document_hashes':expected,
    'cleanup':cleanup, 'external_compilers_present':any(p['Name']=='rustc.exe' for p in inventory['processes']),
    'builder_seal':{'path':str(packet / 'closed-selected-payloads.json'),
                    'sha256':sha(packet / 'closed-selected-payloads.json'),'verified_raw_payloads':62},
    'claim_scope':'diagnostic CPU API wall times under uncontrolled load; no GPU duration, speedup or controlled regression claim',
}
out.write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
print(json.dumps({k:result[k] for k in ('result','completed_records','pass_records','max_pass_ms','cpu_totals_ms','event_counts','pixel_comparison')}))
