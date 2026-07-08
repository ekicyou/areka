//! ghost 結線層の起動・終了統括（`GhostRuntime`／`boot`／`shutdown`）。
//!
//! task 1.5 時点では、仕様上すでに形が確定している起動失敗の器
//! （[`GhostBootError`]）のみを定義する。`GhostRuntime`／`boot` 本体は
//! task 3.1（起動手順の組み上げ）が、`shutdown` 統括は task 3.2（終了統括）
//! が実装する。`GhostShutdownError`（終了統括の段階的失敗収集）の型形状は
//! task 3.2 の実装ロジックに依存するため、本タスクでは定義しない。

use areka_parsers::package::MountError;

/// 起動失敗（design.md「Error Categories and Responses」）。
///
/// マウント解決の失敗（起点不在／読取不能／shell 不在・`MountError` の各
/// variant）を包む。呼び出し側（areka main）はこれを非致命として扱い、
/// ダミー窓・smoke ゲート等の骨格起動を継続する（要件 2.5・8.2）。e2e は
/// 明示 fail として扱う（design.md 該当節）。
///
/// 後続タスクで新たな起動失敗種別が増える可能性に備え `#[non_exhaustive]`
/// を付す。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GhostBootError {
    /// descript.txt 起点のマウント解決が失敗した（要件 2.1・2.5）。
    ///
    /// `MountError` 自体は `areka-parsers` 側の純粋なデータ型（`Display`／
    /// `std::error::Error` 未実装）であるため、`Debug` 表現をメッセージへ
    /// 埋め込む。
    #[error("ghost mount resolution failed: {0:?}")]
    Mount(MountError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn mount_variant_constructs_and_displays() {
        let err = GhostBootError::Mount(MountError::StartPointMissing {
            expected: PathBuf::from("ghost/master/descript.txt"),
        });

        let rendered = err.to_string();
        assert!(
            rendered.contains("ghost mount resolution failed"),
            "unexpected Display output: {rendered}"
        );
        assert!(
            rendered.contains("StartPointMissing"),
            "Display should surface the underlying MountError variant: {rendered}"
        );
    }

    #[test]
    fn mount_variant_is_a_std_error() {
        let err = GhostBootError::Mount(MountError::ShellDirMissing {
            expected: PathBuf::from("ghost/master/shell/master"),
        });

        // 呼び出し側が `Box<dyn std::error::Error>` 等で一律に扱えることの確認。
        let as_std_error: &dyn std::error::Error = &err;
        assert!(as_std_error.source().is_none());
    }
}
