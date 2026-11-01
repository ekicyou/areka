//! I/O 層: surface／バルーンの原寸採寸（task 4.1・DD5）。
//!
//! emo2-boot と同じ公開 API 経路（シェル読み込みの権威
//! `areka_emo_present::shell_target::load_shell_target` → `ShellTarget::build_world` →
//! `Composer::compose`）で、各スコープの初期 surface と
//! **そのスコープが解決した**バルーン面 0 を **bind なし合成**し、原寸
//! （物理 px [`SizePx`]）を得る。
//!
//! - 初期 surface id: スコープごとに descript の既定の面の候補の並び
//!   （[`DefaultSurfaces::candidates`]）からシェルに在る最初の面（descript-window-keys 要件 2.1）
//! - バルーン面 0 のファイル選択は系列解決権威
//!   [`areka_emo_present::balloon::resolve_balloon_faces`] の消費のみで行う。採寸側は
//!   接頭辞連鎖もディレクトリ走査も持たない（列挙が 2 実装に分かれると採寸窓寸と実際に
//!   合成される枠がずれる・design D2）
//! - 合成失敗したスコープは scope0 の寸法で代替し `warn!`（窓自体は生やす——
//!   寸法だけ暫定・design「Error Handling」）
//! - scope0 の採寸自体／いずれかのスコープのバルーン面 0 の採寸が成立しない場合は
//!   代替の根拠が無いため [`PlacementError::Measure`]（log-first・呼び手の `main` が
//!   「起動窓を開けない」を告知して終える）
//! - 採寸に使ったアセット（`EmoWorld`／`AtlasTable`／WIC デコーダ）は本関数の
//!   ローカルとして採寸後に**破棄**される。戻り値は素の数値（[`ScopeInput`]）のみ
//!   （アセット所有・装着は emo2-boot の領分＝二重ロードは M1 受容トレードオフ）
//!
//! 呼び出しスレッドは COM 初期化済みであること（`WicDecoderArm` 前提・本番は
//! MTA UI スレッド）。wintf へは依存しない。
//!
//! # 読み手の段と記録（spec: placement-measure-bake-once）
//!
//! 採寸は窓の大きさを知るためだけの読み込みなので、読み込みの記録（シェル・透過の宣言・
//! 系列の解決）は出さない入口（`_quiet`）を呼ぶ。記録は起動の資産の組み立ての 1 回だけが出す
//! （要件 3.1・3.2）。採寸自身の記録（既定の面が無い・scope 0 で代える・失敗・k 適用）は残す。
//! 焼く段と面の表を組む段の記録も黙る（spec: emo-bake-log-overlap 要件 1.4・1.5・1.6・3.1）:
//! シェルは `load_shell_target_quiet`（焼く段と面の表の組み方も出さない）、バルーンの面 0 は
//! `build_balloon_target_from_faces_quiet` を通る。起動を止める失敗の `error!` は出す（要件 3.5）。
//! 読み手は [`measure_native_scope_sizes`]（本物の読み手を作る）→ [`measure_with_reader`]
//! （動く絵を 1 枚目だけで読む [`first_frame::FirstFrameDecoder`] で包む・数える読み手の
//! 差し込み口）→ [`measure_baked`]（受けた読み手で焼いて測る）の 3 段で渡る（要件 1.1）。

use std::path::Path;

use areka_emo_atlas::{AtlasTable, ElementDecoder, UseSelfAlpha, WicDecoderArm};
use areka_emo_compose::{BindSet, Composer, EmoWorld, PatternState, ScaleRatio};
use areka_emo_present::balloon::{
    ResolvedFace, build_balloon_target_from_faces_quiet, load_balloon_use_self_alpha_quiet,
    resolve_balloon_faces_quiet,
};
use areka_emo_present::shell_target::load_shell_target_quiet;
use tracing::{error, warn};

use super::PlacementError;
use super::descript_window_keys::{DefaultSurfaces, builtin_default_surface, first_existing};
use super::resolver::{ScopeInput, SizePx};

// 採寸の読み手（動く絵を 1 枚目だけで読む包み）。
#[path = "measure_first_frame.rs"]
mod first_frame;

/// 採寸結果（design「placement::measure」Service Interface）。
///
/// アセット（`EmoWorld`/`AtlasTable`）は持たない——素の物理 px 数値のみ
/// （採寸後破棄の契約を型で担保する）。
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

/// 各スコープの初期 surface（`default_surfaces` の候補の並びからシェルに在る最初の面）と
/// **そのスコープが解決した**バルーン面 0 を bind なし合成して原寸（物理 px）を得る
/// （2.9 の幅供給源・3.2 の物理 px 単一通貨・DD5）。
///
/// - 合成失敗したスコープは scope0 の寸法で代替し `warn!`（窓自体は生やす）
/// - scope0 の採寸自体（shell アセット構築含む）・いずれかのスコープのバルーン面 0 の
///   採寸が成立しない場合は代替根拠が無いため `Err(PlacementError::Measure)`
///   （log-first・呼び手の `main` が「起動窓を開けない」を告知して終える）。
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
pub fn measure_scope_sizes(
    shell_dir: &Path,
    balloon_root: &Path,
    scope_ids: &[usize],
    default_surfaces: &DefaultSurfaces,
    scaling: &MeasureScaling,
) -> Result<MeasuredSizes, PlacementError> {
    // (1) native 採寸（原寸・k 非適用）。
    let natives = measure_native_scope_sizes(shell_dir, balloon_root, scope_ids, default_surfaces)?;
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
    default_surfaces: &DefaultSurfaces,
) -> Result<Vec<ScopeInput>, PlacementError> {
    // 実 WIC デコーダ（COM 初期化済みスレッド前提・donor build_and_spawn と同型）。
    let decoder = WicDecoderArm::new().map_err(|e| {
        error!(error = ?e, "measure: WicDecoderArm 生成に失敗（COM 未初期化？）");
        PlacementError::Measure {
            scope: 0,
            reason: format!("WicDecoderArm 生成失敗: {e}"),
        }
    })?;
    measure_with_reader(
        &decoder,
        shell_dir,
        balloon_root,
        scope_ids,
        default_surfaces,
    )
    // decoder はここで破棄される（採寸後破棄の契約）。
}

/// 受けた読み手を採寸の読み手（動く絵を 1 枚目だけで読む）で包んで測る（要件 1.1）。
///
/// 数える読み手を差し込む口。戻り値は [`measure_baked`] に包まない読み手を渡したときと
/// 等しい（要件 2.2）。
fn measure_with_reader(
    reader: &impl ElementDecoder,
    shell_dir: &Path,
    balloon_root: &Path,
    scope_ids: &[usize],
    default_surfaces: &DefaultSurfaces,
) -> Result<Vec<ScopeInput>, PlacementError> {
    let decoder = first_frame::FirstFrameDecoder::new(reader);
    measure_baked(
        &decoder,
        shell_dir,
        balloon_root,
        scope_ids,
        default_surfaces,
    )
}

/// 受けた読み手で焼いて scope ごとの原寸を測る（native 採寸の本体）。
///
/// 読み込みは記録を出さない入口で行う（要件 3.1・3.2）。採寸自身の記録は残す（要件 2.4・2.5）。
fn measure_baked(
    decoder: &impl ElementDecoder,
    shell_dir: &Path,
    balloon_root: &Path,
    scope_ids: &[usize],
    default_surfaces: &DefaultSurfaces,
) -> Result<Vec<ScopeInput>, PlacementError> {
    // shell アセット（シェル読み込みの権威 `load_shell_target` の記録を出さない入口を 1 回）。
    let (shell_world, shell_atlas) = build_shell_assets(shell_dir, decoder)?;
    let mut composer = Composer::new();

    // スコープの測る面: 候補の並びのうちシェルに在る最初の面。どれも無ければ組み込みの面で
    // 合成し、今の失敗の扱い（本体側は Err・相方側は本体側の大きさ）へ委ねる（要件 2.4）。
    let initial_surface = |scope: usize| {
        let candidates = default_surfaces.candidates(scope);
        match first_existing(&candidates, |id| shell_world.surface(id).is_some()) {
            Some(id) => {
                if id != candidates[0] {
                    warn!(
                        event = "default_surface_missing",
                        scope,
                        missing = candidates[0],
                        to = id,
                        "default_surface_missing: descript の既定の面がシェルに無い——組み込みの面で測る"
                    );
                }
                id
            }
            None => builtin_default_surface(scope),
        }
    };

    // 基準採寸: scope0 初期 surface。失敗は代替根拠が無い＝ hard Err。
    let scope0_id = initial_surface(0);
    let scope0_size =
        compose_size(&mut composer, &shell_world, &shell_atlas, scope0_id).map_err(|reason| {
            error!(surface_id = scope0_id, reason = %reason, "measure: scope0 の採寸合成に失敗");
            PlacementError::Measure { scope: 0, reason }
        })?;

    // バルーンの透過の宣言は 1 回だけ読む（記録は出さない・起動側と同じ読みの記録を出さない入口）。
    let balloon_alpha = load_balloon_use_self_alpha_quiet(balloon_root);
    let mut scopes = Vec::with_capacity(scope_ids.len());
    for &scope in scope_ids {
        let char_size = if scope == 0 {
            scope0_size
        } else {
            let surface_id = initial_surface(scope);
            match compose_size(&mut composer, &shell_world, &shell_atlas, surface_id) {
                Ok(size) => size,
                Err(reason) => {
                    // 採寸失敗スコープは scope0 寸法で代替（窓自体は生やす・design Error Handling）。
                    warn!(
                        scope,
                        surface_id,
                        reason = %reason,
                        "measure: スコープの採寸合成に失敗（scope0 の寸法で代替）"
                    );
                    scope0_size
                }
            }
        };
        // バルーンは **当該 scope が解決した系列の面 0**（全 scope 共通の 1 回へ畳まない・
        // 要件 3.1）。失敗は代替根拠が無く hard Err で、帰属は実 scope 番号。
        let balloon_size =
            measure_balloon_surface0(balloon_root, decoder, &mut composer, scope, balloon_alpha)?;
        scopes.push(ScopeInput {
            scope,
            char_size,
            balloon_size,
        });
    }

    // shell_world／shell_atlas はここで破棄される（採寸後破棄の契約）。
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

/// shell dir から採寸用の `(EmoWorld, AtlasTable)` を組む（シェル読み込みの権威を呼ぶだけ）。
///
/// 一覧・`surfaces.txt` の読取と解析・「番号 → 面の画像」の決定・bake はすべて
/// [`areka_emo_present::shell_target::load_shell_target`] が行う。起動時の資産組立
/// （`emo2_boot::assets::build_boot_assets`）も同じ入口を通るので、採寸した窓寸と実際に
/// 合成される枠が食い違わない（要件 3.6——列挙の規則が 2 実装に分かれると実機でしか現れない
/// 欠陥になる。`measure_balloon_surface0` の説明と同じ理由）。
///
/// 失敗の 3 種（一覧・読取・surface 0 個）はいずれも代替の根拠が無いので
/// [`PlacementError::Measure`]（`scope: 0`）へ畳む。真因のログは権威が `error!` で既に出して
/// いるため、ここでは出し直さない。
fn build_shell_assets(
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
) -> Result<(EmoWorld, AtlasTable), PlacementError> {
    let target =
        load_shell_target_quiet(shell_dir, decoder).map_err(|reason| PlacementError::Measure {
            scope: 0,
            reason: reason.to_string(),
        })?;
    let atlas = target.atlas().clone();
    Ok((target.build_world(), atlas))
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
/// [`build_balloon_target_from_faces_quiet`]（記録を出さない兄弟の入口）をそのまま用い、
/// synthetic surfaces.txt の書式まで含めて実装を 1 本に保つ（起動時資産構築と採寸が同じ絵を見る保証）。
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
    decoder: &impl ElementDecoder,
    composer: &mut Composer,
    scope: usize,
    use_self_alpha: UseSelfAlpha,
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
    let faces = resolve_balloon_faces_quiet(balloon_root, scope_key)
        .map_err(|e| balloon_err(format!("系列解決に失敗: {e}")))?;
    // 採寸対象は面 0 のみ（採用面列の残りは bake しない）。
    let face0: Vec<ResolvedFace> = faces.into_iter().filter(|f| f.surface_id == 0).collect();

    let (world, atlas) =
        build_balloon_target_from_faces_quiet(balloon_root, decoder, &face0, use_self_alpha)
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

// 採寸と表示の外形の一致（検体 2 体）と、読み込みの複製が戻っていないことの本文走査。
// `measure_tests.rs` は 1,000 行の上限に余裕が無いので兄弟として置く
// （spec: areka-P0-shell-implicit-surface 要件 7.12）。
#[cfg(test)]
#[path = "measure_template_tests.rs"]
mod template_tests;

// 測りが descript の既定の面で合成することの 3 通り（descript-window-keys 要件 7.2）。
#[cfg(test)]
#[path = "measure_default_surface_tests.rs"]
mod default_surface_tests;

// 採寸の読み手 FirstFrameDecoder の檻（spec: placement-measure-bake-once 要件 1.1・1.4・3.7）。
#[cfg(test)]
#[path = "measure_first_frame_tests.rs"]
mod first_frame_tests;

// 採寸と資産の組み立ての焼く回数の檻（spec: placement-measure-bake-once 要件 1.1・1.2・4.1）。
#[cfg(test)]
#[path = "measure_bake_count_tests.rs"]
mod bake_count_tests;

// 起動の準備 1 回の行の数の檻（spec: emo-bake-log-overlap 要件 6.1・6.3〜6.6）。
#[cfg(test)]
#[path = "measure_bake_log_tests.rs"]
mod bake_log_tests;
