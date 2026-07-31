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

/// SERIKO ループ表の一括（シェル面・バルーン面の 2 表・design「結線・資産・実機経路（assets.rs）」）。
///
/// 面種非依存（裁定 (a)）: 同一 `AnimationTable::from_world` がシェル世界・バルーン世界の双方から
/// 同型に表を組む。`shell`／`balloon` は **surface ID 名前空間の別**（emo2 はシェル surface0 と
/// バルーン面 0 が別物）であり能力の仕切りではない。両表は同一評価経路で駆動される。
///
/// いずれの表も既に `BootAssets` が保持する fold 完了 `EmoWorld` スナップショットから構築するため、
/// 表の構築に **新規のファイル I/O は要らない**（「以後ファイル I/O なし」の事後条件は不変）。
pub struct LoopTables {
    /// シェル面のループ表（`shells[0].emo_world` から `from_world`・scope 資産不在なら空表）。
    pub shell: AnimationTable,
    /// バルーン面のループ表（先頭バルーン `EmoWorld` から `from_world`・資産不在なら空表）。
    ///
    /// バルーン World は scope ごとに別系列から build されるため、この単数形は先頭 scope の表しか
    /// 表せない暫定形である。scope キーの写像（`BTreeMap<u32, AnimationTable>`）への移行は
    /// tasks.md task 3.2 が担う。
    pub balloon: AnimationTable,
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
    /// SERIKO ループ表（シェル面・バルーン面の 2 表・面種非依存＝裁定 (a)）。
    /// 既に保持する `EmoWorld` スナップショットから `AnimationTable::from_world` で構築する
    /// （新規ファイル I/O なし）。`spawn_seriko` の actor 構築へ手渡す（本タスクは起動時資産への保持のみ）。
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
    let mut balloons = Vec::with_capacity(scopes.len());
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
        balloons.push(BalloonScopeAssets {
            scope,
            emo_world,
            atlas,
            model,
        });
    }

    // SERIKO ループ表: 既に build 済みの EmoWorld スナップショット（shells[0]／balloons[0]）から
    // `AnimationTable::from_world` で構築する（面種非依存＝裁定 (a)・**新規ファイル I/O なし**）。
    // scope 資産が不在（空 scopes 等）なら空表を明示（AnimationTable::empty()）。
    //
    // シェル面: 全 scope が同一 `Shell` から build 済みゆえ内容は scope 非依存＝先頭 World から
    // 1 度だけ組めば足りる（この前提が成り立つのは **シェル面に限る**）。
    // バルーン面: World は scope ごとに別系列から build されるため上の前提は成り立たない。
    // ここが先頭 World 1 本で済ませているのは `LoopTables.balloon` が単数のままである暫定形の
    // 帰結であり、scope キーの写像として構築ループ内で導出するのは tasks.md task 3.2 が担う。
    let loop_tables = LoopTables {
        shell: shells
            .first()
            .map(|scope_assets| AnimationTable::from_world(&scope_assets.emo_world))
            .unwrap_or_else(AnimationTable::empty),
        balloon: balloons
            .first()
            .map(|balloon_assets| AnimationTable::from_world(&balloon_assets.emo_world))
            .unwrap_or_else(AnimationTable::empty),
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
mod tests {
    use std::path::PathBuf;

    use areka_seriko::{BindNamespace, SurfaceTarget};
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    use super::*;
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
        let balloon_rebuilt = AnimationTable::from_world(&boot.balloons[0].emo_world);
        assert_eq!(
            boot.loop_tables.balloon.is_empty(),
            balloon_rebuilt.is_empty(),
            "balloon 表は最初のバルーン EmoWorld から from_world 済み（保持 World の再利用）"
        );
        assert_eq!(
            boot.loop_tables.balloon.animations(0).len(),
            balloon_rebuilt.animations(0).len(),
            "balloon 表は保持 World と同一エントリ（新規 I/O なし・面種非依存）"
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
    /// `is_mustselect(Sakura, "腕"/"口"/"眉"/"目") == true`、非宣言カテゴリ（紅）は `false`。
    /// mustselect カテゴリの ID 集合（目は複数 id）は非空であることを確認する（R4.5・R7.1・D11）。
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
            assert!(
                boot.bind_resolver
                    .is_mustselect(BindNamespace::Sakura, category),
                "宣言済み mustselect カテゴリ `{category}` は起動時資産から真（10.3）"
            );
        }
        // 非宣言カテゴリは mustselect でない（捏造しない・R4.5）。
        assert!(
            !boot.bind_resolver.is_mustselect(BindNamespace::Sakura, "紅"),
            "非宣言カテゴリ `紅` は mustselect でない（R4.5）"
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
}
