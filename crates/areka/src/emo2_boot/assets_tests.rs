use std::path::PathBuf;

use areka_emo_text::actor::ResolvedBalloonText;
use areka_seriko::{BindChoicePolicy, BindNamespace, SurfaceTarget};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::*;
// アクタ鍵 → scope の逆写像正本（`ActorKey` 語彙が既存写像と同一であることの往復検査に使う）。
use crate::emo2_boot::target_map::scope_of;
// 本番 boot（design Flow 3 手順1）と同じ作者基準 DPI 読取器（task 2.1）。
use crate::placement::source::{load_balloon_author_dpi, load_descript_source};

/// `(key, value)` のスライスから shell descript KV 相当の `BTreeMap` を組む。
fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// emo2 fixture ルートを `CARGO_MANIFEST_DIR`（`crates/areka`）相対で解決する
/// （placement source/measure・emo-present example と同一アンカー規約）。
fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// emo2 fixture のバルーンルート（placement テストと同一規約）。
fn emo2_balloon_root() -> PathBuf {
    emo2_root().join("emo2-kakukaku")
}

/// 観測可能な完了条件（tasks.md task 2.6）: emo2 fixture を渡した統合テストが
/// `BootAssets` の各フィールドに期待どおりのデータを含んで green で通る。
///
/// 既知 scope 集合 `[0, 1]` に対し `build_boot_assets` が populated な `ScopeAssets`
/// （scope0=surface0／scope1=surface10・DD-9）・scope ごとの balloon 資産・2 層マージ済み
/// `BalloonModel`・emo2 alias 由来の `resolver`・DD-8 の `static_binds`
/// `[1100,1207,1302,1500,1800]` を返すことを固定する。戻り値だけで以後ファイル I/O が
/// 不要になる（＝全 I/O が本関数内で完結する）ことを、各フィールドが実データで
/// populated であることの積極 assert で担保する。
///
/// `bake` は WIC で PNG をデコードするため COM 初期化が要る（GPU は不要・attach なし）。
#[test]
fn build_boot_assets_from_emo2_fixture() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    // 作者基準 DPI は emo2 fixture の実測既定（shell/balloon とも無宣言＝96・task 2.1）。
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    // --- shells: 要求 scope に 1:1 対応・初期 surface id は DD-9（scope0=0／scope>=1=10） ---
    assert_eq!(boot.shells.len(), 2, "要求 scope 集合 [0,1] に 1:1 対応");
    assert_eq!(boot.shells[0].scope, 0);
    assert_eq!(
        boot.shells[0].initial_surface_id, 0,
        "scope0 → 初期 surface 0（DD-9）"
    );
    assert_eq!(boot.shells[1].scope, 1);
    assert_eq!(
        boot.shells[1].initial_surface_id, 10,
        "scope>=1 → 初期 surface 10（DD-9）"
    );

    // 各 scope の EmoWorld は FRESH に build 済みで、初期表示 surface を実際に内包する
    // （scope0=surface0／scope1=surface10）。装着（task 4.x）へ手渡す前段の populated 担保。
    assert!(
        boot.shells[0].emo_world.surface(0).is_some(),
        "scope0 の World は初期 surface 0 を内包する"
    );
    assert!(
        boot.shells[1].emo_world.surface(10).is_some(),
        "scope1 の World は初期 surface 10 を内包する"
    );
    // 共有アトラスは 1 回の bake 由来（Clone 共有）＝非空・全 scope 同一エントリ数。
    assert!(!boot.shells[0].atlas.is_empty(), "shell アトラスは bake 済み（非空）");
    assert_eq!(
        boot.shells[0].atlas.len(),
        boot.shells[1].atlas.len(),
        "AtlasTable は parse/bake 1 回の Clone 共有（scope 間で同一）"
    );

    // --- balloons: scope ごとに populated（面 0 初期表示の資産） ---
    assert_eq!(boot.balloons.len(), 2, "balloon 資産も scope ごとに 1:1");
    assert_eq!(boot.balloons[0].scope, 0);
    assert_eq!(boot.balloons[1].scope, 1);
    // balloon target World は面 0（初期表示・DD-9）を内包する（構築が実枠を組んだ担保）。
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
    let vr0 = boot.balloons[0].model.validrect();
    assert_eq!(
        vr0.top(),
        Some(46),
        "面別層 balloons0s.txt が descript を後勝ち上書き"
    );
    assert_eq!(vr0.bottom(), Some(-56));
    assert_eq!(vr0.left(), Some(36));
    assert_eq!(vr0.right(), Some(-44));
    // windowposition は面別層のみが供給するキー（descript.txt に不在）。
    let wp0 = boot.balloons[0].model.windowposition();
    assert_eq!(wp0.x(), Some(266), "windowposition は面別層のみ供給");
    assert_eq!(wp0.y(), Some(-129));

    // scope1 は balloonk0.png を採用 → balloonk0s.txt が上書き層（balloons0s.txt ではない・R2.2）。
    let vr1 = boot.balloons[1].model.validrect();
    assert_eq!(
        vr1.top(),
        Some(40),
        "相方側は balloonk0s.txt が上書き層（本体側 46 ではない）"
    );
    assert_eq!(vr1.bottom(), Some(-70));
    assert_eq!(vr1.left(), Some(24));
    assert_eq!(vr1.right(), Some(-48));
    let wp1 = boot.balloons[1].model.windowposition();
    assert_eq!(wp1.x(), Some(-190), "相方側 windowposition は balloonk0s.txt 実値");
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
        "複数候補は先頭固定（DD6）"
    );
    assert_eq!(
        boot.resolver.resolve("-1"),
        SurfaceTarget::Hide,
        "非表示センチネルは alias 非依存"
    );

    // --- static_binds: DD-8 の emo2 default==1 集合（昇順） ---
    assert_eq!(
        boot.static_binds.ids(),
        &[1100, 1207, 1302, 1500, 1800],
        "shell descript の sakura.bindgroup{{N}}.default==1（DD-8）"
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
    // balloon 表は **scope 別**（キー＝scope 番号・Req 5.6）。各 scope の表が、その scope が
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

/// 観測可能な完了条件（tasks.md task 3.2・要件 5.6）: バルーンのループ表が **scope 別の写像**
/// として導出され、そのキー集合が scope 集合と一致する。
///
/// 「先頭 scope の資産から 1 本組めば全 scope に足りる」という旧前提の実装（＝写像が単一
/// エントリ `{0}` に縮む形）はこの檻で落ちる。またキー集合が `BootAssets.balloons` の scope
/// 集合と一致することで design「Data Models」不変条件 (c) を固定する。
///
/// 内容についての注記（データ事実・観測等価）: emo2 のバルーン面は `areka-emo-present` の
/// 合成 `surfaces.txt` から build され、そこに `animation*` 行が一切無いため **全 scope とも
/// 空表**になる。したがって本仕様が変えた観測は表の中身ではなく **どの World から何本の表を
/// 導出するか**であり、この檻はキー集合（と導出元 World の scope 一致）を固定する。表内容の
/// 一致は `build_boot_assets_from_emo2_fixture` が scope ごとの再構築突合で担保する。
#[test]
fn build_boot_assets_derives_balloon_loop_tables_per_scope() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    let table_scopes: Vec<u32> = boot.loop_tables.balloon.keys().copied().collect();
    assert_eq!(
        table_scopes,
        vec![0, 1],
        "バルーン表は scope ごとに 1 本ずつ導出される（先頭 scope の 1 本で済ませない・R5.6）"
    );
    let asset_scopes: Vec<u32> = boot.balloons.iter().map(|b| b.scope).collect();
    assert_eq!(
        table_scopes, asset_scopes,
        "バルーン表のキー集合は balloons の scope 集合と一致する（不変条件 (c)）"
    );

    // データ事実（合成 surfaces.txt に animation 行が無い）ゆえ emo2 は全 scope 空表。
    // 中身が観測等価でも、駆動される表が scope ごとに別実体であることが本仕様の是正点。
    for (scope, table) in &boot.loop_tables.balloon {
        assert!(
            table.is_empty(),
            "emo2 の合成 surfaces.txt は animation 行を持たないため scope {scope} も空表（データ事実）"
        );
    }

    // シェル表は単数のまま（全 scope 同一 Shell 由来＝この前提はシェル面に限る）。
    let shell_rebuilt = AnimationTable::from_world(&boot.shells[0].emo_world);
    assert_eq!(
        boot.loop_tables.shell.is_empty(),
        shell_rebuilt.is_empty(),
        "シェル表は単数のまま（scope 非依存という前提が成り立つのはシェル面に限る）"
    );
}

/// 観測可能な完了条件（tasks.md task 3.2・要件 5.6）: 起動シームの転送が **全 scope** を
/// アクタ鍵語彙へ写して seriko へ渡す（単一エントリ暫定でない）。
///
/// `wire_emo2_boot`（mod.rs）と spine ハーネス（spine.rs）が共有する転送
/// [`actor_keyed_balloon_tables`] を実 fixture 由来の写像へ適用し、`SerikoLoopConfig.balloon_tables`
/// のキー集合が `ActorKey::from(scope.to_string())` で `{"0","1"}` になることを固定する。
/// 先頭 scope だけを載せる旧形（`{"0"}`）はこの檻で落ちる。
#[test]
fn boot_seam_transfers_every_balloon_scope_as_actor_key() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");
    let asset_scopes: Vec<u32> = boot.balloons.iter().map(|b| b.scope).collect();

    // 起動シーム（mod.rs／spine.rs）が行う転送そのもの。
    let transferred = actor_keyed_balloon_tables(boot.loop_tables.balloon);

    let keys: Vec<ActorKey> = transferred.keys().cloned().collect();
    assert_eq!(
        keys,
        vec![ActorKey::from("0"), ActorKey::from("1")],
        "全 scope がアクタ鍵語彙で seriko へ渡る（単一エントリ暫定でない・R5.6）"
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

    assert_eq!(keyed.len(), 3, "全エントリが転送される（取りこぼし・併合なし）");
    for scope in [0u32, 1, 10] {
        assert!(
            keyed.contains_key(&ActorKey::from(scope.to_string())),
            "scope {scope} は `ActorKey::from(scope.to_string())` のキーで引ける"
        );
    }
    // 逆写像で元の scope 集合へ戻る（別語彙 `"scope0"` 等を作っていない）。
    let mut round_tripped: Vec<u32> = keyed.keys().filter_map(scope_of).collect();
    round_tripped.sort_unstable();
    assert_eq!(round_tripped, vec![0, 1, 10], "scope_of の逆写像で元の集合へ戻る");
}

/// 空の scope 集合では balloon 表の写像も空になる（縮退・不変条件 (c) の degenerate 端）。
///
/// seriko 側は不在 scope を**空表意味論**（抽選対象ゼロ・乱数非消費・panic なし）で扱うため、
/// 空写像はループ完全不活性と等価である。
#[test]
fn build_boot_assets_yields_empty_balloon_table_map_for_no_scopes() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

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

/// 観測可能な完了条件（tasks.md task 3.1・要件 2.1／7.2）: 起動時資産が **scope ごとに
/// 異なる**バルーン定義を保持する（共有 1 本ではない）。
///
/// emo2-kakukaku fixture では scope0 が `balloons0.png`（→ `balloons0s.txt`）を、scope1 が
/// `balloonk0.png`（→ `balloonk0s.txt`）を採用する。両 scope の `windowposition` /
/// `validrect` が**互いに異なる**こと、および `wordwrappoint.x` が
/// **scope0 は面別層で -49 に上書き・scope1 は面別層に宣言が無いため descript 基層の -34 を継承**
/// することを固定する（R2.1／R2.2／R2.5）。共有 1 本のモデルを全 scope へ配る実装はこの檻で落ちる。
#[test]
fn build_boot_assets_holds_per_scope_balloon_models() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    // 不変条件 (a): balloons の scope 集合＝shells の scope 集合。
    let balloon_scopes: Vec<u32> = boot.balloons.iter().map(|b| b.scope).collect();
    let shell_scopes: Vec<u32> = boot.shells.iter().map(|s| s.scope).collect();
    assert_eq!(
        balloon_scopes, shell_scopes,
        "balloons の scope 集合は shells の scope 集合と一致する（不変条件 (a)）"
    );

    let scope0 = &boot.balloons[0].model;
    let scope1 = &boot.balloons[1].model;

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
    // descript 基層の -34 を継承する（上書き／継承の対照＝R2.5）。
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

/// 本仕様適用**前**の 2 層マージを再現する神託（tasks 5.1・R5.5）。
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

/// R5.5（tasks 5.1）本体側 scope の非回帰: 本体側 scope（scope 0）の**バルーン定義**と、
/// そこから解決される**文字描画領域**が本仕様適用前と同一である。
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
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    /// scope 0 のバルーン面 0（`balloons0.png`）の image px 原寸——適用前の固定名採寸が
    /// 返していた値と同一（`placement::measure` テストの `BALLOON0_W`/`BALLOON0_H`）。
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
        boot.balloons[0].model, oracle,
        "本体側 scope の定義は適用前の固定名 2 層マージと同一（R5.5）"
    );

    // 文字描画領域（`validrect` 絶対矩形・描画開始点・折返し閾値）まで同一である。
    let now = ResolvedBalloonText::resolve(&boot.balloons[0].model, SAKURA_FACE0);
    assert_eq!(
        now,
        ResolvedBalloonText::resolve(&oracle, SAKURA_FACE0),
        "本体側 scope の文字描画領域は適用前と同一（R5.5）"
    );

    // 判別力: 相方側 scope の定義から解決すると別領域になる（＝上の等式は空虚でない）。
    assert_ne!(
        boot.balloons[1].model, oracle,
        "相方側まで神託と一致するなら本 fixture は判別力を持たない"
    );
    assert_ne!(
        ResolvedBalloonText::resolve(&boot.balloons[1].model, SAKURA_FACE0),
        now,
        "相方側 scope の validrect は別値ゆえ文字描画領域も別になる"
    );
}

/// 観測可能な完了条件（tasks.md task 7.1）: 名前宣言を持つ emo2 fixture を渡すと、
/// 起動時資産 `BootAssets.bind_resolver` から対応する着せ替え ID が解決できる。
///
/// emo2 shell descript の `sakura.bindgroup1100.name,腕,伸び` 由来で
/// `resolve(Sakura, "腕", "伸び") == Some(1100)`（task 1.2 の `emo2_declared_names_all_resolve`
/// で存在確認済みの (カテゴリ, パーツ)→id 対）。未宣言の組は `None`（捏造しない・R3.7）。
#[test]
fn build_boot_assets_bind_resolver_resolves_emo2_names() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    // 作者基準 DPI は emo2 fixture の実測既定（shell/balloon とも無宣言＝96・task 2.1）。
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    assert_eq!(
        boot.bind_resolver.resolve(BindNamespace::Sakura, "腕", "伸び"),
        Some(1100),
        "shell descript の名前宣言 `sakura.bindgroup1100.name,腕,伸び` が起動時資産から解決できる（7.1）"
    );
    assert_eq!(
        boot.bind_resolver.resolve(BindNamespace::Sakura, "腕", "存在しない"),
        None,
        "未宣言の (カテゴリ, パーツ) は None（捏造しない・R3.7）"
    );
}

/// 観測可能な完了条件（tasks.md task 10.3）: mustselect 宣言を持つ emo2 fixture を渡すと、
/// 起動時資産 `BootAssets.bind_resolver` から対応するカテゴリが mustselect と判別でき、
/// そのカテゴリの ID 集合が引ける。
///
/// emo2 shell descript の `sakura.bindoption{0..3}.group,カテゴリ,mustselect`（腕/口/眉/目）由来で
/// `policy(Sakura, "腕"/"口"/"眉"/"目") == MustSelect`、非宣言カテゴリ（紅）は正典の既定
/// （`Default`）。mustselect カテゴリの ID 集合（目は複数 id）は非空であることを確認する
/// （bindopt 1.2・R7.1）。
#[test]
fn build_boot_assets_bind_resolver_carries_mustselect() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    // 作者基準 DPI は emo2 fixture の実測既定（shell/balloon とも無宣言＝96・task 2.1）。
    let boot = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");

    // emo2 fixture の sakura.bindoption*.group,カテゴリ,mustselect（腕/口/眉/目）が起動時資産から判別できる。
    for category in ["腕", "口", "眉", "目"] {
        assert_eq!(
            boot.bind_resolver.policy(BindNamespace::Sakura, category),
            BindChoicePolicy::MustSelect,
            "宣言済み mustselect カテゴリ `{category}` は起動時資産から MustSelect（10.3）"
        );
    }
    // 非宣言カテゴリは mustselect でない（捏造しない・正典の既定＝Default・bindopt 1.2）。
    assert_eq!(
        boot.bind_resolver.policy(BindNamespace::Sakura, "紅"),
        BindChoicePolicy::Default,
        "非宣言カテゴリ `紅` は mustselect でない（正典の既定・bindopt 1.2）"
    );
    // mustselect カテゴリの ID 集合が引ける（目は複数 id を持つ）。
    assert!(
        !boot
            .bind_resolver
            .category_ids(BindNamespace::Sakura, "目")
            .is_empty(),
        "mustselect カテゴリ `目` の ID 集合は非空（複数 id・10.3）"
    );
}

/// 観測可能な完了条件（tasks.md task 4.1・要件 1.1）: 呼び手が渡した作者基準 DPI が
/// `BootAssets` の `shell_author_dpi`／`balloon_author_dpi` へそのまま搬送される。
///
/// shell=192／balloon=144 という **互いに異なる非 96 値**を渡し、両フィールドが宣言値
/// そのままで届くことを固定する。96 固定のハードワイヤ（あるいは shell/balloon の取り違え）
/// はこの檻で落ちる。搬送は純粋な値移送ゆえ縮退梯子（`parse_author_dpi`・task 2.1）は
/// 通さない——ここで検査するのは「渡した値が変質せず attach 相まで届くこと」だけである。
#[test]
fn build_boot_assets_carries_supplied_author_dpi() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

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

/// 観測可能な完了条件（tasks.md task 4.1・要件 1.1・design Flow 3 手順1）: task 2.1 の
/// 読取アクセサが emo2 fixture から返す値が、そのまま `BootAssets` から取り出せる。
///
/// 本番 boot（design Flow 3）は `load_descript_source(...).shell_author_dpi()` と
/// `load_balloon_author_dpi(balloon_root)` で作者基準 DPI を **1 度だけ**読み、同じ値を
/// 採寸（`MeasureScaling` の k₀）と attach（`attach_target`）の双方へ配る。ここでは実
/// fixture に対しその経路を再現し、アクセサの戻り値と `BootAssets` の搬送値が一致すること
/// （＝搬送の途中で捏造・既定値差し替えが起きないこと）を固定する。
///
/// emo2 fixture は shell（`seriko.dpi`）・balloon（`dpi`）とも **DPI 無宣言**ゆえ、
/// 正典既定の 96/96 が現実の既定ケースとなる（task 2.1 実測・既存採寸期待値に影響なし）。
#[test]
fn build_boot_assets_carries_emo2_accessor_author_dpi() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    // 本番 boot と同じ読取器（task 2.1）で emo2 fixture の作者基準 DPI を得る。
    let source =
        load_descript_source(&emo2_root()).expect("emo2 fixture は Ok(DescriptSource) を返す");
    let shell_dpi = source.shell_author_dpi();
    let balloon_dpi = load_balloon_author_dpi(&emo2_balloon_root());
    assert_eq!(
        shell_dpi, 96,
        "emo2 shell は seriko.dpi 無宣言＝正典既定 96"
    );
    assert_eq!(balloon_dpi, 96, "emo2 balloon は dpi 無宣言＝正典既定 96");

    let boot = build_boot_assets(
        &emo2_root(),
        &emo2_balloon_root(),
        &[0, 1],
        shell_dpi,
        balloon_dpi,
    )
    .expect("emo2 fixture の BootAssets 組立は成功する");

    assert_eq!(
        boot.shell_author_dpi, shell_dpi,
        "搬送値は読取アクセサ（task 2.1）の戻り値と一致する"
    );
    assert_eq!(
        boot.balloon_author_dpi, balloon_dpi,
        "搬送値は読取アクセサ（task 2.1）の戻り値と一致する"
    );
}

/// 観測可能な完了条件: emo2 fixture 相当 KV から `[1100,1207,1302,1500,1800]` を抽出する。
///
/// noise（`.name` エントリ・`kero.*`・`sakura.menu`・`sakura.bindoption*`・`charset`/`type`/
/// `seriko.*`）を混在させても `.default==1` の 5 件だけが昇順で抽出されることを固定する
/// （実 fixture `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt` 実測）。
#[test]
fn default_bind_ids_extracts_emo2_defaults() {
    let map = kv(&[
        // --- メタ／既定（noise・非 bindgroup） ---
        ("charset", "UTF-8"),
        ("type", "shell"),
        ("seriko.use_self_alpha", "1"), // value=="1" だが bindgroup キーでない → 非抽出
        ("sakura.defaultx", "0"),
        ("kero.defaultx", "0"),
        ("sakura.balloon.alignment", "left"),
        ("kero.balloon.alignment", "right"),
        // --- 腕 ---
        ("sakura.bindgroup1100.name", "腕,伸び"),
        ("sakura.bindgroup1100.default", "1"),
        ("sakura.bindgroup1101.name", "腕,組み"),
        // --- 口 ---
        ("sakura.bindgroup1206.name", "口,‥‥"),
        ("sakura.bindgroup1207.name", "口,にこっ"),
        ("sakura.bindgroup1207.default", "1"),
        ("sakura.bindgroup1208.name", "口,小口"),
        // --- 目 ---
        ("sakura.bindgroup1301.name", "目,ジトー"),
        ("sakura.bindgroup1302.name", "目,通常"),
        ("sakura.bindgroup1302.default", "1"),
        ("sakura.bindgroup1303.name", "目,笑顔"),
        // --- まばたき（default なし） ---
        ("sakura.bindgroup1400.name", "まばたき,通常"),
        ("sakura.bindgroup1403.name", "まばたき,----"),
        // --- 眉 ---
        ("sakura.bindgroup1500.name", "眉,通常"),
        ("sakura.bindgroup1500.default", "1"),
        ("sakura.bindgroup1501.name", "眉,オコ"),
        // --- 紅／キラリ（default なし） ---
        ("sakura.bindgroup1600.name", "紅,差し"),
        ("sakura.bindgroup1700.name", "キラリ,キラリ1"),
        // --- 髪飾り ---
        ("sakura.bindgroup1800.name", "髪飾り,リボン"),
        ("sakura.bindgroup1800.default", "1"),
        ("sakura.bindgroup1801.name", "髪飾り,ボンボン"),
        // --- 着せ替えオプション／メニュー（noise） ---
        ("sakura.bindoption0.group", "腕,mustselect"),
        ("sakura.menu", "auto"),
    ]);
    assert_eq!(
        default_bind_ids(&map),
        vec![1100, 1207, 1302, 1500, 1800],
        "emo2 fixture 相当 KV からは default==1 の 5 件のみを昇順抽出する"
    );
}

/// `default` が `1` 以外（`0`／`2`／空／非数）の値は抽出しない。
#[test]
fn default_bind_ids_excludes_non_one_values() {
    let map = kv(&[
        ("sakura.bindgroup9990.default", "0"),
        ("sakura.bindgroup9991.default", "2"),
        ("sakura.bindgroup9992.default", ""),
        ("sakura.bindgroup9993.default", "true"),
        ("sakura.bindgroup9994.default", "10"), // "10" は "1" と等しくない
    ]);
    assert_eq!(
        default_bind_ids(&map),
        Vec::<u32>::new(),
        "default==1 以外は全て非抽出"
    );
}

/// `kero.*` scope の bindgroup default は本タスク対象外（M-dual 増分）＝非抽出。
#[test]
fn default_bind_ids_excludes_kero_scope() {
    let map = kv(&[
        ("kero.bindgroup50.default", "1"),
        ("kero.bindgroup1100.default", "1"),
        // sakura 側は抽出される対照。
        ("sakura.bindgroup1100.default", "1"),
    ]);
    assert_eq!(
        default_bind_ids(&map),
        vec![1100],
        "kero scope は非抽出（M-dual 増分）・sakura scope のみ抽出"
    );
}

/// 無関係キー（`.name`・任意 noise）は `default==1` を持たない限り無視する。
#[test]
fn default_bind_ids_ignores_unrelated_keys() {
    let map = kv(&[
        ("sakura.bindgroup1100.name", "腕,伸び"), // .name は非抽出
        ("charset", "UTF-8"),
        ("some.random.key", "1"),
        ("sakura.menu", "auto"),
    ]);
    assert_eq!(
        default_bind_ids(&map),
        Vec::<u32>::new(),
        "default==1 の bindgroup キーが無ければ何も抽出しない"
    );
}

/// 中間の N が u32 として parse できないキーは無視する。
#[test]
fn default_bind_ids_ignores_malformed_middle() {
    let map = kv(&[
        ("sakura.bindgroupXYZ.default", "1"), // 非数値
        ("sakura.bindgroup.default", "1"),    // 空（middle なし）
        ("sakura.bindgroup-1.default", "1"),  // 負値は u32 parse 不可
        ("sakura.bindgroup12ab.default", "1"), // 数字混在
    ]);
    assert_eq!(
        default_bind_ids(&map),
        Vec::<u32>::new(),
        "middle が u32 でないキーは非抽出"
    );
}

/// prefix/suffix は厳密一致。パターンを部分文字列として含むだけのキーは match しない。
#[test]
fn default_bind_ids_requires_strict_prefix_and_suffix() {
    let map = kv(&[
        ("xsakura.bindgroup1.default", "1"),   // prefix 前に余分
        ("sakura.bindgroup2.defaultx", "1"),   // suffix 後に余分
        ("sakura.bindgroup3.default.extra", "1"), // suffix の後に別セグメント
        ("prefix.sakura.bindgroup4.default", "1"), // 別 prefix 配下
        // 対照: 厳密一致は抽出される。
        ("sakura.bindgroup5.default", "1"),
    ]);
    assert_eq!(
        default_bind_ids(&map),
        vec![5],
        "prefix/suffix 厳密一致のキーのみ抽出（部分一致は除外）"
    );
}

/// 決定論: 結果は数値昇順（lexicographic ではなく numeric）で返る。
///
/// BTreeMap のキー反復は lexicographic 順（"1000" < "100" < "200" < "90"）であり、
/// numeric 昇順（90 < 100 < 200 < 1000）と一致しない。結果が numeric 昇順であることを
/// 檻に入れ、キー反復順への依存を排除する。
#[test]
fn default_bind_ids_returns_sorted_numeric_ascending() {
    let map = kv(&[
        ("sakura.bindgroup90.default", "1"),
        ("sakura.bindgroup100.default", "1"),
        ("sakura.bindgroup1000.default", "1"),
        ("sakura.bindgroup200.default", "1"),
    ]);
    let ids = default_bind_ids(&map);
    assert_eq!(
        ids,
        vec![90, 100, 200, 1000],
        "numeric 昇順で返る（lexicographic キー順に依存しない）"
    );
    // 重複なし（決定論の担保）。
    let mut sorted_unique = ids.clone();
    sorted_unique.dedup();
    assert_eq!(ids, sorted_unique, "結果に重複が無い");
}

/// 値は trim して比較する（前後空白付き `" 1 "` は抽出・`" 0 "` は非抽出）。
#[test]
fn default_bind_ids_trims_value_whitespace() {
    let map = kv(&[
        ("sakura.bindgroup10.default", " 1 "),
        ("sakura.bindgroup20.default", "\t1\n"),
        ("sakura.bindgroup30.default", " 0 "),
    ]);
    assert_eq!(
        default_bind_ids(&map),
        vec![10, 20],
        "trim 後に \"1\" と一致する値のみ抽出"
    );
}
