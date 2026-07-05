//! `EmoWorld`: emo 専用 per-ghost `bevy_ecs` World（wintf 本体 World とは分離）。
//!
//! 正規化 Surface 定義の常駐点。`SurfaceIndex`／`AliasMap`／`ShellSettings` をリソースとして、
//! `SurfaceId`／`SurfaceMaster`／`AtlasBinding` をコンポーネントとして保持する。スケール前提で
//! 定義・構造のみを保持し、合成済みビットマップ（大容量）は World に永続保持しない。本 spec では
//! Schedule/System を持たず、fold/compose が `&mut World`／`&World` を取る受動データストアとして使う。
