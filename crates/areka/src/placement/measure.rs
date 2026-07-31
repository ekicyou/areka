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
#[allow(dead_code)] // scaffold（task 4.1）: main.rs シーム（task 6）が結線するまで非テストビルドでは未使用
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
    let scope0_size = compose_size(&mut composer, &shell_world, &shell_atlas, 0).map_err(
        |reason| {
            error!(reason = %reason, "measure: scope0（surface id 0）の採寸合成に失敗");
            PlacementError::Measure { scope: 0, reason }
        },
    )?;

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
            reason: format!("surfaces.txt に surface 定義なし: {}", surfaces_txt.display()),
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
        .compose(world, atlas, surface_id, &BindSet::default(), &PatternState::default())
        .map_err(|e| format!("surface {surface_id} の合成失敗: {e}"))?;
    let (w, h) = (composed.width(), composed.height());
    if w == 0 || h == 0 {
        return Err(format!("surface {surface_id} の合成外形が 0 寸: {w}x{h}"));
    }
    // 物理 px は i32 通貨（resolver 契約）。表現不能な巨寸は採寸失敗として報告する
    // （silent wrap しない）。
    let (Ok(w), Ok(h)) = (i32::try_from(w), i32::try_from(h)) else {
        return Err(format!("surface {surface_id} の合成外形が i32 を超過: {w}x{h}"));
    };
    Ok(SizePx { w, h })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

    use crate::placement::PlacementError;
    use crate::placement::test_support::{capture_logs, expect_one};

    /// COM 初期化下でクロージャを実行する（`WicDecoderArm` は COM 必須・
    /// emo-atlas `wic_arm.rs` テストと同一パターン）。
    fn with_com_initialized<F: FnOnce()>(f: F) {
        unsafe {
            // 既に初期化済みでも RPC_E_CHANGED_MODE を許容し続行する。
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        f();
        unsafe {
            CoUninitialize();
        }
    }

    /// emo2 fixture ルートを `CARGO_MANIFEST_DIR`（`crates/areka`）相対で解決する
    /// （source.rs／emo-present example と同一アンカー規約）。
    fn emo2(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2")
            .join(rel)
    }

    // ── テスト用一時ディレクトリ（emo-present balloon.rs と同じ std-only 最小実装）──

    static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Drop 時に自身を再帰削除する一時ディレクトリ。
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "areka-placement-measure-{}-{}",
                std::process::id(),
                n
            ));
            std::fs::create_dir_all(&path).expect("一時ディレクトリ作成");
            TempDir { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// emo2 fixture の実寸（物理 px・PNG 実デコード実測値）。
    /// scope0 初期 surface（surface0 ← surface0.png・記憶 areka-emo2-surface0-baked-balloon と一致）。
    const SCOPE0_W: i32 = 434;
    const SCOPE0_H: i32 = 687;
    /// scope1 初期 surface（surface10 ← CityPop/surface0010.png）。
    const SCOPE1_W: i32 = 336;
    const SCOPE1_H: i32 = 400;
    /// scope0 のバルーン面 0（`balloons0.png`＝本体側系列・PNG IHDR 実測）。
    const BALLOON0_W: i32 = 400;
    const BALLOON0_H: i32 = 224;
    /// scope1 のバルーン面 0（`balloonk0.png`＝相方側系列・PNG IHDR 実測）。
    /// 本体側と**異なる**寸であることが本 fixture の判別力の源である。
    const BALLOON1_W: i32 = 288;
    const BALLOON1_H: i32 = 203;

    /// 観測可能な完了状態（tasks 4.1）: emo2 fixture の shell/balloon に対し
    /// `measure_scope_sizes` が scope0・scope1・balloon の原寸（非ゼロ物理 px）を
    /// 返す。emo2 は決定論 fixture ゆえ厳密寸で固定する（2.9 の幅供給源・3.2 の
    /// 物理 px 単一通貨）。
    #[test]
    fn measure_emo2_fixture_yields_exact_nonzero_sizes() {
        with_com_initialized(|| {
            let out = measure_scope_sizes(
                &emo2("shell/master"),
                &emo2("emo2-kakukaku"),
                &[0, 1],
                &MeasureScaling::IDENTITY,
            )
            .expect("emo2 fixture の採寸は成功する");

            assert_eq!(out.scopes.len(), 2, "出力長＝scope_ids 長");
            // 入力順・scope 番号の転記。
            assert_eq!(out.scopes[0].scope, 0);
            assert_eq!(out.scopes[1].scope, 1);

            // scope0: surface0（surface0.png 原寸）。
            assert_eq!(out.scopes[0].char_size.w, SCOPE0_W);
            assert_eq!(out.scopes[0].char_size.h, SCOPE0_H);
            // scope1: surface10（CityPop/surface0010.png 原寸）。
            assert_eq!(out.scopes[1].char_size.w, SCOPE1_W);
            assert_eq!(out.scopes[1].char_size.h, SCOPE1_H);
            // バルーン面 0 は **scope ごとに解決した系列**の原寸（scope0=balloons0.png・
            // scope1=balloonk0.png）。厳密値の対比は
            // [`measure_emo2_fixture_yields_per_scope_balloon_sizes`] が担う。
            assert_eq!(out.scopes[0].balloon_size.w, BALLOON0_W);
            assert_eq!(out.scopes[0].balloon_size.h, BALLOON0_H);
            assert_eq!(out.scopes[1].balloon_size.w, BALLOON1_W);
            assert_eq!(out.scopes[1].balloon_size.h, BALLOON1_H);
            for s in &out.scopes {
                // 要件の観測文言どおり「非ゼロ物理 px」も明示的に固定する。
                assert!(s.char_size.w > 0 && s.char_size.h > 0, "char 原寸は非ゼロ");
                assert!(
                    s.balloon_size.w > 0 && s.balloon_size.h > 0,
                    "balloon 原寸は非ゼロ"
                );
            }
        });
    }

    /// 観測可能な完了状態（tasks 4.2・要件 3.1）: 実 fixture で**本体側と相方側が
    /// 異なるバルーン寸**を得る。
    ///
    /// emo2-kakukaku は `balloons0.png`（400×224）と `balloonk0.png`（288×203）を併せ持ち、
    /// scope0 の接頭辞連鎖は `balloons` へ・scope1 の連鎖は `balloonk` へ解決する
    /// （系列解決権威 `resolve_balloon_faces`）。バルーンを「全スコープ共通だから 1 回だけ」
    /// 採る実装では scope1 も 400×224 になり、本檻が落ちる。
    #[test]
    fn measure_emo2_fixture_yields_per_scope_balloon_sizes() {
        with_com_initialized(|| {
            let out = measure_scope_sizes(
                &emo2("shell/master"),
                &emo2("emo2-kakukaku"),
                &[0, 1],
                &MeasureScaling::IDENTITY,
            )
            .expect("emo2 fixture の採寸は成功する");

            assert_eq!(
                (out.scopes[0].balloon_size.w, out.scopes[0].balloon_size.h),
                (BALLOON0_W, BALLOON0_H),
                "scope0 は本体側系列（balloons0.png）の実寸"
            );
            assert_eq!(
                (out.scopes[1].balloon_size.w, out.scopes[1].balloon_size.h),
                (BALLOON1_W, BALLOON1_H),
                "scope1 は相方側系列（balloonk0.png）の実寸——本体側の寸ではない"
            );
            assert_ne!(
                out.scopes[0].balloon_size, out.scopes[1].balloon_size,
                "本体側と相方側のバルーン寸は互いに異なる（1 回の共通採寸では再現不能）"
            );
        });
    }

    /// 要件 3.6/7.2: k≠1 でも scope 別バルーン寸は**各々が自分の原寸から**k 倍される。
    /// 丸めは既存権威 `ScaleRatio::scaled_extent` のままであり、新たな丸め規約を導入しない。
    #[test]
    fn measure_emo2_fixture_scales_per_scope_balloon_independently() {
        with_com_initialized(|| {
            let out = measure_scope_sizes(
                &emo2("shell/master"),
                &emo2("emo2-kakukaku"),
                &[0, 1],
                &MeasureScaling {
                    shell: ScaleRatio::ONE,
                    balloon: k(3, 2),
                },
            )
            .expect("k≠1 でも emo2 fixture の採寸は成功する");

            // 400×3/2=600・224×3/2=336。
            assert_eq!(
                (out.scopes[0].balloon_size.w, out.scopes[0].balloon_size.h),
                (600, 336)
            );
            // 288×3/2=432・203×3/2=304.5→305（round half away from zero＝既存権威の規約）。
            assert_eq!(
                (out.scopes[1].balloon_size.w, out.scopes[1].balloon_size.h),
                (432, 305)
            );
            // 丸め権威との一致（自前丸めを持ち込んでいないことの構造確認）。
            assert_eq!(
                (
                    out.scopes[1].balloon_size.w as u32,
                    out.scopes[1].balloon_size.h as u32
                ),
                k(3, 2).scaled_extent(BALLOON1_W as u32, BALLOON1_H as u32)
            );
        });
    }

    /// scope n≥2 は正典既定が無く id10 暫定（warn 付き）＝scope1 と同じキャラ寸になる。
    /// バルーンは scope n≥2 の連鎖にも `balloonk` が載る（正典が三人目以降の流用先として
    /// 名指しする系列）ため、emo2-kakukaku では scope1 と同じ `balloonk0.png` へ解決する。
    #[test]
    fn scope_n_ge_2_measures_interim_id10() {
        with_com_initialized(|| {
            let out = measure_scope_sizes(
                &emo2("shell/master"),
                &emo2("emo2-kakukaku"),
                &[0, 1, 2],
                &MeasureScaling::IDENTITY,
            )
            .expect("scope2 を含んでも採寸は成功する");

            assert_eq!(out.scopes.len(), 3);
            assert_eq!(out.scopes[2].scope, 2);
            assert_eq!(
                out.scopes[2].char_size, out.scopes[1].char_size,
                "scope n≥2 は id10 暫定＝scope1 と同寸"
            );
            assert_eq!(
                (out.scopes[2].balloon_size.w, out.scopes[2].balloon_size.h),
                (BALLOON1_W, BALLOON1_H),
                "scope n≥2 のバルーンは連鎖上の `balloonk` へ解決（本体側 400×224 ではない）"
            );
        });
    }

    /// 合成失敗したスコープ（surface10 未定義シェル）は scope0 の寸法で代替される
    /// （窓自体は生やす——design「Error Handling」）。
    #[test]
    fn failed_scope_substitutes_scope0_size() {
        with_com_initialized(|| {
            // surface0 のみ定義する合成用シェルを一時 dir に組む（実 PNG は fixture から複写）。
            let shell = TempDir::new();
            std::fs::copy(
                emo2("shell/master/surface0.png"),
                shell.path().join("surface0.png"),
            )
            .expect("surface0.png の複写");
            std::fs::write(
                shell.path().join("surfaces.txt"),
                "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n",
            )
            .expect("surfaces.txt の書出し");

            let out = measure_scope_sizes(
                shell.path(),
                &emo2("emo2-kakukaku"),
                &[0, 1],
                &MeasureScaling::IDENTITY,
            )
            .expect("scope1 が採寸不能でも全体は成功する（代替寸で継続）");

            assert_eq!(out.scopes.len(), 2, "窓は欠落させない（スコープ数維持）");
            assert_eq!(out.scopes[0].char_size.w, SCOPE0_W);
            assert_eq!(out.scopes[0].char_size.h, SCOPE0_H);
            assert_eq!(
                out.scopes[1].char_size, out.scopes[0].char_size,
                "採寸失敗スコープは scope0 寸法で代替"
            );
        });
    }

    /// surfaces.txt が読めない（scope0 の採寸自体が成立しない）場合は
    /// `PlacementError::Measure`（代替根拠なし→シームがフォールバック・DD14）。
    #[test]
    fn missing_surfaces_txt_is_measure_error() {
        with_com_initialized(|| {
            let empty_shell = TempDir::new();

            match measure_scope_sizes(
                empty_shell.path(),
                &emo2("emo2-kakukaku"),
                &[0],
                &MeasureScaling::IDENTITY,
            ) {
                Ok(_) => panic!("surfaces.txt 不在で Ok は誤成功"),
                Err(PlacementError::Measure { scope, .. }) => {
                    assert_eq!(scope, 0, "shell 全体の失敗は scope0 起点で報告")
                }
                Err(other) => panic!("Measure であるべき: {other:?}"),
            }
        });
    }

    /// 要件 3.7/5.4（後方互換）: 相方側系列（`balloonk*`）を持たないバルーンでは
    /// **全 scope が同一のバルーン寸**を得る——本仕様適用前の採寸結果と一致する。
    ///
    /// 連鎖の最終受け皿は常に本体側 `balloons` ゆえ、scope1／scope2 も `balloons0.png` へ
    /// 収束する（scope 別化が「相方側資産がある時だけ」効くことの構造的証拠）。
    #[test]
    fn balloonk_absent_yields_identical_size_for_all_scopes() {
        with_com_initialized(|| {
            // 本体側 1 枚だけのバルーンを組む（実 PNG は emo2 fixture から複写）。
            let balloon = TempDir::new();
            std::fs::copy(
                emo2("emo2-kakukaku/balloons0.png"),
                balloon.path().join("balloons0.png"),
            )
            .expect("balloons0.png の複写");

            let out = measure_scope_sizes(
                &emo2("shell/master"),
                balloon.path(),
                &[0, 1, 2],
                &MeasureScaling::IDENTITY,
            )
            .expect("本体側系列のみでも採寸は成功する");

            for s in &out.scopes {
                assert_eq!(
                    (s.balloon_size.w, s.balloon_size.h),
                    (BALLOON0_W, BALLOON0_H),
                    "scope {} は本体側系列へ収束する（適用前と同一寸）",
                    s.scope
                );
            }
        });
    }

    /// バルーン面 0 が成立しない（面画像なし）場合も `PlacementError::Measure`
    /// （`ScopeInput.balloon_size` の供給源が無い＝採寸全体の失敗）。面 0 必在の判定は
    /// 系列解決権威の単一施行点にあり、採寸側は失敗をそのまま帰属付きで畳む。
    #[test]
    fn missing_balloon_frame_is_measure_error() {
        with_com_initialized(|| {
            let empty_balloon = TempDir::new();

            match measure_scope_sizes(
                &emo2("shell/master"),
                empty_balloon.path(),
                &[0],
                &MeasureScaling::IDENTITY,
            ) {
                Ok(_) => panic!("balloon 枠不在で Ok は誤成功"),
                Err(PlacementError::Measure { .. }) => {}
                Err(other) => panic!("Measure であるべき: {other:?}"),
            }
        });
    }

    // ── k 適用（第 2 段）の純関数テスト（GPU/WIC/COM 非依存・要件 5.2）──

    /// テスト用 [`ScopeInput`] 組み立て（char/balloon を別値で与えられる）。
    fn scope_input(scope: usize, char_wh: (i32, i32), balloon_wh: (i32, i32)) -> ScopeInput {
        ScopeInput {
            scope,
            char_size: SizePx {
                w: char_wh.0,
                h: char_wh.1,
            },
            balloon_size: SizePx {
                w: balloon_wh.0,
                h: balloon_wh.1,
            },
        }
    }

    /// `k` を組む（テスト内の分母・分子は既知の非ゼロ値ゆえ `expect`）。
    fn k(num: u32, den: u32) -> ScaleRatio {
        ScaleRatio::new(num, den).expect("テストの k は非ゼロ")
    }

    /// 要件 7.2: `MeasureScaling::IDENTITY` は native 採寸の出力を素通しする
    /// （k=1/1 は既存等倍経路と構造的に同一＝既存期待値の不変性）。
    #[test]
    fn apply_scaling_identity_is_passthrough() {
        let natives = vec![
            scope_input(0, (SCOPE0_W, SCOPE0_H), (BALLOON0_W, BALLOON0_H)),
            scope_input(1, (SCOPE1_W, SCOPE1_H), (BALLOON0_W, BALLOON0_H)),
        ];
        let out = apply_scaling(natives.clone(), &MeasureScaling::IDENTITY)
            .expect("恒等 k の適用は成功する");
        assert_eq!(out, natives, "恒等 k は入力の恒等写像");

        // 既約化で 1/1 になる比（96/96）も同じ恒等経路。
        let same = apply_scaling(
            natives.clone(),
            &MeasureScaling {
                shell: k(96, 96),
                balloon: k(96, 96),
            },
        )
        .expect("96/96 も恒等");
        assert_eq!(same, natives);
        assert_eq!(
            MeasureScaling::default().shell,
            ScaleRatio::ONE,
            "既定は恒等"
        );
    }

    /// 要件 2.5/3.1/3.3: 整数 k・非整数 k とも `scaled_extent` の丸め規約で k 倍される。
    #[test]
    fn apply_scaling_scales_via_scaled_extent_rounding() {
        // k=2/1: 厳密 2 倍。
        let out = apply_scaling(
            vec![scope_input(
                0,
                (SCOPE0_W, SCOPE0_H),
                (BALLOON0_W, BALLOON0_H),
            )],
            &MeasureScaling {
                shell: k(2, 1),
                balloon: k(2, 1),
            },
        )
        .expect("k=2/1 の適用は成功する");
        assert_eq!((out[0].char_size.w, out[0].char_size.h), (868, 1374));
        assert_eq!((out[0].balloon_size.w, out[0].balloon_size.h), (800, 448));

        // k=5/4（120dpi 相当）: 端数は round half away from zero。
        // 434*1.25=542.5→543・687*1.25=858.75→859・400*1.25=500・224*1.25=280。
        let out = apply_scaling(
            vec![scope_input(
                0,
                (SCOPE0_W, SCOPE0_H),
                (BALLOON0_W, BALLOON0_H),
            )],
            &MeasureScaling {
                shell: k(5, 4),
                balloon: k(5, 4),
            },
        )
        .expect("k=5/4 の適用は成功する");
        assert_eq!((out[0].char_size.w, out[0].char_size.h), (543, 859));
        assert_eq!((out[0].balloon_size.w, out[0].balloon_size.h), (500, 280));

        // 丸め権威との一致（自前丸めを持ち込んでいないことの構造確認）。
        let k54 = k(5, 4);
        assert_eq!(
            (out[0].char_size.w as u32, out[0].char_size.h as u32),
            k54.scaled_extent(SCOPE0_W as u32, SCOPE0_H as u32)
        );
    }

    /// 要件 1.1: shell k と balloon k は独立軸——一方の k が他方へ漏れない
    /// （両者に同じ k を適用する実装なら落ちる）。
    #[test]
    fn apply_scaling_uses_independent_shell_and_balloon_k() {
        let out = apply_scaling(
            vec![scope_input(0, (100, 200), (400, 224))],
            &MeasureScaling {
                shell: k(2, 1),
                balloon: k(1, 2),
            },
        )
        .expect("独立 k の適用は成功する");

        assert_eq!(
            (out[0].char_size.w, out[0].char_size.h),
            (200, 400),
            "char は shell k（2/1）"
        );
        assert_eq!(
            (out[0].balloon_size.w, out[0].balloon_size.h),
            (200, 112),
            "balloon は balloon k（1/2）"
        );
        // balloon へ shell k が漏れていれば 800x448 になる（陰性確認）。
        assert_ne!((out[0].balloon_size.w, out[0].balloon_size.h), (800, 448));
    }

    /// 要件 3.1: k 適用は **scope ごとの写像**であり、scope 別の `balloon_size` が
    /// 各々独立に k 倍される（純関数層での固定——実 fixture 経路の対比は
    /// [`measure_emo2_fixture_scales_per_scope_balloon_independently`] が担う）。
    ///
    /// バルーン計算をループ外へ畳んだ実装（「全スコープ共通だから 1 回だけ」）は
    /// 本テストで必ず落ちる。
    #[test]
    fn apply_scaling_maps_balloon_per_scope() {
        let natives = vec![
            scope_input(0, (434, 687), (400, 224)),
            scope_input(1, (336, 400), (300, 150)),
            scope_input(2, (10, 20), (7, 3)),
        ];
        let out = apply_scaling(
            natives,
            &MeasureScaling {
                shell: k(1, 1),
                balloon: k(3, 2),
            },
        )
        .expect("per-scope 適用は成功する");

        assert_eq!(out.len(), 3, "スコープ数を維持する");
        assert_eq!(out[0].scope, 0);
        assert_eq!(out[1].scope, 1);
        assert_eq!(out[2].scope, 2);

        // 各 scope の balloon が「自分の原寸 ×3/2」になる（他 scope の値で上書きされない）。
        assert_eq!((out[0].balloon_size.w, out[0].balloon_size.h), (600, 336));
        assert_eq!((out[1].balloon_size.w, out[1].balloon_size.h), (450, 225));
        // 7*1.5=10.5→11・3*1.5=4.5→5（round half away from zero）。
        assert_eq!((out[2].balloon_size.w, out[2].balloon_size.h), (11, 5));

        // 3 つとも異なる値＝ループ外へ畳んだ実装では再現不能。
        assert_ne!(out[0].balloon_size, out[1].balloon_size);
        assert_ne!(out[1].balloon_size, out[2].balloon_size);

        // char は shell k=1/1 ゆえ原寸のまま（軸の独立性の再確認）。
        assert_eq!((out[0].char_size.w, out[0].char_size.h), (434, 687));
        assert_eq!((out[1].char_size.w, out[1].char_size.h), (336, 400));
    }

    /// 要件 2.5: k 倍後が物理 px 通貨 i32 を超過したら `PlacementError::Measure`
    /// （silent wrap しない・design「Error Handling」の `k 倍寸の i32 超過（採寸）` 行）。
    #[test]
    fn apply_scaling_guards_i32_overflow() {
        // char 側の超過（scale_len は u32::MAX へ飽和するが i32 には収まらない）。
        match apply_scaling(
            vec![scope_input(3, (i32::MAX, 10), (10, 10))],
            &MeasureScaling {
                shell: k(2, 1),
                balloon: ScaleRatio::ONE,
            },
        ) {
            Ok(v) => panic!("i32 超過で Ok は誤成功（wrap している）: {v:?}"),
            Err(PlacementError::Measure { scope, reason }) => {
                assert_eq!(scope, 3, "帰属スコープを報告する");
                assert!(reason.contains("i32 を超過"), "理由: {reason}");
                assert!(
                    !reason.starts_with("balloon:"),
                    "char 起因に balloon 接頭辞は付かない: {reason}"
                );
            }
            Err(other) => panic!("Measure であるべき: {other:?}"),
        }

        // balloon 側の超過は `balloon:` 接頭辞つきで当該 scope に帰属する。
        match apply_scaling(
            vec![
                scope_input(0, (10, 10), (10, 10)),
                scope_input(1, (10, 10), (10, i32::MAX)),
            ],
            &MeasureScaling {
                shell: ScaleRatio::ONE,
                balloon: k(3, 1),
            },
        ) {
            Ok(v) => panic!("i32 超過で Ok は誤成功: {v:?}"),
            Err(PlacementError::Measure { scope, reason }) => {
                assert_eq!(scope, 1, "balloon 起因も per-scope に帰属する");
                assert!(reason.starts_with("balloon: "), "接頭辞: {reason}");
                assert!(reason.contains("i32 を超過"), "理由: {reason}");
            }
            Err(other) => panic!("Measure であるべき: {other:?}"),
        }

        // 境界: i32::MAX ちょうどは通る（恒等 k）。
        let ok = apply_scaling(
            vec![scope_input(0, (i32::MAX, 1), (1, i32::MAX))],
            &MeasureScaling::IDENTITY,
        )
        .expect("恒等 k なら i32::MAX は通貨内");
        assert_eq!(ok[0].char_size.w, i32::MAX);
        assert_eq!(ok[0].balloon_size.h, i32::MAX);
    }

    /// 負値寸法（u32 表現不能）は wrap せず `PlacementError::Measure`（log-first）。
    #[test]
    fn apply_scaling_rejects_negative_extent() {
        match apply_scaling(
            vec![scope_input(2, (-1, 10), (10, 10))],
            &MeasureScaling::IDENTITY,
        ) {
            Ok(v) => panic!("負値で Ok は誤成功: {v:?}"),
            Err(PlacementError::Measure { scope, reason }) => {
                assert_eq!(scope, 2);
                assert!(reason.contains("負値"), "理由: {reason}");
            }
            Err(other) => panic!("Measure であるべき: {other:?}"),
        }
    }

    /// 要件 2.5: 0 寸は 0 のまま・非ゼロ寸は最小 1px（`scaled_extent` の規約が
    /// そのまま採寸へ効いている＝自前丸めを持っていない証拠）。
    #[test]
    fn apply_scaling_inherits_scaled_extent_edge_rules() {
        let out = apply_scaling(
            vec![scope_input(0, (0, 3), (1, 1))],
            &MeasureScaling {
                shell: k(1, 100),
                balloon: k(1, 100),
            },
        )
        .expect("極小 k でも成功する");
        assert_eq!(out[0].char_size.w, 0, "0 寸は 0 のまま");
        assert_eq!(out[0].char_size.h, 1, "非ゼロは最小 1px（3/100→1）");
        assert_eq!((out[0].balloon_size.w, out[0].balloon_size.h), (1, 1));
    }

    /// 空入力は空出力（スコープ数の恒等・非パニック）。
    #[test]
    fn apply_scaling_empty_input_is_empty_output() {
        let out = apply_scaling(Vec::new(), &MeasureScaling::IDENTITY).expect("空入力は成功");
        assert!(out.is_empty());
    }

    /// 要件 7.8（席保全・char 軸）: char 寸も **scope ごとの写像**である。
    ///
    /// # 変異キルの実態（誇張しない）
    ///
    /// 「先頭スコープの char 寸を全スコープへ撒く」変異は、**既存**の
    /// [`apply_scaling_maps_balloon_per_scope`]（`measure.rs:829` の char 寸アサート
    /// 左 (434,687) / 右 (336,400)）と `apply_scaling_identity_is_passthrough`・
    /// emo2 実採寸テスト群でも落ちる。本テストは**共倒れ**であり、char 軸の席保全は
    /// task 2.3 時点で既に本物の檻に入っている。
    ///
    /// 本テストの役割は排他キルではなく、**非恒等 k（5/4）での char per-scope 写像を
    /// 明文化する冗長な錨**——既存の per-scope 検査はいずれも shell k=1/1 か
    /// GPU/COM を要する実 fixture 経路であり、「k≠1 のとき各スコープが自分の原寸から
    /// 個別に k 倍される」ことを GPU 非依存の純関数層で述べた行は他に無い。
    ///
    /// 期待値 543×859／420×500 は task 4.3 の実機観測（primary DPI=120）と同一であり、
    /// 434×1.25=542.5→543 が round half away from zero の生証拠でもある。
    #[test]
    fn apply_scaling_maps_char_per_scope() {
        let out = apply_scaling(
            vec![
                scope_input(0, (SCOPE0_W, SCOPE0_H), (1, 1)),
                scope_input(1, (SCOPE1_W, SCOPE1_H), (1, 1)),
                scope_input(2, (8, 4), (1, 1)),
            ],
            &MeasureScaling {
                shell: k(5, 4),
                balloon: ScaleRatio::ONE,
            },
        )
        .expect("k=5/4 の per-scope 適用は成功する");

        assert_eq!(out.len(), 3, "スコープ数を維持する");
        assert_eq!((out[0].char_size.w, out[0].char_size.h), (543, 859));
        assert_eq!((out[1].char_size.w, out[1].char_size.h), (420, 500));
        assert_eq!((out[2].char_size.w, out[2].char_size.h), (10, 5));

        // 3 つとも異なる値＝先頭スコープ寸の撒き直しでは再現不能。
        assert_ne!(out[0].char_size, out[1].char_size);
        assert_ne!(out[1].char_size, out[2].char_size);
        // balloon は恒等 k ゆえ原寸（軸の独立性の再確認）。
        for s in &out {
            assert_eq!((s.balloon_size.w, s.balloon_size.h), (1, 1));
        }
    }

    /// 要件 2.5: i32 ガードの**厳密境界**（1px の差で Ok/Err が入れ替わる）。
    ///
    /// 既存の [`apply_scaling_guards_i32_overflow`] は「恒等 k で `i32::MAX` が通る」
    /// 「`i32::MAX` の 2〜3 倍が落ちる」しか見ておらず、**k 倍後**の値に対する閾値が
    /// `i32::MAX` ちょうどなのか 1 だけ緩い（＝`i32::MAX + 1` を wrap して受理する）のかを
    /// 弁別しない。ここは 2_147_483_646（通る）と 2_147_483_648（落ちる）の対で
    /// 閾値そのものを固定する——「上限側を 1 だけ緩める」変異は**本テストのみ**が落とす
    /// （排他キル・実測済み）。
    #[test]
    fn apply_scaling_i32_guard_boundary_is_exact() {
        // (i32::MAX - 1) / 2 = 1_073_741_823 → ×2 = 2_147_483_646 ≤ i32::MAX。
        const HALF: i32 = 1_073_741_823;
        let doubling = MeasureScaling {
            shell: k(2, 1),
            balloon: ScaleRatio::ONE,
        };

        let ok = apply_scaling(vec![scope_input(0, (HALF, 1), (1, 1))], &doubling)
            .expect("2_147_483_646 は物理 px 通貨に収まる");
        assert_eq!(ok[0].char_size.w, i32::MAX - 1);

        // 1px 増やすと 2_147_483_648（= i32::MAX + 1）で通貨を溢れる。
        let err = apply_scaling(vec![scope_input(0, (HALF + 1, 1), (1, 1))], &doubling)
            .expect_err("1px 増えただけで i32 を超過する");
        match err {
            PlacementError::Measure { scope, reason } => {
                assert_eq!(scope, 0);
                assert!(reason.contains("i32 を超過"), "理由: {reason}");
            }
            other => panic!("Measure であるべき: {other:?}"),
        }
    }

    /// 負値寸法の balloon 側は `balloon:` 接頭辞つきで**当該スコープ**に帰属する
    /// （既存の負値テストは char 側のみ）。
    #[test]
    fn apply_scaling_rejects_negative_balloon_extent_with_prefix() {
        match apply_scaling(
            vec![
                scope_input(0, (10, 10), (10, 10)),
                scope_input(5, (10, 10), (10, -3)),
            ],
            &MeasureScaling::IDENTITY,
        ) {
            Ok(v) => panic!("負値で Ok は誤成功: {v:?}"),
            Err(PlacementError::Measure { scope, reason }) => {
                assert_eq!(scope, 5, "balloon 起因も per-scope に帰属する");
                assert!(reason.starts_with("balloon: "), "接頭辞: {reason}");
                assert!(reason.contains("負値"), "理由: {reason}");
            }
            Err(other) => panic!("Measure であるべき: {other:?}"),
        }
    }

    // ── k 適用失敗ログの発火（steering `logging.md`「ログ無し失敗経路の禁止」）──
    //
    // 既存の i32 ガード／負値テストは `Err` の中身だけを見ており、design
    // 「Error Handling」表の `k 倍寸の i32 超過（採寸）→ error!` 行は無検査だった。
    //
    // 捕捉は共有ハーネス [`crate::placement::test_support`]（`#[cfg(test)]` 限定）を使う。
    // **素朴な `with_default` 捕捉は非決定的に取りこぼす**——`tracing` の callsite interest
    // キャッシュはプロセス大域かつ「最初に踏んだスレッドが勝つ」ため、subscriber を持たない
    // 他テスト（同じ `error!` callsite を踏む `apply_scaling_guards_i32_overflow` 等）が
    // 先に登録すると `Interest::never()` が焼き付き、捕捉窓の内側でもイベントが捨てられる。
    // 機構と対策は `test_support` のモジュール doc を参照。

    /// design「Error Handling」: k 適用失敗は **`error!`**＋帰属情報
    /// （`scope`／`kind`／`k`／`reason`）を残す（silent wrap の禁止）。
    ///
    /// char 起因は `kind=Char`、balloon 起因は `kind=Balloon` と当該スコープ番号で
    /// 出るため、実機ログだけで「どのスコープのどの寸が溢れたか」が確定する。
    #[test]
    fn scale_size_px_failure_emits_error_log_with_attribution() {
        // char 起因（scope=3・k=2/1）。
        let (res, events) = capture_logs(|| {
            apply_scaling(
                vec![scope_input(3, (i32::MAX, 10), (10, 10))],
                &MeasureScaling {
                    shell: k(2, 1),
                    balloon: ScaleRatio::ONE,
                },
            )
        });
        assert!(res.is_err(), "i32 超過は Err");
        let ev = expect_one(&events, "k 適用に失敗");
        assert_eq!(
            ev.level,
            tracing::Level::ERROR,
            "採寸失敗は error 格（無言 wrap の禁止）: {ev:?}"
        );
        assert_eq!(ev.field("scope"), "3");
        assert_eq!(ev.field("kind"), "Char");
        assert_eq!(ev.field("k"), "2.0");
        assert!(ev.field("reason").contains("i32 を超過"), "{ev:?}");
        assert_eq!(events.len(), 1, "1 失敗 1 ログ: {events:?}");

        // balloon 起因（scope=1・k=3/1）——kind と scope が balloon 軸へ切り替わる。
        let (res, events) = capture_logs(|| {
            apply_scaling(
                vec![
                    scope_input(0, (10, 10), (10, 10)),
                    scope_input(1, (10, 10), (10, i32::MAX)),
                ],
                &MeasureScaling {
                    shell: ScaleRatio::ONE,
                    balloon: k(3, 1),
                },
            )
        });
        assert!(res.is_err());
        let ev = expect_one(&events, "k 適用に失敗");
        assert_eq!(ev.level, tracing::Level::ERROR, "{ev:?}");
        assert_eq!(ev.field("scope"), "1", "先頭スコープ 0 ではない");
        assert_eq!(ev.field("kind"), "Balloon");
        assert_eq!(
            ev.field("k"),
            "3.0",
            "balloon 軸の k が載る（shell の 1.0 ではない）"
        );
        assert_eq!(events.len(), 1, "{events:?}");
    }

    /// k 適用の成功経路は**無言**（`error!` を無条件発火させる変異の陰性対照）。
    #[test]
    fn apply_scaling_success_is_silent() {
        let (res, events) = capture_logs(|| {
            apply_scaling(
                vec![
                    scope_input(0, (SCOPE0_W, SCOPE0_H), (BALLOON0_W, BALLOON0_H)),
                    scope_input(1, (SCOPE1_W, SCOPE1_H), (300, 150)),
                ],
                &MeasureScaling {
                    shell: k(5, 4),
                    balloon: k(3, 2),
                },
            )
        });
        assert!(res.is_ok(), "成功経路");
        assert!(events.is_empty(), "成功経路は無言: {events:?}");
    }

    /// 要件 3.1 の帰属: **native 段**の balloon 失敗は**実スコープ番号**で報告される
    /// （k 適用段と規約が一致する——採寸が scope ループ内へ移り、「帰属先が定まらない」
    /// 状況そのものが無くなった）。
    ///
    /// 要求スコープに 0 を含めない `[1, 2]` で走らせるため、旧来の「scope: 0 固定」実装
    /// との差が出る（既存の [`missing_balloon_frame_is_measure_error`] は `[0]` を渡すため
    /// 両者を弁別できない）。
    #[test]
    fn native_stage_balloon_failure_reports_real_scope() {
        with_com_initialized(|| {
            let empty_balloon = TempDir::new();

            match measure_scope_sizes(
                &emo2("shell/master"),
                empty_balloon.path(),
                &[1, 2],
                &MeasureScaling::IDENTITY,
            ) {
                Ok(v) => panic!("balloon 面不在で Ok は誤成功: {v:?}"),
                Err(PlacementError::Measure { scope, reason }) => {
                    assert_eq!(
                        scope, 1,
                        "採寸ループが最初に倒れたスコープ番号を載せる（scope0 固定ではない）"
                    );
                    assert!(reason.starts_with("balloon: "), "接頭辞: {reason}");
                }
                Err(other) => panic!("Measure であるべき: {other:?}"),
            }
        });
    }

    /// 権威（`areka-emo-present`）の scope 通貨 `u32` に収まらないスコープ番号は、
    /// **無言で切り詰めず**失敗として報告される（別 scope の系列を採る事故を作らない）。
    ///
    /// 64bit 限定の檻——`usize == u32` の環境では超過そのものが起こり得ない。
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn scope_beyond_u32_is_measure_error() {
        with_com_initialized(|| {
            let huge = usize::MAX;
            match measure_scope_sizes(
                &emo2("shell/master"),
                &emo2("emo2-kakukaku"),
                &[huge],
                &MeasureScaling::IDENTITY,
            ) {
                Ok(v) => panic!("u32 超過 scope で Ok は誤成功（切り詰めている）: {v:?}"),
                Err(PlacementError::Measure { scope, reason }) => {
                    assert_eq!(scope, huge, "帰属は要求されたスコープ番号そのもの");
                    assert!(reason.starts_with("balloon: "), "接頭辞: {reason}");
                    assert!(reason.contains("u32 に収まらない"), "理由: {reason}");
                }
                Err(other) => panic!("Measure であるべき: {other:?}"),
            }
        });
    }

    /// 要件 3.1/3.3: 実 fixture 経路（native 採寸→k 適用）でも k≠1 が窓寸へ届く。
    /// `MeasureScaling::IDENTITY` の既存期待値（434×687 等）と厳密に 2 倍関係になる。
    #[test]
    fn measure_emo2_fixture_applies_k_end_to_end() {
        with_com_initialized(|| {
            let out = measure_scope_sizes(
                &emo2("shell/master"),
                &emo2("emo2-kakukaku"),
                &[0, 1],
                &MeasureScaling {
                    shell: k(2, 1),
                    balloon: ScaleRatio::ONE,
                },
            )
            .expect("k≠1 でも emo2 fixture の採寸は成功する");

            assert_eq!(out.scopes[0].char_size.w, SCOPE0_W * 2);
            assert_eq!(out.scopes[0].char_size.h, SCOPE0_H * 2);
            assert_eq!(out.scopes[1].char_size.w, SCOPE1_W * 2);
            assert_eq!(out.scopes[1].char_size.h, SCOPE1_H * 2);
            // balloon k は恒等ゆえ各 scope の原寸のまま（軸の独立性が実経路でも成立）。
            assert_eq!(
                (out.scopes[0].balloon_size.w, out.scopes[0].balloon_size.h),
                (BALLOON0_W, BALLOON0_H)
            );
            assert_eq!(
                (out.scopes[1].balloon_size.w, out.scopes[1].balloon_size.h),
                (BALLOON1_W, BALLOON1_H)
            );
        });
    }
}
