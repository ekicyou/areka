//! `.nar`（ゴースト／バルーンの配布アーカイブ）を読み、呼び出し側から受け取った
//! ベースウェアの根へ入れるエンジン。
//!
//! 公開面はコンテナ読取・エントリ名の復号と検証・`install.txt` の解釈・配置計画・
//! 原子的な展開と `refresh`・閉じた拒否語彙（`RefuseReason`）で構成する。
//! 根の場所は決めず、`&Path` として受け取る（決めるのは `baseware-root-layout`）。
//! 検体の在処も知らない（窓口は開発専用の `sample-ghost-kit` にある）。

mod error;

pub use error::{
    ElementKind, ExistingState, InstalledElement, Integrity, IoPhase, ManifestWarning, NarError,
    RefuseReason, UnsafeWhy, Unsupported,
};
