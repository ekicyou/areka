# areka

Desktop mascot platform for Windows

## Status

⚠️ **Early Development - Version 0.0.1**

This crate is published for name reservation purposes. The API is not stable and may change significantly in future versions.

## About

**areka** is Ukagaka (伺か) baseware for Windows: an alternative to SSP that follows [ukadoc](https://ssp.shillest.net/ukadoc/manual/) and runs existing ghosts, shells, and balloons. It is built on the wintf UI framework.

Implemented in the alpha (2026-10-02):
- Ghost boot, dialogue, petting, right-click menu, and shutdown
- 32-bit SHIORI DLLs hosted in a separate 32-bit helper process
- `.nar` installation (menu or drag-and-drop) and network update
- Switching ghosts, shells, and balloons; remembering the last selection

The crate on crates.io is not the way to install areka for end users: the 32-bit helper executable is not built by `cargo install`. See the repository README for how to build the distribution zip.

## Usage

See <https://github.com/ekicyou/areka>.

## License

MIT
