use std::path::PathBuf;

use areka_emo_text::actor::ResolvedBalloonText;
use areka_parsers::balloon::BalloonModel;
use areka_parsers::charset::decode;
use areka_seriko::{BindChoicePolicy, BindNamespace, SurfaceTarget};
use log_capture_kit::{LineFormat, capture_lines};
use temp_path_kit::TempPath;

// bake の WIC デコードに要る COM は、テストのプロセスの寿命の MTA（`crash_report_test_support`）が持つ。
use super::*;
use crate::sample_test_support::{emo2_balloon_root, emo2_root};
// アクタ鍵 → scope の逆写像正本（`ActorKey` 語彙が既存写像と同一であることの往復検査に使う）。
use crate::target_map::scope_of;

/// emo2 fixture を渡した統合テストが
/// `BootAssets` の各フィールドに期待どおりのデータを含んで green で通る。
///
/// 既知 scope 集合 `[0, 1]` に対し `build_boot_assets` が populated な `ScopeAssets`
/// （scope0=surface0／scope1=surface10・areka-P0-emo2-boot 設計 DD-9）・scope ごとの balloon 資産・2 層マージ済み
/// `BalloonModel`・emo2 alias 由来の `resolver`・DD-8 の `static_binds`
/// `[1100,1207,1302,1500,1800]` を返すことを固定する。戻り値だけで以後ファイル I/O が
/// 不要になる（＝全 I/O が本関数内で完結する）ことを、各フィールドが実データで
/// populated であることの積極 assert で担保する。
///
/// `bake` は WIC で PNG をデコードするため COM 初期化が要る（GPU は不要・attach なし）。
#[test]
fn build_boot_assets_from_emo2_fixture() {
    // 作者基準 DPI は emo2 fixture の実測既定（shell/balloon とも無宣言＝96）。
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    // --- shells: 要求 scope に 1:1 対応・初期 surface id は areka-P0-emo2-boot 設計 DD-9（scope0=0／scope>=1=10） ---
    assert_eq!(boot.shells.len(), 2, "要求 scope 集合 [0,1] に 1:1 対応");
    assert_eq!(boot.shells[0].scope, 0);
    assert_eq!(
        boot.shells[0].initial_surface_id, 0,
        "scope0 → 初期 surface 0"
    );
    assert_eq!(boot.shells[1].scope, 1);
    assert_eq!(
        boot.shells[1].initial_surface_id, 10,
        "scope>=1 → 初期 surface 10"
    );

    // 各 scope の EmoWorld は FRESH に build 済みで、初期表示 surface を実際に内包する
    // （scope0=surface0／scope1=surface10）。装着へ手渡す前段の populated 担保。
    assert!(
        boot.shells[0].emo_world.surface(0).is_some(),
        "scope0 の World は初期 surface 0 を内包する"
    );
    assert!(
        boot.shells[1].emo_world.surface(10).is_some(),
        "scope1 の World は初期 surface 10 を内包する"
    );
    // 共有アトラスは 1 回の bake 由来（Clone 共有）＝非空・全 scope 同一エントリ数。
    assert!(
        !boot.shells[0].atlas.is_empty(),
        "shell アトラスは bake 済み（非空）"
    );
    assert_eq!(
        boot.shells[0].atlas.len(),
        boot.shells[1].atlas.len(),
        "AtlasTable は parse/bake 1 回の Clone 共有（scope 間で同一）"
    );

    // --- balloons: scope ごとに populated（面 0 初期表示の資産） ---
    assert_eq!(boot.balloons.len(), 2, "balloon 資産も scope ごとに 1:1");
    assert_eq!(boot.balloons[0].scope, 0);
    assert_eq!(boot.balloons[1].scope, 1);
    // balloon target World は面 0（初期表示・areka-P0-emo2-boot 設計 DD-9）を内包する（構築が実枠を組んだ担保）。
    assert!(
        boot.balloons[0].emo_world.surface(0).is_some(),
        "balloon target World は初期面 0 を内包する"
    );
    assert!(
        boot.balloons[1].emo_world.surface(0).is_some(),
        "相方側 scope の World も自系列（balloonk0）の面 0 を内包する"
    );
    assert!(
        !boot.balloons[0].atlas.is_empty(),
        "balloon アトラスは bake 済み（非空）"
    );

    // --- balloons[*].model: descript 基層 + **採用面に対応する**面別層の 2 層後勝ちマージ ---
    // scope0 は balloons0.png を採用 → balloons0s.txt が上書き層。
    // descript.txt は validrect 全 0（degenerate）・balloons0s.txt が 46/-56/36/-44 で後勝ち上書き。
    let vr0 = boot.balloons[0].definitions.face0().model.validrect();
    assert_eq!(
        vr0.top(),
        Some(46),
        "面別層 balloons0s.txt が descript を後勝ち上書き"
    );
    assert_eq!(vr0.bottom(), Some(-56));
    assert_eq!(vr0.left(), Some(36));
    assert_eq!(vr0.right(), Some(-44));
    // windowposition は面別層のみが供給するキー（descript.txt に不在）。
    let wp0 = boot.balloons[0].definitions.face0().model.windowposition();
    assert_eq!(wp0.x(), Some(266), "windowposition は面別層のみ供給");
    assert_eq!(wp0.y(), Some(-129));

    // scope1 は balloonk0.png を採用 → balloonk0s.txt が上書き層（balloons0s.txt ではない・areka-P0-kero-balloon 要件 2.2）。
    let vr1 = boot.balloons[1].definitions.face0().model.validrect();
    assert_eq!(
        vr1.top(),
        Some(40),
        "相方側は balloonk0s.txt が上書き層（本体側 46 ではない）"
    );
    assert_eq!(vr1.bottom(), Some(-70));
    assert_eq!(vr1.left(), Some(24));
    assert_eq!(vr1.right(), Some(-48));
    let wp1 = boot.balloons[1].definitions.face0().model.windowposition();
    assert_eq!(
        wp1.x(),
        Some(-190),
        "相方側 windowposition は balloonk0s.txt 実値"
    );
    assert_eq!(wp1.y(), Some(-75));

    // --- resolver: EmoWorld::alias_snapshot() 由来（emo2 実測 alias で決定論解決） ---
    assert_eq!(
        boot.resolver.resolve("通常"),
        SurfaceTarget::Show(2100),
        "単一候補 alias（emo2 実測）"
    );
    assert_eq!(
        boot.resolver.resolve("静観"),
        SurfaceTarget::Show(2106),
        "複数候補は先頭固定"
    );
    assert_eq!(
        boot.resolver.resolve("-1"),
        SurfaceTarget::Hide,
        "非表示センチネルは alias 非依存"
    );

    // --- static_binds: areka-P0-emo2-boot 設計 DD-8 の emo2 default==1 集合（昇順） ---
    assert_eq!(
        boot.static_binds
            .for_scope(&areka_sakura::ActorKey::from("0"))
            .ids(),
        &[1100, 1207, 1302, 1500, 1800],
        "shell descript の sakura.bindgroup{{N}}.default==1"
    );

    // --- loop_tables: 既に保持する EmoWorld スナップショットから from_world で構築（面種非依存・裁定 (a)） ---
    // 新規ファイル I/O なし＝保持済み World の再利用のみ。boot 内で組んだ表が、同じ retained World
    // から再構築した表と一致する（is_empty・当該面 surface のアニメ本数）ことを固定し、reuse を担保する。
    let shell_rebuilt = AnimationTable::from_world(&boot.shells[0].emo_world);
    assert_eq!(
        boot.loop_tables.shell.is_empty(),
        shell_rebuilt.is_empty(),
        "shell 表は shells[0].emo_world から from_world 済み（保持 World の再利用）"
    );
    assert_eq!(
        boot.loop_tables.shell.animations(0).len(),
        shell_rebuilt.animations(0).len(),
        "shell 表は保持 World と同一エントリ（新規 I/O なし・面種非依存）"
    );
    // balloon 表は **scope 別**（キー＝scope 番号・areka-P0-kero-balloon 要件 5.6）。各 scope の表が、その scope が
    // 保持する World から再構築した表と一致する（＝別 scope の World 由来ではない・新規 I/O なし）。
    for balloon_assets in &boot.balloons {
        let rebuilt = AnimationTable::from_world(&balloon_assets.emo_world);
        let table = boot
            .loop_tables
            .balloon
            .get(&balloon_assets.scope)
            .expect("各 scope の balloon 表が存在する（不変条件 (c)）");
        assert_eq!(
            table.is_empty(),
            rebuilt.is_empty(),
            "scope {} の balloon 表は当該 scope の保持 World から from_world 済み",
            balloon_assets.scope
        );
        assert_eq!(
            table.animations(0).len(),
            rebuilt.animations(0).len(),
            "scope {} の balloon 表は当該 scope の保持 World と同一エントリ（新規 I/O なし・面種非依存）",
            balloon_assets.scope
        );
    }
}

/// （areka-P0-kero-balloon 要件 5.6）バルーンのループ表が **scope 別の写像**
/// として導出され、そのキー集合が scope 集合と一致する。
///
/// 「先頭 scope の資産から 1 本組めば全 scope に足りる」という旧前提の実装（＝写像が単一
/// エントリ `{0}` に縮む形）はこの檻で落ちる。またキー集合が `BootAssets.balloons` の scope
/// 集合と一致することを固定する。
///
/// 内容についての注記（データ事実・観測等価）: emo2 のバルーン面は `areka-emo-present` の
/// 合成 `surfaces.txt` から build され、そこに `animation*` 行が一切無いため **全 scope とも
/// 空表**になる。したがって areka-P0-kero-balloon が変えた観測は表の中身ではなく **どの World から何本の表を
/// 導出するか**であり、この檻はキー集合（と導出元 World の scope 一致）を固定する。表内容の
/// 一致は `build_boot_assets_from_emo2_fixture` が scope ごとの再構築突合で担保する。
#[test]
fn build_boot_assets_derives_balloon_loop_tables_per_scope() {
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    let table_scopes: Vec<u32> = boot.loop_tables.balloon.keys().copied().collect();
    assert_eq!(
        table_scopes,
        vec![0, 1],
        "バルーン表は scope ごとに 1 本ずつ導出される（先頭 scope の 1 本で済ませない）"
    );
    let asset_scopes: Vec<u32> = boot.balloons.iter().map(|b| b.scope).collect();
    assert_eq!(
        table_scopes, asset_scopes,
        "バルーン表のキー集合は balloons の scope 集合と一致する（不変条件 (c)）"
    );

    // データ事実（合成 surfaces.txt に animation 行が無い）ゆえ emo2 は全 scope 空表。
    // 中身が観測等価でも、駆動される表が scope ごとに別実体であることが areka-P0-kero-balloon の是正点。
    for (scope, table) in &boot.loop_tables.balloon {
        assert!(
            table.is_empty(),
            "emo2 の合成 surfaces.txt は animation 行を持たないため scope {scope} も空表（データ事実）"
        );
    }
}

/// （areka-P0-kero-balloon 要件 5.6）起動シームの転送が **全 scope** を
/// アクタ鍵語彙へ写して seriko へ渡す（単一エントリ暫定でない）。
///
/// `wire_runtime`（lib.rs）と spine ハーネス（本体の `crates/areka/src/runtime_tests/spine.rs`）が共有する転送
/// [`actor_keyed_balloon_tables`] を実 fixture 由来の写像へ適用し、`SerikoLoopConfig.balloon_tables`
/// のキー集合が `ActorKey::from(scope.to_string())` で `{"0","1"}` になることを固定する。
/// 先頭 scope だけを載せる旧形（`{"0"}`）はこの檻で落ちる。
#[test]
fn boot_seam_transfers_every_balloon_scope_as_actor_key() {
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");
    let asset_scopes: Vec<u32> = boot.balloons.iter().map(|b| b.scope).collect();

    // 起動シーム（lib.rs／本体の `crates/areka/src/runtime_tests/spine.rs`）が行う転送そのもの。
    let transferred = actor_keyed_balloon_tables(boot.loop_tables.balloon);

    let keys: Vec<ActorKey> = transferred.keys().cloned().collect();
    assert_eq!(
        keys,
        vec![ActorKey::from("0"), ActorKey::from("1")],
        "全 scope がアクタ鍵語彙で seriko へ渡る（単一エントリ暫定でない）"
    );
    // 逆写像（`target_map::scope_of`）と往復し、boot 側 scope 集合へ戻ることを固定する
    // （＝attach／再追従と同一語彙であり、ここで別語彙を作っていない）。
    let round_tripped: Vec<u32> = transferred.keys().filter_map(scope_of).collect();
    assert_eq!(
        round_tripped, asset_scopes,
        "アクタ鍵は scope_of の逆写像で boot 側 scope 集合へ戻る（既存写像語彙と同一）"
    );
}

/// 転送 [`actor_keyed_balloon_tables`] の純粋部分: 全エントリを保ち、キーだけを
/// `ActorKey::from(scope.to_string())` へ写す（COM/fixture 不要の決定論檻）。
///
/// 多桁 scope（10）を混ぜ、`to_string()` 語彙（`"10"`）がそのままキーになること、および
/// エントリが取りこぼされたり併合されたりしないことを固定する。
#[test]
fn actor_keyed_balloon_tables_maps_every_scope() {
    let tables = BTreeMap::from([
        (0u32, AnimationTable::empty()),
        (1u32, AnimationTable::empty()),
        (10u32, AnimationTable::empty()),
    ]);

    let keyed = actor_keyed_balloon_tables(tables);

    assert_eq!(
        keyed.len(),
        3,
        "全エントリが転送される（取りこぼし・併合なし）"
    );
    for scope in [0u32, 1, 10] {
        assert!(
            keyed.contains_key(&ActorKey::from(scope.to_string())),
            "scope {scope} は `ActorKey::from(scope.to_string())` のキーで引ける"
        );
    }
    // 逆写像で元の scope 集合へ戻る（別語彙 `"scope0"` 等を作っていない）。
    let mut round_tripped: Vec<u32> = keyed.keys().filter_map(scope_of).collect();
    round_tripped.sort_unstable();
    assert_eq!(
        round_tripped,
        vec![0, 1, 10],
        "scope_of の逆写像で元の集合へ戻る"
    );
}

/// 空の scope 集合では balloon 表の写像も空になる（縮退・不変条件 (c) の degenerate 端）。
///
/// seriko 側は不在 scope を**空表意味論**（抽選対象ゼロ・乱数非消費・panic なし）で扱うため、
/// 空写像はループ完全不活性と等価である。
#[test]
fn build_boot_assets_yields_empty_balloon_table_map_for_no_scopes() {
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[], 96, 96)
        .expect("scope 集合が空でも組立は成功する（縮退許容）");

    assert!(boot.balloons.is_empty(), "scope 資産が無い");
    assert!(
        boot.loop_tables.balloon.is_empty(),
        "balloon 表の写像も空（キー集合⊆balloons の scope 集合・不変条件 (c)）"
    );
    assert!(
        boot.loop_tables.shell.is_empty(),
        "シェル表も空表（scope 資産不在の既定）"
    );
}

/// （areka-P0-kero-balloon 要件 2.1／7.2）起動時資産が **scope ごとに
/// 異なる**バルーン定義を保持する（共有 1 本ではない）。
///
/// emo2-kakukaku fixture では scope0 が `balloons0.png`（→ `balloons0s.txt`）を、scope1 が
/// `balloonk0.png`（→ `balloonk0s.txt`）を採用する。両 scope の `windowposition` /
/// `validrect` が**互いに異なる**こと、および `wordwrappoint.x` が
/// **scope0 は面別層で -49 に上書き・scope1 は面別層に宣言が無いため descript 基層の -34 を継承**
/// することを固定する（areka-P0-kero-balloon 要件 2.1／2.2／2.5）。共有 1 本のモデルを全 scope へ配る実装はこの檻で落ちる。
#[test]
fn build_boot_assets_holds_per_scope_balloon_models() {
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    // 不変条件 (a): balloons の scope 集合＝shells の scope 集合。
    let balloon_scopes: Vec<u32> = boot.balloons.iter().map(|b| b.scope).collect();
    let shell_scopes: Vec<u32> = boot.shells.iter().map(|s| s.scope).collect();
    assert_eq!(
        balloon_scopes, shell_scopes,
        "balloons の scope 集合は shells の scope 集合と一致する（不変条件 (a)）"
    );

    let scope0 = &boot.balloons[0].definitions.face0().model;
    let scope1 = &boot.balloons[1].definitions.face0().model;

    // windowposition: 本体側 balloons0s.txt vs 相方側 balloonk0s.txt の実値。
    assert_eq!(scope0.windowposition().x(), Some(266));
    assert_eq!(scope0.windowposition().y(), Some(-129));
    assert_eq!(scope1.windowposition().x(), Some(-190));
    assert_eq!(scope1.windowposition().y(), Some(-75));
    assert_ne!(
        scope0.windowposition().x(),
        scope1.windowposition().x(),
        "scope ごとに異なる定義を保持する（共有 1 本ではない）"
    );

    // validrect: 同上（相方側は 40/-70/24/-48）。
    assert_eq!(
        (
            scope0.validrect().top(),
            scope0.validrect().bottom(),
            scope0.validrect().left(),
            scope0.validrect().right()
        ),
        (Some(46), Some(-56), Some(36), Some(-44))
    );
    assert_eq!(
        (
            scope1.validrect().top(),
            scope1.validrect().bottom(),
            scope1.validrect().left(),
            scope1.validrect().right()
        ),
        (Some(40), Some(-70), Some(24), Some(-48))
    );

    // wordwrappoint.x: scope0 は面別層が -49 で後勝ち上書き・scope1 は面別層に宣言が無く
    // descript 基層の -34 を継承する（上書き／継承の対照＝areka-P0-kero-balloon 要件 2.5）。
    assert_eq!(
        scope0.wordwrappoint().x(),
        Some(-49),
        "scope0 は balloons0s.txt の wordwrappoint.x が後勝ち上書き"
    );
    assert_eq!(
        scope1.wordwrappoint().x(),
        Some(-34),
        "scope1 は balloonk0s.txt に宣言が無く descript 基層 -34 を継承"
    );
}

/// areka-P0-text-decoration-canon 要件 4.6: 起動時資産が **面 0 の焼き込み済み画像の原点画素**からバルーンの背景色を導く。
///
/// 兄弟テスト（`balloon_background_tests.rs`）は導出規則そのものを合成画像で固定するが、
/// `build_boot_assets` の中で規則が**実際に呼ばれている**ことはそこでは映らない。ここが実
/// fixture（emo2）を通した唯一の配線点の檻である。
///
/// emo2 の 2 面はどちらも原点画素が不透明でない（実測 α ≠ 255）——結果は白（既定）だが、
/// 「導出を呼ばずに白を置く」実装とはログで区別できる。**色だけを見ると恒真**なので、白へ落ちた
/// 理由の記録を scope ごとに 1 件ずつ数えることで配線の実在を判定する（導出の呼出を消すと 0 件で
/// 赤になる）。捕捉が空振りしていないことは、同じ窓に載る他のログ（`build_boot_assets` は確定値を
/// `info!` で出す）が 0 でないことで確かめる。
#[test]
fn build_boot_assets_derives_balloon_background_from_face_zero_origin_pixel() {
    let (boot, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
            .expect("emo2 fixture の BootAssets 組立は成功する")
    });

    let colors: Vec<(u32, (u8, u8, u8))> = boot
        .balloons
        .iter()
        .map(|b| (b.scope, b.background_color))
        .collect();
    assert_eq!(
        colors,
        vec![(0, EMO2_BACKGROUND_SCOPE0), (1, EMO2_BACKGROUND_SCOPE1)],
        "各 scope の背景色は当該 scope の面 0 の原点画素から導く（2026-09-12 実測）"
    );

    assert!(
        !lines.is_empty(),
        "捕捉窓が空振りしている（build_boot_assets は確定値を info! で出す）"
    );
    let derivations: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains("target=areka_runtime::balloon_background "))
        .collect();
    assert_eq!(
        derivations.len(),
        2,
        "背景色の導出は scope ごとに 1 度ずつ通る（呼出を消すと 0 件）: {derivations:?}"
    );
    let seen: Vec<(bool, bool)> = derivations
        .iter()
        .map(|l| (l.contains("reason=\"not_opaque\""), l.contains("file=")))
        .collect();
    assert_eq!(
        seen,
        vec![(true, true), (true, true)],
        "emo2 の 2 面はどちらも原点画素が不透明でない（実測 α ≠ 255）・記録は面のファイル名を載せる: {derivations:?}"
    );
    assert!(
        derivations[0].contains("file=\"balloons0.png\"")
            && derivations[1].contains("file=\"balloonk0.png\""),
        "記録は scope ごとに当該 scope が解決した面 0 を名指しする（scope0=本体側／scope1=相方側）: {derivations:?}"
    );
}

/// emo2 本体側バルーン（面 0）の背景色（2026-09-12 実測・原点画素が不透明でないゆえ白の既定）。
const EMO2_BACKGROUND_SCOPE0: (u8, u8, u8) = (255, 255, 255);
/// emo2 相方側バルーン（面 0）の背景色（2026-09-12 実測・同上）。
const EMO2_BACKGROUND_SCOPE1: (u8, u8, u8) = (255, 255, 255);

/// areka-P0-kero-balloon の適用**前**の 2 層マージを再現する神託（同 spec の要件 5.5）。
///
/// merge-base（969a9b3）の `build_balloon_model` は **固定名** `descript.txt`（基層）＋
/// `balloons0s.txt`（面別上書き層）を `areka_parsers::balloon::parse_str` へ渡すだけの
/// 関数であり、その結果を単一の [`BalloonModel`] として全 scope で共有していた
/// （`BALLOON_FACE0_TXT` 定数）。読取は `decode(bytes, DefaultEncoding::Ansi)`・
/// 読取失敗は空層で継続——現行 `load_scope_balloon_model` の層構成と同じ規約である。
///
/// 適用前のバイナリは手元に残らないが、規則は数行で再現できる。手書き期待値ではなく
/// **適用前の規則そのもの**と突き合わせることで、非回帰の主張を規則同士の等式として固定する。
fn pre_spec_balloon_model(balloon_root: &Path) -> BalloonModel {
    let read = |name: &str| {
        std::fs::read(balloon_root.join(name))
            .ok()
            .map(|bytes| decode(&bytes, DefaultEncoding::Ansi))
    };
    let descript = read("descript.txt").unwrap_or_default();
    let face0 = read("balloons0s.txt");
    areka_parsers::balloon::parse_str(&descript, face0.as_deref())
}

/// 本体側 scope の非回帰（areka-P0-kero-balloon 要件 5.5）: 本体側 scope（scope 0）の**バルーン定義**と、
/// そこから解決される**文字描画領域**が areka-P0-kero-balloon の適用前と同一である。
///
/// 成立の理由（適用前後の経路が同じ 2 ファイルへ落ちること）:
/// - 適用前は固定名 `balloons0s.txt` を上書き層として読んでいた。
/// - 適用後の scope 0 の連鎖は `balloonp0def` → `balloons` であり、emo2-kakukaku は
///   `balloonp0def*` を持たないため面 0 は `balloons0.png` へ解決し、対応する上書き層は
///   `balloons0s.txt` になる。
///
/// ゆえに両者は**同一の 2 層**をマージしており、モデルは bit 同値でなければならない。
/// 文字描画領域は [`ResolvedBalloonText::resolve`] にモデルと**バルーン面の image px 原寸**
/// （scope 0 は `balloons0.png` の 400×224＝適用前の固定名採寸が返していた値そのもの）を
/// 与えて解決する——`validrect` 由来の領域・折返し閾値・描画開始点までを含む同一性の檻。
///
/// 判別力: scope 1 の定義（`balloonk0s.txt` 由来）から解決した領域は別値になる。もし
/// 全 scope が 1 本のモデルへ畳み戻れば、あるいは scope 0 の上書き層が相方側へずれれば落ちる。
#[test]
fn scope0_balloon_model_and_text_region_match_pre_spec_fixed_name_merge() {
    /// scope 0 のバルーン面 0（`balloons0.png`）の image px 原寸——適用前の固定名採寸が
    /// 返していた値と同一（`areka_placement::measure` テストの `BALLOON0_W`/`BALLOON0_H`）。
    const SAKURA_FACE0: (u32, u32) = (400, 224);

    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    let oracle = pre_spec_balloon_model(&emo2_balloon_root());
    // 神託が空虚でないこと（実ファイルが読めており、面別層が実際に効いている）。
    assert_eq!(
        (oracle.windowposition().x(), oracle.windowposition().y()),
        (Some(266), Some(-129)),
        "神託の前提: 適用前は balloons0s.txt を上書き層として読んでいた"
    );

    assert_eq!(
        boot.balloons[0].definitions.face0().model,
        oracle,
        "本体側 scope の定義は適用前の固定名 2 層マージと同一"
    );

    // 文字描画領域（`validrect` 絶対矩形・描画開始点・折返し閾値）まで同一である。
    let now =
        ResolvedBalloonText::resolve(&boot.balloons[0].definitions.face0().model, SAKURA_FACE0);
    assert_eq!(
        now,
        ResolvedBalloonText::resolve(&oracle, SAKURA_FACE0),
        "本体側 scope の文字描画領域は適用前と同一"
    );

    // 判別力: 相方側 scope の定義から解決すると別領域になる（＝上の等式は空虚でない）。
    assert_ne!(
        boot.balloons[1].definitions.face0().model,
        oracle,
        "相方側まで神託と一致するなら本 fixture は判別力を持たない"
    );
    assert_ne!(
        ResolvedBalloonText::resolve(&boot.balloons[1].definitions.face0().model, SAKURA_FACE0),
        now,
        "相方側 scope の validrect は別値ゆえ文字描画領域も別になる"
    );
}

/// 名前宣言を持つ emo2 fixture を渡すと、
/// 起動時資産 `BootAssets.bind_resolver` から対応する着せ替え ID が解決できる。
///
/// emo2 shell descript の `sakura.bindgroup1100.name,腕,伸び` 由来で
/// `resolve(Sakura, "腕", "伸び") == Some(1100)`（`emo2_declared_names_all_resolve`
/// で存在確認済みの (カテゴリ, パーツ)→id 対）。未宣言の組は `None`（捏造しない・areka-P0-mayuna-compose 要件 3.7）。
#[test]
fn build_boot_assets_bind_resolver_resolves_emo2_names() {
    // 作者基準 DPI は emo2 fixture の実測既定（shell/balloon とも無宣言＝96）。
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    assert_eq!(
        boot.bind_resolver
            .resolve(BindNamespace::Sakura, "腕", "伸び"),
        Some(1100),
        "shell descript の名前宣言 `sakura.bindgroup1100.name,腕,伸び` が起動時資産から解決できる"
    );
    assert_eq!(
        boot.bind_resolver
            .resolve(BindNamespace::Sakura, "腕", "存在しない"),
        None,
        "未宣言の (カテゴリ, パーツ) は None（捏造しない）"
    );
}

/// mustselect 宣言を持つ emo2 fixture を渡すと、
/// 起動時資産 `BootAssets.bind_resolver` から対応するカテゴリが mustselect と判別でき、
/// そのカテゴリの ID 集合が引ける。
///
/// emo2 shell descript の `sakura.bindoption{0..3}.group,カテゴリ,mustselect`（腕/口/眉/目）由来で
/// `policy(Sakura, "腕"/"口"/"眉"/"目") == MustSelect`、非宣言カテゴリ（紅）は正典の既定
/// （`Default`）。mustselect カテゴリの ID 集合（目は複数 id）は非空であることを確認する。
#[test]
fn build_boot_assets_bind_resolver_carries_mustselect() {
    // 作者基準 DPI は emo2 fixture の実測既定（shell/balloon とも無宣言＝96）。
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    // emo2 fixture の sakura.bindoption*.group,カテゴリ,mustselect（腕/口/眉/目）が起動時資産から判別できる。
    for category in ["腕", "口", "眉", "目"] {
        assert_eq!(
            boot.bind_resolver.policy(BindNamespace::Sakura, category),
            BindChoicePolicy::MustSelect,
            "宣言済み mustselect カテゴリ `{category}` は起動時資産から MustSelect"
        );
    }
    // 非宣言カテゴリは mustselect でない（捏造しない・正典の既定＝Default）。
    assert_eq!(
        boot.bind_resolver.policy(BindNamespace::Sakura, "紅"),
        BindChoicePolicy::Default,
        "非宣言カテゴリ `紅` は mustselect でない（正典の既定）"
    );
    // mustselect カテゴリの ID 集合が引ける（目は複数 id を持つ）。
    assert!(
        !boot
            .bind_resolver
            .category_ids(BindNamespace::Sakura, "目")
            .is_empty(),
        "mustselect カテゴリ `目` の ID 集合は非空（複数 id）"
    );
}

/// （areka-P0-emo-dpi-scaling 要件 1.1）呼び手が渡した作者基準 DPI が
/// `BootAssets` の `shell_author_dpi`／`balloon_author_dpi` へそのまま搬送される。
///
/// shell=192／balloon=144 という **互いに異なる非 96 値**を渡し、両フィールドが宣言値
/// そのままで届くことを固定する。96 固定のハードワイヤ（あるいは shell/balloon の取り違え）
/// はこの檻で落ちる。搬送は純粋な値移送ゆえ縮退梯子（`parse_author_dpi`）は
/// 通さない——ここで検査するのは「渡した値が変質せず attach 相まで届くこと」だけである。
#[test]
fn build_boot_assets_carries_supplied_author_dpi() {
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 192, 144)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    assert_eq!(
        boot.shell_author_dpi, 192,
        "shell の作者基準 DPI は渡した宣言値のまま搬送される（96 固定でない）"
    );
    assert_eq!(
        boot.balloon_author_dpi, 144,
        "balloon の作者基準 DPI は渡した宣言値のまま搬送される（shell と取り違えない）"
    );
}

// ── 起動経路の surfaces.txt 復号（areka-P0-charset-canon 要件 6.1）──

/// `root` の下に ghost／shell／balloon の最小の検体を組み、balloon のフォルダを返す。
///
/// 画像は emo2 から `surface0.png`／`surface10.png`／`balloons0.png` の 3 枚だけを複写し、
/// シェルの descript.txt と surfaces.txt は呼び手が渡す。
fn synth_ghost(root: &Path, shell_descript: &str, surfaces: &[u8]) -> PathBuf {
    let ghost_master = root.join("ghost").join("master");
    let shell_master = root.join("shell").join("master");
    let balloon_dir = root.join("balloon-synth");
    for dir in [&ghost_master, &shell_master, &balloon_dir] {
        std::fs::create_dir_all(dir).expect("検体ディレクトリ作成");
    }

    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,えも\n",
    )
    .expect("ghost descript");
    std::fs::write(shell_master.join("descript.txt"), shell_descript).expect("shell descript");
    std::fs::write(shell_master.join("surfaces.txt"), surfaces).expect("surfaces.txt の書出し");
    for png in ["surface0.png", "surface10.png"] {
        std::fs::copy(
            emo2_root().join("shell/master").join(png),
            shell_master.join(png),
        )
        .unwrap_or_else(|e| panic!("{png} 複写: {e}"));
    }
    std::fs::write(
        balloon_dir.join("descript.txt"),
        "charset,UTF-8\ndpi,96\nuse_self_alpha,1\n",
    )
    .expect("balloon descript");
    std::fs::copy(
        emo2_balloon_root().join("balloons0.png"),
        balloon_dir.join("balloons0.png"),
    )
    .expect("balloons0.png 複写");
    balloon_dir
}

/// `charset,Shift_JIS` を宣言し、alias の値に Shift_JIS の「通常」を持つ surfaces.txt。
///
/// UTF-8 として不正なバイト並びであることが本質（`92 CA 8F ED` は UTF-8 の妥当な並びでは
/// ない）。適用前の `read_to_string` はここで読取失敗になる。
const SHIFT_JIS_SURFACES_TXT: &[u8] = b"charset,Shift_JIS\nsurface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface10\n{\nelement0,overlay,surface10.png,0,0\n}\nsakura.surface.alias\n{\n\x92\xCA\x8F\xED,[0]\n}\n";

/// areka-P0-charset-canon 要件 6.1: **起動時**の surfaces.txt 読取が、そのファイル自身の `charset` 宣言に従う。
///
/// `build_boot_assets` はシェル dir でなく ghost ルートを取り、マウント解決を経て
/// shell dir へ辿り着く。ゆえに検体は `(ghost_root, balloon_root)` の対を組む必要があるが、
/// これは実 shell を丸ごと複写しなくてよい——`areka-placement/src/test_support.rs` の
/// `synth_declared_dpi_ghost` と同型に、**PNG 3 枚の複写**（`surface0.png`／`surface10.png`／
/// `balloons0.png`）と数行の descript で足りる。その shell だけを `charset,Shift_JIS` 宣言の
/// surfaces.txt に差し替えたものが本検体。
///
/// **この検査がここに居る理由**: `surfaces.txt` の読取と復号は `assets.rs` が自分では行わず、
/// 読み込みの権威 `areka_emo_present::shell_target::load_shell_target_with_limits` が担う。
/// 本検査は、その権威を通った起動経路（`build_boot_assets`）が宣言どおりの文字コードで読めることを
/// 固定する。パーサ層の検査（`charset::decode` と `shell::parse` の等値）は権威への配線までは
/// 見ないため、配線が `read_to_string` へ戻ってもパーサ層の検査は緑のまま通り抜ける。
/// 配置採寸側の同種の検査は `crates/areka-placement/src/measure_tests.rs` にあり、起動側は読取点ごとに
/// 兄弟のテストファイルへ 1 本ずつ置く形にしている。
///
/// 本検査が主張するのは「読取が失敗しないこと」までで、復号後の文字列そのものは見ない
/// （Shift_JIS を別の 8bit 文字コードとして復号する退化はここを素通りする）。文字コード
/// ごとの解析結果の等値はパーサ層（areka-P0-charset-canon）が担当する分業。
///
/// `bake` は WIC で PNG をデコードするため COM 初期化が要る（本ファイルの既存檻と同じ）。
#[test]
fn boot_read_honours_the_files_own_charset_declaration() {
    // 一時パスは共通窓口 `temp-path-kit` から受け取る（プロセス間で一意・Drop で中身ごと消える）。
    // 自前に組むと `std::env::temp_dir` の迂回を見張る番人が赤になる。
    let root = TempPath::new("areka-boot-assets-charset");
    // シェル定義ファイルだけを Shift_JIS で置く（他ファイルの宣言は持ち込まれない・areka-P0-charset-canon 要件 6.2）。
    let balloon_dir = synth_ghost(
        root.path(),
        "charset,UTF-8\nseriko.dpi,96\nseriko.use_self_alpha,1\n",
        SHIFT_JIS_SURFACES_TXT,
    );

    // `BootAssets` は Debug を持たないので、失敗側だけを取り出して表示する。
    let result = build_boot_assets(root.path(), &balloon_dir, &[0], 96, 96);
    assert!(
        result.is_ok(),
        "Shift_JIS 宣言の surfaces.txt でも起動時資産の組立が成立すること: {:?}",
        result.err()
    );
}
