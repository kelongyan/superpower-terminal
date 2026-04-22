# UI Beautification Plan - Scheme B

## Goal

Implement Scheme B with a progressive rollout that keeps the app stable while improving visual quality in measurable stages.

## Phase breakdown

### Phase 1 - Foundation polish

Scope:
- Rebalance spacing, paddings, and control sizing
- Add hover-state driven button/tab feedback
- Improve top bar, status bar, and settings panel visual hierarchy
- Make destructive actions visually distinct

Tasks:
1. Expand layout constants for a less cramped chrome
2. Extend `AppTheme` with hover and subtle emphasis colors
3. Add hover tracking to app state and feed it into `build_ui_model`
4. Update button/tab rendering to react to hover and active state
5. Refine status bar copy hierarchy and panel section spacing

Deliverable:
- Noticeably cleaner and more responsive chrome without renderer changes

### Phase 2 - Modern container language

Scope:
- Introduce rounded rectangles for primary chrome surfaces
- Improve panel/card treatment
- Add softer container separation than plain 1px lines

Tasks:
1. Extend renderer UI primitive data to support corner radius
2. Add rounded-rect shader path for chrome quads
3. Apply rounded corners to tabs, buttons, settings cards, and terminal container
4. Tune active/inactive surfaces across all three themes

Deliverable:
- Modernized shape language with better component grouping

### Phase 3 - Depth and overlays

Scope:
- Add lightweight shadow layers for floating surfaces
- Implement polished search overlay chrome
- Improve tab affordances and settings panel presentation

Tasks:
1. Add shadow helper primitives using layered quads
2. Add floating treatment to settings/search surfaces
3. Build a dedicated search overlay container and action row
4. Add richer close-button affordances and overflow polish for tabs

Deliverable:
- Scheme B completed with stronger depth and modern utility surfaces

## Execution order

1. Land Phase 1 first and validate behavior
2. Use Phase 1 visuals as baseline before shader work
3. Implement Phase 2 renderer changes in isolated commits
4. Finish with Phase 3 overlays and depth tuning

## Validation for every phase

- `cargo fmt`
- `cargo check`
- `cargo test`

## Notes

- Phase 1 is intentionally renderer-safe and low risk
- Phase 2 is the first stage that changes GPU primitives
- Phase 3 should build on stable shapes from Phase 2 instead of mixing concerns
