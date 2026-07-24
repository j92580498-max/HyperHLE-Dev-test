# Agent capability log

This file records what coding-agent configurations have actually accomplished
on tapHLE. It helps a maintainer choose an agent and tells reviewers how much
independent verification a contribution needs. It is not a general benchmark
or a claim that a model will always behave the same way.

Record the exact model, surface, effort/reasoning setting, date, bounded task,
and verified result. Separate a useful lead from an independently safe commit.
Model names, hosted surfaces, prompts, and implementations can change, so add a
new dated row instead of silently rewriting an old result.

## Observed configurations

| Date | Model | Agent surface | Effort | tapHLE result | Current use |
| --- | --- | --- | --- | --- | --- |
| 2026-07-19 | OpenAI GPT-5 session | Codex | Not recorded | Produced reviewed, tested Windows compatibility checkpoints and continuation documentation across the current Ricky, Percy, Fantastic Mr. Fox, and Baby Monkey work. | Suitable as the primary implementation agent, with normal maintainer review and exact-app validation. |
| 2026-07-23 | Terra | Codex | Not recorded | On a bounded SPYmouse task, isolated successive deterministic iPhone OS API blockers and produced a Release-built compatibility checkpoint against the user-confirmed IPA. It did not reach a rating threshold or receive independent review. | Treat as a lead/checkpoint candidate only; require stronger-agent or human review plus an exact Windows rerun before merge or database reporting. |
| 2026-07-19 | Google Gemini 3.1 Pro | Antigravity | High | Found a useful lead around an unhandled `write`, but its proposed `EBADF` behavior was incomplete because Baby Monkey was writing to standard error. It did not independently reach a safe checkpoint and also produced unrelated success stubs, contradictory AdSupport handling, a stale app note, and an untested `_dladdr` frontier. | Use for bounded leads or reviewable subtasks only. A stronger agent or human must review the diff and rerun the exact Windows artifact before trusting a result. |
| 2026-07-18 | Not recorded | Terra | Not recorded | Maintainer experiment did not independently advance the active compatibility work. | Do not use as the sole debugger or checkpoint authority. A narrow maximum-effort experiment is allowed if its output is independently reviewed and retested. |
| 2026-07-18 | Not recorded | Luna | Not recorded | Maintainer experiment did not independently advance the active compatibility work. | Do not use as the sole debugger or checkpoint authority; treat output as an unverified lead. |

Do not infer that one app result proves broad emulator competence. Add concise
rows for materially different model versions, surfaces, or effort settings.
The app continuation note should hold technical details; this table should hold
only enough evidence to guide agent selection and review.

## Commit attribution

Every materially contributing agent gets a `Co-authored-by:` trailer when a
verified name/email identity is available. Add exact model/surface metadata as
separate Git trailers when known:

```text
Co-authored-by: OpenAI Codex <codex@openai.com>
Agent-model: GPT-5
Agent-surface: Codex
```

```text
Co-authored-by: Google Antigravity <antigravity@google.com>
Agent-model: Google Gemini 3.1 Pro
Agent-surface: Antigravity
Agent-effort: High
```

Use the product's exact displayed model name; for example, include `High` in
`Agent-effort`, not in the model name, when it is an effort setting. If the
model or effort is not exposed, write `Not recorded` in this log and omit that
commit trailer rather than guessing.

If a tool has no verified co-author email, use an `Assisted-by:` trailer with
its displayed name and add a row here. Do not invent an email address or
attribute its work to the maintainer. Once a verified identity is established,
add it to this section for future commits without rewriting published history.
