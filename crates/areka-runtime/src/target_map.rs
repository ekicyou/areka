//! scope → 表示対象（TargetId）写像の正本（純関数）。
//!
//! areka-P0-emo2-boot が正本として確立した契約（同 spec の design.md「依存方向」の最左・要件 3.5）。
//! areka-P0-emo2-boot 設計 DD-3 の採番規約（シェル表示対象＝`2*scope`／バルーン表示対象＝`2*scope+1`）で
//! scope からシェル／バルーン両表示対象への互いに素な写像を与え、`ActorKey` の数値 parse
//! （数でない、または大きすぎる番号は `None`）を担う。
//!
//! 不変条件（areka-P0-emo2-boot 設計 DD-3）: `shell_target` は偶数・`balloon_target` は奇数の `TargetId` を返すため、
//! 全 scope にわたりシェル表示対象とバルーン表示対象は決して衝突しない（互いに素）。
//! これらの関数と逆写像 `target_scope` のみが採番の正本であり、areka-P0-emo2-boot の M-dual の拡張はこのモジュールだけを触る。

use areka_emo_present::TargetId;
use areka_sakura::ActorKey;
use tracing::warn;

/// scope の上限（`balloon_target` の `2*scope+1` が u32 に収まる最大）。台本の側と窓の側の正本。
const MAX_SCOPE: u32 = i32::MAX as u32;

/// scope 番号 → シェル表示対象（正本）。
///
/// areka-P0-emo2-boot 設計 DD-3 の採番規約により `TargetId(2 * scope)`（常に偶数）。scope0 → `TargetId(0)`。
///
/// 前提: `scope` は `i32::MAX` 以下（入口の [`scope_of`]・[`window_scope`] が守る）。
pub fn shell_target(scope: u32) -> TargetId {
    TargetId(2 * scope)
}

/// scope 番号 → バルーン表示対象（正本）。
///
/// areka-P0-emo2-boot 設計 DD-3 の採番規約により `TargetId(2 * scope + 1)`（常に奇数）。同一 scope のシェル表示対象
/// （偶数）とは偶奇が異なるため必ず異なる id となる。
///
/// 前提: `scope` は `i32::MAX` 以下（入口の [`scope_of`]・[`window_scope`] が守る）。
pub fn balloon_target(scope: u32) -> TargetId {
    TargetId(2 * scope + 1)
}

/// `ActorKey`（`"0"`／`"1"` 等）→ scope 番号。数値でない key は `None`。
///
/// `ActorKey` は数値アクセサを持たない不透明文字列ゆえ `as_str().parse::<u32>()` で数値化する。
/// 非数値（例 `"側"`・`"kero"`・空文字・負値・混在）と `i32::MAX` を越える番号は `None` を返し、呼び手（adapter）が
/// `warn!` ＋当該指令 drop で握り潰さず観測する（log-first・areka-P0-emo2-boot 要件 3 前提）。
pub fn scope_of(key: &ActorKey) -> Option<u32> {
    key.as_str().parse::<u32>().ok().filter(|&s| s <= MAX_SCOPE)
}

/// 窓の印の scope（`CharWindowMarker.scope` ほか）→ scope 番号。`MAX_SCOPE` を越えると
/// `warn!` を 1 行残して `None`（呼び手は読み飛ばすだけでよい・記録はここ 1 か所）。
pub fn window_scope(scope: usize) -> Option<u32> {
    let s = u32::try_from(scope).ok().filter(|&s| s <= MAX_SCOPE);
    if s.is_none() {
        warn!(
            scope,
            "window scope が上限を越え target を採番できない → この窓を読み飛ばす"
        );
    }
    s
}

/// 表示対象の窓の種類（採番の偶奇で決まる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetWindow {
    /// シェル（偶数の `TargetId`）。
    Shell,
    /// バルーン（奇数の `TargetId`）。
    Balloon,
}

/// 表示対象 → (scope 番号, 窓の種類)。`shell_target`／`balloon_target` の逆写像。
///
/// areka-P0-emo2-boot 設計 DD-3 の採番規約により、偶数ならシェル・奇数ならバルーンで、scope は `id / 2`。
pub fn target_scope(target: TargetId) -> (u32, TargetWindow) {
    let window = if target.0.is_multiple_of(2) {
        TargetWindow::Shell
    } else {
        TargetWindow::Balloon
    };
    (target.0 / 2, window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn shell_target_is_double_scope() {
        for n in [0u32, 1, 2, 7, 1000] {
            assert_eq!(shell_target(n), TargetId(2 * n));
        }
    }

    #[test]
    fn balloon_target_is_double_scope_plus_one() {
        for n in [0u32, 1, 2, 7, 1000] {
            assert_eq!(balloon_target(n), TargetId(2 * n + 1));
        }
    }

    /// areka-P0-emo2-boot 設計 DD-3 不変条件: 全 scope でシェル（偶数）とバルーン（奇数）の TargetId は互いに素。
    #[test]
    fn shell_and_balloon_targets_are_disjoint_across_scopes() {
        const N: u32 = 64;
        let mut seen: HashSet<TargetId> = HashSet::new();
        for scope in 0..N {
            let s = shell_target(scope);
            let b = balloon_target(scope);
            // 採番規約: shell=偶数・balloon=奇数 → 同一 scope 内で必ず異なる。
            assert_eq!(s.0 % 2, 0, "shell target must be even (scope {scope})");
            assert_eq!(b.0 % 2, 1, "balloon target must be odd (scope {scope})");
            assert_ne!(s, b, "shell/balloon collide within scope {scope}");
            // 全 scope 横断でも一意（偶奇分離ゆえ shell 同士・balloon 同士も別 scope なら別値）。
            assert!(
                seen.insert(s),
                "shell target {s:?} collided (scope {scope})"
            );
            assert!(
                seen.insert(b),
                "balloon target {b:?} collided (scope {scope})"
            );
        }
        assert_eq!(
            seen.len(),
            (N * 2) as usize,
            "all shell+balloon targets across {N} scopes must be unique"
        );
    }

    #[test]
    fn scope_of_parses_numeric_actor_key() {
        assert_eq!(scope_of(&ActorKey::from("0")), Some(0));
        assert_eq!(scope_of(&ActorKey::from("1")), Some(1));
        // 複数桁も数値として解釈する。
        assert_eq!(scope_of(&ActorKey::from("12")), Some(12));
    }

    #[test]
    fn scope_of_rejects_non_numeric_actor_key() {
        // 非 ASCII 語（例: "側"）。
        assert_eq!(scope_of(&ActorKey::from("側")), None);
        // 英字名。
        assert_eq!(scope_of(&ActorKey::from("kero")), None);
        // 空文字。
        assert_eq!(scope_of(&ActorKey::from("")), None);
        // 数字と非数字の混在は全体として非数値 → None。
        assert_eq!(scope_of(&ActorKey::from("1a")), None);
        // 負値は u32 parse が拒否 → None。
        assert_eq!(scope_of(&ActorKey::from("-1")), None);
    }

    /// scope 番号 → `ActorKey` → scope 番号 の往復で恒等（採番と parse の整合）。
    #[test]
    fn scope_of_roundtrips_scope_numbers() {
        for scope in [0u32, 1, 5, 42, 1000] {
            let key = ActorKey::from(scope.to_string());
            assert_eq!(scope_of(&key), Some(scope));
        }
    }

    /// `balloon_target` の `2*scope+1` が u32 に収まる最大（`i32::MAX`）までを受け、越える番号は `None`。
    #[test]
    fn scope_of_rejects_a_scope_whose_balloon_target_overflows() {
        assert_eq!(scope_of(&ActorKey::from("2147483647")), Some(2147483647));
        assert_eq!(balloon_target(2147483647), TargetId(u32::MAX));
        assert_eq!(scope_of(&ActorKey::from("2147483648")), None);
        assert_eq!(scope_of(&ActorKey::from("4294967295")), None);
    }

    #[test]
    fn window_scope_rejects_scopes_past_i32_max() {
        let (s, counts) = log_capture_kit::count_levels(|| window_scope(2147483647));
        assert_eq!(s, Some(2147483647));
        assert_eq!(counts, log_capture_kit::LevelCounts::default());
        for scope in [2147483648usize, usize::MAX] {
            let (s, counts) = log_capture_kit::count_levels(|| window_scope(scope));
            assert_eq!(s, None, "scope {scope}");
            assert_eq!(
                counts,
                log_capture_kit::LevelCounts {
                    warn: 1,
                    ..Default::default()
                },
                "scope {scope}"
            );
        }
    }

    /// 正写像 → 逆写像の往復で恒等（スコープ 0・1・2 × シェル・バルーン）。
    #[test]
    fn target_scope_roundtrips_shell_and_balloon_targets() {
        for scope in [0u32, 1, 2] {
            assert_eq!(
                target_scope(shell_target(scope)),
                (scope, TargetWindow::Shell)
            );
            assert_eq!(
                target_scope(balloon_target(scope)),
                (scope, TargetWindow::Balloon)
            );
        }
    }
}
