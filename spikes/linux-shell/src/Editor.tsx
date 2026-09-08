import { useEffect, useRef } from 'react';
import * as monaco from 'monaco-editor/editor/editor.api.js';
import EditorWorker from 'monaco-editor/editor/editor.worker.js?worker';
const code = { alpha: 'export const greeting = "Hello Alpha";\n', beta: 'export const greeting = "Hello Beta";\n' };
const env = globalThis as typeof globalThis & { MonacoEnvironment?: { getWorker: () => Worker } };
env.MonacoEnvironment = { getWorker: () => new EditorWorker() };
export default function Editor({ project }: { project: keyof typeof code }) {
  const mount = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!mount.current) return;
    const original = monaco.editor.createModel(code[project], 'plaintext');
    const modified = monaco.editor.createModel(code[project].replace('Hello', 'Welcome'), 'plaintext');
    const editor = monaco.editor.createDiffEditor(mount.current, { theme: 'vs-dark', automaticLayout: true, minimap: { enabled: false }, fontSize: 12, renderSideBySide: false });
    editor.setModel({ original, modified });
    return () => { editor.dispose(); original.dispose(); modified.dispose(); };
  }, [project]);
  return <div className="editor" ref={mount} aria-label={`${project} diff editor`} />;
}
