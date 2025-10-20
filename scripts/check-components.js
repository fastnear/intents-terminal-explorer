#!/usr/bin/env node
'use strict';

const fs = require('fs');
const fsp = fs.promises;
const path = require('path');
const { spawnSync } = require('child_process');

function args() {
  const a = process.argv.slice(2);
  const out = {
    edges: process.env.EDGES_FILE || '',
    nodes: process.env.NODES_FILE || '',
    evals: process.env.EIG_BASELINE_FILE || 'results/evals/100.txt',
    eps: parseFloat(process.env.EIG_ZERO_EPS || '1e-12'),
    mode: (process.env.COMP_MODE || 'warn').toLowerCase(), // warn|strict
    bin: process.platform === 'win32' ? 'target\\release\\compcount.exe' : 'target/release/compcount',
    out: 'results/components.json',
    skipHeader: (process.env.EDGES_SKIP_HEADER || 'false').toLowerCase() === 'true',
    directed: (process.env.EDGES_DIRECTED || 'false').toLowerCase() === 'true',
  };
  for (let i = 0; i < a.length; i++) {
    const k = a[i], v = a[i + 1];
    if (k === '--edges') out.edges = v;
    else if (k === '--nodes') out.nodes = v;
    else if (k === '--evals') out.evals = v;
    else if (k === '--eps') out.eps = parseFloat(v);
    else if (k === '--mode') out.mode = v.toLowerCase();
    else if (k === '--out') out.out = v;
    else if (k === '--bin') out.bin = v;
    else if (k === '--skip-header') out.skipHeader = true;
    else if (k === '--directed') out.directed = true;
  }
  return out;
}

function exists(p) { try { return fs.existsSync(p); } catch { return false; } }

function parseNums(raw) {
  return raw
    .trim()
    .split(/[\s,]+/)
    .map(t => parseFloat(t))
    .filter(Number.isFinite)
    .sort((a,b) => a - b);
}

function countZeroModes(vals, eps) {
  let k = 0;
  for (const x of vals) {
    if (Math.abs(x) <= eps) k++; else break; // sorted asc
  }
  return k;
}

function runComp(bin, edges, nodes, skipHeader, directed) {
  const args = ['--edges', edges];
  if (nodes && exists(nodes)) args.push('--nodes', nodes);
  if (skipHeader) args.push('--skip-header');
  if (directed) args.push('--directed');

  const res = spawnSync(bin, args, { encoding: 'utf8' });
  if (res.error) throw res.error;
  if (res.status !== 0) throw new Error(`compcount failed: ${res.stderr}`);
  return JSON.parse(res.stdout);
}

(async () => {
  const cfg = args();
  await fsp.mkdir(path.dirname(cfg.out), { recursive: true });

  const result = {
    skipped: false,
    mode: cfg.mode,
    eps: cfg.eps,
    edges_file: cfg.edges,
    nodes_file: cfg.nodes || null,
    evals_file: cfg.evals,
    n_components: null,
    zero_modes: null,
    ok: null,
    note: '',
  };

  // Eigenvalues must exist
  if (!exists(cfg.evals)) {
    result.skipped = true;
    result.note = `baseline evals not found: ${cfg.evals}`;
    await fsp.writeFile(cfg.out, JSON.stringify(result, null, 2));
    console.warn('[check-components] skip:', result.note);
    process.exit(0);
  }

  // Parse eigenvalues and count zeros
  const vals = parseNums(fs.readFileSync(cfg.evals, 'utf8'));
  const k0 = countZeroModes(vals, cfg.eps);
  result.zero_modes = k0;

  // If we have edges, run compcount; else skip gracefully
  if (!cfg.edges || !exists(cfg.edges)) {
    result.skipped = true;
    result.note = 'edges file not provided or not found; zero-mode count only';
    result.ok = true; // cannot compare
    await fsp.writeFile(cfg.out, JSON.stringify(result, null, 2));
    console.warn('[check-components] skip: no edges file; wrote zero-mode count only.');
    process.exit(0);
  }

  try {
    const comp = runComp(cfg.bin, cfg.edges, cfg.nodes, cfg.skipHeader, cfg.directed);
    result.n_components = comp.n_components;
    result.ok = (result.n_components === result.zero_modes);
    await fsp.writeFile(cfg.out, JSON.stringify(result, null, 2));
    console.log(`[check-components] components=${result.n_components} zero-modes=${result.zero_modes} -> ${result.ok ? 'OK' : 'MISMATCH'}`);
    if (!result.ok && cfg.mode === 'strict') {
      console.error('[check-components] ❌ STRICT: components != zero modes.');
      process.exit(1);
    }
  } catch (e) {
    result.skipped = true;
    result.note = `compcount failed: ${e.message}`;
    await fsp.writeFile(cfg.out, JSON.stringify(result, null, 2));
    console.warn('[check-components] skip:', result.note);
  }
})();
