# Phase 02b — Objective-Driven Decomposition & Planning

## Problem

A goal like *"como fazer uma redação do ENEM"* is decomposed into vague topic
buckets ("Fundamentos de Redação", "Técnicas de Redação", "Prática"). The
planner then receives **only the book title and the time budget**, so it writes
generic steps ("Study: Técnicas de Redação"). The resolver finally searches for
that generic phrase, which is why the resources are unrelated.

The content-quality work in Phase 04b cannot compensate for an imprecise
objective. Fix the contract at the top of the pipeline first.

## Root causes in the current code

| Stage | Current contract | Consequence |
|---|---|---|
| Goal | `title` + optional `why_statement`; the title may be a question | No measurable outcome to aim at |
| Decomposer | `{title, description, weight}`; prompt says "books or modules" | Topic buckets, not objectives |
| Planner input | `PlanRequest { book_title, planned_duration_mins, language }` | No goal, why, description, or outcome reaches the planner |
| Planner output | `{title, activity_kind, duration, references_index, activity_type}` | Steps have no learning outcome or success criterion |
| Resolver | `topic = book.title`, `step = step.title` | Queries are built from vague labels |
| Judge | Would score against the same vague labels | Even a perfect judge has no precise target |

## The contract

### 1. Goal outcome (new)

The decomposer first normalizes the goal into something observable:

```json
{
  "goal_outcome": "Escrever uma redação dissertativa-argumentativa do ENEM com nota 800+ em 60 minutos, aplicando as cinco competências da matriz de correção.",
  "goal_success_criteria": [
    "Produz uma redação completa em até 60 minutos",
    "Justifica a nota de cada competência com evidências do texto",
    "Usa repertório sociocultural pertinente e uma proposta de intervenção completa"
  ]
}
```

Stored on `goals.outcome` and `goals.success_criteria`. Editable via
`PATCH /goals/{id}`.

### 2. Module objective (replaces the vague "book" description)

Each module must answer: *what can the learner do after this module that they
could not do before?*

```json
{
  "title": "Matriz de correção e diagnóstico",
  "objective": "Ao final deste módulo, o aluno consegue explicar as cinco competências da matriz de correção do ENEM e classificar trechos de uma redação modelo em cada competência, justificando a nota.",
  "scope_in": [
    "as cinco competências",
    "níveis 0-5 de cada competência",
    "evidências textuais que sobem ou derrubam a nota"
  ],
  "scope_out": ["temas de atualidades", "gramática avançada"],
  "prerequisites": [],
  "assessment": "Corrigir uma redação nota 1000 atribuindo nota 0-5 por competência, com uma justificativa por nota.",
  "weight": 2.0
}
```

Rules:

- `objective` must start with an observable verb ("explicar", "identificar",
  "escrever", "corrigir", "comparar", "aplicar") and state the standard.
- `scope_in` is 3-5 concrete topics; `scope_out` is 1-3 explicit exclusions.
- `assessment` is a task that proves the objective, not a topic.
- Modules are ordered fundamentals → application → assessment.
- The set of module objectives must cover the goal without overlap.

### 3. Step learning outcome (new)

The planner receives the goal outcome and the module objective, and must make
every step advance a named part of it:

```json
{
  "title": "O que cada competência realmente avalia",
  "learning_outcome": "O aluno lista as cinco competências e explica o que cada uma penaliza.",
  "success_criteria": "Acerta pelo menos 4 de 5 afirmações sobre o que cada competência avalia.",
  "content_intent": "explanation",
  "activity_kind": "resource",
  "estimated_duration_mins": 20,
  "activity_type": "video"
}
```

`content_intent` is one of `explanation | worked_example | practice |
assessment | review`. It drives both the content query and the activity type:

| Intent | What the resource should be | Query shape |
|---|---|---|
| explanation | concept explained clearly | "o que é / como funciona X" |
| worked_example | a model, walkthrough or annotated example | "exemplo / análise / passo a passo X" |
| practice | exercises, simulator, rubric to apply | "exercício / simulado / corrigir X" |
| assessment | quiz or graded task | generated quiz or task |
| review | spaced retrieval of earlier material | earlier step outcome |

## Prompt changes

### Decomposer system prompt (new)

```
You are an expert curriculum designer. Decompose a learning goal into an
ordered sequence of modules. A module is a measurable objective, not a topic
bucket.

Return only JSON:
{"goal_outcome":"string","goal_success_criteria":["string"],
 "books":[{"title":"string","objective":"string","scope_in":["string"],
 "scope_out":["string"],"prerequisites":["string"],"assessment":"string",
 "weight":number}]}

Rules:
- Write all user-facing text in {language}.
- goal_outcome: one sentence, observable, with a standard.
- objective: "Ao final deste módulo, o aluno consegue <verb> <what> <standard>."
- scope_in: 3-5 concrete topics or skills.
- scope_out: 1-3 things explicitly out of scope.
- prerequisites: earlier module titles or prior knowledge.
- assessment: a concrete task that proves the objective.
- Order fundamentals -> application -> assessment.
- Do not use generic titles such as "Fundamentos", "Introdução" or "Prática"
  unless the objective is specific.
- Produce between {min_books} and {max_books} modules.
```

Include the goal title, why, horizon, and total available minutes in the user
message.

### Planner system prompt (new)

```
You are an expert instructional designer. Create a study plan for ONE module
that achieves its objective within the available time.

Input: goal outcome, module objective, scope, assessment, prerequisites, and
the previous/next module objectives.

Return only JSON:
{"steps":[{"title":"string","learning_outcome":"string",
 "success_criteria":"string","content_intent":"explanation|worked_example|practice|assessment|review",
 "activity_kind":"resource|quiz|task","estimated_duration_mins":integer,
 "references_index":integer|null,"activity_type":"video|pdf|article|podcast|quiz|task"}]}

Rules:
- Every step must state an observable learning_outcome and a measurable
  success_criteria.
- The final step is an assessment that matches the module assessment.
- explanation and worked_example steps use resource content;
  practice uses tasks or exercises; assessment uses a quiz or task.
- Use review steps to reference earlier steps.
- Do not include resource_url: the application supplies real links.
- Write all user-facing text in {language}.
```

## Data model (migration `0008_objective_contract.sql`)

```sql
ALTER TABLE goals ADD COLUMN outcome TEXT NOT NULL DEFAULT '';
ALTER TABLE goals ADD COLUMN success_criteria JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE books ADD COLUMN objective TEXT NOT NULL DEFAULT '';
ALTER TABLE books ADD COLUMN scope_in JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE books ADD COLUMN scope_out JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE books ADD COLUMN prerequisites JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE books ADD COLUMN assessment TEXT NOT NULL DEFAULT '';

ALTER TABLE steps ADD COLUMN learning_outcome TEXT NOT NULL DEFAULT '';
ALTER TABLE steps ADD COLUMN success_criteria TEXT NOT NULL DEFAULT '';
ALTER TABLE steps ADD COLUMN content_intent TEXT NOT NULL DEFAULT 'explanation'
    CHECK (content_intent IN ('explanation','worked_example','practice','assessment','review'));
```

The Phase 04b migration becomes `0009_content_curation.sql`.

## API changes

- `GoalResponse` gains `outcome` and `success_criteria`.
- `PATCH /goals/{id}` accepts `outcome` and `success_criteria`.
- `BookResponse` gains `objective`, `scope_in`, `scope_out`, `prerequisites`,
  `assessment`.
- New `PATCH /books/{id}` to edit the module contract before generating a plan.
- `StepResponse` gains `learning_outcome`, `success_criteria`, `content_intent`.
- `POST /goals/{id}/redecompose` re-runs decomposition for legacy goals;
  destructive, allowed only when no step is completed.
- **UI:** call books **módulos**; table and API keep `books`.

## Validation and fallbacks

- The decomposer replaces an empty, short or generic objective with a
  deterministic template and de-duplicates module titles; it never silently
  accepts a topic bucket.
- Deterministic fallback: `"Ao final, o aluno consegue aplicar {title}."` plus
  `"Demonstrar {title} em uma tarefa prática."` as the assessment.
- The planner fills a missing `learning_outcome` or `success_criteria` and
  appends an assessment step when the model omits one.
- If the model returns no usable modules/steps, `FallbackDecomposer` and
  `FallbackPlanner` use the mock implementations.
- The LLM output is validated against the requested module count and the
  available time; invalid entries are repaired or dropped rather than silently
  stored.

## Cascade into content curation (Phase 04b)

`ContentBrief` and `ResolveRequest` must consume the new contract:

```rust
ContentBrief {
    goal_outcome: String,
    module_objective: String,
    step_outcome: String,
    success_criteria: String,
    content_intent: String,
    scope_in: Vec<String>,
    // ... existing terms, language, time budget, type
}
```

Consequences:

- `must_terms` come from the objective + `scope_in`, not the book title.
- Web queries are shaped by `content_intent` (explanation vs example vs
  practice), which is what makes the SearXNG results specific.
- The relevance judge scores against `step_outcome + success_criteria`, not
  "Study: Técnicas de Redação".
- The micro-lesson fallback teaches the step outcome, not the book title.

## Worked example — ENEM

Current output (vague):

1. Fundamentos de Redação do ENEM
2. Técnicas de Redação
3. Prática de Redação

Objective-driven output:

1. **Matriz de correção e diagnóstico** — explain the 5 competencies and grade
   passages of a model essay with justification.
2. **Estrutura dissertativa-argumentativa** — write an introduction with thesis
   and repertoire, two body paragraphs, and an intervention proposal in 50 min.
3. **Repertório e coesão** — select and connect relevant sociocultural
   repertoire without repetition.
4. **Simulado cronometrado** — produce a complete essay scoring 800+ in 60 min.

Module 1 planner steps:

1. "O que cada competência realmente avalia" — list the 5 competencies and what
   each penalizes. *explanation*
2. "Anatomia de uma redação nota 1000" — identify one passage that satisfies
   each competency. *worked_example*
3. "Corrigir uma redação em pares" — assign a score per competency with textual
   evidence. *practice*
4. "Quiz de competências" — 4/5 correct on what each competency evaluates.
   *assessment*

Resolver queries now become `redacao enem cinco competencias`, `redacao nota
1000 analise`, `corrigir redacao competencias`, instead of
`fundamentos redacao enem`.

## Tasks

- [x] Migration `0008_objective_contract.sql`.
- [x] Extend `Goal`, `Book`, `Step` models and responses.
- [x] New decomposer prompt/schema + deterministic validation.
- [x] Extend `PlanRequest` and the planner prompt/schema.
- [x] Store the new fields in `routes/goals.rs` and `routes/plan.rs`.
- [x] `PATCH /books/{id}`.
- [x] `POST /goals/{id}/redecompose` (blocked when any step is completed).
- [x] Update mock decomposer/planner to emit objectives for tests.
- [ ] Update `ContentBrief`/`ResolveRequest` in Phase 04b.
- [ ] UI: say "módulos"; show the goal outcome, module objective, step outcome
      and criteria.

Status (2026-10-26): backend slices 1–4 are implemented; `cargo test` is green
(86 tests). The mobile prototype now edits the goal outcome, module contract and
step outcomes, and can redecompose a goal. Phase 04b consumes the new fields.

## Verification (2026-10-26)

Live run against the real OpenRouter free models:

- Goal creation returned a specific outcome ("produzir uma redação ... 800+")
  and four modules with measurable objectives and assessments.
- The first module plan returned specific step outcomes and criteria, ending
  with an assessment step.
- `PATCH /books/{id}` and `POST /goals/{id}/redecompose` round-trip correctly.
- The mock decomposer/planner now respect pt-BR and also emit objectives, so the
  fallback is no longer English-only.

## Known risks

- **Free-model latency.** A live plan took 233 s because the primary free model
  returned empty completions and the client retried. Empty completions now skip
  to the next fallback model immediately, but a 1–3 minute plan is still
  possible. Mitigations: pin a faster model, set per-role
  `LLM_PLANNER_TIMEOUT_SECS` / `LLM_PLANNER_MAX_RETRIES`, or move plan
  generation to an async job with polling before Phase 09.
- **Free-model typos.** The live output contained "ortaxe" and "differentes".
  A stronger model or a short proofread pass fixes this; it is not part of this
  slice.

## Exit criteria

- Creating the ENEM goal produces modules with measurable objectives, not topic
  buckets.
- Every generated step has a learning outcome and a success criterion.
- The resolver receives the step outcome and module objective (verified in the
  trace).
- A module's final step is an assessment matching the module assessment.
- Existing tests pass; new tests cover objective validation, the ENEM fixture,
  and the planner contract.

## Decision log (2026-10-26)

| Decision | Choice |
|---|---|
| Goal normalization | Decomposer normalizes the goal into `outcome` + `success_criteria`; stored and editable |
| Module assessment | Every module ends with a generated assessment step matching its `assessment` |
| Existing goals | Add `POST /goals/{id}/redecompose`, allowed only when no step is completed |
| Naming | UI says **módulos**; table and API keep `books` |
| Module count | LLM chooses 1–6 within horizon bounds, validated by the server |
| Module editing | Add `PATCH /books/{id}` for objective, scope, prerequisites and assessment |

## Execution slices

1. `feat(plan): objective contract migration and models` — migration, Goal/Book/Step
   fields, responses, mock updates.
2. `feat(plan): objective-driven goal decomposition` — new decomposer prompt,
   schema, validation, goal outcome persistence.
3. `feat(plan): objective-driven module planner` — planner prompt/schema,
   learning outcomes, success criteria, content intent, assessment step.
4. `feat(plan): edit and redecompose` — `PATCH /books/{id}`,
   `POST /goals/{id}/redecompose`, `PATCH /goals/{id}` outcome fields.
5. `feat(ui): module objectives and step outcomes` — show "módulos", objective,
   scope, assessment, step outcome and criteria.

Slices 1–3 are the backend contract. Slice 4 makes it recoverable. Slice 5 makes
it visible. Phase 04b starts only after slice 3.
