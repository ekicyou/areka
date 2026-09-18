//! メニューの OS 表示（areka-P0-popup-menu-minimal）。
//!
//! 計画から `HMENU` を組み立て、`TrackPopupMenuEx` で表示して選ばれた識別子を返す。
//! フォアグラウンドの作法とクライアント座標からスクリーン座標への変換もここに置く。
//! メニュー module の `unsafe` はこのファイルだけに閉じる。
