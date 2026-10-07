//! 写し取りの値の決定論テスト（4.2）。写し取りの値が判断の事実の trait を満たし、手で組んだ事実と
//! 同じ答えを返すこと（同じ台本 → 同じ診断）を確かめる。

use std::collections::{BTreeMap, BTreeSet};

use areka_mcp::tools::check_script::Diagnostic;
use areka_parsers::sakura::parse_noted;
use areka_seriko::{SurfaceResolver, SurfaceTarget};

use super::*;
use crate::emo2_boot::consumer_ledger::ConsumerLedger;
use crate::mcp::check_script_judge::{ScriptFacts, diagnose};

/// 手で組んだ事実。スコープ 0 の絵は 0・3・10、スコープ 1 は 10・11、別名 `笑顔` は 3。
/// バルーンはスコープ 0 が 0・2、スコープ 1 が 1。スコープ 2 には何も無い。
struct ByHand(SurfaceResolver);

impl ScriptFacts for ByHand {
    fn resolve_surface(&self, key: &str) -> SurfaceTarget {
        self.0.resolve(key)
    }
    fn shell_has(&self, scope: u32, surface_id: u32) -> Option<bool> {
        match scope {
            0 => Some([0, 3, 10].contains(&surface_id)),
            1 => Some([10, 11].contains(&surface_id)),
            _ => None,
        }
    }
    fn balloon_has(&self, scope: u32, balloon_id: u32) -> Option<bool> {
        match scope {
            0 => Some([0, 2].contains(&balloon_id)),
            1 => Some(balloon_id == 1),
            _ => None,
        }
    }
}

fn aliases() -> BTreeMap<String, Vec<u32>> {
    BTreeMap::from([("笑顔".to_string(), vec![3])])
}

/// 上と同じ事実を、写し取りの値の形（集合の表）で組む。
fn snapshot() -> ScriptFactsSnapshot {
    let ids = |v: &[u32]| v.iter().copied().collect::<BTreeSet<u32>>();
    ScriptFactsSnapshot::new(
        aliases(),
        BTreeMap::from([(0, ids(&[0, 3, 10])), (1, ids(&[10, 11]))]),
        BTreeMap::from([(0, ids(&[0, 2])), (1, ids(&[1]))]),
    )
}

fn check(script: &str, facts: &dyn ScriptFacts) -> Vec<Diagnostic> {
    diagnose(
        script,
        &parse_noted(script),
        &ConsumerLedger::canonical(),
        Some(facts),
    )
}

#[test]
fn snapshot_answers_like_facts_built_by_hand() {
    let by_hand = ByHand(SurfaceResolver::new(aliases()));
    let snap = snapshot();
    let scripts = [
        r"\s[0]\s[3]\s[笑顔]\s[99999]\s[無い名前]\s[-1]",
        r"\b[0]\b[2]\b[5]\b[-1]\b[名前]",
        r"\1\s[10]\s[0]\b[1]\b[0]",
        r"\p[2]\s[12345]\b[9]",
        r"\0\s[11]\![change,shell,x]\s[99999]",
        r"\x\![nope]\f[sub,1]\s[99999]\e\b[7]",
    ];
    let mut fired = 0;
    for script in scripts {
        let want = check(script, &by_hand);
        assert_eq!(check(script, &snap), want, "{script}");
        fired += want.len();
    }
    // 比べた台本が何も診断しない組で素通りしていないこと（較正）。
    assert!(fired >= 8, "too few diagnostics to compare: {fired}");
}

#[test]
fn snapshot_without_a_scope_does_not_judge_it() {
    let snap = snapshot();
    assert_eq!(snap.shell_has(2, 0), None);
    assert_eq!(snap.balloon_has(2, 0), None);
    assert_eq!(snap.shell_has(0, 3), Some(true));
    assert_eq!(snap.shell_has(1, 3), Some(false));
}
