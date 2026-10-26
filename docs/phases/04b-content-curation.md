# Phase 04b — Content Curation & Quality

> **Depends on [02b — Objective-Driven Decomposition & Planning](02b-objective-decomposition.md).**
> The brief and the relevance judge score against the goal outcome, module
> objective, and step learning outcome defined there. Do Phase 02b first.

## Goal

Make every activity point at content that is **short, relevant, in the right
language, and actually usable** — or, when nothing qualifies, at a generated
micro-lesson that serves the same learning objective.

This phase extends Phase 04 (resolver + classifier). It does not replace the
connectors or the deterministic ranker; it wraps them in a curation pipeline
with hard quality gates, an LLM judge, a lesson fallback, and a feedback loop.

## Problem

The first live ENEM plan produced activities with useless resources:

- PDFs were hundreds of pages, unrelated to the step, and often scanned
  (no text layer).
- Videos were off-topic.
- The curated fallback was a generic category homepage, so the step-by-step
  plan had no learning value.

Root causes in the current code:

| Symptom | Where | Cause |
|---|---|---|
| Query is indirect / generic | `content/query.rs` | One keyword bag per step; no per-connector query shaping; goal/book context not passed; objective not captured |
| Title match ≠ relevant content | `content/rank.rs` | Scores title overlap only; never reads article body, video description, or PDF text |
| Long PDFs | `content/mod.rs` | Takes `candidates.first()`; no page/word budget tied to `estimated_duration_mins` |
| Scanned PDFs | `connectors/internet_archive.rs` | Filters `_text.pdf` + `imagecount`, but never verifies a text layer |
| Off-topic videos | `connectors/youtube.rs` | Duration window exists; title/description are not checked against the step |
| Pointless fallback | `content/fallback.rs` | Static category homepages, unrelated to the step |
| No learning signal | none | Connector trust is hardcoded; no user feedback; no resolution trace |

## Target pipeline

```
goal + book + step + time budget + language
  → 1. Brief      objective, key terms, must/exclude terms, type, max length
  → 2. Queries    one shaped query per connector (video / article / pdf)
  → 3. Retrieve   all connectors, ~30 candidates
  → 4. Verify     language, type, length, text layer, liveness, body overlap
  → 5. Curate     deterministic score + optional LLM judge over top K
  → 6. Decide     best candidate | micro-lesson fallback | generic task
  → 7. Persist    activity + selection reason + trace
  → 8. Learn      feedback → connector/domain stats → future ranking
```

## How unrelated content is actually rejected

Length and language are form checks; they do not measure relevance. Relevance
needs its own gate, applied in layers. A candidate must pass every layer.

1. **Retrieval anchoring (P1).** Queries are built from the learning objective,
   not the step title alone, and each connector gets a query shaped for its
   medium. This raises the chance that the pool contains relevant material.
2. **Lexical prefilter (P2).** Language, type, duration/pages, text layer,
   liveness, `must_terms` present, `exclude_terms` absent. Cheap, but it only
   rejects obvious mismatches — a keyword-matching wrong page still passes.
3. **Evidence-based semantic judge (P3).** The final candidate is always shown
   to an LLM together with a bounded excerpt of its *actual content*:

   | Type | Excerpt sent to the judge |
   |---|---|
   | article | first ~1,500 chars of readable body |
   | PDF | first 2 pages of extracted text |
   | video | title + description + channel + duration |
   | academic | title + abstract |

   The judge must return a **verbatim `evidence` quote** from that excerpt that
   teaches the step objective, plus `relevant` and a score. If there is no
   quote, or the quote is not a substring of the excerpt, the candidate is
   rejected. This is what separates "shares keywords" from "teaches this".
4. **Decision.** Highest accepted score wins. If nothing is accepted, the
   activity becomes a micro-lesson (P4) — never a bad resource.

The `ambiguous` mode only controls whether the judge also **re-ranks** the top
5. It must still validate the deterministic winner. Otherwise a confident
keyword match is never checked at all.

### ENEM rubric step — worked example

Objective: *"understand the ENEM essay correction rubric (five competencies)"*.

| Candidate | What rejects it |
|---|---|
| "Anatocismo" (Wikipedia) | Judge: excerpt is compound interest; no evidence quote for the rubric |
| "Redação jurídica" article | Judge: different domain; no evidence quote |
| 300-page "Manual de redação" PDF | P2: page cap |
| Scanned PDF | P2: no text layer |
| 12-min "Como fazer a redação do ENEM" video | Accepted only if the description quotes the competencies/rubric |
| "Rubrica de correção do ENEM" article | Accepted, with the supporting quote stored in `selection_reason` |
| Nothing accepted | P4 generates the micro-lesson |

**Honest trade-off:** rejection is not discovery. SearXNG widens the pool,
but this instance currently returns usable results only from Bing (Google is
empty; DuckDuckGo/Startpage are CAPTCHA'd; Brave is rate-limited). Query order
and the judge matter more than raw result count. When nothing passes, the
lesson fallback is the safety net.

### SearXNG (verified 2026-10-26)

- Endpoint: `http://127.0.0.1:8082/search` (POST form, `format=json`).
- The JSON API was disabled (`formats: [html]`). Enabled `json` in
  `/home/pedro/searxng/core-config/settings.yml` (timestamped backup next to it)
  and restarted `searxng-core`.
- Engine availability from this IP:
  - **works:** `bing` — must be requested explicitly; it is `disabled: true` in
    the default config, but `engines=bing` returns results.
  - **empty:** `google` (HTTP 200, zero results), `mojeek` (disabled).
  - **blocked/limited:** `duckduckgo` (CAPTCHA), `startpage` (CAPTCHA), `brave`
    (429), `aol` (HTTP error), `qwant` (access denied).
- `categories=videos` returns YouTube results with `length`, `iframe_src`,
  `metadata` (channel), and `author`; no YouTube Data API quota is consumed.
- **Query order matters.** `redacao enem rubrica correcao` returns education
  pages; `rubrica correcao introducao redacao enem` returns dictionary/legal
  pages. The web query builder must put the book/topic terms first and drop
  generic title words (`introducao`, `fundamentos`, `basico`).
- Docker: the backend container reaches the host-published port via
  `host.docker.internal:8082` (add
  `extra_hosts: ["host.docker.internal:host-gateway"]` to `docker-compose.yml`);
  host runs use `127.0.0.1:8082`.

## Data model (migration `0009_content_curation.sql`)

`activities`:

- `content_body TEXT` — markdown for generated lessons (the activity itself).
- `content_meta JSONB` — probe results, score, trace, fallback reason.
- `quality_status TEXT NOT NULL DEFAULT 'ok'` — `ok | fallback | rejected`.
- `selection_reason TEXT` — short human-readable "why this resource".
- `type` check gains `lesson`.

New tables:

```sql
CREATE TABLE activity_feedback (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    activity_id UUID NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating SMALLINT NOT NULL CHECK (rating IN (-1, 1)),
    reasons TEXT[] NOT NULL DEFAULT '{}',
    comment TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (activity_id, user_id)
);

CREATE TABLE source_stats (
    connector TEXT NOT NULL,
    domain TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL,
    language TEXT NOT NULL,
    helpful INT NOT NULL DEFAULT 0,
    not_helpful INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (connector, domain, category, language)
);

CREATE TABLE resource_probe_cache (
    url_hash TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    kind TEXT NOT NULL,
    page_count INT,
    word_count INT,
    text_chars INT,
    language TEXT,
    ok BOOLEAN NOT NULL,
    reason TEXT,
    expires_at TIMESTAMPTZ NOT NULL
);
```

## Phases

### P0 — Trace and fixtures (foundation)

- Add `ResolutionTrace` (`brief`, `queries`, candidates with score + filter
  verdicts, chosen, fallback reason) and store it in `content_meta`.
- Add `GET /activities/{id}/trace` (owner-scoped) for debugging.
- Add recorded connector fixtures for the ENEM example and an offline
  integration test that runs the pipeline against them.
- **Exit:** for the ENEM plan we can explain every pick and every rejection.

### P1 — Brief, per-connector queries and SearXNG retrieval

- New `content/brief.rs`:

  ```rust
  struct ContentBrief {
      goal_outcome: String,       // normalized goal from Phase 02b
      module_objective: String,   // book.objective
      step_outcome: String,       // step.learning_outcome
      success_criteria: String,   // step.success_criteria
      content_intent: String,     // explanation | worked_example | practice | assessment | review
      scope_in: Vec<String>,      // book.scope_in
      topic_terms: Vec<String>,   // from objective + scope_in + step
      must_terms: Vec<String>,    // enem, competencias
      exclude_terms: Vec<String>,
      activity_type: String,      // planner hint: video | article | pdf
      max_minutes: u32,           // from estimated_duration_mins
      language: String,
  }
  ```

- Deterministic extraction first (reuse `query::tokens`, classifier, step
  prefixes). Optional LLM refinement behind `CONTENT_BRIEF_LLM=on`, strict JSON,
  ~300 max tokens, cached.
- Replace the single `query::build` call with `query::build_for(connector, brief)`:
  - web: **topic terms first**, then intent-specific terms; max 8
    (`explanation` → "o que é / como funciona"; `worked_example` → "exemplo /
    análise"; `practice` → "exercício / simulado"; `assessment` → quiz)
  - video: topic + step outcome + `aula` / `explicação`
  - article: topic + step
  - pdf: topic + `guia pdf` / `guide pdf`
- New `ConnectorKind::Searxng`:
  - `categories=general` for articles and `categories=videos` for videos; run
    the two requests in parallel.
  - `engines` configurable (`SEARXNG_ENGINES`, default `bing` on this host).
  - 10 results, 8–10 s timeout, skip cleanly when the instance is unreachable.
  - Normalize `length` (`18:49`) and `iframe_src` into duration/embed fields.
- `ResolveRequest` gains the Phase 02b fields (`goal_outcome`,
  `module_objective`, `step_outcome`, `success_criteria`, `content_intent`,
  `estimated_duration_mins`, `activity_kind`), so relevance and length are known
  before retrieval.
- **Exit:** the ENEM rubric step produces specific web/video/article queries;
  no stopwords or instruction verbs; tests cover PT and EN.

### P2 — Hard verification gates (fixes long/unrelated/scanned)

- New `content/verify.rs`, run only on the top K cheap-ranked candidates
  (`CONTENT_VERIFY_TOP_K`, default 5), concurrently with a small limit.
- **Deterministic relevance gate (applies to every connector):**
  - require at least one `must_terms` hit in the title, excerpt or body;
  - zero body/abstract overlap → reject, **including academic connectors**
    (the current `is_academic` bypass is removed);
  - `exclude_terms` hit → reject.
  This is the cheap relevance filter; P3 adds the semantic check.
- **Articles:** fetch (timeout 8 s, cap 2 MB), extract main text (`scraper`),
  require 300–3000 words, PT/EN language ratio, at least one `must_terms` hit
  and no `exclude_terms` hit in the body, and reject paywall/login pages.
  Body-term overlap is a **prefilter only**; semantic relevance is decided in P3.
- **PDFs:** `HEAD` size ≤ 15 MB; add `POST /probe` to the PDF sidecar (first two
  pages only) returning `page_count`, `text_chars`, `has_text_layer`; require
  ≤ `CONTENT_MAX_PDF_PAGES` (default 30) and a real text layer; Internet Archive
  also requires `access-restricted-item = false`, `imagecount ≤ 40`, and a
  direct `_text.pdf` URL.
- **Videos:** duration inside `[CONTENT_MIN_VIDEO_SECS, min(CONTENT_MAX_VIDEO_SECS,
  max_minutes)]`, embeddable, not live, and at least one `must_terms` hit in the
  title or description.
- **Liveness:** final URL 2xx and content-type matches the claimed type.
- Cache probe results in `resource_probe_cache` (24 h).
- **Exit:** the ENEM fixture can no longer select a >30-page PDF, a scanned PDF,
  or an off-topic video.

### P3 — LLM relevance judge (semantic layer, old option D)

Runs after P2. It catches keyword matches that pass the deterministic gate but
do not actually teach the objective. The judge's target is
`step_outcome + success_criteria` from Phase 02b, never the vague book title.

**Chosen mode: `ambiguous`.** The important correction: even in `ambiguous`
mode, the deterministic winner is **always validated** by the evidence-quote
test. `ambiguous` only controls whether the judge *re-ranks* the top 5 when the
top two scores are close. A confident keyword match therefore cannot skip the
relevance check.

- New `content/curator.rs` using the shared `LlmClient`.
- Input per candidate: brief (objective, step, must/exclude terms, type, time
  budget) + a bounded excerpt (see the relevance section above) + title, domain,
  type, duration/pages.
- Output JSON:
  `{ decisions: [{ url, relevant, score 0-10, evidence, reason }] }`.
- Accept a candidate only when `relevant = true`, `score >= 7`, and `evidence`
  is a non-empty verbatim substring of the excerpt that was sent. Otherwise
  reject it.
- Validate every `url` against the candidate set; never accept a hallucinated
  URL.
- Try accepted candidates in score order; if none is accepted, go to P4.
- Modes: `off | ambiguous | always` (`CONTENT_CURATOR_MODE`). `always` re-ranks
  the top 5 every time; `ambiguous` re-ranks only when the deterministic top two
  are within a small margin.
- Fall back to the next deterministic candidate on timeout/error (never to an
  unvalidated one). Cache by `hash(brief + candidate excerpts)`.
- **Exit:** no candidate without a supporting evidence quote can be selected;
  the ENEM fixture rejects `Anatocismo` and the scanned/long PDFs even when the
  judge is not re-ranking.

### P4 — Micro-lesson fallback (user suggestion #1)

**Chosen behavior: automatic when nothing passes, plus an explicit button.**
Add `POST /steps/{id}/generate-lesson` for the button path.

- New `content/tutor.rs`.
- Triggered when no candidate passes verification, or curator confidence is
  below threshold.
- Prompt: 150–250 words, 3–5 bullets, one worked example, one quick exercise, in
  the user's language, no invented links or citations.
- Stored as `type = 'lesson'`, `content_body = <markdown>`,
  `source_connector = 'llm_tutor'`, `quality_status = 'fallback'`.
- Max 400 tokens, temperature ~0.4, cached by `(objective, language)`.
- If the LLM is unavailable, keep the existing generic task.
- **Exit:** an ENEM step with no usable video returns a lesson that actually
  teaches the rubric.

### P5 — Feedback loop (user suggestion #3)

**Chosen behavior: record the rating and expose a manual "Another" action.**
A 👎 never triggers an automatic LLM or connector call.

- `POST /activities/{id}/feedback`
  `{ rating: "helpful" | "not_helpful", reasons: [...], comment? }`.
- Reason chips: `too_long`, `unrelated`, `not_digitized`, `broken_link`,
  `wrong_language`, `too_advanced`, `other`.
- On `not_helpful`: store, set `quality_status = 'rejected'`, exclude that URL
  for the user+topic, and offer the next candidate.
- `POST /steps/{id}/regenerate-resource` accepts `reason` and `exclude_url` and
  never returns a URL the user rejected.
- Update `source_stats`; add a Bayesian-smoothed connector/domain adjustment to
  `rank.rs` (small weight, so it cannot override hard gates).
- `GET /admin/content-quality` reports fallback rate, rejection rate, worst
  connectors and domains.
- **Exit:** a 👎 on a resource makes the next regeneration avoid it; stats are
  visible to an admin.

### P6 — Source policy

SearXNG retrieval is already in P1. This phase makes the source mix pickier.

- Domain policy (config or table): allowlist, denylist, paywall domains,
  preferred domains per category/language (e.g. Brasil Escola, Mundo Educação,
  Descomplica, Khan Academy, gov.br/inep, Wikipedia).
- **Exit:** the domain policy removes known-bad sources and boosts known-good
  ones without adding a new provider.

Future option (documented, not implemented): Brave/Tavily providers behind the
same `ResourceCandidate` interface.

### P7 — Activity UX and regeneration

- Activity card shows: type, language, duration or pages, source domain, a short
  "why this" line, and actions **Not useful**, **Another**, **Short lesson**.
- Generated lessons render markdown inline (reuse the PDF content renderer).
- Regeneration replaces the resource and preserves the trace.
- **Exit:** the user can recover from a bad resource without editing data.

## Config additions

```env
CONTENT_VERIFY_TOP_K=5
CONTENT_MAX_PDF_PAGES=30
CONTENT_MAX_PDF_BYTES=15000000
CONTENT_MIN_ARTICLE_WORDS=300
CONTENT_MAX_ARTICLE_WORDS=3000
CONTENT_MIN_VIDEO_SECS=180
CONTENT_MAX_VIDEO_SECS=1200
CONTENT_BRIEF_LLM=off
CONTENT_CURATOR_MODE=ambiguous     # off | ambiguous | always
CONTENT_FALLBACK_LESSON=auto      # auto | on_demand | off
CONTENT_WEB_SEARCH_PROVIDER=searxng  # none | searxng | brave | tavily
SEARXNG_URL=http://127.0.0.1:8082
SEARXNG_ENGINES=bing
SEARXNG_LANGUAGE=pt-BR
SEARXNG_CATEGORIES=general,videos
SEARXNG_MAX_RESULTS=10
SEARXNG_TIMEOUT_SECS=10
LLM_CURATOR_MODEL=
LLM_CURATOR_FALLBACK_MODELS=
LLM_TUTOR_MODEL=
LLM_TUTOR_FALLBACK_MODELS=
```

## Testing

- **Unit:** brief extraction, per-connector queries, each verification gate,
  feedback-adjusted ranking, curator JSON validation.
- **Fixtures:** recorded connector responses for the ENEM plan; deterministic
  pipeline test with no network.
- **Integration:** `generate_plan` over fixtures → every activity either passes
  the gates or is a lesson; no rejected URL is reused.
- **Manual:** run the ENEM goal against live connectors and inspect
  `/activities/{id}/trace`.

## Non-goals for this phase

- OCR of scanned PDFs (we reject them; OCR can be a later option).
- YouTube transcript extraction (title/description only for now).
- Vector/embedding search (the LLM judge covers the precision gap first).
- Search providers other than SearXNG (Brave/Tavily are future options).

## Decision log (2026-10-26)

| Decision | Choice |
|---|---|
| General web search | SearXNG at `http://127.0.0.1:8082`; JSON enabled; Bing is the only currently usable engine; queries are topic-first |
| LLM curator (old D) | `ambiguous` — always validate the winner by evidence quote; re-rank top 5 only when scores are close |
| PDF policy | Allow up to 30 pages, require a real text layer; reject scanned/restricted |
| Lesson fallback | Automatic when nothing passes, plus an explicit button |
| Feedback | Record rating + reasons; regeneration is manual ("Another") |
| Build order | Deterministic first (P0 → P1 incl. SearXNG → P2 → P4 → P5), judge last |

## Execution slices

Each slice is a reviewable commit on `feat/content-curation`:

0. `feat(plan): objective-driven decomposition and planning` — Phase 02b.
1. `feat(content): trace, fixtures and content_meta` — P0.
2. `feat(content): brief, per-connector queries and searxng connector` — P1.
3. `feat(content): verification gates and pdf probe` — P2.
4. `feat(content): llm micro-lesson fallback` — P4.
5. `feat(content): feedback, source stats and regenerate exclusions` — P5.
6. `feat(content): ambiguous llm judge` — P3.
7. `feat(content): domain policy and activity quality ui` — P6 + P7.

Slice 0 is the upstream contract: without a measurable objective, the gates
have no precise target. Slices 1–3 are the minimum that makes the ENEM example
stop returning long, unrelated or scanned resources. Slice 4 guarantees every
step still teaches something when the pool is empty. Slices 5–7 improve
precision over time.

## Status (2026-10-26)

Implemented end to end:

- Migration `0009_content_curation.sql`: activity lesson body/trace/quality/reason,
  `lesson` type, `activity_feedback`, `source_stats`, `resource_probe_cache`.
- `ContentBrief` and per-connector queries (topic-first web queries).
- SearXNG connector (`general` + `videos`, configurable engines, JSON API).
- Verification gates: article body extraction/length/language/overlap, video
  duration/metadata, PDF size/page/text-layer via `POST /probe`, domain policy.
- Deterministic relevance gate (must terms, zero-overlap reject, exclude terms).
- Ambiguous LLM judge with evidence-quote validation and a hard 45 s timeout.
- Micro-lesson tutor with auto/on-demand modes and a hard 45 s timeout.
- Activity feedback, `source_stats`, rejected-URL exclusions on regeneration.
- Resolution trace stored in `content_meta` and exposed at
  `GET /activities/{id}/trace`.
- Mobile prototype: module contract, step outcomes/content intent, generate
  lesson, load activity/trace, and feedback forms.

Live verification:

- A 4-step ENEM plan with the mock planner resolved in **16 s**: SearXNG
  returned YouTube results, a 40-minute video was rejected for duration, and
  the trace/feedback endpoints round-tripped.
- The real-LLM tutor attempt was blocked by OpenRouter free-model 429s/empty
  completions; the pipeline falls back to the curated resource and the API now
  reports "tutor unavailable or returned no lesson" instead of claiming it is
  unconfigured.

Deferred / known limitations:

- `resource_probe_cache` exists but the verifier does not read or write it yet.
- `source_stats` are recorded, and rejected URLs are excluded immediately, but
  the aggregate helpful/not-helpful signal is not yet applied to ranking.
- Domain policy is a hardcoded trusted/denied host list, not a config table.
- Recorded connector fixtures are not committed; unit tests and the live run
  cover the paths.

## Assumptions

- pt-BR is preferred; English remains an allowed fallback with its badge.
- The planner's `activity_type` is a hint, not a hard constraint: if no video
  passes verification, the resolver may pick an article or a lesson.
- Free LLM models can be slow; every LLM path is optional and timeout-bounded,
  and has a deterministic fallback.

