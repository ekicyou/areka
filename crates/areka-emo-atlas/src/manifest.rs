//! マニフェスト導出（列挙・間接参照解決・重複排除）。
//!
//! 設計決定 **D6**（要件 **R1.1–1.6, 5.6**）。
//!
//! shell モデル（`areka-parsers::shell::Shell`）と surface 表現された balloon から、
//! 全 surface が参照する element 画像パス集合を列挙する。`Pattern.surface_id` の
//! 間接 bind 参照を surface id 索引で辿って参照先 surface の element を展開し、
//! 負値センチネル・不在 id・`Range`/alias（画像を持たない）は除外する。
//! 訪問済み集合による循環検出を含む。列挙結果は正規化パスで重複排除する。

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
pub struct Manifest {
    /// index == `ElementId.0`（密・決定的：(SetId 昇順, rel_path 昇順)）。
    pub keys: Vec<AtlasKey>,
}

/// マニフェスト導出器（列挙・間接 bind 解決・重複排除・決定的採番）。
pub struct ManifestDeriver;

impl ManifestDeriver {
    /// 全セットの element パス（直接＋間接 bind 参照）を重複なく導出し、
    /// (SetId 昇順, 相対パス昇順) で `ElementId` を決定的に採番する（D7・golden 安定）。
    ///
    /// 入力は読み取り専用（改変しない）。実パスは生成しない（実パス化はデコード段・一度きり）。
    pub fn derive(&self, sets: &[SurfaceSet<'_>]) -> Manifest {
        // 重複排除キー＝(SetId, rel_path)。BTreeSet で挿入と同時に (set, path) 昇順整列。
        let mut keys: BTreeSet<(u32, String)> = BTreeSet::new();

        for (set_index, set) in sets.iter().enumerate() {
            let set_id = set_index as u32;

            // surface id → surface 索引（重複 id は先出優先・Precondition）。
            let mut by_id: HashMap<i64, &areka_parsers::shell::Surface> = HashMap::new();
            for surface in set.surfaces {
                by_id.entry(surface.id as i64).or_insert(surface);
            }

            for surface in set.surfaces {
                // 直接 element の列挙（1.1/1.2・base 自己参照も通常 element 扱い）。
                Self::collect_elements(surface, set_id, &mut keys);

                // 間接 bind 参照の解決（1.3/D6）。当該 surface を起点に visited で循環打ち切り。
                let mut visited: BTreeSet<i64> = BTreeSet::new();
                visited.insert(surface.id as i64);
                Self::resolve_indirect(surface, set_id, &by_id, &mut visited, &mut keys);
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

    /// surface の直接 element パスを (set_id, rel_path) として追加（無改変・1.5）。
    fn collect_elements(
        surface: &areka_parsers::shell::Surface,
        set_id: u32,
        keys: &mut BTreeSet<(u32, String)>,
    ) {
        for element in &surface.elements {
            keys.insert((set_id, element.path.as_str().to_string()));
        }
    }

    /// `Animation.patterns[].surface_id` を辿り参照先 surface の element を列挙する（1.3/D6）。
    ///
    /// - `surface_id < 0`（レイヤクリア/停止センチネル）は画像を持たず除外。
    /// - set 内に存在しない id も除外（不在参照）。
    /// - `visited`（surface id 集合）で循環・重複訪問を打ち切る（構造として停止保証）。
    fn resolve_indirect(
        surface: &areka_parsers::shell::Surface,
        set_id: u32,
        by_id: &HashMap<i64, &areka_parsers::shell::Surface>,
        visited: &mut BTreeSet<i64>,
        keys: &mut BTreeSet<(u32, String)>,
    ) {
        for animation in &surface.animations {
            for pattern in &animation.patterns {
                let ref_id = pattern.surface_id;
                // 負値センチネルは画像を持たない（1.3/D6）。
                if ref_id < 0 {
                    continue;
                }
                // 既訪問（循環含む）は打ち切り。
                if visited.contains(&ref_id) {
                    continue;
                }
                // 不在 id は除外（画像を持たない参照）。
                let Some(referenced) = by_id.get(&ref_id) else {
                    continue;
                };
                visited.insert(ref_id);
                // 参照先 surface の直接 element を列挙。
                Self::collect_elements(referenced, set_id, keys);
                // 推移的 bind も辿る（visited で停止保証）。
                Self::resolve_indirect(referenced, set_id, by_id, visited, keys);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::{AlphaParams, UseSelfAlpha};
    use crate::table::{AtlasKey, SetId};
    use areka_parsers::shell::{Animation, DrawMethod, Element, Interval, Pattern, Surface};
    use std::path::Path;

    // ---- 合成モデルビルダ（emo2 相当構造・実 fixture パースは統合 task 3.2 送り）----

    fn elem(layer: u32, path: &str) -> Element {
        Element {
            layer,
            path: areka_parsers::shell::ElementPath::new(path.to_string()),
            x: 0,
            y: 0,
        }
    }

    /// element のみを持つ最小 surface（collision/animation 空）。
    fn surface(id: u32, elements: Vec<Element>) -> Surface {
        Surface {
            id,
            targets: vec![areka_parsers::shell::AppendTarget::Single(id)],
            elements,
            collisions: Vec::new(),
            animations: Vec::new(),
        }
    }

    /// 単一 pattern（surface_id 参照）を持つ bind animation。
    fn bind_anim(anim_id: u32, ref_surface_id: i64) -> Animation {
        Animation {
            id: anim_id,
            interval: Interval::Bind,
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: ref_surface_id,
                wait: 0,
                x: 0,
                y: 0,
            }],
        }
    }

    fn params() -> AlphaParams {
        AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        }
    }

    /// SurfaceSet を借用で組む小ヘルパ（base_dir はダミー・derive は join しない）。
    fn set<'a>(surfaces: &'a [Surface], base: &'a Path) -> SurfaceSet<'a> {
        SurfaceSet {
            surfaces,
            base_dir: base,
            alpha_params: params(),
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

    /// 1.3: bind pattern の間接参照先 surface の element も列挙される。
    #[test]
    fn indirect_bind_resolution_enumerates_referenced_surface() {
        let base = Path::new("shell/master");
        // A(=10) は表示 surface・自身は base 画像のみ。animation が helper B(=1100) を bind。
        let mut a = surface(10, vec![elem(0, "surface10.png")]);
        a.animations = vec![bind_anim(0, 1100)];
        // B(=1100) は表示されないが A から間接参照される element を持つ。
        let b = surface(1100, vec![elem(0, "arm_up.png"), elem(1, "arm_dn.png")]);
        let surfaces = vec![a, b];
        let sets = [set(&surfaces, base)];
        let m = ManifestDeriver.derive(&sets);
        let got = keys_of(&m);
        assert!(got.contains(&(0, "surface10.png".to_string())));
        // helper B の element が間接参照で列挙される（1.3）
        assert!(got.contains(&(0, "arm_up.png".to_string())));
        assert!(got.contains(&(0, "arm_dn.png".to_string())));
    }

    /// 1.3/D6: 負 surface_id・不在 id の bind は画像を持たず除外（panic しない）。
    #[test]
    fn negative_and_nonexistent_bind_excluded() {
        let base = Path::new("shell/master");
        let mut a = surface(10, vec![elem(0, "surface10.png")]);
        a.animations = vec![
            bind_anim(0, -2),   // レイヤクリア/停止センチネル → 画像なし
            bind_anim(1, 9999), // set 内に存在しない id → 画像なし
        ];
        let surfaces = vec![a];
        let sets = [set(&surfaces, base)];
        let m = ManifestDeriver.derive(&sets);
        let got = keys_of(&m);
        assert_eq!(
            got,
            vec![(0, "surface10.png".to_string())],
            "only the direct element; no phantom paths from sentinel/missing refs"
        );
    }

    /// D6: A→B→A の bind ループでも derive が停止し両 surface の element を 1 度ずつ列挙。
    #[test]
    fn cycle_binds_terminate() {
        let base = Path::new("shell/master");
        let mut a = surface(10, vec![elem(0, "a.png")]);
        a.animations = vec![bind_anim(0, 20)];
        let mut b = surface(20, vec![elem(0, "b.png")]);
        b.animations = vec![bind_anim(0, 10)]; // 循環参照
        let surfaces = vec![a, b];
        let sets = [set(&surfaces, base)];
        let m = ManifestDeriver.derive(&sets); // 無限ループしないこと
        let got = keys_of(&m);
        assert!(got.contains(&(0, "a.png".to_string())));
        assert!(got.contains(&(0, "b.png".to_string())));
        assert_eq!(got.len(), 2, "each surface's element listed once");
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
}
