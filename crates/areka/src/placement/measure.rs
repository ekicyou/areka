//! I/O 層: surface／バルーンの原寸採寸（task 4.1・DD5）。
//!
//! emo2-boot と同系の公開 API 経路（`areka_parsers::shell::parse` →
//! `areka_emo_atlas::bake` → `EmoWorld::build`＋`bind_atlas` →
//! `Composer::compose`・donor: `crates/areka/examples/emo-present.rs` の
//! `build_shell_target`／`build_balloon_assets`）で、各スコープの初期 surface と
//! **そのスコープが解決した**バルーン面 0 を **bind なし合成**し、原寸
//! （物理 px [`SizePx`]）を得る。
//!
//! - 初期 surface id: scope0 → `0`・scope1 → `10`（ukadoc 正典: 相方既定
//!   サーフェス）・scope n≥2 → `10`（正典既定なし・`warn!` 付き暫定）
//! - バルーン面 0 のファイル選択は系列解決権威
//!   [`areka_emo_present::balloon::resolve_balloon_faces`] の消費のみで行う。採寸側は
//!   接頭辞連鎖もディレクトリ走査も持たない（列挙が 2 実装に分かれると採寸窓寸と実際に
//!   合成される枠がずれる・design D2）
//! - 合成失敗したスコープは scope0 の寸法で代替し `warn!`（窓自体は生やす——
//!   寸法だけ暫定・design「Error Handling」）
//! - scope0 の採寸自体／いずれかのスコープのバルーン面 0 の採寸が成立しない場合は
//!   代替の根拠が無いため [`PlacementError::Measure`]（log-first・シームが
//!   フォールバック・DD14）
//! - 採寸に使ったアセット（`EmoWorld`／`AtlasTable`／WIC デコーダ）は本関数の
//!   ローカルとして採寸後に**破棄**される。戻り値は素の数値（[`ScopeInput`]）のみ
//!   （アセット所有・装着は emo2-boot の領分＝二重ロードは M1 受容トレードオフ）
//!
//! 呼び出しスレッドは COM 初期化済みであること（`WicDecoderArm` 前提・本番は
//! MTA UI スレッド）。wintf へは依存しない。

use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, PackConfig, SetId, SurfaceSet, UseSelfAlpha, WicDecoderArm, bake,
};
use areka_emo_compose::{BindSet, Composer, EmoWorld, PatternState, ScaleRatio};
use areka_emo_present::balloon::{
    ResolvedFace, build_balloon_target_from_faces, resolve_balloon_faces,
};
use tracing::{error, warn};

use super::PlacementError;
use super::resolver::{ScopeInput, SizePx};

/// scope1 以降の初期 surface id（ukadoc 正典: 相方既定サーフェス＝10）。
/// scope n≥2 には正典既定が無く、同値を `warn!` 付き暫定として用いる（design）。
const KERO_INITIAL_SURFACE_ID: u32 = 10;

/// 採寸結果（design「placement::measure」Service Interface）。
///
/// アセット（`EmoWorld`/`AtlasTable`）は持たない——素の物理 px 数値のみ
/// （採寸後破棄の契約を型で担保する）。
#[allow(dead_code)]
// scaffold（task 4.1）: main.rs シーム（task 6）が結線するまで非テストビルドでは未使用
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasuredSizes {
    /// スコープごとの採寸入力（`scope_ids` と同順・同長）。
    pub scopes: Vec<ScopeInput>,
}

/// 採寸時の表示スケール（boot が D7 で構築: primary モニタ DPI ÷ 各 author_dpi）。
///
/// シェル（キャラ surface）とバルーンは作者基準 DPI（`seriko.dpi`／`dpi`）が別々に
/// 宣言され得るため、k も別軸で持つ（要件 1.1・design「measure.rs（k₀ 適用・R7.8 席保全）」）。
/// `MeasureScaling::IDENTITY`（k=1/1）は既存の等倍採寸と構造的に同一の出力を返す
/// （`ScaleRatio::scale_len` が恒等時に入力を素通しするため・要件 7.2）。
///
/// scaffold: main.rs boot シーム（task 4.3）が構築するまで非テストビルドでは
/// 既定値（恒等）しか現れないため `dead_code` を許容する。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct MeasureScaling {
    /// キャラ surface へ適用する k（primary モニタ DPI ÷ shell author_dpi）。
    pub shell: ScaleRatio,
    /// バルーン surface へ適用する k（primary モニタ DPI ÷ balloon author_dpi）。
    pub balloon: ScaleRatio,
}

impl MeasureScaling {
    /// 恒等スケール（k=1/1・要件 7.2 の既存等倍経路）。
    ///
    /// 既存呼び手のシグネチャ追随（task 5）と既存 emo2 期待値（434×687 等）の
    /// 不変性検証に用いる。
    pub const IDENTITY: MeasureScaling = MeasureScaling {
        shell: ScaleRatio::ONE,
        balloon: ScaleRatio::ONE,
    };
}

impl Default for MeasureScaling {
    /// 既定は恒等（k 未確定時の縮退値）。
    fn default() -> Self {
        MeasureScaling::IDENTITY
    }
}

/// k 適用対象の寸法種別（失敗 reason の帰属表示・log フィールドに用いる）。
#[derive(Debug, Clone, Copy)]
enum SizeKind {
    /// キャラ surface 寸（`ScopeInput::char_size`）。
    Char,
    /// バルーン surface 寸（`ScopeInput::balloon_size`）。
    Balloon,
}

impl SizeKind {
    /// 既存 Measure エラーの reason 接頭辞。
    ///
    /// バルーン起因の失敗は `balloon:` を冠する既存流儀（本ファイルの
    /// `measure_balloon_surface0` と同一）に合わせる。
    fn reason_prefix(self) -> &'static str {
        match self {
            SizeKind::Char => "",
            SizeKind::Balloon => "balloon: ",
        }
    }
}

/// 各スコープの初期 surface（scope0=id0・scope1=id10・scope n≥2=id10 暫定＋warn）と
/// **そのスコープが解決した**バルーン面 0 を bind なし合成して原寸（物理 px）を得る
/// （2.9 の幅供給源・3.2 の物理 px 単一通貨・DD5）。
///
/// - 合成失敗したスコープは scope0 の寸法で代替し `warn!`（窓自体は生やす）
/// - scope0 の採寸自体（shell アセット構築含む）・いずれかのスコープのバルーン面 0 の
///   採寸が成立しない場合は代替根拠が無いため `Err(PlacementError::Measure)`
///   （log-first・シームが `spawn_dummy_window` へフォールバック・DD14）。
///   balloon 起因の失敗は `reason` に `balloon:` 接頭辞を付け、**当該スコープ番号**で
///   報告する（バルーン専用 variant は持たない・task 1 の失敗型を消費）
/// - 採寸に使ったアセットは本関数のローカルとして return 時に破棄される
///
/// 呼び出しスレッドは COM 初期化済みであること（本番＝MTA UI スレッド）。
///
/// # 2 段構成（design「measure.rs（k₀ 適用・R7.8 席保全）」）
///
/// 1. **native 採寸** [`measure_native_scope_sizes`]（既存ロジック・per-scope ループと
///    [`ScopeInput`] の構造を温存）
/// 2. **k 適用** [`apply_scaling`]（`scaled_extent` 経由で char=`scaling.shell`・
///    balloon=`scaling.balloon` を **各 `ScopeInput` へ個別に**写像）
///
/// (2) は per-scope の写像であり、`balloon_size` は実際に scope 別値である
/// （`areka-P0-emo-dpi-scaling` が要件 7.8 として保全した席を、本 spec の scope 別採寸が
/// 使い切った形）。`MeasureScaling::IDENTITY` 指定時は `ScaleRatio::scale_len` が素通しゆえ
/// native 採寸の出力と厳密に同一である（要件 7.2）。
///
/// 失敗の scope 帰属は (1) native 段・(2) k 適用段のいずれでも**実スコープ番号**で一貫する
/// （バルーン採寸が scope ループ内へ移り、帰属先が定まらない状況が無くなった）。
#[allow(dead_code)] // scaffold（task 4.1）: 結線は task 6
pub fn measure_scope_sizes(
    shell_dir: &Path,
    balloon_root: &Path,
    scope_ids: &[usize],
    scaling: &MeasureScaling,
) -> Result<MeasuredSizes, PlacementError> {
    // (1) native 採寸（原寸・k 非適用）。
    let natives = measure_native_scope_sizes(shell_dir, balloon_root, scope_ids)?;
    // (2) k 適用（per-scope 写像）。
    let scopes = apply_scaling(natives, scaling)?;
    Ok(MeasuredSizes { scopes })
}

/// native 採寸（k 非適用の原寸・[`measure_scope_sizes`] の第 1 段）。
///
/// 1 本の per-scope ループが、当該スコープのキャラ surface と当該スコープが解決した
/// バルーン面 0 の**双方**を採寸して [`ScopeInput`] を組む（要件 3.1）。
/// 失敗規約・代替規約は [`measure_scope_sizes`] のドキュメントに同じ。
fn measure_native_scope_sizes(
    shell_dir: &Path,
    balloon_root: &Path,
    scope_ids: &[usize],
) -> Result<Vec<ScopeInput>, PlacementError> {
    // 実 WIC デコーダ（COM 初期化済みスレッド前提・donor build_and_spawn と同型）。
    let decoder = WicDecoderArm::new().map_err(|e| {
        error!(error = ?e, "measure: WicDecoderArm 生成に失敗（COM 未初期化？）");
        PlacementError::Measure {
            scope: 0,
            reason: format!("WicDecoderArm 生成失敗: {e}"),
        }
    })?;

    // shell アセット（parse→bake→build＋bind・donor build_shell_target と同経路）。
    let (shell_world, shell_atlas) = build_shell_assets(shell_dir, &decoder)?;
    let mut composer = Composer::new();

    // 基準採寸: scope0 初期 surface（id 0）。失敗は代替根拠が無い＝ hard Err。
    let scope0_size =
        compose_size(&mut composer, &shell_world, &shell_atlas, 0).map_err(|reason| {
            error!(reason = %reason, "measure: scope0（surface id 0）の採寸合成に失敗");
            PlacementError::Measure { scope: 0, reason }
        })?;

    let mut scopes = Vec::with_capacity(scope_ids.len());
    for &scope in scope_ids {
        let char_size = if scope == 0 {
            scope0_size
        } else {
            if scope >= 2 {
                warn!(
                    scope,
                    surface_id = KERO_INITIAL_SURFACE_ID,
                    "measure: scope n≥2 の初期 surface は正典既定なし（id10 暫定で採寸）"
                );
            }
            match compose_size(
                &mut composer,
                &shell_world,
                &shell_atlas,
                KERO_INITIAL_SURFACE_ID,
            ) {
                Ok(size) => size,
                Err(reason) => {
                    // 採寸失敗スコープは scope0 寸法で代替（窓自体は生やす・design Error Handling）。
                    warn!(
                        scope,
                        surface_id = KERO_INITIAL_SURFACE_ID,
                        reason = %reason,
                        "measure: スコープの採寸合成に失敗（scope0 の寸法で代替）"
                    );
                    scope0_size
                }
            }
        };
        // バルーンは **当該 scope が解決した系列の面 0**（全 scope 共通の 1 回へ畳まない・
        // 要件 3.1）。失敗は代替根拠が無く hard Err で、帰属は実 scope 番号。
        let balloon_size = measure_balloon_surface0(balloon_root, &decoder, &mut composer, scope)?;
        scopes.push(ScopeInput {
            scope,
            char_size,
            balloon_size,
        });
    }

    // shell_world／shell_atlas／decoder はここで破棄される（採寸後破棄の契約）。
    Ok(scopes)
}

/// k 適用（[`measure_scope_sizes`] の第 2 段・**per-scope 写像**）。
///
/// native 採寸で得た各 [`ScopeInput`] を**1 件ずつ** [`scale_scope_input`] へ通す。
/// バルーン寸の供給点はループ内の `input.balloon_size` であり、その値は
/// [`measure_native_scope_sizes`] が scope ごとに解決した面 0 の実寸ゆえ **scope 別に異なる**
/// （emo2 実 fixture では scope0=400×224・scope1=288×203）。ループ外へ「バルーンは全スコープ
/// 共通だから 1 回だけ計算する」形へ畳むと、相方側の寸が本体側の寸で上書きされる。
///
/// 純関数（I/O・COM 非依存）ゆえ GPU/WIC 無しの単体テストで全分岐を網羅できる。
fn apply_scaling(
    natives: Vec<ScopeInput>,
    scaling: &MeasureScaling,
) -> Result<Vec<ScopeInput>, PlacementError> {
    let mut scaled = Vec::with_capacity(natives.len());
    for input in natives {
        scaled.push(scale_scope_input(input, scaling)?);
    }
    Ok(scaled)
}

/// 1 スコープぶんの k 適用（char=`scaling.shell`・balloon=`scaling.balloon`）。
///
/// 2 軸は独立であり、一方の k が他方へ漏れない（shell/balloon の author_dpi は
/// 別々に宣言され得る・要件 1.1）。`scope` は入力の転記（採寸の同一性を崩さない）。
fn scale_scope_input(
    input: ScopeInput,
    scaling: &MeasureScaling,
) -> Result<ScopeInput, PlacementError> {
    Ok(ScopeInput {
        scope: input.scope,
        char_size: scale_size_px(input.char_size, scaling.shell, input.scope, SizeKind::Char)?,
        // per-scope のバルーン供給点（scope 別に解決した面 0 の実寸）。ループ外へ畳まない。
        balloon_size: scale_size_px(
            input.balloon_size,
            scaling.balloon,
            input.scope,
            SizeKind::Balloon,
        )?,
    })
}

/// 物理 px 寸法の k 倍（**丸めは [`ScaleRatio::scaled_extent`] 単一権威**・D4）。
///
/// 自前の丸め・`as f32` 乗算は行わない（消費点ごとの個別丸めが見切れ／隙間を生む）。
///
/// # 失敗（log-first・silent wrap しない）
///
/// - k 適用前の寸法が負値（u32 表現不能）
/// - k 倍後の寸法が物理 px 通貨 i32 を超過（`scale_len` は `u32::MAX` へ飽和するのみで
///   i32 域検査は呼び手＝本関数の責務）
///
/// いずれも `error!` のうえ既存流儀の [`PlacementError::Measure`] を返す
/// （design「Error Handling」の `k 倍寸の i32 超過（採寸）` 行・要件 2.5）。
/// バルーン起因は reason へ `balloon:` 接頭辞を冠し、`scope` は**当該スコープ番号**を
/// 載せる（per-scope 写像ゆえ帰属が確定する・R7.8 席保全の帰結）。
fn scale_size_px(
    size: SizePx,
    k: ScaleRatio,
    scope: usize,
    kind: SizeKind,
) -> Result<SizePx, PlacementError> {
    let fail = |reason: String| {
        error!(
            scope,
            kind = ?kind,
            k = k.as_f32(),
            reason = %reason,
            "measure: k 適用に失敗（物理 px 通貨へ収まらない）"
        );
        PlacementError::Measure {
            scope,
            reason: format!("{}{reason}", kind.reason_prefix()),
        }
    };

    let (Ok(w), Ok(h)) = (u32::try_from(size.w), u32::try_from(size.h)) else {
        return Err(fail(format!("k 適用前の寸法が負値: {}x{}", size.w, size.h)));
    };
    // 丸め単一権威（round half away from zero・非ゼロは最小 1px）。
    let (sw, sh) = k.scaled_extent(w, h);
    let (Ok(w), Ok(h)) = (i32::try_from(sw), i32::try_from(sh)) else {
        return Err(fail(format!(
            "k={} 倍後の寸法が i32 を超過: {sw}x{sh}",
            k.as_f32()
        )));
    };
    Ok(SizePx { w, h })
}

/// shell dir の surfaces.txt から `(EmoWorld, AtlasTable)` を組む
/// （donor `build_shell_target` と同経路: read→parse→bake→build＋bind）。
///
/// emo2 の surfaces.txt は `charset,UTF-8` 宣言＝UTF-8 読み（donor と同じ
/// `read_to_string`）。`use_self_alpha` は emo2 実測（`seriko.use_self_alpha,1`）
/// に合わせ `On` 固定（donor 同値・descript 由来のパラメタ化は将来シーム）。
fn build_shell_assets(
    shell_dir: &Path,
    decoder: &WicDecoderArm,
) -> Result<(EmoWorld, AtlasTable), PlacementError> {
    let surfaces_txt = shell_dir.join("surfaces.txt");
    let content = std::fs::read_to_string(&surfaces_txt).map_err(|e| {
        error!(
            path = %surfaces_txt.display(),
            error = %e,
            "measure: shell surfaces.txt の読取に失敗"
        );
        PlacementError::Measure {
            scope: 0,
            reason: format!("surfaces.txt 読取失敗: {}: {e}", surfaces_txt.display()),
        }
    })?;

    let shell = areka_parsers::shell::parse(&content);
    if shell.surfaces.is_empty() {
        error!(
            path = %surfaces_txt.display(),
            "measure: surfaces.txt が surface を 1 つも産まなかった"
        );
        return Err(PlacementError::Measure {
            scope: 0,
            reason: format!(
                "surfaces.txt に surface 定義なし: {}",
                surfaces_txt.display()
            ),
        });
    }

    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: shell_dir,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], decoder, PackConfig::default());
    // emo2 shell は α 無し `purple/a/null.png` 1 枚が normalize seam として脱落する
    // （既知・許容・donor 同様 warn 継続。採寸対象 surface0/10 の element には無害）。
    for err in &baked.errors {
        warn!(error = %err, "measure: shell bake で脱落した element（採寸には無害の可能性）");
    }

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&baked.table, SetId(0));
    Ok((world, baked.table))
}

/// **当該 scope** が解決したバルーン系列の面 0 を採寸する（要件 3.1・design D2）。
///
/// # ファイル選択は権威の消費のみ
///
/// どのファイルが当該 scope の面 0 かは系列解決権威
/// [`resolve_balloon_faces`]（`areka-emo-present`）だけが決める。採寸側は接頭辞連鎖も
/// ディレクトリ走査も固定名（かつての `balloons0.png` 決め打ち）も持たない——列挙規則が
/// 独立 2 実装へ分かれると「採寸した窓寸 ≠ 実際に合成された枠」という実機でしか現れない
/// 欠陥になるため（design「最重要の構造リスク」）。合成も権威の
/// [`build_balloon_target_from_faces`] をそのまま用い、synthetic surfaces.txt の書式まで
/// 含めて実装を 1 本に保つ（起動時資産構築と採寸が同じ絵を見る保証）。
///
/// 採寸に要るのは面 0 のみゆえ、権威が返した採用面列から**面 0 だけ**を構築へ渡す
/// （他の面を bake しない）。面 0 の必在は権威側の単一施行点（R1.7）であり、ここで
/// 再判定しない——面 0 を欠く入力では `resolve_balloon_faces` が既に `error!`＋`Err` を
/// 返しており、万一の空列も構築側の log-first ガードが受け止める。
///
/// # 失敗
///
/// `error!`＋[`PlacementError::Measure`]（`reason` に `balloon:` 接頭辞）。scope ループ内から
/// 呼ばれるため、帰属は**実 scope 番号**である（全 scope 共通の 1 回採寸ではなくなった＝
/// 「帰属先が定まらない」状況自体が消えた・要件 3.1）。
///
/// なお design の Service Interface は `scope: u32` と記すが、placement 層の scope 通貨は
/// `usize`（[`ScopeInput::scope`]・[`PlacementError::Measure`]）である。境界変換を呼び手へ
/// 散らさないため本関数が `usize` を受け、権威呼び出しの直前で 1 度だけ u32 へ変換する。
fn measure_balloon_surface0(
    balloon_root: &Path,
    decoder: &WicDecoderArm,
    composer: &mut Composer,
    scope: usize,
) -> Result<SizePx, PlacementError> {
    let balloon_err = |reason: String| {
        error!(
            balloon_root = %balloon_root.display(),
            scope,
            reason = %reason,
            "measure: scope のバルーン面 0 の採寸に失敗"
        );
        PlacementError::Measure {
            scope,
            reason: format!("balloon: {reason}"),
        }
    };

    // 権威の scope 通貨は u32（`areka-emo-present`）。表現できない scope は無言で
    // 切り詰めず失敗として報告する（別 scope の系列を採ってしまう事故を作らない）。
    let scope_key = u32::try_from(scope)
        .map_err(|_| balloon_err(format!("scope 番号が u32 に収まらない: {scope}")))?;

    // 系列解決（連鎖の導出・ディレクトリ列挙・面 ID 単位の選択はすべて権威側の 1 箇所）。
    let faces = resolve_balloon_faces(balloon_root, scope_key)
        .map_err(|e| balloon_err(format!("系列解決に失敗: {e}")))?;
    // 採寸対象は面 0 のみ（採用面列の残りは bake しない）。
    let face0: Vec<ResolvedFace> = faces.into_iter().filter(|f| f.surface_id == 0).collect();

    let (world, atlas) = build_balloon_target_from_faces(balloon_root, decoder, &face0)
        .map_err(|e| balloon_err(format!("面 0 の採寸資産の構築に失敗: {e}")))?;

    compose_size(composer, &world, &atlas, 0).map_err(balloon_err)
    // world／atlas はここで破棄される（採寸後破棄の契約）。
}

/// `surface_id` を bind なし（`BindSet::default()`）で合成し原寸（物理 px）を返す。
///
/// 失敗（surface 不在・空合成・0 寸・i32 変換不能）は理由文字列で返し、
/// warn/error への昇格と scope 帰属は呼び手が決める。
fn compose_size(
    composer: &mut Composer,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
) -> Result<SizePx, String> {
    // 静的採寸経路ゆえ SERIKO ループは駆動しない → 空 pattern（`PatternState::default()`）で合成する
    // （空 pattern は拡張前と観測等価・R5.4）。
    let composed = composer
        .compose(
            world,
            atlas,
            surface_id,
            &BindSet::default(),
            &PatternState::default(),
        )
        .map_err(|e| format!("surface {surface_id} の合成失敗: {e}"))?;
    let (w, h) = (composed.width(), composed.height());
    if w == 0 || h == 0 {
        return Err(format!("surface {surface_id} の合成外形が 0 寸: {w}x{h}"));
    }
    // 物理 px は i32 通貨（resolver 契約）。表現不能な巨寸は採寸失敗として報告する
    // （silent wrap しない）。
    let (Ok(w), Ok(h)) = (i32::try_from(w), i32::try_from(h)) else {
        return Err(format!(
            "surface {surface_id} の合成外形が i32 を超過: {w}x{h}"
        ));
    };
    Ok(SizePx { w, h })
}

#[cfg(test)]
#[path = "measure_tests.rs"]
mod tests;
