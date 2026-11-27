//! 構築入力（BootAssets）の組立と shell descript からの static bindset 抽出。
//!
//! `build_boot_assets` が、shell（`load_shell_target` を 1 回→scope ごとに `ShellTarget::take_world`）・
//! balloon（scope ごとに `resolve_balloon_faces`→`build_balloon_target_from_faces`＋
//! `load_scope_balloon_definitions`＝[`BalloonScopeAssets`]）・`SurfaceResolver`（`alias_snapshot()`）・
//! 既定の着せ替えと名前の表（シェルの着せ替えの読みからスコープごとに `DefaultBinds::from_groups`・
//! `BindResolver::from_groups`）を所有する。戻り値だけで以後ファイル I/O 不要にする
//! （parse／bake は 1 回・`AtlasTable` は Clone 共有）。失敗は `BootWiringError` で観測可能にする。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use areka_emo_atlas::{AlphaParams, AnimationLimits, AtlasTable, ElementDecoder, WicDecoderArm};
use areka_emo_compose::EmoWorld;
use areka_emo_present::BalloonLoadError;
use areka_emo_present::balloon::{
    build_balloon_target_from_faces_with_limits, load_balloon_use_self_alpha, resolve_balloon_faces,
};
use areka_emo_present::balloon_definitions::{
    ScopeBalloonDefinitions, load_scope_balloon_definitions,
};
use areka_emo_present::shell_target::load_shell_target_with_limits;
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::resolve_with_shell;
use areka_sakura::ActorKey;
use areka_seriko::{AnimationTable, BindResolver, DefaultBinds, SurfaceResolver};
use tracing::error;

use super::BootWiringError;
use super::balloon_background;
use super::shell_box_assets::{ShellBoxAssets, box_font_search_dirs, resolve_box_markers};
use super::shell_cursor_assets::{ShellCursorAssets, build_shell_cursors};
use areka_placement::DefaultSurfaces;
use areka_placement::source::read_kv_lenient;

/// descript 定義ファイル名（読めないシェルを起動の失敗にするために読む）。
const DESCRIPT_TXT: &str = "descript.txt";

/// 1 scope 分のシェル表示資産（`attach_target` へ手渡す 1 組）。
///
/// `emo_world` は scope 専用に `ShellTarget::take_world` で受け取った非 Clone World（装着で move 消費）。
/// `atlas` は読み込みを 1 回で済ませた共有アトラス（`AtlasTable` は内部 Arc の安価 Clone）。
pub struct ScopeAssets {
    /// この資産が対応する scope 番号。
    pub scope: u32,
    /// scope 専用 build 済みの表示 World（アトラス装着済み・装着で move 消費）。
    pub emo_world: EmoWorld,
    /// 共有アトラス（読み込み 1 回・Clone 共有）。
    pub atlas: AtlasTable,
    /// 初期表示 surface id＝descript の既定の面（`*.seriko.defaultsurface`・書いていなければ
    /// 本体側 0／それ以外 10）。
    pub initial_surface_id: u32,
}

/// 1 scope 分のバルーン表示資産（[`ScopeAssets`] と対称）。
///
/// scope・表示 World・アトラス・**その scope 専用の**マージ済み定義を 1 件に束ねる。
/// 「全 scope 共有のバルーン定義 1 本」を型として表現できなくして、ある scope のバルーンが別 scope の系列
/// 由来の定義で駆動される事故を構造的に防ぐ。
///
/// `emo_world` は当該 scope が解決した系列から build した非 Clone World（装着で move 消費）。
/// `atlas` は当該 scope の面画像を bake した表（`AtlasTable` は内部 Arc の安価 Clone）。
pub struct BalloonScopeAssets {
    /// この資産が対応する scope 番号。
    pub scope: u32,
    /// scope の系列から build 済みの表示 World（`bind_atlas(SetId(0))` 済み・装着で move 消費）。
    pub emo_world: EmoWorld,
    /// 当該 scope の bake 済みアトラス。
    pub atlas: AtlasTable,
    /// scope 別の面ごとの定義の束（面ごとに 2 層マージ済みのモデル＋その面の矢印の絵・
    /// 文字層・`windowposition`／`validrect` の源）。
    pub definitions: ScopeBalloonDefinitions,
    /// バルーンの背景色（面 0 の焼き込み済み画像の原点画素・sRGB 非 premultiplied）。
    ///
    /// 無効表示（`\f[disable]`）の文字色を「既定の文字色を背景色の側へ寄せた色」として導く
    /// ための混色の相手。原点画素が採れないとき（面が引けない・全透明・トリムで原点が bbox の
    /// 外・α が 255 でない）は白へ落ちる（導出と記録は `balloon_background` モジュール）。
    ///
    /// 消費点は `frame/attach.rs` の `connect_balloon_text`——装着（`register_actor_view`）の
    /// **前**に `TextLayerRuntime::set_balloon_background` へ渡す（順序は `frame_attach_tests.rs` の
    /// `connect_balloon_text_resolves_text_with_the_background_at_attach_time` が見張る）。
    /// 導出は `assets_tests.rs` の emo2 fixture のテストが固定する。
    pub background_color: (u8, u8, u8),
    /// 普通のバルーンの名前（バルーンのフォルダ名・警告の名前の欄に出す）。
    pub name: String,
}

/// SERIKO ループ表の一括（シェル面 1 表＋バルーン面の scope 別表）。
///
/// 面種非依存: 同一 `AnimationTable::from_world` がシェル世界・バルーン世界の双方から
/// 同型に表を組む。`shell`／`balloon` は **surface ID 名前空間の別**（emo2 はシェル surface0 と
/// バルーン面 0 が別物）であり能力の仕切りではない。いずれの表も同一評価経路で駆動される。
///
/// いずれの表も既に `BootAssets` が保持する fold 完了 `EmoWorld` スナップショットから構築するため、
/// 表の構築に **新規のファイル I/O は要らない**。
pub struct LoopTables {
    /// シェル面のループ表（`shells[0].emo_world` から `from_world`・scope 資産不在なら空表）。
    ///
    /// 単数でよいのは **シェル面に限る**: 全 scope が同一 `Shell` から build されるため
    /// 表の内容が scope 非依存になる。バルーン面にこの前提は無い（下記 `balloon` 参照）。
    pub shell: AnimationTable,
    /// バルーン面の **scope 別**ループ表（キー＝scope 番号）。
    ///
    /// バルーン World は scope ごとに解決される系列（`balloons*`／`balloonk*` 等）が異なるため、
    /// ある scope のバルーンが別 scope の系列由来の定義で駆動されない形を型で担保する。各表は
    /// 当該 scope の `EmoWorld` から `from_world` で導出する（導出は構築ループ内の単一導出点）。
    ///
    /// キー集合は `BootAssets.balloons` の scope 集合と一致する（scope ごとに 1 表ずつ導出するため）。
    /// 表は `spawn_seriko` が attach より前に消費し `BalloonScopeAssets` は attach で move 消費される
    /// ため、表を資産へ同梱せず本並行構造で持つ。反復順の決定論のため `BTreeMap`。
    pub balloon: BTreeMap<u32, AnimationTable>,
}

/// バルーン表の scope キーを seriko のアクタ鍵語彙へ変換する（起動シームの転送）。
///
/// boot 側の `u32` scope を `SerikoLoopConfig.balloon_tables` が要求する [`ActorKey`] へ
/// `ActorKey::from(scope.to_string())` で写す。この写像は attach（`frame/attach.rs`）・文字層の再追従・
/// `target_map::scope_of` の逆写像と**同一語彙**であり、ここで別語彙を作らないことが scope 同定の
/// 一貫性を保つ要点である。表そのものは値移送（内容は変えない）。
///
/// 純粋関数（状態・I/O なし）。`BTreeMap` 同士の変換ゆえ反復順は決定論（`ActorKey` の順序は
/// 文字列順のため、数値順とは一致しないことがある——キー集合の同一性のみが意味を持つ）。
pub fn actor_keyed_balloon_tables(
    tables: BTreeMap<u32, AnimationTable>,
) -> BTreeMap<ActorKey, AnimationTable> {
    tables
        .into_iter()
        .map(|(scope, table)| (ActorKey::from(scope.to_string()), table))
        .collect()
}

/// 表示結線に必要な load-time 資産の一括。返る資産だけで attach フェーズが完結する（以後ファイル I/O なし）。
pub struct BootAssets {
    /// scope ごとのシェル表示資産（`GhostWindows` の scope 集合に対応）。
    pub shells: Vec<ScopeAssets>,
    /// scope ごとのバルーン表示資産（面 0 初期表示・World／アトラス／scope 別定義の 3 点組）。
    ///
    /// この scope 集合は `shells` の scope 集合と一致する。
    pub balloons: Vec<BalloonScopeAssets>,
    /// `Emote{key}` → surface 解決器（`EmoWorld::alias_snapshot()` 由来）。
    pub resolver: SurfaceResolver,
    /// スコープごとの起動時オンの静的 bind 集合（シェルの `*.bindgroup{N}.default,1`）。
    pub static_binds: DefaultBinds,
    /// bind 名前解決情報（`MountModel.bindgroups` の名前宣言由来・`(カテゴリ, パーツ)`→着せ替え ID）。
    /// `wire_runtime` が `spawn_seriko` の actor 構築へ手渡す（bind cue の着せ替え名の解決）。
    pub bind_resolver: BindResolver,
    /// SERIKO ループ表（シェル面 1 表＋バルーン面の scope 別表・面種非依存）。
    /// 既に保持する `EmoWorld` スナップショットから `AnimationTable::from_world` で構築する
    /// （新規ファイル I/O なし）。`spawn_seriko` の actor 構築へ手渡す（起動シームが
    /// [`actor_keyed_balloon_tables`] で scope キーをアクタ鍵語彙へ写して転送する）。
    pub loop_tables: LoopTables,
    /// シェル面の作者基準 DPI（ukadoc shell descript `seriko.dpi`・既定 96）。
    ///
    /// 表示スケール k＝窓の実モニタ DPI ÷ 作者基準 DPI の**分母**。attach 相が
    /// `attach_target(.., author_dpi)` へそのまま供給する（搬送のみ・値の解釈はしない）。
    pub shell_author_dpi: u16,
    /// バルーン面の作者基準 DPI（ukadoc balloon descript `dpi`・既定 96）。
    ///
    /// シェルとは別パッケージ・別キーゆえ独立に保持する（両者が異なる宣言値を持ち得る）。
    pub balloon_author_dpi: u16,
    /// 箱の束（装着の相が文字の層へ渡す）。
    pub boxes: ShellBoxAssets,
    /// シェルのカーソル（装着の相が箱の束の隣で NonSend へ入れる）。
    pub cursors: ShellCursorAssets,
}

/// 構築入力（[`BootAssets`]）を一括組立する。
///
/// 既定の解決のシェルを読む形で、`shell: None` で [`build_boot_assets_with_shell`] へ委ねるだけである
/// （シェルの側は [`build_shell_assets`]、バルーンの側は [`build_balloon_assets`]）。
///
/// 組立経路は `resolve_with_shell`（shell dir・名前なしは既定の解決）→ シェル読み込みの権威
/// `load_shell_target` を **1 回**（フォルダの一覧・`surfaces.txt` の読取と解析・面の画像の決定・bake をすべて権威が行う）→
/// scope ごとに `ShellTarget::take_world`（`EmoWorld` は非 Clone・装着で move 消費ゆえ scope 数だけ
/// 受け取る。`AtlasTable` は安価 Clone）。
/// balloon は scope ごとに 系列解決（`resolve_balloon_faces`）→ 構築
/// （`build_balloon_target_from_faces`）→ 定義読込（`load_scope_balloon_definitions`）を行い
/// [`BalloonScopeAssets`] へ束ねる（scope 専用の定義を保持する＝共有 1 本を作らない）。
/// `SurfaceResolver` は `EmoWorld::alias_snapshot()` から、static bindset は shell descript KV の
/// シェルの着せ替えの読み（`MountModel.bindgroups`）からスコープごとに組む。
///
/// 作者基準 DPI（`shell_author_dpi`／`balloon_author_dpi`）は
/// **呼び手が読んだ値をそのまま搬送する**（本関数は解釈も再読取もしない）。boot シームが
/// `DescriptSource::shell_author_dpi()`／`load_balloon_author_dpi()`（areka_placement::source）で **1 度だけ**読み、
/// 同じ値を採寸の k₀（`MeasureScaling`）と attach（`attach_target`）の双方へ配ることで、両者が同一の分母に載る。
/// ここで descript を読み直すと (a) 二重 I/O になり、(b) 採寸と attach が別々の読取結果に載る隙
/// （読取間の差し替え）を作るため、内部読取は採らない。縮退梯子（無宣言=96／不正・0=warn+96）は
/// `areka_placement::source::parse_author_dpi` が適用済みで、本関数へ届く時点で常に有効な非ゼロ DPI である。
///
/// # 事前条件
/// - 呼び出しスレッドは COM 初期化済み（`WicDecoderArm` 前提・本番は MTA UI スレッド）。
/// - `scopes` は呼び手（`wire_runtime` の `derive_scopes`）が渡す固定の最大の組 `[0, 1]`（ゴーストは読まない・
///   scope 1 が無いゴーストでは `frame::plan_attachments` が unused に分ける）。
/// - `shell_author_dpi`／`balloon_author_dpi` は縮退梯子適用済みの非ゼロ値（`DescriptSource` の読取器の出力）。
///
/// # 事後条件
/// - 返る資産だけで attach フェーズが完結する（**以後ファイル I/O なし**）。全 I/O は本関数内で完結。
///
/// # 失敗（panic しない）
/// - `resolve_with_shell` 失敗 → [`BootWiringError::Mount`]（`StartPointMissing` 系は呼び手が warn 分類）。
/// - WIC デコーダ生成失敗 → [`BootWiringError::Decoder`]。
/// - シェルのフォルダの一覧失敗／`surfaces.txt` が在るのに読めない／`descript.txt` 読取失敗
///   → [`BootWiringError::ShellRead`]（`ShellLoadError` からの写し替え）。
/// - シェルに面が 1 つも無い（`surfaces.txt` が無いか面を定義せず、面の画像も無い）
///   → [`BootWiringError::ShellEmpty`]。
/// - バルーン系列解決／target 構築失敗（走査失敗・面 0 不在・bake 脱落）
///   → [`BootWiringError::Balloon`]（`#[from] BalloonLoadError`・真因ログは権威側が既に出す）。
#[cfg(any(test, feature = "test-support"))] // 本番の起動はシェル名つきの兄弟を通る（呼び手はテスト）
pub fn build_boot_assets(
    ghost_root: &Path,
    balloon_root: &Path,
    scopes: &[u32],
    shell_author_dpi: u16,
    balloon_author_dpi: u16,
) -> Result<BootAssets, BootWiringError> {
    build_boot_assets_with_shell(
        ghost_root,
        balloon_root,
        scopes,
        shell_author_dpi,
        balloon_author_dpi,
        None,
        &DefaultSurfaces::builtin(),
    )
}

/// [`build_boot_assets`] のシェル名つきの兄弟。
///
/// `shell` が `Some(名)` なら `shell/<名>/` のシェルを、`None` なら既定の解決
/// （`seriko.defaultsurfacedirectoryname`、無ければ `master`）のシェルを読む。中身は
/// [`build_shell_assets`] と [`build_balloon_assets`] を続けて呼んで 1 つに束ねるだけである。
/// 名前は検査しない（`resolve_with_shell` の契約どおり・目録に在る名前だけを渡すのは呼び手の責務）。
/// 事前条件・事後条件・失敗は [`build_boot_assets`] と同じ。
pub fn build_boot_assets_with_shell(
    ghost_root: &Path,
    balloon_root: &Path,
    scopes: &[u32],
    shell_author_dpi: u16,
    balloon_author_dpi: u16,
    shell: Option<&str>,
    default_surfaces: &DefaultSurfaces,
) -> Result<BootAssets, BootWiringError> {
    // COM 初期化済みスレッドが前提。
    let decoder = WicDecoderArm::new().map_err(BootWiringError::Decoder)?;
    let shell_assets = build_shell_assets(
        ghost_root,
        shell,
        scopes,
        shell_author_dpi,
        default_surfaces,
        &decoder,
    )?;
    let balloon_assets = build_balloon_assets(balloon_root, scopes, balloon_author_dpi, &decoder)?;
    Ok(BootAssets {
        shells: shell_assets.shells,
        balloons: balloon_assets.balloons,
        resolver: shell_assets.resolver,
        static_binds: shell_assets.static_binds,
        bind_resolver: shell_assets.bind_resolver,
        loop_tables: LoopTables {
            shell: shell_assets.loop_table,
            balloon: balloon_assets.loop_tables,
        },
        shell_author_dpi: shell_assets.author_dpi,
        balloon_author_dpi: balloon_assets.author_dpi,
        boxes: shell_assets.boxes,
        cursors: shell_assets.cursors,
    })
}

/// シェルの側だけの資産（[`build_shell_assets`] の戻り値・[`BootAssets`] のシェルの欄の一括）。
///
/// シェルを差し替えるときは、これだけを作り直す（バルーンは元のまま）。
pub struct ShellAssets {
    /// scope ごとのシェル表示資産。
    pub shells: Vec<ScopeAssets>,
    /// `Emote{key}` → surface 解決器（先頭 World の `alias_snapshot()` 由来）。
    pub resolver: SurfaceResolver,
    /// スコープごとの起動時オンの静的 bind 集合（シェルの `*.bindgroup{N}.default,1`）。
    pub static_binds: DefaultBinds,
    /// bind 名前解決情報（`MountModel.bindgroups` 由来）。
    pub bind_resolver: BindResolver,
    /// シェル面のループ表（先頭 World から・scope 資産不在なら空表）。
    pub loop_table: AnimationTable,
    /// シェル面の作者基準 DPI（呼び手の値の素通し）。
    pub author_dpi: u16,
    /// 焼く段で落ちた絵の理由（`ShellTarget::bake_errors` の写し・記録は読み込みの権威が出し済み）。
    ///
    /// 起動（[`build_boot_assets_with_shell`]）は読まずに今日どおり読み飛ばして続ける。切替は
    /// 空でなければ差し替えの前の失敗にする。
    pub bake_failures: Vec<String>,
    /// 箱の束（置き場所の表・別名の写し・面の表の番号・フォントを探す場所の順）。
    pub boxes: ShellBoxAssets,
    /// シェルのカーソル（cursorブレスとゴーストの descript の差し替え・読んだファイル）。
    pub cursors: ShellCursorAssets,
}

/// バルーンの側だけの資産（[`build_balloon_assets`] の戻り値・[`BootAssets`] のバルーンの欄の一括）。
///
/// バルーンを差し替えるときは、これだけを作り直す（シェルは元のまま）。
pub struct BalloonAssets {
    /// scope ごとのバルーン表示資産。
    pub balloons: Vec<BalloonScopeAssets>,
    /// バルーン面の scope 別ループ表（キー集合＝`balloons` の scope 集合）。
    pub loop_tables: BTreeMap<u32, AnimationTable>,
    /// バルーン面の作者基準 DPI（呼び手の値の素通し）。
    pub author_dpi: u16,
}

/// シェルの側だけの資産を作る（[`build_boot_assets`] のシェルの半分）。
///
/// `shell` が `Some(名)` なら `shell/<名>/`、`None` なら既定の解決のシェルを読む
/// （`resolve_with_shell`）。失敗の種類は [`build_boot_assets`] のシェルの側と同じ。
pub fn build_shell_assets(
    ghost_root: &Path,
    shell: Option<&str>,
    scopes: &[u32],
    author_dpi: u16,
    default_surfaces: &DefaultSurfaces,
    decoder: &impl ElementDecoder,
) -> Result<ShellAssets, BootWiringError> {
    build_shell_assets_with_limits(
        ghost_root,
        shell,
        scopes,
        author_dpi,
        default_surfaces,
        decoder,
        AnimationLimits::from_env(),
    )
}

/// [`build_shell_assets`] の動く絵の上限を外から渡す版（環境変数に左右されないテストのため）。
pub fn build_shell_assets_with_limits(
    ghost_root: &Path,
    shell: Option<&str>,
    scopes: &[u32],
    author_dpi: u16,
    default_surfaces: &DefaultSurfaces,
    decoder: &impl ElementDecoder,
    limits: AnimationLimits,
) -> Result<ShellAssets, BootWiringError> {
    // 起点は ghost/master/descript.txt（placement source と同経路）。
    let model = resolve_with_shell(ghost_root, DefaultEncoding::Ansi, shell)
        .map_err(BootWiringError::Mount)?;

    let bind_resolver = BindResolver::from_groups(&model.bindgroups);

    let shell_dir = model.shell.dir;

    // 着せ替えの読みは descript.txt が無くても空で返るので、読めないシェルの起動の失敗は焼く前にここで残す。
    let descript_path = shell_dir.join(DESCRIPT_TXT);
    if let Err(source) = std::fs::read(&descript_path) {
        return Err(BootWiringError::ShellRead {
            path: descript_path,
            source,
        });
    }

    // 読み込みの権威を **1 回**呼ぶ。文字コードの扱い・記録・失敗の種類はすべて権威が持つ。
    // 採寸（`areka_placement::measure`）も同じ入口を通るので、「番号 → 面の画像」の対応・土台の絵・
    // 透過の扱いが表示と採寸で食い違わない。
    let mut target = load_shell_target_with_limits(&shell_dir, decoder, limits)?;
    let atlas = target.atlas().clone();
    let bake_failures = target.bake_errors().iter().map(|e| e.to_string()).collect();

    // `take_world` の 1 つ目は読み込みで組んだ面の表を受け取り、2 つ目から組み直す。EmoWorld は非 Clone・
    // 装着で move 消費ゆえ scope 数だけ要る。同じ `ShellTarget` から作るので内容は毎回同じで、resolver 用の
    // alias スナップショットは scope 非依存ゆえ最初に組んだ World から一度だけ採る。
    let mut shells = Vec::with_capacity(scopes.len());
    let mut resolver_snapshot: Option<BTreeMap<String, Vec<u32>>> = None;
    let mut surface_ids = BTreeSet::new();
    for &scope in scopes {
        let emo_world = target.take_world();
        if resolver_snapshot.is_none() {
            resolver_snapshot = Some(emo_world.alias_snapshot());
            surface_ids = emo_world.surface_ids().collect();
        }
        let initial_surface_id = default_surfaces.surface(scope as usize);
        shells.push(ScopeAssets {
            scope,
            emo_world,
            atlas: atlas.clone(),
            initial_surface_id,
        });
    }
    // scopes が空でも resolver は必ず構築する（空 alias 表＝解決なし）。
    let aliases = resolver_snapshot.unwrap_or_default();
    // seriko と文字の層が同じ解決を使うよう、同じ別名の写しと面の表の番号から組む。
    let boxes = ShellBoxAssets {
        layout: target.boxes().clone(),
        aliases: aliases.clone(),
        surface_ids,
        font_dirs: box_font_search_dirs(&shell_dir, &model.shiori.dir),
        fallback_candidates: scopes
            .iter()
            .map(|&scope| {
                let candidates = default_surfaces.candidates(scope as usize);
                (ActorKey::from(scope.to_string()), candidates)
            })
            .collect(),
        // 箱の矢印の絵は面の絵と同じ透過の宣言で読む。
        markers: resolve_box_markers(
            target.boxes(),
            scopes,
            &shell_dir,
            decoder,
            AlphaParams {
                use_self_alpha: target.use_self_alpha(),
            },
        ),
    };
    let resolver = SurfaceResolver::new(aliases);
    // カーソルはシェルのフォルダ → `ghost\master` の順でファイルを探す。
    let ghost_kv = read_kv_lenient(&model.shiori.dir.join(DESCRIPT_TXT));
    let cursors = build_shell_cursors(
        target.cursor_braces(),
        &ghost_kv,
        &shell_dir,
        &model.shiori.dir,
    );

    // 既定の組はスコープ 0 に `sakura.*`・1 に `kero.*`・2 以上に `char*.*`。
    let static_binds = DefaultBinds::from_groups(&model.bindgroups);

    // 全 scope が同一 `Shell` から build 済みで内容は scope 非依存なので、先頭 World から 1 度だけ組めば足りる
    // （**シェル面に限る**）。scope 資産が不在（空 scopes 等）なら空表。
    let loop_table = shells
        .first()
        .map(|scope_assets| AnimationTable::from_world(&scope_assets.emo_world))
        .unwrap_or_else(AnimationTable::empty);

    Ok(ShellAssets {
        shells,
        resolver,
        static_binds,
        bind_resolver,
        loop_table,
        author_dpi,
        bake_failures,
        boxes,
        cursors,
    })
}

/// バルーンの側だけの資産を作る（[`build_boot_assets`] のバルーンの半分）。
///
/// 失敗の種類は [`build_boot_assets`] のバルーンの側と同じ（[`BootWiringError::Balloon`]）。
pub fn build_balloon_assets(
    balloon_root: &Path,
    scopes: &[u32],
    author_dpi: u16,
    decoder: &impl ElementDecoder,
) -> Result<BalloonAssets, BootWiringError> {
    build_balloon_assets_with_limits(
        balloon_root,
        scopes,
        author_dpi,
        decoder,
        AnimationLimits::from_env(),
    )
}

/// [`build_balloon_assets`] の動く絵の上限を外から渡す版（環境変数に左右されないテストのため）。
pub fn build_balloon_assets_with_limits(
    balloon_root: &Path,
    scopes: &[u32],
    author_dpi: u16,
    decoder: &impl ElementDecoder,
    limits: AnimationLimits,
) -> Result<BalloonAssets, BootWiringError> {
    // 系列解決（`resolve_balloon_faces`）は scope あたり 1 回だけ呼び、その戻りを構築と定義読込の双方へ
    // 使い回す——公開ラッパ `build_balloon_target` を使うとディレクトリ列挙が scope あたり 2 回走るため、
    // 解決済み面列を受ける版を直接呼ぶ。2 層マージ規則・層別ログレベル・確定値の info! は権威クレート
    // （`areka-emo-present` の `balloon`）が持ち、ここは呼ぶだけである。
    // バルーン World は scope ごとに別系列から build されるので、ループ表も scope ごとにこのループ内で
    // 組む（シェル面のような「先頭 World から 1 度だけ」は成り立たない）。表を `BalloonScopeAssets` へ
    // 同梱しないのは、表を spawn_seriko が attach より前に消費するのに対し資産は attach で move 消費される
    // ためで、並行構造の `LoopTables.balloon` 側で保持する。
    let mut balloons = Vec::with_capacity(scopes.len());
    let mut balloon_tables: BTreeMap<u32, AnimationTable> = BTreeMap::new();
    // 普通のバルーンの名前＝バルーンのフォルダ名（名前の無い根なら道のりそのもの）。
    let name = balloon_root.file_name().map_or_else(
        || balloon_root.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    // 透過の宣言はバルーン 1 つにつき 1 回だけ読む（記録をスコープの数だけ出さない）。
    let use_self_alpha = load_balloon_use_self_alpha(balloon_root);
    for &scope in scopes {
        let faces = resolve_balloon_faces(balloon_root, scope)?;
        let (emo_world, atlas) = build_balloon_target_from_faces_with_limits(
            balloon_root,
            decoder,
            &faces,
            use_self_alpha,
            limits,
        )?;
        // 面 0 必在は `resolve_balloon_faces` が施行済みなので先頭は必ず存在する。万一の不在は
        // 権威の契約違反であり、真因を残して構築失敗に畳む（定義なしのバルーンを黙って組まない）。
        let Some(face0) = faces.first() else {
            error!(
                balloon_root = %balloon_root.display(),
                scope,
                "assets: 解決済みバルーン面列が空（resolve_balloon_faces の面 0 必在契約違反）"
            );
            return Err(BootWiringError::Balloon(BalloonLoadError::NoFace0 {
                path: balloon_root.to_path_buf(),
                scope,
            }));
        };
        // 警告はスコープにつき 1 回。
        let definitions = load_scope_balloon_definitions(
            balloon_root,
            scope,
            &faces,
            decoder,
            AlphaParams { use_self_alpha },
        );
        // 無効表示の混色の相手＝面 0 の原点画素。採れないときは白＋`debug!`。
        let background_color = balloon_background::face_origin_color(&atlas, &face0.file_name);
        // World を資産へ move する前に導出する。
        balloon_tables.insert(scope, AnimationTable::from_world(&emo_world));
        balloons.push(BalloonScopeAssets {
            scope,
            emo_world,
            atlas,
            definitions,
            background_color,
            name: name.clone(),
        });
    }

    Ok(BalloonAssets {
        balloons,
        loop_tables: balloon_tables,
        author_dpi,
    })
}

/// 空の定義の面 0 だけの束（ファイルを読まないテストの支えが組む資産用）。
#[cfg(any(test, feature = "test-support"))]
pub fn empty_definitions(scope: u32) -> ScopeBalloonDefinitions {
    ScopeBalloonDefinitions::single(
        scope,
        areka_emo_present::balloon_definitions::FaceDefinition {
            model: areka_parsers::balloon::parse_str("", None),
            arrows: Default::default(),
        },
    )
}

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "assets_shell_tests.rs"]
mod shell_tests;

#[cfg(test)]
#[path = "assets_markers_tests.rs"]
mod markers_tests;
