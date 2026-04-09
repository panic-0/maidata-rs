# maidata-note-export Merge Rules

This document records the intended merge pipeline for `maidata-note-export`.

## Phase Order

1. Build touch groups.
   Touch and touch-hold notes at the same timestamp are grouped by distance connectivity.
   This is a logical grouping step first; output materialization can happen later.

2. Resolve slide with tap / slide.
   Cases:
   - Slide head + tap at the same timestamp and same start position:
     drop the tap and keep the slide.
   - Pure tail 1 + head 2 contact, same position, `end(1) == start(2)`:
     do **not** merge by itself.
     Boundary contact alone is not considered overlap.
   - Slide tail 1 + slide tail 2, same position, `end(1) == end(2)`:
     find the maximal common suffix. Remove duplicated judgment zones from slide 2.
     If the remaining slide 2 cannot still be represented by a connection slide, emit
     `NOTE WARNING` and skip the whole chart.
   - Slide head 1 + slide head 2, same position, `start(1) == start(2)`:
     symmetric to shared-tail. Remove duplicated prefix from slide 2.
     If the remaining slide 2 cannot still be represented by a connection slide, emit
     `NOTE WARNING` and skip the whole chart.
   - Temporary restriction:
     Y-like "merge in the middle" patterns are forbidden for now.
     When encountered, emit `NOTE WARNING` and skip the whole chart.

3. Resolve touch-group with slide.
   A touch group can be deleted by a slide only when **every** touch in that group is within
   the distance threshold of the slide note being merged against.
   If any member is too far, the whole group stays and none of it merges into the slide.

4. Materialize remaining touch groups into output notes.

## Overlap Threshold

Slide-slide merge requires actual overlapped judgment zones in the middle.
Sharing only a single boundary checkpoint is not enough.

Examples:
- `1-3` and `3-5`:
  do not merge.
- `1-3`, `3-5`, `2-4`:
  can collapse transitively to `1-5`, because the middle slide creates true overlap on
  `2-3` and `3-4`, not just boundary contact.

## Current Engineering Rule

The exporter is allowed to be conservative.
When an overlap cannot be converted into a stable linear slide representation, the exporter
should prefer `NOTE WARNING` + skip-chart over emitting ambiguous slide ownership.
