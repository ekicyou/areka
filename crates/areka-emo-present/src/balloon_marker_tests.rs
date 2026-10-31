use super::*;

/// 連鎖の接頭辞だけの簡約形。
fn prefixes(family: &SeriesFamily, scope: u32) -> Vec<String> {
    marker_chain(family, scope, None)
        .chain
        .into_iter()
        .map(|p| p.prefix)
        .collect()
}

/// `(上の実ファイル名, 下の実ファイル名)` の簡約形。
fn picked(
    names: &[&str],
    family: &SeriesFamily,
    scope: u32,
    explicit: Option<&str>,
) -> (Option<String>, Option<String>) {
    let got = select_marker_names(names, &marker_chain(family, scope, explicit));
    (got.up.map(|n| n.file_name), got.down.map(|n| n.file_name))
}

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
}

/// 要件 2.1: スコープ 0・1・2 の連鎖の順（旧名 `arrow` が最後）。
#[test]
fn arrow_chain_order_per_scope() {
    assert_eq!(
        prefixes(&ARROW_FAMILY, 0),
        ["arrowp0def", "arrows", "arrow"]
    );
    assert_eq!(
        prefixes(&ARROW_FAMILY, 1),
        ["arrowp1def", "arrowk", "arrowp0def", "arrows", "arrow"]
    );
    assert_eq!(
        prefixes(&ARROW_FAMILY, 2),
        ["arrowp2def", "arrowk", "arrowp0def", "arrows", "arrow"]
    );
}

/// 要件 2.7: クリック待ちの族も同じ順（旧名 `clickwait` が最後）。
#[test]
fn clickwait_chain_order_per_scope() {
    assert_eq!(
        prefixes(&CLICKWAIT_FAMILY, 0),
        ["clickwaitp0def", "clickwaits", "clickwait"]
    );
    assert_eq!(
        prefixes(&CLICKWAIT_FAMILY, 1),
        [
            "clickwaitp1def",
            "clickwaitk",
            "clickwaitp0def",
            "clickwaits",
            "clickwait"
        ]
    );
    assert_eq!(
        prefixes(&CLICKWAIT_FAMILY, 2),
        [
            "clickwaitp2def",
            "clickwaitk",
            "clickwaitp0def",
            "clickwaits",
            "clickwait"
        ]
    );
}

/// 要件 2.1: 番号ごとに別に辿る（`arrows0` と `arrow1` だけなら上＝`arrows0`・下＝`arrow1`）。
#[test]
fn each_number_walks_the_chain_separately() {
    assert_eq!(
        picked(&["arrow1.png", "arrows0.png"], &ARROW_FAMILY, 0, None),
        (s("arrows0.png"), s("arrow1.png"))
    );
    // 先の名前が両方を持てば、後の名前は使わない。
    assert_eq!(
        picked(
            &["arrow0.png", "arrow1.png", "arrows0.png", "arrows1.png"],
            &ARROW_FAMILY,
            0,
            None
        ),
        (s("arrows0.png"), s("arrows1.png"))
    );
}

/// 要件 2.1: スコープ 1・2 は自分の名前 → `arrowk` → 本体の名前の順で、番号ごとに落ちる。
#[test]
fn scope1_and_scope2_fall_back_per_number() {
    let names = ["arrowk0.png", "arrow1.png", "arrowp2def1.png"];
    assert_eq!(
        picked(&names, &ARROW_FAMILY, 1, None),
        (s("arrowk0.png"), s("arrow1.png"))
    );
    assert_eq!(
        picked(&names, &ARROW_FAMILY, 2, None),
        (s("arrowk0.png"), s("arrowp2def1.png"))
    );
    // スコープ 0 は arrowk も arrowp2def も見ない。
    assert_eq!(
        picked(&names, &ARROW_FAMILY, 0, None),
        (None, s("arrow1.png"))
    );
}

/// 要件 2.2/2.3: 書かれた名前は全スコープで先頭。無い番号は既定の連鎖へ落ちる。
#[test]
fn explicit_name_heads_the_chain_for_every_scope() {
    let names = [
        "myarrow0.png",
        "arrowp0def0.png",
        "arrowp1def1.png",
        "arrow1.png",
    ];
    for scope in [0, 1, 2] {
        let got = select_marker_names(&names, &marker_chain(&ARROW_FAMILY, scope, Some("myarrow")));
        let up = got.up.expect("上は書かれた名前");
        assert_eq!(
            (up.file_name.as_str(), up.prefix.as_str(), up.explicit),
            ("myarrow0.png", "myarrow", true),
            "scope {scope}"
        );
        let down = got.down.expect("下は既定の連鎖");
        assert!(!down.explicit, "scope {scope}");
        let expect = if scope == 1 {
            "arrowp1def1.png"
        } else {
            "arrow1.png"
        };
        assert_eq!(down.file_name, expect, "scope {scope}");
    }
    // 書かれていない（None・空）ときは連鎖の先頭が既定の名前。
    assert_eq!(marker_chain(&ARROW_FAMILY, 0, Some("")).explicit, None);
    assert_eq!(
        picked(&names, &ARROW_FAMILY, 0, None),
        (s("arrowp0def0.png"), s("arrow1.png"))
    );
}

/// 要件 2.6: 大小を区別せず照らし、`.png` だけを採る。実ファイル名は原形のまま。番号 0・1 以外は見ない。
#[test]
fn case_insensitive_png_only_numbers_0_and_1() {
    assert_eq!(
        picked(&["ARROWS0.PNG", "Arrow1.Png"], &ARROW_FAMILY, 0, None),
        (s("ARROWS0.PNG"), s("Arrow1.Png"))
    );
    assert_eq!(
        picked(
            &[
                "arrows0.bmp",
                "arrows1.txt",
                "arrows2.png",
                "arrowsX.png",
                "arrows.png"
            ],
            &ARROW_FAMILY,
            0,
            None
        ),
        (None, None)
    );
    // 書かれた名前も大小無視。
    assert_eq!(
        picked(&["MyArrow0.PNG"], &ARROW_FAMILY, 0, Some("myarrow")),
        (s("MyArrow0.PNG"), None)
    );
    // 大小違いの重複は辞書順最小（走査順に依らない）。
    let a = picked(&["arrow0.png", "ARROW0.png"], &ARROW_FAMILY, 0, None);
    let b = picked(&["ARROW0.png", "arrow0.png"], &ARROW_FAMILY, 0, None);
    assert_eq!(a, b);
    assert_eq!(a.0, s("ARROW0.png"));
}

/// 要件 2.4/2.5: 片方だけ・0 枚。
#[test]
fn one_side_only_and_none() {
    assert_eq!(
        picked(&["arrow0.png"], &ARROW_FAMILY, 0, None),
        (s("arrow0.png"), None)
    );
    assert_eq!(
        picked(&["arrows1.png"], &ARROW_FAMILY, 1, None),
        (None, s("arrows1.png"))
    );
    let empty: [&str; 0] = [];
    assert_eq!(
        select_marker_names(&empty, &marker_chain(&ARROW_FAMILY, 0, None)),
        MarkerNames::default()
    );
    assert_eq!(
        picked(&["balloons0.png", "marker0.png"], &ARROW_FAMILY, 0, None),
        (None, None)
    );
}

/// 要件 2.7: クリック待ちの族も矢印と同じ規則で解ける（番号ごと・書かれた名前が先頭・落ちる）。
#[test]
fn clickwait_resolves_like_arrow() {
    assert_eq!(
        picked(
            &["clickwait1.png", "clickwaits0.png"],
            &CLICKWAIT_FAMILY,
            0,
            None
        ),
        (s("clickwaits0.png"), s("clickwait1.png"))
    );
    let names = [
        "clickwaitk0.png",
        "clickwait1.png",
        "clickwaitp2def1.png",
        "cw0.png",
    ];
    assert_eq!(
        picked(&names, &CLICKWAIT_FAMILY, 1, None),
        (s("clickwaitk0.png"), s("clickwait1.png"))
    );
    assert_eq!(
        picked(&names, &CLICKWAIT_FAMILY, 2, None),
        (s("clickwaitk0.png"), s("clickwaitp2def1.png"))
    );
    assert_eq!(
        picked(&names, &CLICKWAIT_FAMILY, 2, Some("cw")),
        (s("cw0.png"), s("clickwaitp2def1.png"))
    );
    // 矢印の絵はクリック待ちに混ざらない（逆も）。
    assert_eq!(
        picked(&["arrow0.png"], &CLICKWAIT_FAMILY, 0, None),
        (None, None)
    );
    assert_eq!(
        picked(&["clickwait0.png"], &ARROW_FAMILY, 0, None),
        (None, None)
    );
}

/// script-small-tags 要件 5.2: 行の中の印の族の連鎖の順（スコープ 0・1・2・旧名 `marker` が最後）。
#[test]
fn inline_marker_chain_order_per_scope() {
    assert_eq!(
        prefixes(&MARKER_FAMILY, 0),
        ["markerp0def", "markers", "marker"]
    );
    assert_eq!(
        prefixes(&MARKER_FAMILY, 1),
        ["markerp1def", "markerk", "markerp0def", "markers", "marker"]
    );
    assert_eq!(
        prefixes(&MARKER_FAMILY, 2),
        ["markerp2def", "markerk", "markerp0def", "markers", "marker"]
    );
}

/// script-small-tags 要件 5.2: 番号の無い 1 枚を、書いた名前 → 連鎖の順で選ぶ（番号つきは選ばない）。
#[test]
fn single_marker_name_prefers_explicit_and_ignores_numbered() {
    let pick = |names: &[&str], scope: u32, explicit: Option<&str>| {
        select_single_marker_name(names, &marker_chain(&MARKER_FAMILY, scope, explicit))
    };
    let names = ["marker.png", "markers.png", "markerk.png", "my.png"];
    assert_eq!(pick(&names, 0, Some("my")), s("my.png"));
    assert_eq!(pick(&names, 0, None), s("markers.png"));
    assert_eq!(pick(&names, 1, None), s("markerk.png"));
    assert_eq!(pick(&names, 2, None), s("markerk.png"));
    assert_eq!(
        pick(&["marker.png", "markerp2def.png"], 2, None),
        s("markerp2def.png")
    );
    // 書いた名前が無ければ連鎖の残りで探す。
    assert_eq!(pick(&["marker.png"], 1, Some("my")), s("marker.png"));
    // 番号つき・別の拡張子は選ばない。
    assert_eq!(
        pick(&["marker0.png", "markers1.png", "marker.bmp"], 0, None),
        None
    );
    assert_eq!(pick(&["my0.png"], 0, Some("my")), None);
    // 大小は無視し、大小違いが並ぶときは辞書順最小。
    assert_eq!(
        pick(&["Marker.PNG", "MARKER.png"], 0, None),
        s("MARKER.png")
    );
}
