# dola

Declarative Orchestration for Live Animation

## Status

⚠️ **Early Development**

This crate is usable, but it is still an early release. The API is not stable yet and may change significantly in future versions.

## About

**dola** is a platform-independent declarative animation framework inspired by Windows Animation Manager. It provides a data model for defining animations (variables, transitions, keyframes, storyboards) that can be serialized with serde (the optional `toml` feature reads and writes TOML via `DolaDocument::from_toml_str` / `to_toml_string`; any other format can be handled through the serde derives with a crate of your choice).

Key features:
- Declarative animation definitions (variables, transitions, storyboards)
- Serde-serializable data model (TOML via the optional `toml` feature; other formats through the serde derives with a crate of your choice)
- Easing function library (ease-in, ease-out, parametric curves)
- Cue sheets: absolute-time scripts broadcast to subscribers (the playback engine behind areka's Sakura Script runner)
- Validation system for animation consistency

## Usage

Not recommended for production use at this stage. Please check back for future releases.

## License

MIT
