---
name: "speckit-coldread"
description: "（4.冷读测试） Spawn a fresh zero-context agent to answer key requirement questions from the spec documents alone, then close any semantic gaps so the spec stands on its own for downstream agents."
compatibility: "Requires spec-kit project structure with .specify/ directory"
metadata:
  author: "github-spec-kit"
  source: "templates/commands/coldread.md"
---


## User Input

```text
$ARGUMENTS
```

You **MUST** consider the user input before proceeding (if not empty).

## Pre-Execution Checks

**Check for extension hooks (before cold read)**:
- Check if `.specify/extensions.yml` exists in the project root.
- If it exists, read it and look for entries under the `hooks.before_coldread` key
- If the YAML cannot be parsed or is invalid, do not skip silently: tell the user that `.specify/extensions.yml` could not be read (include the parser error) and that no hooks were checked, including any mandatory (`optional: false`) hooks registered there, then continue normally
- Filter out hooks where `enabled` is explicitly `false`. Treat hooks without an `enabled` field as enabled by default.
- For each remaining hook, do **not** attempt to interpret or evaluate hook `condition` expressions:
  - If the hook has no `condition` field, or it is null/empty, treat the hook as executable
  - If the hook defines a non-empty `condition`, skip the hook and leave condition evaluation to the HookExecutor implementation
- When constructing command invocations from hook command names, replace dots (`.`) with hyphens (`-`). For example, `speckit.git.commit` → `/speckit-git-commit`.
- For each executable hook, output the following based on its `optional` flag:
  - **Optional hook** (`optional: true`):
    ```
    ## Extension Hooks

    **Optional Pre-Hook**: {extension}
    Command: `/{command}`
    Description: {description}

    Prompt: {prompt}
    To execute: `/{command}`
    ```
  - **Mandatory hook** (`optional: false`):
    ```
    ## Extension Hooks

    **Automatic Pre-Hook**: {extension}
    Executing: `/{command}`
    EXECUTE_COMMAND: {command}

    Wait for the result of the hook command before proceeding to the Goal.
    ```
    After emitting the block above you MUST actually invoke the hook and wait for it to finish before continuing. Run it the same way you would run the command yourself in this agent/session (the invocation may differ from the literal `{command}` id shown above, e.g. a skills-mode agent runs it as `/skill:speckit-...` or `$speckit-...`). Emitting the block alone does not run the hook.
- If no hooks are registered or `.specify/extensions.yml` does not exist, skip silently

## Goal

Verify that the current feature spec communicates every load-bearing requirement decision to an agent that has **zero conversation context** and can only read repository documents. A spec written right after a requirements discussion reads as self-evident to its author, but may contain semantic gaps for every downstream agent (`/speckit-plan`, `/speckit-tasks`, `/speckit-implement`). This command detects those gaps with a cold read: a fresh agent answers key questions from the documents alone, wrong answers are attributed to the documentation first, and the documents are fixed until a cold reader gets them right.

## Operating Constraints

- **Intent travels through documents only.** The sole channel from this session to the cold reader is the question list plus the documents themselves. Reference answers, conversation context, hints, and corrections MUST NOT enter the reader's prompt. An agent that shares this conversation's context is not a cold reader.
- **The reader is never the first suspect.** A wrong answer is by default a documentation defect: the spec failed to express the semantics on its own. Fix the document, not the reader.
- **This command MAY edit the spec** (and documents the spec explicitly references) to close gaps. It MUST NOT weaken or reinterpret a requirement to match a misreading; the fix direction is always to make the intended semantics explicit.
- **High information density is acceptable; semantic gaps are not.** Do not shorten or dilute the spec while fixing it.

## Execution Steps

### 1. Initialize Cold-Read Context

Run `.specify/scripts/bash/check-prerequisites.sh --json --paths-only` once from repo root and parse JSON for FEATURE_DIR. Derive:

- SPEC = FEATURE_DIR/spec.md

Abort with an error message if spec.md is missing (instruct the user to run `/speckit-specify` first).
For single quotes in args like "I'm Groot", use escape syntax: e.g 'I'\''m Groot' (or double-quote if possible: "I'm Groot").

### 2. Define the Reading Set

Load spec.md. Collect every project document the spec explicitly references (for example `.specify/memory/constitution.md`). The READING SET is exactly: spec.md plus those explicitly referenced documents — nothing else. The cold reader may read only the READING SET, and every question must in principle be answerable from the READING SET alone.

### 3. Build the Question Set (Examiner Phase)

As the examiner you know the intended semantics from the requirements discussion. Write 8–15 questions that a downstream agent must answer correctly from the READING SET alone, prioritized by implementation impact of a wrong answer (an answer that would change what gets built ranks above anything cosmetic).

Cover these archetypes where applicable:

- **Entities & relationships**: "What are X and Y? What is their relationship, and how do they collaborate?"
- **Decisions & boundaries**: "For decision D, what was chosen? Which alternatives were explicitly rejected or remain out of scope?"
- **Scenario reasoning**: "In scenario S, what is the required behavior? Which entity is the source of truth afterward?"
- **Term traps**: terms a cold reader might silently replace with an industry default that differs from this spec's meaning.

For each question, also record the **reference answer** (the intended semantics). Reference answers are private grading material:

- They MUST NOT appear in the questions, and MUST NOT be passed to the reader in any form.
- Questions must not leak or lead toward the answer.
- If you cannot state a reference answer because the decision itself is unsettled, do not invent one — flag it for the user instead (Step 6).

### 4. Spawn the Cold Reader (Isolated Fresh Agent)

Launch a brand-new subagent through this environment's subagent/Task mechanism with exactly the following prompt, substituting the bracketed values (the question list contains questions only):

```text
You are a cold reader. You have NOT participated in any discussion about this
project and have no prior context beyond this prompt.

Read ONLY these files:
- [SPEC path]
- [paths of other READING SET documents, one per line; omit if none]

Rules:
- Base every answer strictly on these documents. Do not fill gaps with
  industry defaults, common patterns, or assumptions.
- Do not ask anyone for clarification; nobody will answer.
- If the documents do not determine the answer, answer "NOT DETERMINED"
  and state precisely what information is missing. That is a valid and
  important answer.
- Support every answer with a short verbatim quote and its location. An
  answer without documentary support does not count.

Questions:
[Q1..Qn — questions only, no reference answers]

Answer format, one block per question:
Q<n>: <your answer, or NOT DETERMINED>
Evidence: "<quote>" (<file>:<section>)
```

If this environment provides no way to launch an isolated agent, STOP and instruct the user: save the question list to a temporary file, open a brand-new session with no prior context, have that agent answer from the READING SET only, then paste the answers back here and continue with Step 5.

### 5. Grade and Attribute

Compare each answer against its reference answer and classify:

- **PASS** — materially consistent with the reference answer.
- **GAP** — a reasonable cold reading of the documents yields a materially different answer (including answers anchored in a quote that legitimately supports the wrong reading — that is ambiguity, the worst kind of gap).
- **NOT DETERMINED** — the reader could not determine the answer.

Attribution rule: treat GAP and NOT DETERMINED as documentation defects by default. Only if the reference answer is present verbatim at an obvious location and the reader still missed it, retest that single question with one additional fresh reader (same prompt form). If the second reader passes, treat it as reader noise and move on; otherwise it is still a documentation clarity problem.

### 6. Close the Gaps

For every confirmed GAP / NOT DETERMINED, edit the document in the READING SET where the semantics belong (usually spec.md) so the answer becomes explicit and locatable:

- define the term at its first use;
- state entity relationships and collaboration in one authoritative place;
- record the decision, its rationale, and the rejected alternatives;
- spell out the scenario outcome and the resulting source of truth.

If the intended semantics are themselves unsettled (you have no authoritative answer from the discussion), do NOT guess a fix: present the question to the user in the clarification format used by `/speckit-specify` (or hand off to `/speckit-clarify`), then encode the answer into the spec.

### 7. Re-Verify (Max 2 Rounds)

After applying fixes, re-ask only the previously failed questions to a NEW fresh reader (never reuse a reader that has already seen the unfixed documents). Repeat grading and fixing. Stop after 2 re-verification rounds; anything still failing is reported as unresolved.

### 8. Report

Output the report in the conversation (do not write a report file; the durable artifact is the fixed spec):

## Cold-Read Report

| Q | Archetype | Verdict (final) | Rounds | Gap Closed By |
|---|-----------|-----------------|--------|---------------|
| Q1 | Scenario reasoning | PASS | 2 | Defined sync source-of-truth in spec Data Flow |

**Per-question detail** (for every question that was not an initial PASS): reader answer vs reference answer, the documentary evidence the reader cited, what was missing, the edit applied.

**Metrics:**

- Questions asked
- Initial: PASS / GAP / NOT DETERMINED
- Resolved by doc fixes
- Unresolved

**Verdict:** either "spec.md is cold-read self-contained for a zero-context agent" or the list of unresolved questions with a recommended next action (`/speckit-clarify` when a decision is unsettled, manual edit otherwise).

## Mandatory Post-Execution Hooks

**You MUST complete this section before reporting completion to the user.**

Check if `.specify/extensions.yml` exists in the project root.
- If it does not exist, or no hooks are registered under `hooks.after_coldread`, skip to the Completion Report.
- If it exists, read it and look for entries under the `hooks.after_coldread` key.
- If the YAML cannot be parsed or is invalid, do not skip silently: tell the user that `.specify/extensions.yml` could not be read (include the parser error) and that no hooks were checked, including any mandatory (`optional: false`) hooks registered there, then continue to the Completion Report.
- Filter out hooks where `enabled` is explicitly `false`. Treat hooks without an `enabled` field as enabled by default.
- For each remaining hook, do **not** attempt to interpret or evaluate hook `condition` expressions:
  - If the hook has no `condition` field, or it is null/empty, treat the hook as executable
  - If the hook defines a non-empty `condition`, skip the hook and leave condition evaluation to the HookExecutor implementation
- When constructing command invocations from hook command names, replace dots (`.`) with hyphens (`-`). For example, `speckit.git.commit` → `/speckit-git-commit`.
- For each executable hook, output the following based on its `optional` flag:
  - **Mandatory hook** (`optional: false`) — **You MUST emit `EXECUTE_COMMAND:` for each mandatory hook**:
    ```
    ## Extension Hooks

    **Automatic Hook**: {extension}
    Executing: `/{command}`
    EXECUTE_COMMAND: {command}
    ```
    After emitting the block above you MUST actually invoke the hook and wait for it to finish before continuing. Run it the same way you would run the command yourself in this agent/session (the invocation may differ from the literal `{command}` id shown above, e.g. a skills-mode agent runs it as `/skill:speckit-...` or `$speckit-...`). Emitting the block alone does not run the hook.
  - **Optional hook** (`optional: true`):
    ```
    ## Extension Hooks

    **Optional Hook**: {extension}
    Command: `/{command}`
    Description: {description}

    Prompt: {prompt}
    To execute: `/{command}`
    ```

## Completion Report

Report completion to the user with:

- SPEC path and the READING SET
- Question count and final verdict summary
- List of edits applied to the spec (section-level)
- Unresolved questions, if any
- Recommended next step: `/speckit-plan` when the verdict is clean

## Operating Principles

- The reader is never the first suspect — the document is. Fix the document, not the reader.
- Intent travels through documents only; the prompt channel carries questions, never answers.
- Test decisions, not trivia: a question whose wrong answer changes nothing is not worth asking.
- High information density is acceptable; semantic gaps are not.
- Never weaken a requirement to make a misreading correct; make the intended meaning explicit instead.
- A fresh reader per round; never reuse a reader that has already seen earlier document states.

## Context

$ARGUMENTS
