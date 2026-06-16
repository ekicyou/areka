# Agentic SDLC and Spec-Driven Development

Kiro-style Spec-Driven Development on an agentic SDLC

## Project Context

### Paths
- Steering: `.kiro/steering/`
- Specs: `.kiro/specs/`

### Steering vs Specification

**Steering** (`.kiro/steering/`) - Guide AI with project-wide rules and context
**Specs** (`.kiro/specs/`) - Formalize development process for individual features

### Active Specifications
- Check `.kiro/specs/` for active specifications
- Use `/kiro-spec-status [feature-name]` to check progress

## Development Guidelines
- Think in English, generate responses in Japanese. All Markdown content written to project files (e.g., requirements.md, design.md, tasks.md, research.md, validation reports) MUST be written in the target language configured for this specification (see spec.json.language).

## Branch & Merge Strategy (PR-based)
- **Merge into `main` happens only via Pull Request.** Never push directly to the default branch.
- **1 feature = 1 branch = 1 PR**: a spec's whole lifecycle (requirements → design → tasks → implementation) runs on a single Claude Code (harness) worktree branch; integration is one squash-merged PR at completion.
- **Branches come from the harness worktree** — skills do NOT create/switch/delete branches. `/kiro-start` (entry) and `/kiro-complete` (exit) operate on the supplied worktree branch.
- Authority: `.kiro/steering/workflow.md` (branch strategy + completion procedure).

## Minimal Workflow
- Phase 0 (optional): `/kiro-steering`, `/kiro-steering-custom`
- Discovery: `/kiro-discovery "idea"` — determines action path, writes brief.md + roadmap.md for multi-spec projects
- Phase 1 (Specification):
  - Post-discovery single-spec entry: `/kiro-start {feature}` — on the harness worktree branch, inits the spec (consumes brief.md) and generates requirements (commits, no push). STOPs on the default branch.
  - Then step by step on the same branch:
    - `/kiro-validate-gap {feature}` (optional: for existing codebase)
    - `/kiro-spec-design {feature} [-y]`
    - `/kiro-validate-design {feature}` (optional: design review)
    - `/kiro-spec-tasks {feature} [-y]`
  - Without discovery / quick path: `/kiro-spec-quick {feature} [--auto]` or `/kiro-spec-init "description"` → `/kiro-spec-requirements {feature}`
  - Multi-spec: `/kiro-spec-batch` — creates all specs from roadmap.md in parallel by dependency wave
- Phase 2 (Implementation): `/kiro-impl {feature} [tasks]`
  - Without task numbers: autonomous mode (subagent per task + independent review + final validation)
  - With task numbers: manual mode (selected tasks in main context, still reviewer-gated before completion)
  - `/kiro-validate-impl {feature}` (standalone re-validation)
- Completion (explicit approval required): `/kiro-complete {feature}` — DoD gate → archive to `completed/` → final commit → **PR create + squash merge** (the only path into `main`). Use only when the developer explicitly approves.
- Progress check: `/kiro-spec-status {feature}` (use anytime)

## Skills Structure
Skills are located in `.claude/skills/kiro-*/SKILL.md`
- Each skill is a directory with a `SKILL.md` file
- Skills run inline with access to conversation context
- Skills may delegate parallel research to subagents for efficiency
- Additional files (templates, examples) can be added to skill directories
- `kiro-start` — post-discovery single-spec entry (init + requirements on the harness worktree branch; no branch creation, no push)
- `kiro-complete` — spec completion exit (DoD gate → archive → PR-based squash merge into `main`; the only path into the default branch)
- `kiro-review` — task-local adversarial review protocol used by reviewer subagents
- `kiro-debug` — root-cause-first debug protocol used by debugger subagents
- `kiro-verify-completion` — fresh-evidence gate before success or completion claims
- **If there is even a 1% chance a skill applies to the current task, invoke it.** Do not skip skills because the task seems simple.

## Development Rules
- 3-phase approval workflow: Requirements → Design → Tasks → Implementation
- Human review required each phase; use `-y` only for intentional fast-track
- Keep steering current and verify alignment with `/kiro-spec-status`
- Follow the user's instructions precisely, and within that scope act autonomously: gather the necessary context and complete the requested work end-to-end in this run, asking questions only when essential information is missing or the instructions are critically ambiguous.

## Steering Configuration
- Load entire `.kiro/steering/` as project memory
- Default files: `product.md`, `tech.md`, `structure.md`
- Custom files are supported (managed via `/kiro-steering-custom`)
