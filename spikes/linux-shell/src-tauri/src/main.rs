use gtk::prelude::*;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};
use tauri::{Manager, Webview, WebviewUrl, webview::WebviewBuilder, window::WindowBuilder};

const OUTPUT_LIMIT: usize = 16_384;
type Output = Arc<Mutex<VecDeque<u8>>>;
struct Terminal {
    id: usize,
    child: Box<dyn Child + Send + Sync>,
    _master: Box<dyn MasterPty + Send>,
    output: Output,
}
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
struct ProofState {
    terminals: Mutex<Vec<Terminal>>,
    denied: AtomicUsize,
    reports: Arc<Mutex<Vec<String>>>,
    shutdown: Arc<AtomicBool>,
}
impl ProofState {
    fn stop(&self) -> Result<(), String> {
        let mut terminals = self
            .terminals
            .lock()
            .map_err(|_| "terminal lock poisoned")?;
        for terminal in terminals.iter_mut() {
            let status = match terminal.child.try_wait().map_err(|e| e.to_string())? {
                Some(status) => status,
                None => {
                    terminal.child.kill().map_err(|e| e.to_string())?;
                    terminal.child.wait().map_err(|e| e.to_string())?
                }
            };
            println!("PROOF_PTY_STOP id={} status={status:?}", terminal.id);
        }
        Ok(())
    }
}
#[derive(Serialize)]
struct TerminalSnapshot {
    id: usize,
    pid: Option<u32>,
    output: String,
    running: bool,
}
#[derive(Serialize)]
struct Snapshot {
    terminals: Vec<TerminalSnapshot>,
    denied_commands: usize,
    preview_reports: Vec<String>,
}

fn trusted(label: &str, url: &tauri::Url) -> bool {
    label == "workbench"
        && url.scheme() == "tauri"
        && url.host_str() == Some("localhost")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
}
fn authorize(webview: &Webview, state: &ProofState) -> Result<(), String> {
    let url = webview.url().map_err(|e| e.to_string())?;
    if !trusted(webview.label(), &url) {
        state.denied.fetch_add(1, Ordering::Relaxed);
        println!("PROOF_COMMAND_DENIED label={}", webview.label());
        return Err("untrusted WebView label/origin".into());
    }
    Ok(())
}
#[tauri::command]
fn snapshot(webview: Webview, state: tauri::State<'_, ProofState>) -> Result<Snapshot, String> {
    authorize(&webview, &state)?;
    let mut terminals = state
        .terminals
        .lock()
        .map_err(|_| "terminal lock poisoned")?;
    let terminals = terminals
        .iter_mut()
        .map(|terminal| {
            let output = terminal.output.lock().map_err(|_| "output lock poisoned")?;
            let bytes: Vec<u8> = output.iter().copied().collect();
            Ok(TerminalSnapshot {
                id: terminal.id,
                pid: terminal.child.process_id(),
                output: String::from_utf8_lossy(&bytes).into(),
                running: terminal
                    .child
                    .try_wait()
                    .map_err(|e| e.to_string())?
                    .is_none(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Snapshot {
        terminals,
        denied_commands: state.denied.load(Ordering::Relaxed),
        preview_reports: state
            .reports
            .lock()
            .map_err(|_| "report lock poisoned")?
            .clone(),
    })
}
#[tauri::command]
fn stop_ptys(webview: Webview, state: tauri::State<'_, ProofState>) -> Result<(), String> {
    authorize(&webview, &state)?;
    state.stop()
}

fn terminals() -> Result<Vec<Terminal>, Box<dyn std::error::Error>> {
    let mut terminals = Vec::new();
    for id in 0..3 {
        let pair = native_pty_system().openpty(PtySize {
            rows: 24,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        // Acquire every fallible PTY handle before starting a child process.
        let mut reader = pair.master.try_clone_reader()?;
        let mut cmd = CommandBuilder::new("python3");
        cmd.args(["-I", "-u", "-c", "import os,time\nprint('real PTY: stdin=%s stdout=%s' % (os.isatty(0),os.isatty(1)))\ni=0\nwhile True:\n print('PTY workload tick',i,flush=True)\n i+=1\n time.sleep(.25)"]);
        let child = pair.slave.spawn_command(cmd)?;
        println!("PROOF_PTY_START id={id} pid={:?}", child.process_id());
        drop(pair.slave);
        let output: Output = Arc::new(Mutex::new(VecDeque::new()));
        let sink = output.clone();
        thread::spawn(move || {
            let mut buf = [0; 2048];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let Ok(mut queue) = sink.lock() else { break };
                        queue.extend(&buf[..n]);
                        while queue.len() > OUTPUT_LIMIT {
                            queue.pop_front();
                        }
                    }
                    Err(error) => {
                        println!("PROOF_PTY_READER_END id={id} reason={error}");
                        break;
                    }
                }
            }
        });
        terminals.push(Terminal {
            id,
            child,
            _master: pair.master,
            output,
        });
    }
    Ok(terminals)
}

const PREVIEW: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Untrusted Preview</title></head><body style="font:16px system-ui;background:#fff3db;color:#30261b;padding:25px"><h1>Untrusted application Preview</h1><p>This is a separate native WebView on an ephemeral loopback origin.</p><label>Preview focus test <input placeholder="Type here then in floating Lead"></label><button onclick="this.textContent='Clicked at '+new Date().toISOString()">Interact</button><pre id="status">Running attempted native command probes…</pre><script>
async function probe(command) {
 let result;
 try {
  if (!window.__TAURI_INTERNALS__) result = command + ': IPC absent';
  else { await window.__TAURI_INTERNALS__.invoke(command); result = command + ': UNEXPECTED ALLOWED'; }
 } catch (_) { result = command + ': denied'; }
 document.getElementById('status').textContent += '\n' + result;
 await fetch('/report', {method:'POST',body:result});
}
probe('snapshot').then(()=>probe('stop_ptys'));
</script></body></html>"#;

fn preview_server(
    reports: Arc<Mutex<Vec<String>>>,
    shutdown: Arc<AtomicBool>,
) -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    listener.set_nonblocking(true)?;
    thread::spawn(move || {
        while !shutdown.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
                    let mut request = vec![0; 8192];
                    let mut used = 0;
                    loop {
                        match stream.read(&mut request[used..]) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => used += n,
                        }
                        let text = String::from_utf8_lossy(&request[..used]);
                        if let Some((headers, body)) = text.split_once("\r\n\r\n") {
                            let length = headers
                                .lines()
                                .find_map(|line| {
                                    line.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .and_then(|v| v.trim().parse::<usize>().ok())
                                })
                                .unwrap_or(0);
                            if body.len() >= length {
                                break;
                            }
                        }
                        if used == request.len() {
                            break;
                        }
                    }
                    let request = String::from_utf8_lossy(&request[..used]);
                    let report = request.starts_with("POST /report ");
                    if report && let Some((_, body)) = request.split_once("\r\n\r\n") {
                        let body: String =
                            body.chars().filter(|c| !c.is_control()).take(128).collect();
                        println!("PROOF_PREVIEW_REPORT {body}");
                        if let Ok(mut reports) = reports.lock()
                            && reports.len() < 16
                        {
                            reports.push(body);
                        }
                    }
                    let body = if report { "ok" } else { PREVIEW };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25))
                }
                Err(error) => {
                    eprintln!("PROOF_PREVIEW_SERVER_ERROR {error}");
                    break;
                }
            }
        }
    });
    Ok(port)
}

fn main() {
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![snapshot, stop_ptys])
        .setup(|app| {
            let reports = Arc::new(Mutex::new(Vec::new()));
            let shutdown = Arc::new(AtomicBool::new(false));
            let port = preview_server(reports.clone(), shutdown.clone())?;
            app.manage(ProofState {
                terminals: Mutex::new(terminals()?),
                denied: AtomicUsize::new(0),
                reports,
                shutdown,
            });
            let window = WindowBuilder::new(app, "proof")
                .title("Symbiote experimental Linux shell")
                .inner_size(1400.0, 900.0)
                .build()?;
            window.add_child(
                WebviewBuilder::new("workbench", WebviewUrl::App("index.html".into()))
                    .on_navigation(|url| trusted("workbench", url)),
                tauri::LogicalPosition::new(0.0, 0.0),
                tauri::LogicalSize::new(1400.0, 900.0),
            )?;
            let origin = format!("http://127.0.0.1:{port}");
            let allowed = origin.clone();
            window.add_child(
                WebviewBuilder::new(
                    "untrusted-preview",
                    WebviewUrl::External(format!("{origin}/").parse()?),
                )
                .incognito(true)
                .on_navigation(move |url| url.origin().ascii_serialization() == allowed)
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny),
                tauri::LogicalPosition::new(700.0, 270.0),
                tauri::LogicalSize::new(675.0, 360.0),
            )?;
            window.add_child(
                WebviewBuilder::new("lead-overlay", WebviewUrl::App("index.html#lead".into()))
                    .on_navigation(|url| trusted("workbench", url)),
                tauri::LogicalPosition::new(1040.0, 520.0),
                tauri::LogicalSize::new(320.0, 110.0),
            )?;
            // Tauri's Linux default VBox stacks sibling WebViews. Explicit native layout
            // preserves separate trust surfaces while testing the required overlap.
            let container = window.default_vbox()?;
            let children = container.children();
            if children.len() != 3 {
                return Err("unexpected native child layout".into());
            }
            let fixed = gtk::Fixed::new();
            for (child, (x, y, width, height)) in children.iter().zip([
                (0, 0, 1400, 900),
                (700, 270, 675, 360),
                (1040, 520, 320, 110),
            ]) {
                container.remove(child);
                child.set_size_request(width, height);
                fixed.put(child, x, y);
            }
            container.add(&fixed);
            fixed.show_all();
            println!(
                "PROOF_READY preview_origin={origin} pid={} synthetic_streams=4 real_ptys=3",
                std::process::id()
            );
            if let Ok(seconds) = std::env::var("SHELL_PROOF_SECONDS") {
                let seconds: u64 = seconds.parse()?;
                if !(1..=300).contains(&seconds) {
                    return Err("SHELL_PROOF_SECONDS must be 1..300".into());
                }
                let handle = app.handle().clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(seconds));
                    handle.exit(0);
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("proof app setup failed");
    app.run(|handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            let state = handle.state::<ProofState>();
            state.shutdown.store(true, Ordering::Relaxed);
            if let Err(error) = state.stop() {
                eprintln!("PROOF_CLEANUP_FAILED {error}");
            }
            println!("PROOF_EXIT");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_command_origin_and_webview_label_are_both_required() {
        let app: tauri::Url = "tauri://localhost/index.html".parse().unwrap();
        assert!(trusted("workbench", &app));
        for label in ["untrusted-preview", "lead-overlay", "proof", ""] {
            assert!(!trusted(label, &app));
        }
        for url in [
            "http://127.0.0.1:1234/",
            "https://localhost/",
            "tauri://evil/",
            "tauri://user@localhost/",
        ] {
            assert!(!trusted("workbench", &url.parse().unwrap()));
        }
    }
}
