//! 構築入力（BootAssets）の組立と shell descript からの static bindset 抽出。
//!
//! `build_boot_assets`（shell: `surfaces.txt` 読取→`areka_parsers::shell::parse`→bake→scope ごとに
//! `EmoWorld::build`＋`bind_atlas`／balloon: scope ごとに `resolve_balloon_faces`→
//! `build_balloon_target_from_faces`＋`load_scope_balloon_model`＝[`BalloonScopeAssets`]／
//! `SurfaceResolver`＝`alias_snapshot()`／static bindset＝`default_bind_ids`→`build_static_bindset`）と
//! `default_bind_ids`（`sakura.bindgroup{N}.default==1` の N 抽出・DD-8・ukadoc 正典）を所有する。
//! 戻り値だけで以後ファイル I/O 不要にする（parse／bake は 1 回・`AtlasTable` は Clone 共有）。
//! 失敗は `BootWiringError`（`#[from]` 変換群）で観測可能化する。
//!
//! `default_bind_ids` は tasks.md task 2.3 で実装済み。`build_boot_assets` の骨格は残り、
//! 実装は tasks.md task 2.6 が担う。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, PackConfig, SetId, SurfaceSet, UseSelfAlpha, WicDecoderArm, bake,
};
use areka_emo_compose::{BindSet, ComposeError, EmoWorld};
use areka_emo_present::PresentError;
use areka_emo_present::balloon::{
    build_balloon_target_from_faces, load_scope_balloon_model, resolve_balloon_faces,
};
use areka_parsers::balloon::BalloonModel;
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_parsers::kv::parse_kv;
use areka_parsers::package::resolve;
use areka_sakura::ActorKey;
use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver, build_static_bindset};
use tracing::{error, warn};

use super::BootWiringError;

/// shell descript KV から `default==1` の bindgroup id を抽出する純関数（DD-8・ukadoc 正典）。
///
/// `sakura.bindgroup{N}.default` 形（`N` は `u32`）のキーで、値を trim した結果が `"1"` の
/// エントリの `N` を集める。抽出は sakura scope 限定であり、`kero.*` scope の bindgroup default
/// は本タスクの対象外（M-dual 増分・design.md Non-Goals「kero scope の bindgroup default 分離」）。
///
/// 除外条件:
/// - 値（trim 後）が `"1"` でないもの（`"0"`／`"2"`／空／`"10"` 等）。
/// - `kero.*` 等 `sakura.bindgroup` 以外の prefix を持つキー。
/// - `.name`／`.group` 等 `.default` 以外の suffix を持つキーや、その他の無関係キー。
/// - 中間 `N` が `u32` として parse できないキー（`XYZ`／空／負値／数字混在）。
///
/// prefix/suffix は厳密一致で判定する（`strip_prefix`/`strip_suffix`）。パターンを部分文字列
/// として含むだけのキー（例 `xsakura.bindgroup1.default`／`sakura.bindgroup1.defaultx`）は
/// match せず、中間を `u32` として厳密 parse することで数値部の false-positive も防ぐ。
///
/// 純粋関数（状態・I/O なし）。戻り値は決定論のため数値昇順にソートし重複を除去する
/// （`BTreeMap` のキー反復は lexicographic 順のため numeric 昇順とは一致しない）。
pub fn default_bind_ids(shell_kv: &BTreeMap<String, String>) -> Vec<u32> {
    /// sakura scope の bindgroup キー prefix（厳密一致）。
    const PREFIX: &str = "sakura.bindgroup";
    /// bindgroup default キー suffix（厳密一致）。
    const SUFFIX: &str = ".default";

    let mut ids: Vec<u32> = shell_kv
        .iter()
        .filter_map(|(key, value)| {
            // 値（trim 後）が "1" のエントリだけを対象にする。
            if value.trim() != "1" {
                return None;
            }
            // `sakura.bindgroup` <N> `.default` を厳密に剥がし、中間を u32 として parse する。
            key.strip_prefix(PREFIX)
                .and_then(|rest| rest.strip_suffix(SUFFIX))
                .and_then(|mid| mid.parse::<u32>().ok())
        })
        .collect();

    // 決定論: numeric 昇順ソート＋重複除去（キー反復順に依存しない安定した戻り値）。
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// shell 定義ファイル名（surface ツリー・`shell/<dir>` 配下）。
const SURFACES_TXT: &str = "surfaces.txt";
/// descript 定義ファイル名（shell の static bindset 抽出に読む）。
const DESCRIPT_TXT: &str = "descript.txt";
/// scope>=1 の初期表示 surface id（DD-9・ukadoc 相方既定サーフェス＝10・placement measure と同値）。
const KERO_INITIAL_SURFACE_ID: u32 = 10;

/// 1 scope 分のシェル表示資産（`attach_target` へ手渡す 1 組）。
///
/// `emo_world` は scope 専用に `EmoWorld::build` した非 Clone World（装着で move 消費）。
/// `atlas` は parse/bake を 1 回で済ませた共有アトラス（`AtlasTable` は内部 Arc の安価 Clone）。
pub struct ScopeAssets {
    /// この資産が対応する scope 番号。
    pub scope: u32,
    /// scope 専用 build 済みの表示 World（`bind_atlas(SetId(0))` 済み・装着で move 消費）。
    pub emo_world: EmoWorld,
    /// 共有アトラス（parse/bake 1 回・Clone 共有）。
    pub atlas: AtlasTable,
    /// 初期表示 surface id（scope0=0／scope>=1=10・DD-9）。
    pub initial_surface_id: u32,
}

/// 1 scope 分のバルーン表示資産（[`ScopeAssets`] と対称・design D4=S1）。
///
/// scope・表示 World・アトラス・**その scope 専用の**マージ済み定義を 1 件に束ねる。
/// 「全 scope 共有のバルーン定義 1 本」を型として表現できなくすることが本保持器の要点であり
/// （旧 `BootAssets.balloon_model` の撤去・Req 2.1）、ある scope のバルーンが別 scope の系列
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
    /// scope 別 2 層マージ済み定義（文字層・`windowposition`／`validrect` の源・Req 2.1）。
    pub model: BalloonModel,
}

/// SERIKO ループ表の一括（シェル面 1 表＋バルーン面の scope 別表・design「結線・資産・実機経路
/// （assets.rs）」）。
///
/// 面種非依存（裁定 (a)）: 同一 `AnimationTable::from_world` がシェル世界・バルーン世界の双方から
/// 同型に表を組む。`shell`／`balloon` は **surface ID 名前空間の別**（emo2 はシェル surface0 と
/// バルーン面 0 が別物）であり能力の仕切りではない。いずれの表も同一評価経路で駆動される。
///
/// いずれの表も既に `BootAssets` が保持する fold 完了 `EmoWorld` スナップショットから構築するため、
/// 表の構築に **新規のファイル I/O は要らない**（「以後ファイル I/O なし」の事後条件は不変）。
pub struct LoopTables {
    /// シェル面のループ表（`shells[0].emo_world` から `from_world`・scope 資産不在なら空表）。
    ///
    /// 単数なのは **シェル面に限る**前提の帰結である: 全 scope が同一 `Shell` から build されるため
    /// 表の内容が scope 非依存になる。バルーン面にこの前提は無い（下記 `balloon` 参照）。
    pub shell: AnimationTable,
    /// バルーン面の **scope 別**ループ表（キー＝scope 番号・design D9'・Req 5.6）。
    ///
    /// バルーン World は scope ごとに解決される系列（`balloons*`／`balloonk*` 等）が異なるため、
    /// ある scope のバルーンが別 scope の系列由来の定義で駆動されない形を型で担保する。各表は
    /// 当該 scope の `EmoWorld` から `from_world` で導出する（導出は構築ループ内の単一導出点）。
    ///
    /// キー集合は `BootAssets.balloons` の scope 集合と一致する（Data Models 不変条件 (c)）。
    /// 表は `spawn_seriko` が attach より前に消費し `BalloonScopeAssets` は attach で move 消費される
    /// ため、表を資産へ同梱せず本並行構造で持つ（D9' 裁定）。反復順の決定論のため `BTreeMap`。
    pub balloon: BTreeMap<u32, AnimationTable>,
}

/// バルーン表の scope キーを seriko のアクタ鍵語彙へ変換する（起動シームの転送・Req 5.6）。
///
/// boot 側の `u32` scope を `SerikoLoopConfig.balloon_tables` が要求する [`ActorKey`] へ
/// `ActorKey::from(scope.to_string())` で写す。この写像は attach（`frame.rs`）・文字層の再追従・
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

/// 表示結線に必要な load-time 資産の一括（design「構築入力 / assets」Service Interface）。
///
/// 事後条件: 返る資産だけで attach フェーズが完結する（以後ファイル I/O なし）。
pub struct BootAssets {
    /// scope ごとのシェル表示資産（`GhostWindows` の scope 集合に対応）。
    pub shells: Vec<ScopeAssets>,
    /// scope ごとのバルーン表示資産（面 0 初期表示・World／アトラス／scope 別定義の 3 点組）。
    ///
    /// 不変条件 (a): この scope 集合は `shells` の scope 集合と一致する（DD-12 の対応関係）。
    pub balloons: Vec<BalloonScopeAssets>,
    /// `Emote{key}` → surface 解決器（`EmoWorld::alias_snapshot()` 由来）。
    pub resolver: SurfaceResolver,
    /// 起動時オンの静的 bind 集合（shell descript `sakura.bindgroup{N}.default==1`・DD-8）。
    pub static_binds: BindSet,
    /// bind 名前解決情報（`MountModel.bindgroups` の名前宣言由来・`(カテゴリ, パーツ)`→着せ替え ID）。
    /// task 7.2 で `spawn_seriko` の actor 構築へ手渡す（本タスクは起動時資産への保持のみ）。
    pub bind_resolver: BindResolver,
    /// SERIKO ループ表（シェル面 1 表＋バルーン面の scope 別表・面種非依存＝裁定 (a)）。
    /// 既に保持する `EmoWorld` スナップショットから `AnimationTable::from_world` で構築する
    /// （新規ファイル I/O なし）。`spawn_seriko` の actor 構築へ手渡す（起動シームが
    /// [`actor_keyed_balloon_tables`] で scope キーをアクタ鍵語彙へ写して転送する）。
    pub loop_tables: LoopTables,
    /// シェル面の作者基準 DPI（ukadoc shell descript `seriko.dpi`・既定 96・areka-P0-emo-dpi-scaling D1）。
    ///
    /// 表示スケール k＝窓の実モニタ DPI ÷ 作者基準 DPI の**分母**。attach 相が
    /// `attach_target(.., author_dpi)` へそのまま供給する（搬送のみ・値の解釈はしない）。
    pub shell_author_dpi: u16,
    /// バルーン面の作者基準 DPI（ukadoc balloon descript `dpi`・既定 96・同 D1）。
    ///
    /// シェルとは別パッケージ・別キーゆえ独立に保持する（両者が異なる宣言値を持ち得る）。
    pub balloon_author_dpi: u16,
}

/// 構築入力（[`BootAssets`]）を一括組立する（tasks.md task 2.6・design「構築入力 / assets」）。
///
/// 組立経路は donor（`examples/emo-present.rs`）と placement measure の実績どおり:
/// `resolve`（shell dir）→ `surfaces.txt` 読取 → `areka_parsers::shell::parse` → `bake`
/// （WIC decoder・`UseSelfAlpha::On`・`PackConfig::default()`）を **1 回**行い、scope ごとに
/// `EmoWorld::build`＋`bind_atlas(SetId(0))`（`EmoWorld` は非 Clone・`AtlasTable` は安価 Clone）。
/// balloon は scope ごとに 系列解決（`resolve_balloon_faces`）→ 構築
/// （`build_balloon_target_from_faces`）→ 定義読込（`load_scope_balloon_model`）を行い
/// [`BalloonScopeAssets`] へ束ねる（scope 専用の定義を保持する＝共有 1 本を作らない・Req 2.1）。
/// `SurfaceResolver` は `EmoWorld::alias_snapshot()` から、static bindset は shell descript KV の
/// `default_bind_ids`（DD-8・task 2.3）→ `build_static_bindset` で組む。
///
/// 作者基準 DPI（`shell_author_dpi`／`balloon_author_dpi`・areka-P0-emo-dpi-scaling task 4.1）は
/// **呼び手が読んだ値をそのまま搬送する**（本関数は解釈も再読取もしない）。設計 Flow 3 手順1 が
/// 定めるとおり、boot シームが `DescriptSource::shell_author_dpi()`／`load_balloon_author_dpi()`
/// （placement::source・task 2.1）で **1 度だけ**読み、同じ値を採寸の k₀（`MeasureScaling`）と
/// attach（`attach_target`）の双方へ配ることで、両者が同一の分母に載ることを保証する。
/// ここで descript を読み直すと (a) 既に読んだファイルの二重 I/O になり、(b) 採寸と attach が
/// 別々の読取結果に載る隙（読取間の差し替え）を作るため、内部読取は採らない。縮退梯子
/// （無宣言=96／不正・0=warn+96）は単一権威 `placement::source::parse_author_dpi` が既に適用済みで、
/// 本関数へ届く時点で常に有効な非ゼロ DPI である。
///
/// # 事前条件
/// - 呼び出しスレッドは COM 初期化済み（`WicDecoderArm` 前提・本番は MTA UI スレッド）。
/// - `scopes` は呼び手（`wire_emo2_boot`）が placement と同じ入力から自前導出する（DD-12）。
/// - `shell_author_dpi`／`balloon_author_dpi` は縮退梯子適用済みの非ゼロ値（task 2.1 の読取器出力）。
///
/// # 事後条件
/// - 返る資産だけで attach フェーズが完結する（**以後ファイル I/O なし**）。全 I/O は本関数内で完結。
///
/// # 失敗（log-first・panic しない・R7.3）
/// - `resolve` 失敗 → [`BootWiringError::Mount`]（`StartPointMissing` 系は呼び手が warn 分類）。
/// - WIC デコーダ生成失敗 → [`BootWiringError::Decoder`]。
/// - `surfaces.txt`／`descript.txt` 読取失敗 → [`BootWiringError::ShellRead`]。
/// - `surfaces.txt` が surface を産まない → [`BootWiringError::ShellEmpty`]。
/// - バルーン系列解決／target 構築失敗（走査失敗・面 0 不在・bake 脱落）
///   → [`BootWiringError::Balloon`]（`#[from] PresentError`・真因ログは権威側が既に出す）。
pub fn build_boot_assets(
    ghost_root: &Path,
    balloon_root: &Path,
    scopes: &[u32],
    shell_author_dpi: u16,
    balloon_author_dpi: u16,
) -> Result<BootAssets, BootWiringError> {
    // 実 WIC デコーダ（COM 初期化済みスレッド前提・donor build_and_spawn／placement measure と同型）。
    let decoder = WicDecoderArm::new().map_err(BootWiringError::Decoder)?;

    // マウント解決で shell dir を得る（起点 ghost/master/descript.txt・placement source と同経路）。
    let model = resolve(ghost_root, DefaultEncoding::Ansi).map_err(BootWiringError::Mount)?;

    // bind 名前解決情報: `MountModel.bindgroups` の名前宣言（`(カテゴリ, パーツ)`→着せ替え ID）を
    // 起動時資産へ焼き込む（既存 default_bind_ids/static_binds 経路は無改変・R independence）。
    // sakura_names/kero_names は昇順転写ゆえ FORWARD 反復で insert すれば、parsers の
    // last-declaration-wins（後宣言優先）と一致する（BTreeMap::insert が後勝ちで上書き）。
    let mut sakura_map: BTreeMap<(String, String), u32> = BTreeMap::new();
    for name in &model.bindgroups.sakura_names {
        sakura_map.insert((name.category.clone(), name.part.clone()), name.id);
    }
    let mut kero_map: BTreeMap<(String, String), u32> = BTreeMap::new();
    for name in &model.bindgroups.kero_names {
        kero_map.insert((name.category.clone(), name.part.clone()), name.id);
    }
    // mustselect（排他選択）カテゴリ集合を `MountModel.bindgroups` の名前宣言
    // （`sakura/kero.bindoption*.group,カテゴリ,mustselect`・Req 4.5・D11）から起動時資産へ構築する。
    let sakura_mustselect: BTreeSet<String> =
        model.bindgroups.sakura_mustselect.iter().cloned().collect();
    let kero_mustselect: BTreeSet<String> =
        model.bindgroups.kero_mustselect.iter().cloned().collect();
    let bind_resolver = BindResolver::new(sakura_map, kero_map, sakura_mustselect, kero_mustselect);

    let shell_dir = model.shell.dir;

    // シェル: surfaces.txt 読取 → parse → bake を **1 回**（donor build_shell_target・placement measure 同経路）。
    let surfaces_path = shell_dir.join(SURFACES_TXT);
    let content = std::fs::read_to_string(&surfaces_path).map_err(|source| {
        BootWiringError::ShellRead {
            path: surfaces_path.clone(),
            source,
        }
    })?;
    let shell = areka_parsers::shell::parse(&content);
    if shell.surfaces.is_empty() {
        return Err(BootWiringError::ShellEmpty {
            path: surfaces_path,
        });
    }
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: &shell_dir,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &decoder, PackConfig::default());
    // emo2 shell は α 無し `purple/a/null.png` 1 枚が normalize seam として脱落する（既知・許容）。
    // donor／placement measure と同様 warn 継続（初期 surface の表示には無害の可能性）。
    for err in &baked.errors {
        warn!(error = %err, "assets: shell bake で脱落した element（既知の α 無し null.png 等・表示には無害の可能性）");
    }
    let atlas = baked.table;

    // scope ごとに FRESH な EmoWorld を build＋bind_atlas（EmoWorld は非 Clone・装着で move 消費ゆえ
    // scope 数だけ build。AtlasTable は Clone 共有）。resolver 用 alias スナップショットは scope 非依存
    // ゆえ最初に build した World から一度だけ採る。
    let mut shells = Vec::with_capacity(scopes.len());
    let mut resolver_snapshot: Option<BTreeMap<String, Vec<u32>>> = None;
    for &scope in scopes {
        let mut emo_world = EmoWorld::build(&shell);
        emo_world.bind_atlas(&atlas, SetId(0));
        if resolver_snapshot.is_none() {
            resolver_snapshot = Some(emo_world.alias_snapshot());
        }
        let initial_surface_id = if scope == 0 { 0 } else { KERO_INITIAL_SURFACE_ID };
        shells.push(ScopeAssets {
            scope,
            emo_world,
            atlas: atlas.clone(),
            initial_surface_id,
        });
    }
    // scopes が空でも resolver は必ず構築する（空 alias 表＝解決なし・degenerate 許容）。
    let resolver = SurfaceResolver::new(resolver_snapshot.unwrap_or_default());

    // static bindset: shell descript KV → default_bind_ids（DD-8・task 2.3）→ build_static_bindset。
    let descript_path = shell_dir.join(DESCRIPT_TXT);
    let shell_kv = match std::fs::read(&descript_path) {
        Ok(bytes) => parse_kv(&decode(&bytes, DefaultEncoding::Ansi)),
        Err(source) => {
            return Err(BootWiringError::ShellRead {
                path: descript_path,
                source,
            });
        }
    };
    let static_binds = build_static_bindset(&default_bind_ids(&shell_kv));

    // バルーン: scope ごとに 解決 → 構築 → 定義読込 を **同一箇所で**導出する（単一導出点）。
    // 系列解決（`resolve_balloon_faces`）は scope あたり 1 回だけ呼び、その戻りを構築
    // （`build_balloon_target_from_faces`）と定義読込（`load_scope_balloon_model`）の双方へ
    // 使い回す——公開ラッパ `build_balloon_target` を使うとディレクトリ列挙が scope あたり
    // 2 回走るため、解決済み面列を受ける版を直接呼ぶ。
    // 2 層マージ規則・層別ログレベル（D8）・確定値の info!（R6.3）はすべて権威クレート
    // （`areka-emo-present` の `balloon`）が持ち、本シームは呼ぶだけである（Req 2.1）。
    // SERIKO のバルーン側ループ表もこのループ内で **同一箇所から**導出する（単一導出点・D9'）。
    // シェル面と違いバルーン World は scope ごとに別系列から build されるため、「先頭 World から
    // 1 度だけ組めば足りる」という前提はバルーン面には無い（Req 5.6）。表は当該 scope の World から
    // `from_world` で組むだけで **新規ファイル I/O は起きない**。表を `BalloonScopeAssets` へ同梱
    // しないのは、表を spawn_seriko が attach より前に消費するのに対し資産は attach で move 消費
    // されるためで（消費タイミングが交差する）、並行構造の `LoopTables.balloon` 側で保持する。
    let mut balloons = Vec::with_capacity(scopes.len());
    let mut balloon_tables: BTreeMap<u32, AnimationTable> = BTreeMap::new();
    for &scope in scopes {
        let faces = resolve_balloon_faces(balloon_root, scope)?;
        let (emo_world, atlas) = build_balloon_target_from_faces(balloon_root, &decoder, &faces)?;
        // 面 0 必在（R1.7）は `resolve_balloon_faces` が権威として施行済みゆえ先頭は必ず存在する。
        // 万一の不在は権威の契約違反であり、log-first で真因を残して構築失敗に畳む（無言で
        // 定義なしのバルーンを組まない）。
        let Some(face0) = faces.first() else {
            error!(
                balloon_root = %balloon_root.display(),
                scope,
                "assets: 解決済みバルーン面列が空（resolve_balloon_faces の面 0 必在契約違反）"
            );
            return Err(BootWiringError::Balloon(PresentError::Compose(
                ComposeError::EmptyComposition(0),
            )));
        };
        let model = load_scope_balloon_model(balloon_root, scope, face0);
        // 表は World を資産へ move する前に、この scope の World から導出する（単一導出点）。
        balloon_tables.insert(scope, AnimationTable::from_world(&emo_world));
        balloons.push(BalloonScopeAssets {
            scope,
            emo_world,
            atlas,
            model,
        });
    }

    // SERIKO ループ表（面種非依存＝裁定 (a)・**新規ファイル I/O なし**＝保持済み World の再利用のみ）。
    //
    // シェル面: 全 scope が同一 `Shell` から build 済みゆえ内容は scope 非依存＝先頭 World から
    // 1 度だけ組めば足りる（この前提が成り立つのは **シェル面に限る**）。scope 資産が不在
    // （空 scopes 等）なら空表を明示（`AnimationTable::empty()`）。
    // バルーン面: 上の前提は成り立たない（系列が scope ごとに異なる）ため、表は上の構築ループ内で
    // scope ごとに導出済みである。ここではその写像をそのまま載せる（キー集合＝`balloons` の
    // scope 集合・不変条件 (c)・scope 資産不在なら空写像）。
    let loop_tables = LoopTables {
        shell: shells
            .first()
            .map(|scope_assets| AnimationTable::from_world(&scope_assets.emo_world))
            .unwrap_or_else(AnimationTable::empty),
        balloon: balloon_tables,
    };

    Ok(BootAssets {
        shells,
        balloons,
        resolver,
        static_binds,
        bind_resolver,
        loop_tables,
        // 作者基準 DPI は呼び手が読んだ値の素通し搬送（解釈・再読取・既定差し替えをしない）。
        shell_author_dpi,
        balloon_author_dpi,
    })
}

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
