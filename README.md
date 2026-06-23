# Atlas

Repository lens or "Atlas". A small project to help seed claude cli with json and md files that points to where the project functions and files exist. SHOULD help with lessening file traversal in certain projects.

Atlas is a Tauri desktop app for quickly understanding a GitHub repository. It builds a best-effort developer onboarding map from real repository structure, then lets you export the results as an onboarding pack.

## What Atlas does

Atlas accepts a GitHub repository URL and produces:

- stack and framework detection
- likely entry points
- best-effort function/class/type symbol extraction
- approximate call trace from likely entry points
- monorepo/app scope detection
- AWS Well-Architected + UX/DX audit findings ranked P0-P5
- downloadable onboarding artifacts
- a `claude-snippet.md` file you can paste into `CLAUDE.md` to help future Claude sessions navigate the repo

Atlas is intentionally **best-effort**. It does not build a compiler-grade AST and does not guarantee complete call graphs. The goal is fast onboarding signal, not perfect static analysis.

## Onboarding pack

The export pack includes:

- `overview.md` — high-level repository overview
- `start-here.md` — suggested reading order, commands, entry points, warnings
- `symbols.json` — extracted functions/classes/types
- `call-trace.json` — approximate entry-point flow
- `repo-analysis.json` — full structured analysis
- `warnings.json` — truncation/coverage warnings
- `claude-snippet.md` — short navigation protocol for `CLAUDE.md`

Use **Export Pack → Save all to Downloads** to write a folder like:

```txt
~/Downloads/atlas-owner-repo-YYYY-MM-DD/
```

## Audit

Atlas includes a deterministic-first audit pass. It flags evidence-backed issues such as:

- possible hardcoded secrets
- security/auth TODOs
- dynamic code execution
- broad CORS origins
- external requests without visible timeout
- debug logging in non-test source

Audit findings are ranked P0-P5 and grouped by pillar:

- Operational Excellence
- Security
- Reliability
- Performance Efficiency
- Cost Optimization
- Sustainability
- Developer Experience
- User Experience

The LLM layer can add context and ranking, but findings must still cite concrete file/line evidence.

## Monorepo support

Before analysis, Atlas detects conventional workspace folders such as:

- `apps/*`
- `app/*`
- `packages/*`
- `services/*`
- `libs/*`

If multiple candidates are found, Atlas lets you choose a specific app/package or scan the whole repo with bounded limits.

## Requirements

- macOS/Linux development environment
- Rust toolchain
- Bun
- Tauri prerequisites for your OS
- GitHub token recommended for private repos or higher rate limits
- Optional: Claude CLI or AWS Bedrock model ARN for LLM summaries/audit context

## Configuration

Atlas stores settings in:

```txt
~/.config/atlas/config
```

It can read older config locations as fallback:

```txt
~/.config/repo-lens/config
~/.config/relevant-reviews/config
```

Supported settings:

```txt
model=
github_token=
aws_profile=
```

GitHub token resolution order:

1. saved app config
2. `GH_TOKEN`
3. `GITHUB_TOKEN`

## Development

Install dependencies:

```bash
cd app
bun install
```

Run the Tauri app:

```bash
cd app
bun run tauri dev
```

Do not use browser-only `bun run dev` for normal app testing because Atlas depends on Tauri APIs.

## Checks

Rust:

```bash
cd app/src-tauri
cargo check
```

TypeScript:

```bash
cd app
node node_modules/typescript/lib/_tsc.js --noEmit
```

Frontend build:

```bash
cd app
node node_modules/vite/bin/vite.js build
```

Note: if local `.bin/tsc` or `.bin/vite` shims are broken, reinstall dependencies:

```bash
cd app
rm -rf node_modules
bun install
```

## Project structure

```txt
app/
  src/                  React frontend
  src/components/       UI components
  src-tauri/            Rust/Tauri backend
    src/analyze.rs      repository analysis orchestration
    src/audit.rs        audit orchestration
    src/audit_rules.rs  deterministic audit rules
    src/repo_analyzer.rs stack/monorepo/file selection logic
    src/symbol_extractor.rs best-effort symbol extraction
    src/entry_tracer.rs approximate call trace
    src/onboarding_pack.rs export artifact builder
```

## Limitations

Atlas is designed for fast orientation. It may miss files or relationships when:

- GitHub returns truncated repository trees
- the repo exceeds bounded file limits
- generated or vendored files hide important logic
- framework routing conventions are unusual
- dynamic language patterns are too indirect for regex-based extraction

When Atlas reports warnings, treat the output as incomplete and verify with targeted source inspection.
