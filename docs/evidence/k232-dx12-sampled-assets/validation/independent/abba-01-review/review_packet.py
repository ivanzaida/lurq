"""Offline independent reduction of closed ABBA reports and full RGBA images."""
import hashlib
import json
import math
from pathlib import Path
from PIL import Image, ImageChops

WARM = Path('H:/projects/pencil-web/.codex/worktrees/k222-composed-surfaces')
PACKET = WARM / '.tmp/k232-profiler-candidate/dx12-sampled-assets/abba-01'
OUT = Path(__file__).resolve().parent
checked = {}


def digest(path):
    raw = path.read_bytes()
    result = hashlib.sha256(raw).hexdigest()
    checked[str(path)] = {'sha256': result, 'bytes': len(raw)}
    return result


def load(path):
    digest(path)
    return json.loads(path.read_bytes())


def close(a, b):
    assert math.isclose(a, b, rel_tol=1e-10, abs_tol=1e-7), (a, b)


def main():
    summary = load(PACKET / 'comparison-summary.json')
    receipts = load(PACKET / 'run-receipts.json')
    inputs = load(PACKET / 'frozen-abba-inputs.json')
    terminal = load(PACKET / 'terminal.json')
    assert terminal['exit_code'] == 0 and terminal['cases_completed'] == 4
    assert terminal['source_restored']
    assert [r['name'] for r in receipts] == inputs['order'] == ['A1', 'B1', 'B2', 'A2']
    for item in inputs['inputs']:
        root, identity = Path(item['root']), item['identity']
        assert digest(root / 'kontur-desktop.exe') == identity['binary_sha256']
        assert digest(root / 'smoke.py') == item['wrapper_sha256']
        assert identity['exit_code'] == 0
        assert identity['kontur_head'] == '61d73d08b7c047d6c62ad55d6ed127a495881772'
        assert digest(root / 'Cargo.lock.local') == identity['local_lock_sha256']
    derived = []
    for receipt, reported in zip(receipts, summary['runs']):
        assert receipt['exit_code'] == 0 and receipt['name'] == reported['name']
        root = Path(receipt['capture_root']) / 'capture'
        profile = load(root / 'pair-profile.json')
        detail = load(root / 'upload-details.json')
        state = load(root / 'summary.json')
        assert profile['finalized'] and not profile['truncated']
        assert profile['dropped_samples'] == profile['boundary_excluded_samples'] == 0
        assert profile['in_flight'] == []
        assert profile['returned_samples'] == profile['completed_samples'] == len(profile['samples'])
        passes = [s['data'] for s in profile['samples'] if s['data']['kind'] == 'pass']
        assert len(passes) == reported['pass_records'] == len(detail['passes'])
        canvas = [p['frame']['render']['canvas'] for p in passes]
        timings = {k: sum(c['asset_upload_details']['cpu_timings_ms'][k] for c in canvas)
                   for k in detail['cpu_totals_ms']}
        events = {k: sum(c['asset_upload_details']['counts'][k] for c in canvas)
                  for k in detail['event_counts']}
        assert events == detail['event_counts'] == reported['upload_event_counts']
        for k, v in timings.items():
            close(v, detail['cpu_totals_ms'][k])
            close(v, reported['upload_cpu_children_ms'][k])
        upload = sum(c['cpu_timings_ms']['asset_upload'] for c in canvas)
        close(upload, reported['asset_upload_ms'])
        close(upload, detail['coarse_asset_upload_ms'])
        size = sum(c['counts']['uploaded_asset_bytes'] for c in canvas)
        assert size == detail['uploaded_asset_bytes'] == reported['uploaded_asset_bytes']
        close(max(p['cpu_timings_ms']['total'] for p in passes), reported['max_pass_ms'])
        before = load(root / 'before-launch-hashes.json')
        assert before == load(root / 'after-run-hashes.json')
        cleanup = load(root / 'cleanup.json')
        launch = load(root / 'launch.json')
        opened = load(root / 'document-open.json')['document']
        files = Path(launch['profile']) / 'Kontur/local-documents'
        assert digest(files / (opened['id'] + '.kontur')) == before['document']
        assert digest(files / 'ledger.json') == before['ledger']
        assert opened['revision'] == 1 and opened['containerSha256'] == before['document']
        assert cleanup['pid'] == launch['pid'] and cleanup['exited'] and cleanup['port_closed']
        assert launch['port'] == 4903 and launch['binary_sha256'] == reported['binary_sha256']
        assert state['build_identity']['lurq_head'] == receipt['source_head'] == reported['source_head']
        for k in ('page', 'zoom', 'pan_x', 'pan_y', 'selection', 'visible_items', 'render_instances'):
            assert state['viewport_before'][k] == state['viewport_after'][k]
        for end in ('before', 'after'):
            assert state['viewport_' + end]['renderer'] == 'dx12'
            assert not state['viewport_' + end]['canvas_error']
            assert not state['viewport_' + end]['text_refusal']
        assert state['window'] == reported['window']
        assert state['canvas_bounds'] == reported['canvas_bounds']
        assert load(root / 'window-00.json') == load(root / 'window-after.json')
        inventory = load(root / 'runtime-inventory-before-canvas.json')
        assert any(p['Name'] in ('cargo.exe', 'rustc.exe', 'link.exe')
                   for p in inventory['processes'])
        derived.append({'name': receipt['name'], 'fixture': state['fixture'],
            'upload_ms': upload, 'children_ms': timings, 'events': events,
            'uploaded_bytes': size, 'document_ledger_hashes': before,
            'cleanup': cleanup, 'completed_passes': len(passes),
            'inclusive_upload_residual_ms': upload - sum(v for k, v in timings.items()
                                                       if k != 'cache_eviction')})
    assert all(r['events'] == derived[0]['events'] for r in derived)
    assert all(r['document_ledger_hashes'] == derived[0]['document_ledger_hashes'] for r in derived)
    pixel_report = load(PACKET / 'decoded-pixel-comparisons.json')
    pixels = []
    for pair in pixel_report['comparisons']:
        a_path, b_path = Path(pair['first']), Path(pair['second'])
        assert digest(a_path) == pair['first_png_sha256']
        assert digest(b_path) == pair['second_png_sha256']
        a, b = [Image.open(p).convert('RGBA') for p in (a_path, b_path)]
        assert a.size == b.size == (1404, 1257)
        diff = ImageChops.difference(a, b)
        mask = diff.getchannel('R')
        for channel in 'GBA':
            mask = ImageChops.lighter(mask, diff.getchannel(channel))
        count = a.width * a.height - mask.histogram()[0]
        bbox = list(mask.getbbox()) if mask.getbbox() else None
        maximum = max(diff.getchannel(c).getextrema()[1] for c in 'RGBA')
        assert (count, bbox, maximum) == (pair['differing_pixels'],
            pair['difference_bbox_half_open'], pair['max_channel_difference'])
        pixels.append({'pair': pair['pair'], 'differing_pixels': count,
                       'bbox_half_open': bbox, 'max_channel_difference': maximum})
    result = {'verdict': 'SCOPED_CORRECTNESS_PASS_DIAGNOSTIC_TIMINGS',
        'runs': derived, 'pixels': pixels, 'reviewed_raw_hashes': checked,
        'review_scope': 'Offline closed-run reduction; no reviewer app/Cargo execution'}
    (OUT / 'derived-review.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'verdict': result['verdict'], 'runs': len(derived), 'pixel_pairs': len(pixels)}))


if __name__ == '__main__':
    main()
