import React, { lazy, Suspense, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import './style.css';

type PtySnapshot = { id: number; pid: number | null; output: string; running: boolean };
type Snapshot = { terminals: PtySnapshot[]; denied_commands: number; preview_reports: string[] };
type Project = 'alpha' | 'beta';
const Editor = lazy(() => import('./Editor'));

function Lead() {
  return <aside className="floating"><strong>Lead overlay · shell proof</strong><input aria-label="Lead draft" placeholder="Type to test focus over Preview"/><span>Voice capture not implemented</span></aside>;
}

function Workbench() {
  const [project, setProject] = useState<Project>('alpha');
  const [streams, setStreams] = useState<string[][]>([[], [], [], []]);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState('');
  const [tab, setTab] = useState(0);
  const [drafts, setDrafts] = useState<Record<Project, string>>({ alpha: '', beta: '' });
  useEffect(() => {
    let count = 0;
    const timer = setInterval(() => { count++; setStreams(old => old.map((lines, i) => [...lines.slice(-39), `Synthetic stream ${i + 1} · tick ${count}`])); }, 100);
    return () => clearInterval(timer);
  }, []);
  useEffect(() => {
    let disposed = false;
    let busy = false;
    const timer = setInterval(() => {
      if (busy) return;
      busy = true;
      void invoke<Snapshot>('snapshot').then(value => { if (!disposed) setSnapshot(value); }).catch((e: unknown) => { if (!disposed) setError(String(e)); }).finally(() => { busy = false; });
    }, 250);
    return () => { disposed = true; clearInterval(timer); };
  }, []);
  return <main>
    <header><h1>Symbiote <small>Linux shell proof · not production</small></h1><label>Project <select value={project} onChange={e => setProject(e.target.value as Project)}><option value="alpha">Alpha</option><option value="beta">Beta</option></select></label></header>
    <section className="streams">{streams.map((lines, i) => <article key={i}><strong>Synthetic stream {i + 1}</strong><pre>{lines.slice(-4).join('\n')}</pre></article>)}</section>
    <section className="left"><h2>{project} · real Monaco diff</h2><Suspense fallback={<p>Loading editor…</p>}><Editor project={project}/></Suspense><label>Project-local draft <input value={drafts[project]} onChange={e => setDrafts({ ...drafts, [project]: e.target.value })}/></label><p>Draft restores across switching in memory. No durable Host restore claimed.</p></section>
    <section className="preview-label"><h2>Untrusted loopback Preview · separate native WebView</h2><p>Fixed rectangle: resize/composition remain explicit proof obligations.</p></section>
    <section className="terminals"><nav>{[0, 1, 2].map(i => <button aria-pressed={tab === i} key={i} onClick={() => setTab(i)}>PTY {i + 1} · PID {snapshot?.terminals[i]?.pid ?? 'pending'}</button>)}<button onClick={() => void invoke('stop_ptys').catch((e: unknown) => setError(String(e)))}>Stop PTYs</button></nav><pre>{snapshot?.terminals[tab]?.output ?? 'Connecting to native PTYs…'}</pre></section>
    <footer>Denied native commands: {snapshot?.denied_commands ?? 0} · Preview probes: {snapshot?.preview_reports.join(' | ') ?? 'pending'} {error && <strong role="alert">{error}</strong>}</footer>
  </main>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode>{location.hash === '#lead' ? <Lead/> : <Workbench/>}</React.StrictMode>);
