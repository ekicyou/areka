# wintf

Windows Tategaki Framework - Rust UI library with Japanese vertical text support

## Status

⚠️ **Early Development**

This crate is usable, but it is still an early release. The API is not stable yet and may change significantly in future versions.

## About

**wintf** (Windows Tategaki Framework) is a Rust UI framework for Windows that integrates Windows.UI.Composition, Direct2D, and DirectWrite with an ECS architecture. It provides GPU-composited transparent windows with click-through to other processes, Japanese vertical text rendering, and high-precision hit testing for desktop mascot applications like Ukagaka.

Key features include:
- ECS-based declarative UI management (bevy_ecs)
- Hardware-accelerated composition with Windows.UI.Composition
- Vertical and horizontal Japanese text support via DirectWrite
- Flexbox layout engine (Taffy)
- Advanced pointer event handling and drag system
- Per-monitor DPI awareness

## Usage

Not recommended for production use at this stage. Please check back for future releases.

## License

MIT
