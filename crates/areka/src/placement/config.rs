//! descript KV → `PlacementConfig` 変換。
//!
//! 4 層カスケード・両表記寛容（`defaultx`/`defaultleft` 等）・scope 検出・
//! zorder/sticky/dpi 転記シーム。入力は BTreeMap×2（純粋・panic しない）。
//!
//! 依存規約（placement/mod.rs）: areka-parsers の出力形（`kv::parse_kv` の
//! `BTreeMap<String, String>`）のみを消費し、wintf/bevy_ecs を import しない。

use std::collections::{BTreeMap, BTreeSet};

use tracing::warn;

/// `seriko.alignmenttodesktop` の解釈（2.8: 未使用値はシーム受理）。
#[allow(dead_code)] // scaffold（task 2.1）: resolver（task 3.1）が消費するまで非テストビルドでは未使用
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alignment {
    /// 既定（emo2 実使用）。Y は work area 下端固定・`defaulttop` 無視（2.2/2.4）。
    Bottom,
    /// `defaultleft`/`defaulttop` が有効な X/Y 初期座標になる（2.6）。
    Free,
    /// `top`/`left`/`right`/未知値: 転記保持・実挙動は `Bottom` と同じ＋警告（2.8・DD9）。
    /// warn ログはシーム値を実際に消費する resolver 側（呼び出し側）へ委ねる。
    Seam(String),
}

/// バルーンの左右位置（暫定 offset の向きにのみ使用・DD7）。既定 `Left`。
#[allow(dead_code)] // scaffold（task 2.1）: resolver（task 3.1）が消費するまで非テストビルドでは未使用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BalloonSide {
    /// バルーンをキャラ窓の左側へ（既定・emo2 scope0）。
    #[default]
    Left,
    /// バルーンをキャラ窓の右側へ（emo2 scope1）。
    Right,
}

/// スコープ 1 つぶんのカスケード解決済み配置構成。
#[allow(dead_code)] // scaffold（task 2.1）: resolver（task 3.1）が消費するまで非テストビルドでは未使用
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeConfig {
    /// 4 層カスケード解決済み（既定 `Bottom`・2.2/2.3）。
    pub alignment: Alignment,
    /// X 初期座標指定。同層内 `defaultx` ＞ `defaultleft`・層間 shell ＞ ghost（2.5/2.7・DD3）。
    pub default_x: Option<i32>,
    /// Y 初期座標指定。同層内 `defaulty` ＞ `defaulttop`・層間 shell ＞ ghost（2.6/2.7）。
    pub default_y: Option<i32>,
    /// `balloon.alignment`（`left` 既定／`right`）。暫定 offset の向き決定にのみ使用（1.2・DD7）。
    pub balloon_alignment: BalloonSide,
    /// `balloon.offsetx`/`balloon.offsety`（両方あるときのみ Some・emo2 は未使用＝None）。
    pub balloon_offset: Option<(i32, i32)>,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            alignment: Alignment::Bottom,
            default_x: None,
            default_y: None,
            balloon_alignment: BalloonSide::Left,
            balloon_offset: None,
        }
    }
}

/// ghost/shell descript KV から解決した配置構成の全体（スコープ別＋シーム転記）。
#[allow(dead_code)] // scaffold（task 2.1）: resolver（task 3.1）が消費するまで非テストビルドでは未使用
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementConfig {
    /// 検出済みスコープ（0 起点・`BTreeMap` で昇順）。emo2 → {0, 1}（DD6）。
    pub scopes: BTreeMap<usize, ScopeConfig>,
    /// shell `seriko.zorder` の生転記（実挙動なし・5.2）。
    pub zorder_raw: Option<String>,
    /// shell `seriko.sticky-window` の生転記（実挙動なし・5.2）。
    pub sticky_window_raw: Option<String>,
    /// shell `seriko.dpi` の生転記（96 素通しシーム・DD11）。
    pub shell_dpi_raw: Option<String>,
}

/// KV → 構成（純粋・パニックしない・数値化不能は None＋`warn!`）。
///
/// カスケード（正典表準拠・2.3）: `alignment` は「ghost 全体 → ghost スコープ →
/// shell 全体 → shell スコープ」の後勝ち 4 層。`default_x/y`・`balloon.*` は
/// スコープ別キーのみの 2 層（shell が ghost に勝つ）。
///
/// 同層内の優先順位: 正典プレフィックス（`sakura.`/`kero.`）＞ 別名（`char0.`/`char1.`）、
/// 同一プレフィックス内では `defaultx` ＞ `defaultleft`・`defaulty` ＞ `defaulttop`（DD3）・
/// `alignmenttodesktop` ＞ 別綴 `alignmentondesktop`。
#[allow(dead_code)] // scaffold（task 2.1）: source/resolver（task 2.2/3.1）が結線するまで非テストビルドでは未使用
pub fn build_placement_config(
    ghost_kv: &BTreeMap<String, String>,
    shell_kv: &BTreeMap<String, String>,
) -> PlacementConfig {
    let mut scopes = BTreeMap::new();
    for scope in detect_scopes(ghost_kv, shell_kv) {
        scopes.insert(scope, resolve_scope(scope, ghost_kv, shell_kv));
    }

    PlacementConfig {
        scopes,
        // シーム転記の所在は shell descript（正典表・5.2/DD11）。ghost 側同名キーは対象外。
        zorder_raw: shell_kv.get("seriko.zorder").cloned(),
        sticky_window_raw: shell_kv.get("seriko.sticky-window").cloned(),
        shell_dpi_raw: shell_kv.get("seriko.dpi").cloned(),
    }
}

/// スコープ検出（DD6）: scope0 常設・scope1 は「ghost `kero.name` **or** shell の
/// `kero.` 接頭キー存在」・`char{n}.`（n≥2）接頭キーは ghost/shell どちら側でも
/// scope n を追加。`char0.`/`char1.` は別名（値の受理のみ）でありスコープ検出
/// シグナルではない。
fn detect_scopes(
    ghost_kv: &BTreeMap<String, String>,
    shell_kv: &BTreeMap<String, String>,
) -> BTreeSet<usize> {
    let mut set = BTreeSet::new();
    set.insert(0usize);

    if ghost_kv.contains_key("kero.name") || shell_kv.keys().any(|k| k.starts_with("kero.")) {
        set.insert(1);
    }

    for key in ghost_kv.keys().chain(shell_kv.keys()) {
        if let Some(n) = char_scope_of(key) {
            if n >= 2 {
                set.insert(n);
            }
        }
    }

    set
}

/// `char{n}.` 接頭キーからスコープ番号を抽出する。`charset` 等の非該当キー・
/// 数値化不能（空・巨大値含む）は None（panic しない）。
fn char_scope_of(key: &str) -> Option<usize> {
    let rest = key.strip_prefix("char")?;
    let (digits, _) = rest.split_once('.')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// スコープ別キーのプレフィックス（正典, 別名）。scope0=`sakura.`（別名 `char0.`）・
/// scope1=`kero.`（別名 `char1.`）・scope n≥2=`char{n}.`（別名なし）。
fn scope_prefixes(scope: usize) -> (String, Option<&'static str>) {
    match scope {
        0 => ("sakura.".to_owned(), Some("char0.")),
        1 => ("kero.".to_owned(), Some("char1.")),
        n => (format!("char{n}."), None),
    }
}

/// 1 つの KV（＝1 層）からスコープ別キーを探す。優先順位はプレフィックス外側
/// （正典 ＞ 別名）×サフィックス内側（`suffixes` の並び順＝正典表記 ＞ 別表記）。
fn scoped_value<'a>(
    kv: &'a BTreeMap<String, String>,
    scope: usize,
    suffixes: &[&str],
) -> Option<&'a str> {
    let (canon, alias) = scope_prefixes(scope);
    let prefixes = [Some(canon.as_str()), alias];
    for prefix in prefixes.into_iter().flatten() {
        for suffix in suffixes {
            if let Some(v) = kv.get(&format!("{prefix}{suffix}")) {
                return Some(v);
            }
        }
    }
    None
}

/// スコープ別キーの 2 層カスケード（ghost ＜ shell・後勝ち＝shell 優先）。
fn cascade2<'a>(
    ghost_kv: &'a BTreeMap<String, String>,
    shell_kv: &'a BTreeMap<String, String>,
    scope: usize,
    suffixes: &[&str],
) -> Option<&'a str> {
    scoped_value(shell_kv, scope, suffixes).or_else(|| scoped_value(ghost_kv, scope, suffixes))
}

/// 1 スコープぶんの構成を KV から解決する。
fn resolve_scope(
    scope: usize,
    ghost_kv: &BTreeMap<String, String>,
    shell_kv: &BTreeMap<String, String>,
) -> ScopeConfig {
    // alignmenttodesktop の綴り（正典 ＞ 別綴 alignmentondesktop・正典表 row 1 注記）
    const ALIGN_SUFFIXES: [&str; 2] = [
        "seriko.alignmenttodesktop",
        "seriko.alignmentondesktop",
    ];
    fn global_align(kv: &BTreeMap<String, String>) -> Option<&String> {
        ALIGN_SUFFIXES.iter().find_map(|k| kv.get(*k))
    }

    // 4 層カスケード: 後勝ち＝強い層から順に探す（shell scope > shell 全体 > ghost scope > ghost 全体）
    let alignment_raw = scoped_value(shell_kv, scope, &ALIGN_SUFFIXES)
        .or_else(|| global_align(shell_kv).map(String::as_str))
        .or_else(|| scoped_value(ghost_kv, scope, &ALIGN_SUFFIXES))
        .or_else(|| global_align(ghost_kv).map(String::as_str));
    let alignment = match alignment_raw {
        None | Some("bottom") => Alignment::Bottom,
        Some("free") => Alignment::Free,
        Some(other) => Alignment::Seam(other.to_owned()),
    };

    // 両表記寛容（2.7）: X は defaultx ＞ defaultleft・Y は defaulty ＞ defaulttop（DD3）
    let default_x = parse_i32(
        cascade2(ghost_kv, shell_kv, scope, &["defaultx", "defaultleft"]),
        "defaultx/defaultleft",
        scope,
    );
    let default_y = parse_i32(
        cascade2(ghost_kv, shell_kv, scope, &["defaulty", "defaulttop"]),
        "defaulty/defaulttop",
        scope,
    );

    let balloon_alignment = match cascade2(ghost_kv, shell_kv, scope, &["balloon.alignment"]) {
        None | Some("left") => BalloonSide::Left,
        Some("right") => BalloonSide::Right,
        Some(other) => {
            warn!(scope, value = other, "balloon.alignment の未知値（既定 left として続行）");
            BalloonSide::Left
        }
    };

    // balloon offset は両成分が揃ったときのみ Some（design「両方あるときのみ」）
    let offset_x = parse_i32(
        cascade2(ghost_kv, shell_kv, scope, &["balloon.offsetx"]),
        "balloon.offsetx",
        scope,
    );
    let offset_y = parse_i32(
        cascade2(ghost_kv, shell_kv, scope, &["balloon.offsety"]),
        "balloon.offsety",
        scope,
    );
    let balloon_offset = offset_x.zip(offset_y);

    ScopeConfig {
        alignment,
        default_x,
        default_y,
        balloon_alignment,
        balloon_offset,
    }
}

/// 数値スロットの解釈。数値化不能は panic せず `warn!`＋None（log-first・純粋維持）。
fn parse_i32(raw: Option<&str>, key: &str, scope: usize) -> Option<i32> {
    let raw = raw?;
    match raw.parse::<i32>() {
        Ok(v) => Some(v),
        Err(_) => {
            warn!(scope, key, value = raw, "配置キーの数値化に失敗（None として続行）");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// テスト補助: `(key, value)` ペア列から `parse_kv` 出力相当の `BTreeMap` を組む。
    fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// scope0 のみ（信号なし）の空 KV ペア。
    fn empty() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    // ------------------------------------------------------------------
    // T-C1: 4 層カスケード（ghost 全体＜ghost scope＜shell 全体＜shell scope）
    // ------------------------------------------------------------------

    /// T-C1: `alignment` の 4 層カスケードを、各層の存在有無 2^4=16 全パターンで固定する。
    /// 層マーカーには未知値（→ `Alignment::Seam`）を使い、勝者の層をそのまま観測する。
    /// 全層欠落時は既定 `Bottom`（2.2）。
    #[test]
    fn t_c1_alignment_four_layer_cascade_all_patterns() {
        for mask in 0u32..16 {
            let mut ghost = BTreeMap::new();
            let mut shell = BTreeMap::new();
            if mask & 1 != 0 {
                // 第 1 層（最弱）: ghost 全体
                ghost.insert("seriko.alignmenttodesktop".into(), "L1".into());
            }
            if mask & 2 != 0 {
                // 第 2 層: ghost スコープ別
                ghost.insert("sakura.seriko.alignmenttodesktop".into(), "L2".into());
            }
            if mask & 4 != 0 {
                // 第 3 層: shell 全体
                shell.insert("seriko.alignmenttodesktop".into(), "L3".into());
            }
            if mask & 8 != 0 {
                // 第 4 層（最強）: shell スコープ別
                shell.insert("sakura.seriko.alignmenttodesktop".into(), "L4".into());
            }

            let cfg = build_placement_config(&ghost, &shell);
            let expected = if mask & 8 != 0 {
                Alignment::Seam("L4".into())
            } else if mask & 4 != 0 {
                Alignment::Seam("L3".into())
            } else if mask & 2 != 0 {
                Alignment::Seam("L2".into())
            } else if mask & 1 != 0 {
                Alignment::Seam("L1".into())
            } else {
                Alignment::Bottom
            };
            assert_eq!(
                cfg.scopes[&0].alignment, expected,
                "mask={mask:04b} で勝者層が不一致"
            );
        }
    }

    /// T-C1: 実値（`bottom`/`free`）でも後勝ちが成立する（shell スコープ別が最強）。
    #[test]
    fn t_c1_alignment_real_values_later_layer_wins() {
        // ghost 全体 free ＜ shell スコープ別 bottom → Bottom
        let ghost = kv(&[("seriko.alignmenttodesktop", "free")]);
        let shell = kv(&[("sakura.seriko.alignmenttodesktop", "bottom")]);
        let cfg = build_placement_config(&ghost, &shell);
        assert_eq!(cfg.scopes[&0].alignment, Alignment::Bottom);

        // ghost スコープ別 bottom ＜ shell 全体 free → Free
        let ghost = kv(&[("sakura.seriko.alignmenttodesktop", "bottom")]);
        let shell = kv(&[("seriko.alignmenttodesktop", "free")]);
        let cfg = build_placement_config(&ghost, &shell);
        assert_eq!(cfg.scopes[&0].alignment, Alignment::Free);
    }

    /// T-C1: `default_x` はスコープ別キーのみの 2 層カスケード（ghost ＜ shell）。
    /// 存在有無 2^2=4 全パターン。
    #[test]
    fn t_c1_default_x_two_layer_cascade_all_patterns() {
        for mask in 0u32..4 {
            let mut ghost = BTreeMap::new();
            let mut shell = BTreeMap::new();
            if mask & 1 != 0 {
                ghost.insert("sakura.defaultx".into(), "10".into());
            }
            if mask & 2 != 0 {
                shell.insert("sakura.defaultx".into(), "20".into());
            }
            let cfg = build_placement_config(&ghost, &shell);
            let expected = if mask & 2 != 0 {
                Some(20)
            } else if mask & 1 != 0 {
                Some(10)
            } else {
                None
            };
            assert_eq!(cfg.scopes[&0].default_x, expected, "mask={mask:02b}");
        }
    }

    /// T-C1: `balloon.alignment` も 2 層カスケード（shell が ghost に勝つ）。
    #[test]
    fn t_c1_balloon_alignment_two_layer_cascade() {
        let ghost = kv(&[("sakura.balloon.alignment", "left")]);
        let shell = kv(&[("sakura.balloon.alignment", "right")]);
        let cfg = build_placement_config(&ghost, &shell);
        assert_eq!(cfg.scopes[&0].balloon_alignment, BalloonSide::Right);

        // ghost 側のみでも拾う
        let cfg = build_placement_config(&ghost, &empty());
        assert_eq!(cfg.scopes[&0].balloon_alignment, BalloonSide::Left);

        // 未指定は既定 Left
        let cfg = build_placement_config(&empty(), &empty());
        assert_eq!(cfg.scopes[&0].balloon_alignment, BalloonSide::Left);
    }

    // ------------------------------------------------------------------
    // T-C2: 両表記寛容（defaultx⇔defaultleft／defaulty⇔defaulttop）と優先順位
    // ------------------------------------------------------------------

    /// T-C2: `defaultleft` 単独は X スロットへ、`defaulttop` 単独は Y スロットへ受理する。
    #[test]
    fn t_c2_variant_notation_alone_is_accepted() {
        let shell = kv(&[("sakura.defaultleft", "42"), ("sakura.defaulttop", "24")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].default_x, Some(42));
        assert_eq!(cfg.scopes[&0].default_y, Some(24));
    }

    /// T-C2: 同層競合時は `defaultx` が `defaultleft` に勝つ（DD3）。
    #[test]
    fn t_c2_defaultx_wins_over_defaultleft_same_layer() {
        let shell = kv(&[("sakura.defaultx", "1"), ("sakura.defaultleft", "2")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].default_x, Some(1));
    }

    /// T-C2: 同層競合時は `defaulty` が `defaulttop` に勝つ（DD3 と同方向）。
    #[test]
    fn t_c2_defaulty_wins_over_defaulttop_same_layer() {
        let shell = kv(&[("sakura.defaulty", "3"), ("sakura.defaulttop", "4")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].default_y, Some(3));
    }

    /// T-C2: 層が違えば表記より層が勝つ（shell の `defaultleft` ＞ ghost の `defaultx`）。
    #[test]
    fn t_c2_layer_beats_notation() {
        let ghost = kv(&[("sakura.defaultx", "5")]);
        let shell = kv(&[("sakura.defaultleft", "7")]);
        let cfg = build_placement_config(&ghost, &shell);
        assert_eq!(cfg.scopes[&0].default_x, Some(7));
    }

    /// T-C2: 別綴 `alignmentondesktop` を寛容受理し、同層に正典綴があれば正典側が勝つ。
    #[test]
    fn t_c2_alignmentondesktop_spelling_variant() {
        let shell = kv(&[("seriko.alignmentondesktop", "free")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].alignment, Alignment::Free);

        let shell = kv(&[
            ("seriko.alignmenttodesktop", "bottom"),
            ("seriko.alignmentondesktop", "free"),
        ]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].alignment, Alignment::Bottom);
    }

    /// T-C2: `char0.`/`char1.` は `sakura.`/`kero.` の de-facto 別名として同スロットへ
    /// 寛容受理し、正典キーと衝突時は正典側が優先される。
    #[test]
    fn t_c2_char0_char1_prefix_aliases() {
        // char0.defaultx 単独 → scope0 の X へ
        let shell = kv(&[("char0.defaultx", "11"), ("kero.defaultx", "0")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].default_x, Some(11));

        // 正典 sakura.defaultx が char0.defaultx に勝つ
        let shell = kv(&[
            ("sakura.defaultx", "1"),
            ("char0.defaultx", "9"),
            ("kero.defaultx", "0"),
        ]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].default_x, Some(1));

        // char1.defaulty は scope1（kero）の Y スロットへ（kero.* で scope1 検出済み）
        let shell = kv(&[("kero.defaultx", "0"), ("char1.defaulty", "13")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&1].default_y, Some(13));

        // 正典 kero.defaulty が char1.defaulty に勝つ
        let shell = kv(&[("kero.defaulty", "2"), ("char1.defaulty", "8")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&1].default_y, Some(2));
    }

    /// T-C2: 数値化不能な値はパニックせず None（純粋・呼び出し側 warn の契約）。
    #[test]
    fn t_c2_unparseable_numeric_becomes_none() {
        let shell = kv(&[("sakura.defaultx", "abc"), ("sakura.defaulty", "12.5")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].default_x, None);
        assert_eq!(cfg.scopes[&0].default_y, None);
    }

    // ------------------------------------------------------------------
    // T-C3: スコープ検出シグナル（DD6）
    // ------------------------------------------------------------------

    /// T-C3: 信号なしでは scope0 のみ常設。
    #[test]
    fn t_c3_no_signal_yields_scope0_only() {
        let cfg = build_placement_config(&empty(), &empty());
        assert_eq!(cfg.scopes.keys().copied().collect::<Vec<_>>(), vec![0]);
    }

    /// T-C3: ghost `kero.name` で scope1 が生える。
    #[test]
    fn t_c3_ghost_kero_name_adds_scope1() {
        let ghost = kv(&[("kero.name", "エモ")]);
        let cfg = build_placement_config(&ghost, &empty());
        assert_eq!(cfg.scopes.keys().copied().collect::<Vec<_>>(), vec![0, 1]);
    }

    /// T-C3: shell の `kero.` 接頭キー存在でも scope1 が生える。
    #[test]
    fn t_c3_shell_kero_prefix_adds_scope1() {
        let shell = kv(&[("kero.balloon.alignment", "right")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes.keys().copied().collect::<Vec<_>>(), vec![0, 1]);
    }

    /// T-C3: `char{n}.`（n≥2）キーは ghost/shell どちら側でも scope n を追加する。
    #[test]
    fn t_c3_char_n_prefix_adds_scope_n() {
        let ghost = kv(&[("kero.name", "エモ"), ("char2.name", "三人目")]);
        let shell = kv(&[("char3.defaultx", "0")]);
        let cfg = build_placement_config(&ghost, &shell);
        assert_eq!(
            cfg.scopes.keys().copied().collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        // 追加スコープにもスコープ別キーが効く
        assert_eq!(cfg.scopes[&3].default_x, Some(0));
    }

    /// T-C3: `charset` キーは `char{n}.` 検出に誤マッチしない。
    /// `char0.`/`char1.`（別名）はスコープ検出シグナルではない（DD6 は kero 信号のみ）。
    #[test]
    fn t_c3_detection_rejects_non_signals() {
        let shell = kv(&[("charset", "UTF-8"), ("char1.defaultx", "0")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes.keys().copied().collect::<Vec<_>>(), vec![0]);
    }

    // ------------------------------------------------------------------
    // T-C4: emo2 実測 KV 固定（正典表検証行の檻）
    // ------------------------------------------------------------------

    /// T-C4: emo2 実測 descript 相当の入力（`parse_kv` 実出力形）から
    /// `{0: bottom/defaultx 0/balloon left, 1: bottom/defaultx 0/balloon right}` を得る。
    /// あわせてデモ由来の固定座標定数（(400,200)/(335,0) 等）が構成のどの数値
    /// スロットにも現れないことを固定する（1.5——本モジュールは KV の純粋転記であり、
    /// 入力に無い数値は出力に出現し得ない）。
    #[test]
    fn t_c4_emo2_actual_kv_pins_canonical_result() {
        // emo2 ghost/master/descript.txt の配置関連部分（実測・配置系キーなし＋kero.name）
        let ghost_kv = areka_parsers::kv::parse_kv(
            "charset,UTF-8\n\
             type,ghost\n\
             name,えも？？\n\
             sakura.name,むらさき\n\
             kero.name,エモ\n\
             shiori,pasta.dll\n",
        );
        // emo2 shell/master/descript.txt の配置関連部分（実測）
        let shell_kv = areka_parsers::kv::parse_kv(
            "charset,UTF-8\n\
             type,shell\n\
             name,「コンフィズリー」＆「City-Pop'n」\n\
             seriko.use_self_alpha,1\n\
             seriko.alignmenttodesktop,bottom\n\
             sakura.defaultx,0\n\
             kero.defaultx,0\n\
             sakura.balloon.alignment,left\n\
             kero.balloon.alignment,right\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             sakura.menu,auto\n",
        );

        let cfg = build_placement_config(&ghost_kv, &shell_kv);

        assert_eq!(cfg.scopes.keys().copied().collect::<Vec<_>>(), vec![0, 1]);

        let s0 = &cfg.scopes[&0];
        assert_eq!(s0.alignment, Alignment::Bottom);
        assert_eq!(s0.default_x, Some(0));
        assert_eq!(s0.default_y, None);
        assert_eq!(s0.balloon_alignment, BalloonSide::Left);
        assert_eq!(s0.balloon_offset, None);

        let s1 = &cfg.scopes[&1];
        assert_eq!(s1.alignment, Alignment::Bottom);
        assert_eq!(s1.default_x, Some(0));
        assert_eq!(s1.default_y, None);
        assert_eq!(s1.balloon_alignment, BalloonSide::Right);
        assert_eq!(s1.balloon_offset, None);

        // デモ定数の不在: 全数値スロットを走査し 400/200/335 が一切出現しない
        let demo_constants = [400, 200, 335];
        for (scope, sc) in &cfg.scopes {
            let mut values = vec![];
            values.extend(sc.default_x);
            values.extend(sc.default_y);
            if let Some((ox, oy)) = sc.balloon_offset {
                values.push(ox);
                values.push(oy);
            }
            for v in values {
                assert!(
                    !demo_constants.contains(&v),
                    "scope {scope} にデモ定数 {v} が混入"
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // T-C5: zorder/sticky/dpi の raw 転記シーム（5.2・DD11）
    // ------------------------------------------------------------------

    /// T-C5: shell の `seriko.zorder`／`seriko.sticky-window`／`seriko.dpi` を
    /// 生文字列のまま転記する（実挙動フィールドなし）。値中のカンマも保持。
    #[test]
    fn t_c5_seam_fields_raw_transcription() {
        let shell_kv = areka_parsers::kv::parse_kv(
            "seriko.zorder,1,0\n\
             seriko.sticky-window,0,1\n\
             seriko.dpi,120\n",
        );
        let cfg = build_placement_config(&empty(), &shell_kv);
        assert_eq!(cfg.zorder_raw.as_deref(), Some("1,0"));
        assert_eq!(cfg.sticky_window_raw.as_deref(), Some("0,1"));
        assert_eq!(cfg.shell_dpi_raw.as_deref(), Some("120"));
    }

    /// T-C5: 未指定なら None。ghost 側の同名キーは転記対象でない（所在は shell descript）。
    #[test]
    fn t_c5_seam_fields_absent_and_shell_only() {
        let ghost = kv(&[("seriko.zorder", "1"), ("seriko.dpi", "144")]);
        let cfg = build_placement_config(&ghost, &empty());
        assert_eq!(cfg.zorder_raw, None);
        assert_eq!(cfg.sticky_window_raw, None);
        assert_eq!(cfg.shell_dpi_raw, None);
    }

    /// T-C5 補: `balloon.offsetx/offsety` は両方あるときのみ Some、片方では None。
    #[test]
    fn t_c5_balloon_offset_requires_both_components() {
        let shell = kv(&[
            ("sakura.balloon.offsetx", "30"),
            ("sakura.balloon.offsety", "-5"),
        ]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].balloon_offset, Some((30, -5)));

        let shell = kv(&[("sakura.balloon.offsetx", "30")]);
        let cfg = build_placement_config(&empty(), &shell);
        assert_eq!(cfg.scopes[&0].balloon_offset, None);
    }
}
