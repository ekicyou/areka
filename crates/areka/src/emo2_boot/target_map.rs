//! scope → 表示対象（TargetId）写像の正本（純関数・std のみ）。
//!
//! 本仕様が新たに正本として確立する唯一の契約（design.md「依存方向」の最左・R3.5）。
//! DD-3 の採番規約（シェル表示対象＝`2*scope`／バルーン表示対象＝`2*scope+1`）で
//! scope からシェル／バルーン両表示対象への互いに素な写像を与え、`ActorKey` の数値 parse
//! （非数値は `None`）を担う。
//!
//! 不変条件（DD-3）: `shell_target` は偶数・`balloon_target` は奇数の `TargetId` を返すため、
//! 全 scope にわたりシェル表示対象とバルーン表示対象は決して衝突しない（互いに素）。
//! この 3 関数のみが本仕様の新正本であり、M-dual 拡張はこのモジュールだけを触る。

use areka_emo_present::TargetId;
use areka_sakura::ActorKey;

/// scope 番号 → シェル表示対象（正本）。
///
/// DD-3 の採番規約により `TargetId(2 * scope)`（常に偶数）。scope0 → `TargetId(0)` で
/// donor（emo-present.rs の `boot_present_system`）が用いる `TargetId(0)` 慣行を包含する。
pub fn shell_target(scope: u32) -> TargetId {
    TargetId(2 * scope)
}

/// scope 番号 → バルーン表示対象（正本）。
///
/// DD-3 の採番規約により `TargetId(2 * scope + 1)`（常に奇数）。同一 scope のシェル表示対象
/// （偶数）とは偶奇が異なるため必ず異なる id となる。
pub fn balloon_target(scope: u32) -> TargetId {
    TargetId(2 * scope + 1)
}

/// `ActorKey`（`"0"`／`"1"` 等）→ scope 番号。数値でない key は `None`。
///
/// `ActorKey` は数値アクセサを持たない不透明文字列ゆえ `as_str().parse::<u32>()` で数値化する。
/// 非数値（例 `"側"`・`"kero"`・空文字・負値・混在）は `None` を返し、呼び手（adapter）が
/// `warn!` ＋当該指令 drop で握り潰さず観測する（log-first・R3 前提）。
pub fn scope_of(key: &ActorKey) -> Option<u32> {
    key.as_str().parse::<u32>().ok()
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

    /// DD-3 不変条件: 全 scope でシェル（偶数）とバルーン（奇数）の TargetId は互いに素。
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
}
