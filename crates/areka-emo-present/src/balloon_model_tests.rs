use super::*;

use super::test_support::{CapturedEvent, TempDir, capture_events, emo2_balloon_root};

/// 実 fixture の当該 scope の面 0 を解決するテストヘルパ（本体は権威経路そのもの）。
fn emo2_face0(scope: u32) -> ResolvedFace {
    let faces = resolve_balloon_faces(&emo2_balloon_root(), scope)
        .expect("emo2-kakukaku は面 0 を持つ");
    faces
        .into_iter()
        .find(|f| f.surface_id == 0)
        .expect("面 0 必在契約")
}

/// 檻 8-a（R2.1/R2.2/R2.5）per-scope マージ実値: 実 fixture `emo2-kakukaku` で、
/// scope 0（本体側 `balloons0s.txt`）と scope 1（相方側 `balloonk0s.txt`）が**互いに異なる実値**
/// へマージされる。
///
/// - scope 0: `validrect 46,-56,36,-44` / `windowposition 266,-129`（`balloons0s.txt` 実測）
/// - scope 1: `validrect 40,-70,24,-48` / `windowposition -190,-75`（`balloonk0s.txt` 実測）
///
/// 採用面の上書き層のみが効くこと（R2.2）——scope 1 が `balloonk0` を採る以上、
/// `balloons0s.txt` の値（266/-129・46/-56/36/-44）はどれも scope 1 の定義に現れない。
#[test]
fn load_scope_balloon_model_merges_per_scope_on_emo2_fixture() {
    let dir = emo2_balloon_root();

    let face0_sakura = emo2_face0(0);
    assert_eq!(
        face0_sakura.override_file_name(),
        "balloons0s.txt",
        "前提: scope 0 の面 0 は本体側系列を採る"
    );
    let sakura = load_scope_balloon_model(&dir, 0, &face0_sakura);

    let face0_kero = emo2_face0(1);
    assert_eq!(
        face0_kero.override_file_name(),
        "balloonk0s.txt",
        "前提: scope 1 の面 0 は相方側系列を採る（R2.2）"
    );
    let kero = load_scope_balloon_model(&dir, 1, &face0_kero);

    // --- scope 0（本体側）の実値 ---
    let vr = sakura.validrect();
    assert_eq!(
        (vr.top(), vr.bottom(), vr.left(), vr.right()),
        (Some(46), Some(-56), Some(36), Some(-44)),
        "scope 0 の validrect は balloons0s.txt が descript を後勝ち上書きした値"
    );
    let wp = sakura.windowposition();
    assert_eq!(
        (wp.x(), wp.y()),
        (Some(266), Some(-129)),
        "scope 0 の windowposition は balloons0s.txt のみが供給する（descript に不在）"
    );

    // --- scope 1（相方側）の実値 ---
    let vr = kero.validrect();
    assert_eq!(
        (vr.top(), vr.bottom(), vr.left(), vr.right()),
        (Some(40), Some(-70), Some(24), Some(-48)),
        "scope 1 の validrect は balloonk0s.txt の値（balloons0s.txt を引かない・R2.2）"
    );
    let wp = kero.windowposition();
    assert_eq!(
        (wp.x(), wp.y()),
        (Some(-190), Some(-75)),
        "scope 1 の windowposition は balloonk0s.txt の値（R2.2）"
    );

    // --- 2 scope が実際に別物であること（全 scope 共通 1 本への畳み込みの検出） ---
    assert_ne!(
        sakura.windowposition(),
        kero.windowposition(),
        "本体側と相方側の windowposition が同値＝scope 別化が効いていない"
    );
    assert_ne!(
        sakura.validrect(),
        kero.validrect(),
        "本体側と相方側の validrect が同値＝scope 別化が効いていない"
    );
}

/// 檻 8-b（R2.5）継承: 面別上書き層で**指定されなかった**項目は既定設定（`descript.txt`）から
/// 継承され、指定された項目のみが上書きされる。
///
/// 実 fixture の `wordwrappoint.x` がこの 2 面性をそのまま体現する——`balloonk0s.txt` は
/// `wordwrappoint` を持たないゆえ scope 1 は descript の `-34` を継承し、`balloons0s.txt` は
/// `wordwrappoint.x,-49` を持つゆえ scope 0 はそちらで上書きされる。加えて双方の scope が
/// 上書き層のどちらも触れない項目（`font.name` / `origin` / `font.height`）を descript から
/// 等しく継承する。
#[test]
fn load_scope_balloon_model_inherits_unspecified_keys_from_descript() {
    let dir = emo2_balloon_root();
    let sakura = load_scope_balloon_model(&dir, 0, &emo2_face0(0));
    let kero = load_scope_balloon_model(&dir, 1, &emo2_face0(1));

    assert_eq!(
        kero.wordwrappoint().x(),
        Some(-34),
        "balloonk0s.txt は wordwrappoint を指定しない＝descript の -34 を継承する（R2.5）"
    );
    assert_eq!(
        sakura.wordwrappoint().x(),
        Some(-49),
        "balloons0s.txt は wordwrappoint.x,-49 を指定する＝そちらが後勝ちする（R2.5 の対照）"
    );
    // 上書き層のどちらも触れない項目は両 scope とも descript から継承される。
    for (scope, model) in [(0u32, &sakura), (1u32, &kero)] {
        assert_eq!(
            model.wordwrappoint().y(),
            Some(0),
            "scope {scope}: wordwrappoint.y は descript 継承"
        );
        assert_eq!(
            model.origin().x(),
            Some(0),
            "scope {scope}: origin.x は descript 継承"
        );
        assert_eq!(
            model.font().name(),
            Some("Yu Gothic UI"),
            "scope {scope}: font.name は descript 継承（charset,UTF-8 宣言どおりデコードされる）"
        );
        assert_eq!(
            model.font().height(),
            Some(28),
            "scope {scope}: font.height は descript 継承"
        );
    }
}

/// 檻 8-c（R2.4・D8）上書きファイル不在: 面別上書きファイルが存在しないとき、
/// 既定設定の値のみで正常な定義を返し、**失敗として扱わない**。ログレベルは
/// `debug!`（正常縮退）であって `warn!`／`error!` ではない（D8——相方側で毎起動 warn が
/// 鳴る事故を防ぐ）。
#[test]
fn load_scope_balloon_model_debug_logs_missing_override_and_continues() {
    let dir = TempDir::new();
    dir.write(
        "descript.txt",
        "validrect.top,7\nvalidrect.bottom,-8\nwordwrappoint.x,-34\n",
    );
    dir.touch("balloons0.png"); // 面 0 は在るが balloons0s.txt は置かない。

    let face0 = {
        let faces = resolve_balloon_faces(dir.path(), 0).expect("面 0 は在る");
        faces.into_iter().find(|f| f.surface_id == 0).unwrap()
    };
    let (model, events) = capture_events(|| load_scope_balloon_model(dir.path(), 0, &face0));

    // R2.4: 既定設定のみで正常な定義が返る（欠落は失敗でない）。
    assert_eq!(
        model.validrect().top(),
        Some(7),
        "上書き層不在でも既定設定の値がそのまま定義になる（R2.4）"
    );
    assert_eq!(model.validrect().bottom(), Some(-8));
    assert_eq!(
        model.windowposition().x(),
        None,
        "どちらの層にも無いキーは None（捏造しない）"
    );

    // D8: 不在は debug!。warn!／error! を出さない。
    let missing: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| {
            e.fields
                .get("message")
                .is_some_and(|m| m.contains("面別上書き"))
        })
        .collect();
    assert_eq!(
        missing.len(),
        1,
        "面別上書き層の不在は 1 イベントとして記録される: {events:?}"
    );
    assert_eq!(
        missing[0].level,
        tracing::Level::DEBUG,
        "不在は正常縮退＝debug!（D8）: {:?}",
        missing[0]
    );
    assert_eq!(
        missing[0].field("file"),
        Some("balloons0s.txt"),
        "D8: 不在だった上書きファイル名が乗る"
    );
    assert!(
        events
            .iter()
            .all(|e| e.level != tracing::Level::WARN && e.level != tracing::Level::ERROR),
        "上書き層不在は warn!／error! を一切出さない（D8）: {events:?}"
    );
}

/// 檻 8-c'（R2.4/R6.4・D8）上書き層のその他 I/O エラー: **不在ではない**読取失敗
/// （権限・I/O 障害等）は `debug!` へ降格せず `warn!` で観測可能に残し、既定設定のみで継続する。
///
/// 「不在か否か」の判定分岐そのものを押さえる檻である——分岐を落として全失敗を `debug!` に
/// すればこの檻が、全失敗を `warn!` にすれば檻 8-c が RED になる。異常を決定論的に作るため、
/// 上書きファイル名と同名の**ディレクトリ**を置く（`std::fs::read` は NotFound 以外の
/// エラーを返す）。
#[test]
fn load_scope_balloon_model_warns_on_non_notfound_override_error() {
    let dir = TempDir::new();
    dir.write("descript.txt", "validrect.top,7\n");
    dir.touch("balloons0.png");
    // 上書きファイルの位置にディレクトリを置く＝不在ではない読取失敗。
    std::fs::create_dir(dir.path().join("balloons0s.txt")).expect("同名ディレクトリ作成");

    let face0 = {
        let faces = resolve_balloon_faces(dir.path(), 0).expect("面 0 は在る");
        faces.into_iter().find(|f| f.surface_id == 0).unwrap()
    };
    let (model, events) = capture_events(|| load_scope_balloon_model(dir.path(), 0, &face0));

    // 既定設定のみで継続する（失敗として畳まない）。
    assert_eq!(model.validrect().top(), Some(7), "既定設定のみで継続する");

    let hits: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| {
            e.fields
                .get("message")
                .is_some_and(|m| m.contains("面別上書き"))
        })
        .collect();
    assert_eq!(hits.len(), 1, "面別上書き層の事象は 1 イベント: {events:?}");
    assert_eq!(
        hits[0].level,
        tracing::Level::WARN,
        "不在以外の入出力エラーは warn!（debug! へ降格しない・D8）: {:?}",
        hits[0]
    );
    assert_eq!(
        hits[0].field("scope"),
        Some("0"),
        "どの scope の上書き層が読めなかったかが乗る"
    );
}

/// 檻 8-d（R6.4・D8）基層読取失敗: 既定設定 `descript.txt` の読取失敗は現行どおり
/// `warn!`＋空層継続（`debug!` へ降格しない）。両層とも欠ければ全スカラ `None` の
/// 定義を返す（panic しない・parsers の寛容契約に整合）。
#[test]
fn load_scope_balloon_model_warns_on_missing_descript() {
    let dir = TempDir::new();
    dir.touch("balloons0.png"); // descript.txt も balloons0s.txt も置かない。

    let face0 = {
        let faces = resolve_balloon_faces(dir.path(), 0).expect("面 0 は在る");
        faces.into_iter().find(|f| f.surface_id == 0).unwrap()
    };
    let (model, events) = capture_events(|| load_scope_balloon_model(dir.path(), 0, &face0));

    assert_eq!(
        model.validrect().top(),
        None,
        "両層とも欠ければ None（空層継続）"
    );
    assert_eq!(model.windowposition().x(), None);

    let descript_warns: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| {
            e.fields
                .get("message")
                .is_some_and(|m| m.contains("バルーン既定設定"))
        })
        .collect();
    assert_eq!(
        descript_warns.len(),
        1,
        "基層読取失敗は 1 イベント: {events:?}"
    );
    assert_eq!(
        descript_warns[0].level,
        tracing::Level::WARN,
        "基層 descript.txt の読取失敗は現行どおり warn!（D8）: {:?}",
        descript_warns[0]
    );
}

/// 檻 8-e（R6.3・観測点 3）確定値の記録: 2 層マージで確定した `windowposition` /
/// `validrect` の**実値**が scope とともに `info!` で記録される。
///
/// scope は `ResolvedFace` に無い（採用接頭辞は縮退で `balloons` にもなり得るため接頭辞から
/// 逆算できない）ゆえ、この檻は引数として渡した scope がそのままログへ乗ることを固定する。
#[test]
fn load_scope_balloon_model_info_logs_scope_and_resolved_values() {
    let dir = emo2_balloon_root();
    let face0 = emo2_face0(1);
    let (model, events) = capture_events(|| load_scope_balloon_model(&dir, 1, &face0));

    let infos: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::INFO
                && e.fields
                    .get("message")
                    .is_some_and(|m| m.contains("scope 別バルーン定義"))
        })
        .collect();
    assert_eq!(infos.len(), 1, "確定値の info! は 1 行: {events:?}");
    let info = infos[0];

    assert_eq!(info.field("scope"), Some("1"), "R6.3: scope が乗る");
    // 実値はモデルの確定値そのもの（ログと戻り値が乖離しないことを併せて固定する）。
    let wp = model.windowposition();
    let vr = model.validrect();
    assert_eq!((wp.x(), wp.y()), (Some(-190), Some(-75)));
    assert_eq!(
        info.field("windowposition_x"),
        Some("Some(-190)"),
        "R6.3: windowposition の実値が乗る"
    );
    assert_eq!(info.field("windowposition_y"), Some("Some(-75)"));
    assert_eq!(
        (vr.top(), vr.bottom(), vr.left(), vr.right()),
        (Some(40), Some(-70), Some(24), Some(-48))
    );
    assert_eq!(
        info.field("validrect_top"),
        Some("Some(40)"),
        "R6.3: validrect の実値が乗る"
    );
    assert_eq!(info.field("validrect_bottom"), Some("Some(-70)"));
    assert_eq!(info.field("validrect_left"), Some("Some(24)"));
    assert_eq!(info.field("validrect_right"), Some("Some(-48)"));
    assert_eq!(
        info.field("file"),
        Some("balloonk0s.txt"),
        "どの上書き層で確定したかが乗る（scope 別化の突合点）"
    );
}
