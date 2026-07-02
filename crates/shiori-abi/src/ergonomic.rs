//! snake_case インヘレント安全面（プレースホルダ）。
//!
//! 旧 `ShioriExt` 拡張トレイト（raw `unsafe fn -> HRESULT` を `Result` へ変換する層）は Task 8.1 で
//! 撤去された。新しい安全面は「vtable 面（PascalCase）＝[`crate::interface`]／安全面（snake_case）＝
//! `IShioriFactory::create`・`IShiori::get`/`notify`・`IShioriHost::raise`/`complete`/`get_property`/
//! `set_property`」というインヘレント impl として **Task 8.3（SafeSurface）が再構築**する
//! （design.md §SafeSurface・R8.2/9.4/10.4/12.5）。
//!
//! それまでは本モジュールは意図的に空で、`shiori-abi` クレート単体をコンパイル可能な最小状態に保つ。
