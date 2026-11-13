//! マニフェスト導出（列挙・重複排除）。
//!
//! 設計決定 **D6**（要件 **R1.1–1.6, 5.6**）。
//!
//! shell モデル（`areka-parsers::shell::Shell`）と surface 表現された balloon から、
//! 全部の面の element 画像パスを直に数える。`Pattern.surface_id` の参照先も同じセットの面なので、
//! 参照をたどらなくても数え漏れは無い（負の番号・無い番号の参照は面を足さない）。
//! 列挙結果は正規化パスで重複排除する。

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use crate::normalize::AlphaParams;
use crate::table::{AtlasKey, SetId};

/// 出所単位の自己完結な入力束（shell で 1 個・balloon で 1 個…複数可・D1）。
///
/// surface 群・基準 dir・透過設定が出所ごとに束で対応する（ディスカッション #1）。
/// shell と balloon は基準 dir も descript も別ゆえ**セットを分けて**渡す（1.4）。
pub struct SurfaceSet<'a> {
    /// element 相対パス＋bind pattern を持つ surface 群（ソース語彙・読み取り専用）。
    pub surfaces: &'a [areka_parsers::shell::Surface],
    /// このセットの基準 dir（shell dir / balloon dir）。derive は join しない
    /// （実パス化はデコード段の一度きり・1.5）。
    pub base_dir: &'a std::path::Path,
    /// このセットの descript 由来透過設定（derive は運ぶのみで読まない・3.6）。
    pub alpha_params: AlphaParams,
}

/// マニフェスト＝`ElementId` 順に整列した `AtlasKey` 一覧（採番の正本・D3/D7）。
pub(crate) struct Manifest {
    /// index == `ElementId.0`（密・決定的：(SetId 昇順, rel_path 昇順)）。
    pub(crate) keys: Vec<AtlasKey>,
}

/// マニフェスト導出器（列挙・重複排除・決定的採番）。
pub(crate) struct ManifestDeriver;

impl ManifestDeriver {
    /// 全セットの全部の面の element パスを直に数えて重複なく導出し（参照先も同じセットの面なので
    /// 数え漏れは無い）、(SetId 昇順, 相対パス昇順) で `ElementId` を決定的に採番する（D7・golden 安定）。
    ///
    /// 入力は読み取り専用（改変しない）。実パスは生成しない（実パス化はデコード段・一度きり）。
    pub(crate) fn derive(&self, sets: &[SurfaceSet<'_>]) -> Manifest {
        // 重複排除キー＝(SetId, rel_path)。BTreeSet で挿入と同時に (set, path) 昇順整列。
        let mut keys: BTreeSet<(u32, String)> = BTreeSet::new();

        for (set_index, set) in sets.iter().enumerate() {
            let set_id = set_index as u32;

            for surface in set.surfaces {
                // 直接 element の列挙（1.1/1.2・base 自己参照も通常 element 扱い）。
                for element in &surface.elements {
                    keys.insert((set_id, element.path.as_str().to_string()));
                }
            }
        }

        // (SetId 昇順, rel_path 昇順) で ElementId を採番（BTreeSet が既に整列済み）。
        let keys = keys
            .into_iter()
            .map(|(set, rel_path)| AtlasKey {
                set: SetId(set),
                rel_path,
            })
            .collect();

        Manifest { keys }
    }
}

/// 着せ替え（`bind`）のアニメーションのパターンからだけ参照される面の絵の集合を求める
/// （real-run-log-noise 要件 3.7）。
///
/// 鍵は (セットの番号, 相対パス)。マニフェストの鍵と同じ綴り（`element.path.as_str()`）。
/// 絵が入るのは、その絵を持つ面（持ち主）がどれも、同じセットのどこかのパターンから参照され、
/// しかも参照するパターンがどれも `bind` か `bind+random` のアニメーションに属するときだけ。
/// 参照されない持ち主・`bind` でないアニメーションから参照される持ち主が 1 つでもあれば入れない
/// （迷ったら警告の側）。負の番号・無い番号の参照と、欄 2 が面の番号でないメソッド
/// （`start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・
/// `insert`・`move`）のパターンは数えない。抜き色の道か・α を持つかは見ない。
/// 下流の emo-compose の `is_bind_interval` は使わず、同じ判定をここで持つ。純粋・決定的。
pub(crate) fn bind_only_paths(sets: &[SurfaceSet<'_>]) -> BTreeSet<(u32, String)> {
    let mut out = BTreeSet::new();
    for (set_index, set) in sets.iter().enumerate() {
        let set_id = set_index as u32;
        let ids: BTreeSet<i64> = set.surfaces.iter().flat_map(surface_ids).collect();
        // 面の番号 → bind だけから参照されているか（false＝bind でない参照が 1 つでもある）。
        let mut referenced: HashMap<i64, bool> = HashMap::new();
        for animation in set.surfaces.iter().flat_map(|s| &s.animations) {
            let is_bind = matches!(
                animation.interval,
                areka_parsers::shell::Interval::Bind
                    | areka_parsers::shell::Interval::BindRandom { .. }
            );
            for pattern in &animation.patterns {
                // 欄 2 が面の番号でない 8 語は面を指さない。
                let names_animation = areka_parsers::shell::pattern_word(pattern.method.as_str())
                    != areka_parsers::shell::PatternWord::Draw;
                if !names_animation && ids.contains(&pattern.surface_id) {
                    *referenced.entry(pattern.surface_id).or_insert(true) &= is_bind;
                }
            }
        }
        // 絵 → 持ち主がどれも bind だけから参照されているか。
        let mut paths: BTreeMap<&str, bool> = BTreeMap::new();
        for surface in set.surfaces {
            // 多 id のブレスは全部の番号が bind だけから参照されるときだけ（spec: audit-parsers-sakura 指摘 38・番号の無いブレスは入れない）。
            let owner_ids = surface_ids(surface);
            let owner_ok = !owner_ids.is_empty()
                && owner_ids
                    .iter()
                    .all(|id| referenced.get(id).copied().unwrap_or(false));
            for element in &surface.elements {
                *paths.entry(element.path.as_str()).or_insert(true) &= owner_ok;
            }
        }
        out.extend(
            paths
                .into_iter()
                .filter(|&(_, ok)| ok)
                .map(|(path, _)| (set_id, path.to_string())),
        );
    }
    out
}

/// ブレスの見出しの記述子が指す全部の番号（範囲を広げ、`!N`・`!a-b` を引く）。
///
/// 記述子が除外だけのブレスは番号を持たない。包含の範囲の幅は parsers が上限をかける・除外の範囲は広げない。
fn surface_ids(surface: &areka_parsers::shell::Surface) -> Vec<i64> {
    use areka_parsers::shell::AppendTarget;
    let span = |a: u32, b: u32| a.min(b) as i64..=a.max(b) as i64;
    let mut excluded = Vec::new();
    let mut ids = Vec::new();
    for target in &surface.targets {
        match *target {
            AppendTarget::Single(n) => ids.push(n as i64),
            AppendTarget::Range { start, end } => ids.extend(span(start, end)),
            AppendTarget::Exclude(n) => excluded.push(span(n, n)),
            AppendTarget::ExcludeRange { start, end } => excluded.push(span(start, end)),
            // `#[non_exhaustive]` の知らない記述子は番号を持たない。
            _ => {}
        }
    }
    ids.retain(|id| !excluded.iter().any(|range| range.contains(id)));
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::{AtlasKey, SetId};
    use crate::test_support::{elem, set, surface};
    use areka_parsers::shell::{Animation, DrawMethod, Interval, Pattern, Surface};
    use std::path::Path;

    // ---- 合成モデルビルダ（emo2 相当構造・実 fixture は `emo2_e2e.rs`）----

    /// 単一 pattern（surface_id 参照）を持つ bind animation。
    fn bind_anim(anim_id: u32, ref_surface_id: i64) -> Animation {
        Animation {
            id: anim_id,
            interval: Interval::Bind,
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: ref_surface_id,
                target: ref_surface_id.to_string(),
                wait: 0,
                x: 0,
                y: 0,
            }],
        }
    }

    /// derive 結果の rel_path を SetId 付きで列挙（比較用）。
    fn keys_of(m: &Manifest) -> Vec<(u32, String)> {
        m.keys
            .iter()
            .map(|k| (k.set.0, k.rel_path.clone()))
            .collect()
    }

    /// 1.1/1.2: 2 surface・複数 element の直接列挙（base 自己参照も通常 element 扱い）。
    #[test]
    fn direct_enumeration_all_elements() {
        let base = Path::new("shell/master");
        let surfaces = vec![
            surface(0, vec![elem(0, "surface0.png"), elem(1, "eye0.png")]),
            surface(1, vec![elem(0, "surface1.png")]),
        ];
        let sets = [set(&surfaces, base)];
        let m = ManifestDeriver.derive(&sets);
        let got = keys_of(&m);
        assert!(got.contains(&(0, "surface0.png".to_string())));
        assert!(got.contains(&(0, "eye0.png".to_string())));
        assert!(got.contains(&(0, "surface1.png".to_string())));
        assert_eq!(got.len(), 3, "3 distinct element paths enumerated");
    }

    /// 1.3: 面 0 → 1 → … → 49,999 を `overlay` の参照の鎖でつないだ 5 万面のセットを、
    /// スタック 256 KiB のスレッドで数え終え、絵を 1 度ずつ（鍵 5 万個）数える
    /// （参照をたどる再帰があるとスタックがあふれて落ちる）。
    #[test]
    fn chain_of_50000_surfaces_counts_in_small_stack() {
        const N: u32 = 50_000;
        let surfaces: Vec<Surface> = (0..N)
            .map(|id| {
                let mut s = surface(id, vec![elem(0, &format!("s{id}.png"))]);
                s.animations = vec![bind_anim(0, i64::from(id) + 1)];
                s
            })
            .collect();
        let count = std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(move || {
                let base = Path::new("shell/master");
                ManifestDeriver.derive(&[set(&surfaces, base)]).keys.len()
            })
            .expect("spawn")
            .join()
            .expect("derive in small stack");
        assert_eq!(count, N as usize);
    }

    /// 1.5: サブディレクトリ入りパスは無改変で AtlasKey.rel_path に保持される。
    #[test]
    fn subdirectory_path_unchanged() {
        let base = Path::new("shell/master");
        let surfaces = vec![surface(0, vec![elem(0, "purple/2/a.png")])];
        let sets = [set(&surfaces, base)];
        let m = ManifestDeriver.derive(&sets);
        let got = keys_of(&m);
        assert_eq!(got, vec![(0, "purple/2/a.png".to_string())]);
    }

    /// 1.6/5.6: 同一 set 内で 2 surface が同じ rel_path を参照 → 単一キーへ重複排除。
    #[test]
    fn dedup_same_path_same_set() {
        let base = Path::new("shell/master");
        let surfaces = vec![
            surface(0, vec![elem(0, "shared.png")]),
            surface(1, vec![elem(0, "shared.png")]),
        ];
        let sets = [set(&surfaces, base)];
        let m = ManifestDeriver.derive(&sets);
        let got = keys_of(&m);
        assert_eq!(got, vec![(0, "shared.png".to_string())], "single dedup key");
    }

    /// 1.4/D3: shell set と balloon set が同名 rel_path を持つ → SetId 別の 2 キー。
    #[test]
    fn set_scoped_keys_two_sets() {
        let shell_base = Path::new("shell/master");
        let balloon_base = Path::new("balloon/foo");
        let shell = vec![surface(0, vec![elem(0, "surface0.png")])];
        let balloon = vec![surface(0, vec![elem(0, "surface0.png")])];
        let sets = [set(&shell, shell_base), set(&balloon, balloon_base)];
        let m = ManifestDeriver.derive(&sets);
        let got = keys_of(&m);
        // 同名だが SetId 0 と 1 の 2 キー（balloon も同一機構・1.4／dedup は (set, rel_path)）
        assert!(got.contains(&(0, "surface0.png".to_string())));
        assert!(got.contains(&(1, "surface0.png".to_string())));
        assert_eq!(got.len(), 2);
    }

    /// D3/D7: keys が (SetId 昇順, rel_path 昇順) で整列・derive は決定的（2 回一致）。
    #[test]
    fn deterministic_sorted_numbering() {
        let shell_base = Path::new("shell/master");
        let balloon_base = Path::new("balloon/foo");
        // わざと非整列で投入（b.png, a.png / set1 に z.png）
        let shell = vec![surface(0, vec![elem(0, "b.png"), elem(1, "a.png")])];
        let balloon = vec![surface(0, vec![elem(0, "z.png")])];
        let sets = [set(&shell, shell_base), set(&balloon, balloon_base)];
        let m = ManifestDeriver.derive(&sets);
        let got = keys_of(&m);
        assert_eq!(
            got,
            vec![
                (0, "a.png".to_string()),
                (0, "b.png".to_string()),
                (1, "z.png".to_string()),
            ],
            "sorted by (SetId asc, rel_path asc)"
        );
        // ElementId == index 対応の確認
        assert_eq!(
            m.keys[0],
            AtlasKey {
                set: SetId(0),
                rel_path: "a.png".into()
            }
        );
        assert_eq!(
            m.keys[2],
            AtlasKey {
                set: SetId(1),
                rel_path: "z.png".into()
            }
        );

        // 決定性: 2 回目も同一
        let m2 = ManifestDeriver.derive(&sets);
        assert_eq!(keys_of(&m), keys_of(&m2));
    }

    // ---- 檻 T8: 着せ替え（bind）のアニメーションからだけ参照される面の絵（要件 3.7）----

    /// 指定の interval を持つ単一 pattern のアニメーション（参照先は `ref_surface_id`）。
    fn anim_with(anim_id: u32, interval: Interval, ref_surface_id: i64) -> Animation {
        let mut a = bind_anim(anim_id, ref_surface_id);
        a.interval = interval;
        a
    }

    /// 基底の面 0 に `animations` を持たせ、持ち主の面 `owners` を並べたセットの集合を求める。
    fn bind_only_of(animations: Vec<Animation>, owners: Vec<Surface>) -> BTreeSet<(u32, String)> {
        let mut base_surface = surface(0, vec![elem(0, "surface0.png")]);
        base_surface.animations = animations;
        let mut surfaces = vec![base_surface];
        surfaces.extend(owners);
        let base = Path::new("shell/master");
        bind_only_paths(&[set(&surfaces, base)])
    }

    fn null_key() -> (u32, String) {
        (0, "purple/a/null.png".to_string())
    }

    /// T8 ⑴: `bind` だけから参照される持ち主の絵は入る。参照されない基底の面の絵は入らない。
    #[test]
    fn bind_only_paths_includes_bind_only_owner() {
        let got = bind_only_of(
            vec![bind_anim(0, 3000)],
            vec![surface(3000, vec![elem(0, "purple/a/null.png")])],
        );
        assert_eq!(got, BTreeSet::from([null_key()]));
    }

    /// T8 ⑵: `bind+random` からの参照も着せ替えとして入る。
    #[test]
    fn bind_only_paths_includes_bind_random_owner() {
        let got = bind_only_of(
            vec![anim_with(0, Interval::BindRandom { k: 2 }, 3000)],
            vec![surface(3000, vec![elem(0, "purple/a/null.png")])],
        );
        assert_eq!(got, BTreeSet::from([null_key()]));
    }

    /// T8 ⑶: `runonce`・未知の語（`always`）からの参照は入らない。
    #[test]
    fn bind_only_paths_excludes_runonce_and_unknown_interval() {
        for interval in [Interval::Runonce, Interval::Other("always".into())] {
            let got = bind_only_of(
                vec![anim_with(0, interval.clone(), 3000)],
                vec![surface(3000, vec![elem(0, "purple/a/null.png")])],
            );
            assert!(
                got.is_empty(),
                "{interval:?} must not count as bind: {got:?}"
            );
        }
    }

    /// T8 ⑷: `bind` と `bind` でないの両方から参照される持ち主の絵は入らない。
    #[test]
    fn bind_only_paths_excludes_owner_also_referenced_by_non_bind() {
        let got = bind_only_of(
            vec![
                bind_anim(0, 3000),
                anim_with(1, Interval::Random { k: 3 }, 3000),
            ],
            vec![surface(3000, vec![elem(0, "purple/a/null.png")])],
        );
        assert!(got.is_empty(), "{got:?}");
    }

    /// T8 ⑸: どこからも参照されない持ち主の絵は入らない。
    #[test]
    fn bind_only_paths_excludes_unreferenced_owner() {
        let got = bind_only_of(
            Vec::new(),
            vec![surface(3000, vec![elem(0, "purple/a/null.png")])],
        );
        assert!(got.is_empty(), "{got:?}");
    }

    /// T8 ⑹: 同じ絵を `bind` だけの面と参照されない面の両方が持つときは入らない。
    #[test]
    fn bind_only_paths_excludes_path_shared_with_unreferenced_owner() {
        let got = bind_only_of(
            vec![bind_anim(0, 3000)],
            vec![
                surface(3000, vec![elem(0, "purple/a/null.png")]),
                surface(3001, vec![elem(0, "purple/a/null.png")]),
            ],
        );
        assert!(got.is_empty(), "{got:?}");
    }

    /// T8 ⑺: 負の番号・無い番号の参照は数えない（`bind` の参照としても、持ち主を作る参照としても）。
    #[test]
    fn bind_only_paths_ignores_negative_and_missing_refs() {
        // 負の番号・無い番号は bind でないアニメーションからでも ⑴ を崩さない。
        let got = bind_only_of(
            vec![
                bind_anim(0, 3000),
                anim_with(1, Interval::Runonce, -1),
                anim_with(2, Interval::Runonce, 9999),
            ],
            vec![surface(3000, vec![elem(0, "purple/a/null.png")])],
        );
        assert_eq!(got, BTreeSet::from([null_key()]));
        // 負の番号・無い番号の bind の参照だけでは、参照されない持ち主は入らない。
        let got = bind_only_of(
            vec![bind_anim(0, -1), bind_anim(1, 9999)],
            vec![surface(3000, vec![elem(0, "purple/a/null.png")])],
        );
        assert!(got.is_empty(), "{got:?}");
    }

    /// T8 ⑻: 番号がアニメーションの番号のメソッド（`insert`・`start`・`alternativestart` など）の
    /// パターンは面の参照と数えない（`bind` のアニメーションからでも持ち主を作らない・迷ったら警告の側）。
    #[test]
    fn bind_only_paths_ignores_patterns_whose_number_is_an_animation_id() {
        for method in [
            "insert",
            "start",
            "Alternative-Start",
            "parallelstop",
            "move",
            " Mo_ve ",
        ] {
            let mut anim = bind_anim(0, 3000);
            anim.patterns[0].method = DrawMethod::new(method.to_string());
            let got = bind_only_of(
                vec![anim],
                vec![surface(3000, vec![elem(0, "purple/a/null.png")])],
            );
            assert!(got.is_empty(), "{method}: {got:?}");
        }
    }

    /// T8 ⑼: 多 id のブレス（`surface1-3`）の絵は、ブレスの全部の番号が `bind` だけから参照されるときだけ入る
    /// （先頭の番号 1 だけが参照されても 2・3 は参照されないので入らない・spec: audit-parsers-sakura 指摘 38）。
    #[test]
    fn bind_only_paths_requires_every_id_of_range_brace() {
        let mut eye = surface(1, vec![elem(0, "eye.png")]);
        eye.targets = vec![areka_parsers::shell::AppendTarget::Range { start: 1, end: 3 }];
        let got = bind_only_of(vec![bind_anim(0, 1)], vec![eye.clone()]);
        assert!(got.is_empty(), "{got:?}");
        let got = bind_only_of(
            vec![bind_anim(0, 1), bind_anim(1, 2), bind_anim(2, 3)],
            vec![eye],
        );
        assert_eq!(got, BTreeSet::from([(0, "eye.png".to_string())]));
    }

    /// 除外は番号から引く（`surface1-3,!2` は 1 と 3）。除外の範囲は広げないので、
    /// `!0-4294967295` でも大きな集めを作らずに返る（上限は包含の範囲にだけかかる）。
    #[test]
    fn surface_ids_subtracts_exclusions_without_expanding_them() {
        use areka_parsers::shell::AppendTarget;
        let mut s = surface(1, Vec::new());
        s.targets = vec![
            AppendTarget::Range { start: 1, end: 3 },
            AppendTarget::Exclude(2),
        ];
        assert_eq!(surface_ids(&s), vec![1, 3]);
        s.targets = vec![
            AppendTarget::Single(0),
            AppendTarget::ExcludeRange {
                start: 0,
                end: u32::MAX,
            },
        ];
        assert_eq!(surface_ids(&s), Vec::<i64>::new());
    }
}
