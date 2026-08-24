use super::*;

use super::test_support::{CapturedEvent, TempDir, capture_events};

/// R1.5/R7.2 系列を明示した面判定: 面であるか否かは **どの系列（scope）で見るか**に依存する。
///
/// 単一接頭辞固定時代の「`balloonk0.png` は枠でない」という無条件の判定を、系列を明示した
/// 判定へ**意味を変えて**更新したもの（design.md「Implementation Notes / Integration」）——
/// scope 0 の連鎖では `balloonk0.png` を採用せず、scope 1 の連鎖では面 0 として採用する。
/// 一方、正典でバルーン面系列と定義されていないファイル（`balloonc*`/`arrow*`/`marker*`/
/// `online*`・非数字・非 png）はどの系列から見ても面でない。
#[test]
fn face_judgment_is_series_explicit_not_fixed_prefix() {
    // 相方側の面: scope 0 の連鎖では採用されず、scope 1 の連鎖では面 0 になる。
    assert!(
        selected(&["balloonk0.png", "balloons0.png"], 0)
            .iter()
            .all(|f| f.3 != "balloonk0.png"),
        "scope 0 の連鎖に balloonk は無く相方側の面を採用しない"
    );
    assert_eq!(
        selected(&["balloonk0.png"], 1),
        vec![(
            0,
            "balloonk".to_string(),
            ChainTier::Own,
            "balloonk0.png".to_string()
        )],
        "scope 1 の連鎖では balloonk0.png が面 0 として採用される"
    );
    // 本体側の面と大小無視（系列を明示しても判定 3 段そのものは不変）。
    assert_eq!(face_id_of("balloons", "balloons0.png"), Some(0));
    assert_eq!(face_id_of("balloons", "balloons12.png"), Some(12));
    assert_eq!(face_id_of("balloons", "BALLOONS3.PNG"), Some(3), "大小無視");
    // 非バルーン面（どの系列から見ても面でない・R5.3）。
    for prefix in ["balloons", "balloonk", "balloonp0def", "balloonp1def"] {
        for name in [
            "balloonc0.png", // 入力ボックス
            "arrow0.png",    // スクロール矢印
            "marker.png",    // マーカー
            "online0.png",   // 受信アニメ
            "balloons.png",  // 数字が無い
            "balloonsX.png", // 非数字
            "balloons0.txt", // 非 png
        ] {
            assert_eq!(
                face_id_of(prefix, name),
                None,
                "系列 {prefix} が非バルーン面 {name} を採用した"
            );
        }
    }
}

// ── 檻 1: scope→接頭辞優先連鎖の導出（R1.1/1.6/1.8/1.9/1.10・R7.1）─────────────

/// `(接頭辞, tier)` の組で連鎖を突き合わせるための簡約形（表明の可読性のため）。
fn chain_pairs(family: &SeriesFamily, scope: u32) -> Vec<(String, ChainTier)> {
    prefix_chain(family, scope)
        .into_iter()
        .map(|p| (p.prefix, p.tier))
        .collect()
}

/// R1.1: scope 0 の連鎖は `balloonp0def`（正規名）→ `balloons`（旧名）の 2 段で、
/// 双方とも当該 scope 自身の候補＝tier `Own`。scope 0 は相方系列段もデフォルト段も持たない。
#[test]
fn prefix_chain_scope0_is_canonical_then_legacy_all_own() {
    assert_eq!(
        chain_pairs(&BALLOON_FAMILY, 0),
        vec![
            ("balloonp0def".to_string(), ChainTier::Own),
            ("balloons".to_string(), ChainTier::Own),
        ],
        "scope 0 は Own 段のみ（正規名が先頭・旧名が後続）"
    );
}

/// R1.1: scope 1 の連鎖は Own（`balloonp1def` → `balloonk`）＋ Default（`balloonp0def` →
/// `balloons`）の 4 段。scope 1 は相方系列段を持たない（自身が相方系列そのもの）。
#[test]
fn prefix_chain_scope1_is_own_then_default() {
    assert_eq!(
        chain_pairs(&BALLOON_FAMILY, 1),
        vec![
            ("balloonp1def".to_string(), ChainTier::Own),
            ("balloonk".to_string(), ChainTier::Own),
            ("balloonp0def".to_string(), ChainTier::Default),
            ("balloons".to_string(), ChainTier::Default),
        ],
        "scope 1 は Own 2 段＋Default 2 段（KeroNamed 段なし）"
    );
}

/// R1.1/R1.6: scope 5（n≧2 の代表）の連鎖は Own（`balloonp5def` のみ＝旧名なし）＋
/// KeroNamed（`balloonk`）＋ Default（`balloonp0def` → `balloons`）。
#[test]
fn prefix_chain_scope5_names_kero_then_default() {
    assert_eq!(
        chain_pairs(&BALLOON_FAMILY, 5),
        vec![
            ("balloonp5def".to_string(), ChainTier::Own),
            ("balloonk".to_string(), ChainTier::KeroNamed),
            ("balloonp0def".to_string(), ChainTier::Default),
            ("balloons".to_string(), ChainTier::Default),
        ],
        "scope 5 は Own 1 段＋KeroNamed 1 段＋Default 2 段"
    );
}

/// R1.1: 相方系列の段は正典が名指しした系列（`balloonk`）であって、scope 1 の解決へ再帰的に
/// 縮退するものではない。ゆえに n≧2 の連鎖に scope 1 の正規名 `balloonp1def` は現れない。
#[test]
fn prefix_chain_ge2_excludes_scope1_canonical_name() {
    for scope in 2u32..=8 {
        let chain = prefix_chain(&BALLOON_FAMILY, scope);
        assert!(
            !chain.iter().any(|p| p.prefix == "balloonp1def"),
            "scope {scope} の連鎖に scope 1 の正規名が混入している: {chain:?}"
        );
        // 名指し相方系列は旧名 `balloonk` 1 本で、tier は KeroNamed。
        let kero: Vec<_> = chain
            .iter()
            .filter(|p| p.tier == ChainTier::KeroNamed)
            .collect();
        assert_eq!(kero.len(), 1, "scope {scope} の相方系列段は 1 段");
        assert_eq!(
            kero[0].prefix, "balloonk",
            "相方系列段は正典名指しの balloonk"
        );
    }
}

/// R1.1/R1.8/R1.9: 各段は「正規名が先頭・旧名が後続」であり、scope を数値のみで扱う
/// （2 値列挙もさくらスクリプト語彙も内部の正準表現にしない）ことを、任意 scope の
/// 正規名 `balloonp{s}def` が常に連鎖先頭であることで固定する。
#[test]
fn prefix_chain_head_is_scope_canonical_name_for_any_scope() {
    for scope in [0u32, 1, 2, 3, 7, 42, 100] {
        let chain = prefix_chain(&BALLOON_FAMILY, scope);
        assert_eq!(
            chain[0],
            SeriesPrefix {
                prefix: format!("balloonp{scope}def"),
                tier: ChainTier::Own,
            },
            "scope {scope} の連鎖先頭は当該 scope の正規名（tier=Own）"
        );
    }
}

/// R1.9（縮退シーム）/R7.1(c): 装飾族の**一段深い旧名**（`arrows` → `arrow`）も、
/// 同じ表構造（可変長の旧名候補列）で構造改変なしに表現できる。本仕様は吹き出し族のみ
/// 実装するが、表データが族でパラメタ化されている事実をここで固定する。
#[test]
fn series_family_table_expresses_deeper_legacy_names() {
    const ARROW_FAMILY: SeriesFamily = SeriesFamily {
        base: "arrow",
        scope0_legacy: &["arrows", "arrow"],
        scope1_legacy: &["arrowk"],
    };

    assert_eq!(
        chain_pairs(&ARROW_FAMILY, 0),
        vec![
            ("arrowp0def".to_string(), ChainTier::Own),
            ("arrows".to_string(), ChainTier::Own),
            ("arrow".to_string(), ChainTier::Own),
        ],
        "scope 0 の旧名候補が 2 段でも構造改変なしに連鎖へ載る"
    );
    assert_eq!(
        chain_pairs(&ARROW_FAMILY, 2),
        vec![
            ("arrowp2def".to_string(), ChainTier::Own),
            ("arrowk".to_string(), ChainTier::KeroNamed),
            ("arrowp0def".to_string(), ChainTier::Default),
            ("arrows".to_string(), ChainTier::Default),
            ("arrow".to_string(), ChainTier::Default),
        ],
        "デフォルト段にも可変長の旧名候補列がそのまま展開される"
    );
}

// ── 檻 2-5: 面 ID 単位の連鎖探索（純核 `select_faces`・R1.2/1.3/1.4/1.5・R7.1）─────

/// `(surface_id, 採用接頭辞, tier, ファイル名)` の簡約形（表明の可読性のため）。
fn selected(names: &[&str], scope: u32) -> Vec<(u32, String, ChainTier, String)> {
    let chain = prefix_chain(&BALLOON_FAMILY, scope);
    select_faces(names, &chain)
        .into_iter()
        .map(|f| (f.surface_id, f.prefix, f.tier, f.file_name))
        .collect()
}

/// 檻 2（R1.2/R1.10）正規名優先: 正規名 `{base}p{s}def{ID}` と旧名が併存する場合、
/// 連鎖先頭の正規名が採られる（scope 0 の `balloonp0def`・scope 1 の `balloonp1def` とも）。
#[test]
fn select_faces_prefers_canonical_name_over_legacy() {
    assert_eq!(
        selected(&["balloons0.png", "balloonp0def0.png"], 0),
        vec![(
            0,
            "balloonp0def".to_string(),
            ChainTier::Own,
            "balloonp0def0.png".to_string()
        )],
        "scope 0: 正規名 balloonp0def0.png が旧名 balloons0.png に優先する"
    );
    assert_eq!(
        selected(&["balloons0.png", "balloonk0.png", "balloonp1def0.png"], 1),
        vec![(
            0,
            "balloonp1def".to_string(),
            ChainTier::Own,
            "balloonp1def0.png".to_string()
        )],
        "scope 1: 正規名 balloonp1def0.png が旧名 balloonk0.png にも Default 段にも優先する"
    );
}

/// 檻 3（R1.3）ID 単位フォールバック: `balloonk0` があり `balloonk1` が無いとき、
/// scope 1 の面 0 は `balloonk0`（tier=Own）・面 1 は `balloons1`（tier=Default）。
/// ある ID の欠落を理由に系列全体を後段の接頭辞へ切り替えない。
#[test]
fn select_faces_falls_back_per_face_id_not_per_series() {
    let names = ["balloonk0.png", "balloons0.png", "balloons1.png"];
    assert_eq!(
        selected(&names, 1),
        vec![
            (
                0,
                "balloonk".to_string(),
                ChainTier::Own,
                "balloonk0.png".to_string()
            ),
            (
                1,
                "balloons".to_string(),
                ChainTier::Default,
                "balloons1.png".to_string()
            ),
        ],
        "面 0 は相方側・面 1 のみデフォルト段へ縮退する（系列一括切替でない）"
    );
    assert_eq!(
        selected(&names, 0),
        vec![
            (
                0,
                "balloons".to_string(),
                ChainTier::Own,
                "balloons0.png".to_string()
            ),
            (
                1,
                "balloons".to_string(),
                ChainTier::Own,
                "balloons1.png".to_string()
            ),
        ],
        "scope 0 の連鎖に balloonk は無く、相方側の面を一切採らない"
    );
}

/// R1.3（ID 集合の和）: 面 ID は連鎖内の**全**接頭辞にまたがる和であり、先頭接頭辞の
/// ID 集合に閉じない。先頭にしか無い ID も後段にしか無い ID も双方が採られる。
#[test]
fn select_faces_unions_face_ids_across_all_prefixes() {
    assert_eq!(
        selected(&["balloonp1def2.png", "balloons0.png"], 1),
        vec![
            (
                0,
                "balloons".to_string(),
                ChainTier::Default,
                "balloons0.png".to_string()
            ),
            (
                2,
                "balloonp1def".to_string(),
                ChainTier::Own,
                "balloonp1def2.png".to_string()
            ),
        ],
        "後段にしか無い ID 0 と先頭にしか無い ID 2 の双方が面集合に入る"
    );
}

/// 檻 4（R1.4）後方互換収束: 連鎖の先頭側接頭辞の画像を 1 枚も含まないバルーンでは、
/// 全 scope が `balloons` 系列へ解決され、同一の面集合（ID と採用ファイル）を得る。
/// tier は連鎖上の地位ゆえ scope で異なる（scope 0 は Own・scope≧1 は Default）。
#[test]
fn select_faces_converges_to_legacy_series_for_all_scopes() {
    let names = ["balloons0.png", "balloons1.png", "balloons2.png"];
    let faces_of = |scope: u32| -> Vec<(u32, String, String)> {
        select_faces(&names, &prefix_chain(&BALLOON_FAMILY, scope))
            .into_iter()
            .map(|f| (f.surface_id, f.prefix, f.file_name))
            .collect()
    };

    let scope0 = faces_of(0);
    assert_eq!(scope0.len(), 3, "balloons 3 枚がそのまま面集合になる");
    for scope in [1u32, 2, 7] {
        assert_eq!(
            faces_of(scope),
            scope0,
            "scope {scope} の面集合が scope 0 と一致しない（後方互換収束の破れ）"
        );
    }
}

/// 檻 5（R1.5）非バルーン面除外: 入力ウィンドウ用 `balloonc*`・装飾用 `arrow*`/`marker*`/
/// `online*`・非数字・非 png はどの連鎖でも採用されない。接頭辞は完全一致ゆえ
/// `balloonc0.png` を `balloonk` の面と誤認する事故は構造的に起こり得ない。
#[test]
fn select_faces_rejects_non_balloon_faces() {
    let names = [
        "balloonc0.png", // 入力ウィンドウ（バルーン面でない）
        "balloonc1.png", // 同上
        "arrow0.png",    // スクロール矢印（装飾）
        "arrows0.png",   // 同上
        "marker.png",    // マーカー（装飾）
        "online0.png",   // 受信アニメ（装飾）
        "balloonsX.png", // 残余が非数字
        "balloons0.txt", // 非 png
        "balloons.png",  // 残余が空
        "balloonk.png",  // 残余が空（相方側）
    ];
    for scope in [0u32, 1, 5] {
        assert!(
            selected(&names, scope).is_empty(),
            "scope {scope} の連鎖が非バルーン面を採用した: {:?}",
            selected(&names, scope)
        );
    }
    assert_eq!(
        face_id_of("balloonk", "balloonc0.png"),
        None,
        "balloonc を balloonk と誤認しない（接頭辞完全一致）"
    );
}

/// R1.5（判定 3 段）: 接頭辞 strip（大小無視）→ `.png` strip → 残余の全数字化。
/// いずれか 1 段でも満たさなければ面でない。
#[test]
fn face_id_of_is_strict_three_stage_predicate() {
    assert_eq!(face_id_of("balloons", "balloons0.png"), Some(0));
    assert_eq!(face_id_of("balloons", "balloons12.png"), Some(12));
    assert_eq!(face_id_of("balloonk", "balloonk1.png"), Some(1));
    assert_eq!(
        face_id_of("balloonp0def", "BalloonP0Def3.PNG"),
        Some(3),
        "大小無視で判定する"
    );
    assert_eq!(
        face_id_of("balloons", "balloonk0.png"),
        None,
        "接頭辞は完全一致（部分一致で拾わない）"
    );
    assert_eq!(face_id_of("balloons", "balloons.png"), None, "残余が空");
    assert_eq!(
        face_id_of("balloons", "balloonsX.png"),
        None,
        "残余が非数字"
    );
    assert_eq!(face_id_of("balloons", "balloons0.txt"), None, "非 png");
    assert_eq!(
        face_id_of("balloons", "balloons+0.png"),
        None,
        "符号付きは全数字でない"
    );
}

/// 決定論（設計 Postconditions）: 戻りは surface id **昇順**であり、入力（ディレクトリ
/// 走査順）に依存しない。辞書順ではなく数値順である（`balloons10` は `balloons2` の後）。
#[test]
fn select_faces_is_deterministic_regardless_of_input_order() {
    let ascending = [
        "balloons0.png",
        "balloons1.png",
        "balloons2.png",
        "balloons10.png",
    ];
    let shuffled = [
        "balloons10.png",
        "balloons2.png",
        "balloons0.png",
        "balloons1.png",
    ];
    let a = selected(&ascending, 0);
    assert_eq!(a, selected(&shuffled, 0), "入力順に結果が依存しない");
    assert_eq!(
        a.iter().map(|f| f.0).collect::<Vec<_>>(),
        vec![0, 1, 2, 10],
        "surface id 昇順（辞書順でない）"
    );
}

/// ファイル名は**原形保持**（実 WIC デコードが実パスを読むため大小を正規化しない）。
/// 同一 (ID, 接頭辞) に大小違いが併存する病的入力では辞書順最小を採り、走査順に
/// 結果を左右させない。
#[test]
fn select_faces_preserves_original_file_name_case() {
    assert_eq!(
        selected(&["BALLOONS0.PNG"], 0),
        vec![(
            0,
            "balloons".to_string(),
            ChainTier::Own,
            "BALLOONS0.PNG".to_string()
        )],
        "採用ファイル名は原形のまま保持される"
    );
    let a = selected(&["BALLOONS0.PNG", "balloons0.png"], 0);
    let b = selected(&["balloons0.png", "BALLOONS0.PNG"], 0);
    assert_eq!(a, b, "大小違い併存でも走査順に依存しない");
    assert_eq!(a.len(), 1, "同一 ID の採用面は 1 つ");
}

// ── 檻 6: 採用接頭辞からの面別上書きファイル名導出（R2.2/R2.3・R7.1）─────────────

/// 檻 6（R2.2/R2.3）: 面別上書きファイル名は**採用接頭辞に対応して**導出され、ID 単位
/// フォールバックで後段へ落ちた面ではその後段接頭辞の名前になる。scope 1 が `balloonk0`
/// を採ったなら `balloonk0s.txt`・面 1 が `balloons1` へ縮退したなら `balloons1s.txt`
/// （連鎖の他の接頭辞の上書き層を引かない）。
#[test]
fn override_file_name_follows_adopted_prefix_per_face() {
    let names = ["balloonk0.png", "balloons0.png", "balloons1.png"];
    let faces = select_faces(&names, &prefix_chain(&BALLOON_FAMILY, 1));

    assert_eq!(faces.len(), 2, "面 0/1 の 2 面が解決される");
    assert_eq!(
        faces[0].override_file_name(),
        "balloonk0s.txt",
        "採用面 balloonk0 の上書き層は balloonk0s.txt（balloons0s.txt を引かない・R2.2）"
    );
    assert_eq!(
        faces[1].override_file_name(),
        "balloons1s.txt",
        "デフォルト段へ縮退した面 1 の上書き層は後段接頭辞の balloons1s.txt（R2.3）"
    );
}

/// 檻 6（続き・R2.2）: 正規名を採った面は正規名の上書き層を引き、実ファイル名の大小に
/// 依らず**連鎖上の接頭辞**（小文字正準形）から導出される。
#[test]
fn override_file_name_uses_chain_prefix_not_file_case() {
    let faces = select_faces(&["balloonp1def2.png"], &prefix_chain(&BALLOON_FAMILY, 1));
    assert_eq!(
        faces[0].override_file_name(),
        "balloonp1def2s.txt",
        "正規名採用面は正規名の上書き層を引く"
    );

    let upper = select_faces(&["BALLOONS0.PNG"], &prefix_chain(&BALLOON_FAMILY, 0));
    assert_eq!(
        upper[0].file_name, "BALLOONS0.PNG",
        "実ファイル名は原形保持（前提の再確認）"
    );
    assert_eq!(
        upper[0].override_file_name(),
        "balloons0s.txt",
        "上書き層名は連鎖上の接頭辞から導出する（実ファイル名の大小に依らない）"
    );
}

// ── 公開 API `resolve_balloon_faces`（列挙 1 回＋選択核・R1.7/6.1/6.2/6.4）─────────

/// R1.7: 面 ID 0 がどの接頭辞でも解決できないとき、log-first で `Err` を返す
/// （全消費者共通の単一施行点・既存縮退経路へ伝播させる）。
#[test]
fn resolve_balloon_faces_requires_face_zero() {
    let dir = TempDir::new();
    // 面 1/2 は在るが面 0 が無い（＝面 0 必在契約の違反）。
    dir.touch("balloons1.png");
    dir.touch("balloons2.png");

    let err = resolve_balloon_faces(dir.path(), 0).expect_err("面 0 不在なら Err のはず");
    assert!(
        matches!(
            err,
            PresentError::Compose(ComposeError::EmptyComposition(0))
        ),
        "面 0 不在は既存縮退経路（EmptyComposition(0)）へ畳む: {err:?}"
    );
}

/// R6.4: バルーンディレクトリの走査自体に失敗したとき、log-first で `Err` を返す。
#[test]
fn resolve_balloon_faces_errors_on_unreadable_directory() {
    let dir = TempDir::new();
    let missing = dir.path().join("no-such-balloon-dir");

    let err = resolve_balloon_faces(&missing, 0).expect_err("走査失敗なら Err のはず");
    assert!(
        matches!(
            err,
            PresentError::Compose(ComposeError::EmptyComposition(0))
        ),
        "走査失敗も既存縮退経路へ畳む: {err:?}"
    );
}

/// 決定論（設計 Postconditions）: 同一ディレクトリに対する解決は surface id **昇順**で、
/// 反復呼び出しで同一結果を返す（ディレクトリ走査順は非決定ゆえ明示ソートに依る）。
/// 非バルーン面は列挙段を通り抜けても選択核で落ちる。
#[test]
fn resolve_balloon_faces_is_deterministic_and_ascending() {
    let dir = TempDir::new();
    for name in [
        "balloons10.png",
        "balloons2.png",
        "balloons0.png",
        "balloonc0.png", // 入力ウィンドウ（面でない）
        "arrow0.png",    // 装飾（面でない）
    ] {
        dir.touch(name);
    }

    let first = resolve_balloon_faces(dir.path(), 0).expect("面 0 が在るゆえ Ok");
    let second = resolve_balloon_faces(dir.path(), 0).expect("面 0 が在るゆえ Ok");
    assert_eq!(first, second, "同一入力に対し決定論的な解決結果を返す");
    assert_eq!(
        first.iter().map(|f| f.surface_id).collect::<Vec<_>>(),
        vec![0, 2, 10],
        "surface id 昇順（辞書順でない）・非バルーン面は含まれない"
    );
}

/// R1.2/R1.3/R6.2: 実ディレクトリ経由でも ID 単位フォールバックが効き、scope 1 の面 0 は
/// 相方側（tier=Own）・面 1 のみデフォルト段（tier=Default＝warn 対象）へ縮退する。
/// 同ディレクトリを scope 0 で解決すると相方側の面を一切採らない。
#[test]
fn resolve_balloon_faces_falls_back_per_face_id_on_real_directory() {
    let dir = TempDir::new();
    dir.touch("balloonk0.png");
    dir.touch("balloons0.png");
    dir.touch("balloons1.png");

    let scope1 = resolve_balloon_faces(dir.path(), 1).expect("面 0 は balloonk0 で解決する");
    assert_eq!(
        scope1
            .iter()
            .map(|f| (
                f.surface_id,
                f.prefix.as_str(),
                f.tier,
                f.override_file_name()
            ))
            .collect::<Vec<_>>(),
        vec![
            (0, "balloonk", ChainTier::Own, "balloonk0s.txt".to_string()),
            (
                1,
                "balloons",
                ChainTier::Default,
                "balloons1s.txt".to_string()
            ),
        ],
        "面 0 は相方側・面 1 のみデフォルト段へ縮退（系列一括切替でない）"
    );

    let scope0 = resolve_balloon_faces(dir.path(), 0).expect("scope 0 の面 0 は balloons0");
    assert!(
        scope0.iter().all(|f| f.prefix == "balloons"),
        "scope 0 の連鎖に balloonk は無く相方側の面を採らない: {scope0:?}"
    );
}

/// R6.2 の本体側縮退 warn だけを抜き出す（他の warn 経路と混同しないよう level ＋ 文言で絞る）。
fn default_fallback_warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::WARN
                && e.fields
                    .get("message")
                    .is_some_and(|m| m.contains("デフォルト定義側"))
        })
        .collect()
}

/// 檻 7-a（R6.2）発火する側: scope 1 で [`ChainTier::Default`] を採った面**だけ**に warn が出て、
/// その 1 イベントに要求フィールド（scope・面 ID・採用ファイル）が揃う。同じ解決で
/// [`ChainTier::Own`] を採った面 0（`balloonk0`）には warn が出ない。
///
/// 判定分岐の**両述語**を同時に押さえる檻である——`scope >= 1` を `scope >= 2` へ改変すれば
/// warn が 0 件になり、`tier == Default` を `tier == KeroNamed` へ改変すれば scope 1 の連鎖に
/// KeroNamed 段が無いためやはり 0 件になる（いずれも RED）。
#[test]
fn resolve_warns_only_for_default_tier_face_at_scope1() {
    let dir = TempDir::new();
    dir.touch("balloonk0.png");
    dir.touch("balloons0.png");
    dir.touch("balloons1.png");

    let (result, events) = capture_events(|| resolve_balloon_faces(dir.path(), 1));
    let faces = result.expect("面 0 は balloonk0 で解決する");
    assert_eq!(
        faces
            .iter()
            .map(|f| (f.surface_id, f.tier))
            .collect::<Vec<_>>(),
        vec![(0, ChainTier::Own), (1, ChainTier::Default)],
        "前提: 面 0 は Own 採用・面 1 のみ Default 採用"
    );

    let warns = default_fallback_warns(&events);
    assert_eq!(
        warns.len(),
        1,
        "warn は Default 採用の 1 面のみ（Own 採用の面 0 には出ない）: {events:?}"
    );
    let warned = warns[0];
    assert_eq!(warned.field("scope"), Some("1"), "R6.2: scope が乗る");
    assert_eq!(
        warned.field("surface_id"),
        Some("1"),
        "R6.2: 縮退した面 ID が乗る（Own 採用の面 0 ではない）"
    );
    assert_eq!(
        warned.field("file"),
        Some("balloons1.png"),
        "R6.2: 採用ファイルが乗る"
    );
    assert_eq!(
        warned.field("prefix"),
        Some("balloons"),
        "採用接頭辞はデフォルト段のもの"
    );
}

/// 檻 7-b（R6.2）発火しない側 その 1: scope 0 は連鎖にデフォルト段を持たない（末尾の
/// `balloons` が自身の Own 候補）ゆえ、縮退の概念が無く warn を一切出さない。
#[test]
fn resolve_emits_no_fallback_warn_at_scope0() {
    let dir = TempDir::new();
    dir.touch("balloons0.png");
    dir.touch("balloons1.png");

    let (result, events) = capture_events(|| resolve_balloon_faces(dir.path(), 0));
    let faces = result.expect("面 0 が在るゆえ Ok");
    assert!(
        faces.iter().all(|f| f.tier == ChainTier::Own),
        "前提: scope 0 の採用面は全て Own（デフォルト段が存在しない）: {faces:?}"
    );
    assert!(
        events.iter().all(|e| e.level != tracing::Level::WARN),
        "scope 0 の解決は warn を一切出さない: {events:?}"
    );
}

/// 檻 7-c（R6.2）発火しない側 その 2: scope 2 で [`ChainTier::KeroNamed`]（名指し相方系列
/// `balloonk`）を採った面には warn が出ず、同じ解決で [`ChainTier::Default`] へ落ちた面
/// だけに出る。tier 述語が Default 以外へずれれば面の対応が崩れて RED になる。
#[test]
fn resolve_does_not_warn_for_kero_named_tier_at_scope2() {
    let dir = TempDir::new();
    dir.touch("balloonk0.png");
    dir.touch("balloons0.png");
    dir.touch("balloons1.png");

    let (result, events) = capture_events(|| resolve_balloon_faces(dir.path(), 2));
    let faces = result.expect("面 0 は名指し相方系列 balloonk0 で解決する");
    assert_eq!(
        faces
            .iter()
            .map(|f| (f.surface_id, f.tier))
            .collect::<Vec<_>>(),
        vec![(0, ChainTier::KeroNamed), (1, ChainTier::Default)],
        "前提: 面 0 は KeroNamed 採用・面 1 は Default 採用"
    );

    let warns = default_fallback_warns(&events);
    assert_eq!(
        warns.len(),
        1,
        "warn は Default 採用の面 1 のみ（KeroNamed 採用の面 0 には出ない）: {events:?}"
    );
    assert_eq!(
        warns[0].field("surface_id"),
        Some("1"),
        "KeroNamed 採用の面 0 ではなく Default 採用の面 1 が記録される"
    );
    assert_eq!(warns[0].field("scope"), Some("2"), "R6.2: scope が乗る");
}
