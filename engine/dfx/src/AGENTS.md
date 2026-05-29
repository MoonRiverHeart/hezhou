# DFX System

Logging, crash handling, trace analysis, performance monitoring. cdylib crate for standalone FFI export.

## Overview
DfxSystem wraps Logger, CrashHandler, TraceAnalyzer, PerformanceMonitor behind OnceLock global. Exposes 25+ `dfx_*` extern "C" FFI functions. Provides 6 Rust macros (dfx_trace..dfx_fatal) + scoped trace macro.

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Fix logging | `logger.rs` | File output + buffered log, append+flush pattern |
| Fix crash handler | `crash.rs` | Stack trace capture, Windows SEH |
| Fix trace output | `trace.rs` | JSON format, save_to_file → traces/trace_latest.json |
| Fix perf metrics | `perf.rs` | FPS, frame time, CPU/memory — TODO: Linux /proc support |
| Add FFI function | `lib.rs` | #[unsafe(no_mangle)] extern "C" dfx_* pattern |

## Key Architecture
```
DfxSystem {
    logger: Arc<Mutex<Logger>>,
    crash_handler: Arc<Mutex<CrashHandler>>,
    trace_analyzer: Arc<Mutex<TraceAnalyzer>>,
    perf_monitor: Arc<Mutex<PerformanceMonitor>>,
}
GLOBAL_DFX: OnceLock<Arc<Mutex<DfxSystem>>>
```
- init_dfx() → sets GLOBAL_DFX (OnceLock, safe for single init)
- get_dfx() → clones Arc from OnceLock (None if not initialized)
- All FFI functions: null-pointer check + unsafe { (*system).xxx.lock().yyy() }

## Conventions
- FFI naming: `dfx_create`, `dfx_destroy`, `dfx_enable_xxx`, `dfx_set_xxx`, `dfx_get_xxx` (dfx_ prefix)
- Macros: `dfx_trace!`, `dfx_debug!`, `dfx_info!`, `dfx_warn!`, `dfx_error!`, `dfx_fatal!` — module + message + format args
- Scoped trace: `dfx_scoped_trace!(name, category)` → ScopedTrace RAII guard
- Log levels: Trace(0), Debug(1), Info(2), Warn(3), Error(4), Fatal(5)
- Log format: `[time][level][thread][module] message (file:line)`
- Trace output: JSON → traces/trace_latest.json (loaded by trace_viewer_demo)

## Anti-Patterns
- NEVER call DfxSystem methods without checking `get_dfx()` returns Some first
- NEVER skip null-pointer checks in FFI functions (system, module, message, path params)