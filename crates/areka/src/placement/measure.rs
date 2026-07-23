//! I/O 層: surface／バルーンの原寸採寸（task 4.1・DD5）。
//!
//! emo2-boot と同系の公開 API 経路（`areka_parsers::shell::parse` →
//! `areka_emo_atlas::bake` → `EmoWorld::build`＋`bind_atlas` →
//! `Composer::compose`・donor: `crates/areka/examples/emo-present.rs` の
//! `build_shell_target`／`build_balloon_assets`）で、各スコープの初期 surface と
//! balloon surface0 を **bind なし合成**し、原寸（物理 px [`SizePx`]）を得る。
//!
//! - 初期 surface id: scope0 → `0`・scope1 → `10`（ukadoc 正典: 相方既定
//!   サーフェス）・scope n≥2 → `10`（正典既定なし・`warn!` 付き暫定）
//! - 合成失敗したスコープは scope0 の寸法で代替し `warn!`（窓自体は生やす——
//!   寸法だけ暫定・design「Error Handling」）
//! - scope0 の採寸自体／balloon surface0 の採寸が成立しない場合は代替の根拠が
//!   無いため [`PlacementError::Measure`]（log-first・シームがフォールバック・DD14）
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
use areka_emo_compose::{BindSet, Composer, EmoWorld, PatternState};
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

/// 各スコープの初期 surface（scope0=id0・scope1=id10・scope n≥2=id10 暫定＋warn）と
/// balloon surface0 を bind なし合成して原寸（物理 px）を得る（2.9 の幅供給源・
/// 3.2 の物理 px 単一通貨・DD5）。
///
/// - 合成失敗したスコープは scope0 の寸法で代替し `warn!`（窓自体は生やす）
/// - scope0 の採寸自体（shell アセット構築含む）・balloon surface0 の採寸が
///   成立しない場合は代替根拠が無いため `Err(PlacementError::Measure)`
///   （log-first・シームが `spawn_dummy_window` へフォールバック・DD14）。
///   balloon 起因の失敗は `reason` に `balloon:` 接頭辞を付け `scope: 0` で報告する
///   （バルーン専用 variant は持たない・task 1 の失敗型を消費）
/// - 採寸に使ったアセットは本関数のローカルとして return 時に破棄される
///
/// 呼び出しスレッドは COM 初期化済みであること（本番＝MTA UI スレッド）。
#[allow(dead_code)] // scaffold（task 4.1）: 結線は task 6
pub fn measure_scope_sizes(
    shell_dir: &Path,
    balloon_root: &Path,
    scope_ids: &[usize],
) -> Result<MeasuredSizes, PlacementError> {
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

    // balloon surface0 の採寸（全スコープ共通・失敗は hard Err）。
    let balloon_size = measure_balloon_surface0(balloon_root, &decoder, &mut composer)?;

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
        scopes.push(ScopeInput {
            scope,
            char_size,
            balloon_size,
        });
    }

    // shell_world／shell_atlas／decoder はここで破棄される（採寸後破棄の契約）。
    Ok(MeasuredSizes { scopes })
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

/// balloon_root の枠画像 `balloons0.png`（大小無視）を synthetic surfaces.txt へ転記し、
/// シェルと同一の公開 API 経路（parse→bake→build＋bind→compose）で surface0 を採寸する
/// （donor `build_balloon_target`→採寸合成の最小再実装。採寸に要るのは surface0 のみ
/// ゆえ枠 0 番だけを転記する）。
fn measure_balloon_surface0(
    balloon_root: &Path,
    decoder: &WicDecoderArm,
    composer: &mut Composer,
) -> Result<SizePx, PlacementError> {
    let balloon_err = |reason: String| {
        error!(
            balloon_root = %balloon_root.display(),
            reason = %reason,
            "measure: balloon surface0 の採寸に失敗"
        );
        PlacementError::Measure {
            scope: 0,
            reason: format!("balloon: {reason}"),
        }
    };

    // `balloons0.png`（大小無視・実ファイル名は原形保持＝実 WIC が実パスを読む）を探す。
    // `balloonc*`/`balloonk*`/`arrow*` 等の非枠は名前不一致で自然に外れる（donor と同じ分類）。
    let read_dir = std::fs::read_dir(balloon_root)
        .map_err(|e| balloon_err(format!("枠画像ディレクトリの走査失敗: {e}")))?;
    let mut frame0: Option<String> = None;
    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, "measure: balloon ディレクトリエントリの取得に失敗（スキップ）");
                continue;
            }
        };
        if let Some(name) = entry.file_name().to_str()
            && name.eq_ignore_ascii_case("balloons0.png")
        {
            frame0 = Some(name.to_string());
            break;
        }
    }
    let frame0 =
        frame0.ok_or_else(|| balloon_err("枠画像 balloons0.png が見つからない".to_string()))?;

    // synthetic surfaces.txt（surface0 に単一 overlay element・donor と同一書式）。
    let text = format!("surface0\n{{\nelement0,overlay,{frame0},0,0\n}}\n");
    let shell = areka_parsers::shell::parse(&text);

    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: balloon_root,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On, // PNG α 尊重（emo2 kakukaku 実測・donor R5.2 同値）
        },
    };
    let baked = bake(&[set], decoder, PackConfig::default());
    if !baked.errors.is_empty() {
        let reasons: Vec<String> = baked.errors.iter().map(|e| e.to_string()).collect();
        return Err(balloon_err(format!("枠画像の bake に失敗: {}", reasons.join("; "))));
    }

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&baked.table, SetId(0));

    compose_size(composer, &world, &baked.table, 0).map_err(balloon_err)
    // world／baked.table はここで破棄される（採寸後破棄の契約）。
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
    /// balloon surface0（balloons0.png）。
    const BALLOON_W: i32 = 400;
    const BALLOON_H: i32 = 224;

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
            // balloon surface0（balloons0.png 原寸）は全スコープ共通。
            for s in &out.scopes {
                assert_eq!(s.balloon_size.w, BALLOON_W);
                assert_eq!(s.balloon_size.h, BALLOON_H);
                // 要件の観測文言どおり「非ゼロ物理 px」も明示的に固定する。
                assert!(s.char_size.w > 0 && s.char_size.h > 0, "char 原寸は非ゼロ");
                assert!(
                    s.balloon_size.w > 0 && s.balloon_size.h > 0,
                    "balloon 原寸は非ゼロ"
                );
            }
        });
    }

    /// scope n≥2 は正典既定が無く id10 暫定（warn 付き）＝scope1 と同寸になる。
    #[test]
    fn scope_n_ge_2_measures_interim_id10() {
        with_com_initialized(|| {
            let out = measure_scope_sizes(
                &emo2("shell/master"),
                &emo2("emo2-kakukaku"),
                &[0, 1, 2],
            )
            .expect("scope2 を含んでも採寸は成功する");

            assert_eq!(out.scopes.len(), 3);
            assert_eq!(out.scopes[2].scope, 2);
            assert_eq!(
                out.scopes[2].char_size, out.scopes[1].char_size,
                "scope n≥2 は id10 暫定＝scope1 と同寸"
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

            let out = measure_scope_sizes(shell.path(), &emo2("emo2-kakukaku"), &[0, 1])
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

            match measure_scope_sizes(empty_shell.path(), &emo2("emo2-kakukaku"), &[0]) {
                Ok(_) => panic!("surfaces.txt 不在で Ok は誤成功"),
                Err(PlacementError::Measure { scope, .. }) => {
                    assert_eq!(scope, 0, "shell 全体の失敗は scope0 起点で報告")
                }
                Err(other) => panic!("Measure であるべき: {other:?}"),
            }
        });
    }

    /// balloon surface0 が成立しない（枠画像なし）場合も `PlacementError::Measure`
    /// （`ScopeInput.balloon_size` の供給源が無い＝採寸全体の失敗）。
    #[test]
    fn missing_balloon_frame_is_measure_error() {
        with_com_initialized(|| {
            let empty_balloon = TempDir::new();

            match measure_scope_sizes(&emo2("shell/master"), empty_balloon.path(), &[0]) {
                Ok(_) => panic!("balloon 枠不在で Ok は誤成功"),
                Err(PlacementError::Measure { .. }) => {}
                Err(other) => panic!("Measure であるべき: {other:?}"),
            }
        });
    }
}
