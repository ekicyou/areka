# dola

Declarative Orchestration for Live Animation

## Status

⚠️ **Early Development**

This crate is usable, but it is still an early release. The API is not stable yet and may change significantly in future versions.

## About

**dola** is a platform-independent declarative animation framework inspired by Windows Animation Manager. It provides a data model for defining animations (variables, transitions, keyframes, storyboards) that can be serialized with serde (the optional `toml` feature reads and writes TOML via `DolaDocument::from_toml_str` / `to_toml_string`; any other format can be handled through the serde derives with a crate of your choice).

The optional `test-support` feature exposes `dola::cue::test_support`, a helper that builds a canonical command-carrier `TalkCue` for tests. Enable it only from `[dev-dependencies]`; it is not part of the stable API and is not meant for production builds.

Key features:
- Declarative animation definitions (variables, transitions, storyboards)
- Serde-serializable data model (TOML via the optional `toml` feature; other formats through the serde derives with a crate of your choice)
- Easing function library (ease-in, ease-out, parametric curves)
- Cue sheets: scripts of offsets from an origin stamped at dispatch (`absolute_start_time: Option<TimePoint>`), broadcast to subscribers (the playback engine behind areka's Sakura Script runner)
- Validation system for animation consistency

## Time

All times in dola are integers of 100 ns ticks. Three types keep their meanings apart:

- `TimePoint`: a moment on the clock (`i64` ticks).
- `TimeSpan`: a length, never negative.
- `TimeDelta`: a signed difference between two moments.

`TimeScale` is a playback speed (finite and positive). It maps elapsed time to script time, rounding toward negative infinity so that a moment is never reported before it has arrived.

### Reading "now"

On Windows, `dola::now()` is the single clock. It reads `QueryUnbiasedInterruptTimePrecise` and returns a `TimePoint`. Time while the machine sleeps or hibernates is not counted, so deadlines and cue playback measured on this clock do not advance across a sleep. The clock has no per-process origin and no scale. On other targets `now()` does not exist; the types, conversions and serde adapters compile everywhere.

### Converting from author units

| From | To | Rule |
| --- | --- | --- |
| milliseconds (`u32`) | `TimeSpan::from_millis` | exact (1 ms = 10,000 ticks) |
| milliseconds (`u64`) | `TimeSpan::try_from_millis` | `Err` when out of range |
| milliseconds (`i64`) | `TimeDelta::from_millis_saturating` | clamps to the ends |
| `std::time::Duration` | `TimeSpan::try_from` | rounds to the nearest 100 ns; `Err` only when out of range |
| seconds as a real number | `try_from_secs_f64` on `TimeSpan` / `TimeDelta` | multiplied by 10^7 and rounded to the nearest tick |

Rounding goes to the nearest 100 ns, and an exact tie goes away from zero. Failures are reported as `TimeRangeError`: `NotFinite` (NaN or infinity), `Negative` (a negative `TimeSpan`) and `OutOfRange`. A ratio multiplication that builds a length (`TimeSpan::mul_div`, `mul_unit`) uses the same rounding.

### Text form

`Display` writes seconds with exactly 7 decimal places (`1.5000000`, `-0.0000001`). `FromStr` reads `[-]integer[.fraction]` with at most 7 fraction digits (`3`, `-2`, `1.5`, `0.0000001`). Parsing fails with `TimeParseError` (`Malformed` or `OutOfRange`).

### Serialization

- Storyboard documents write `TimeSpan` / `TimeDelta` fields as seconds. A document value must satisfy `|ticks| < 2^51`; within that range, writing seconds and rounding back to the nearest tick returns the original integer.
- Cue sheets write `TimePoint` / `TimeSpan` / `TimeDelta` fields as integer ticks.

## Usage

Not recommended for production use at this stage. Please check back for future releases.

## License

MIT
