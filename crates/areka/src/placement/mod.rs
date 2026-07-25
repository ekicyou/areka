//! 窓配置機構（areka-P0-window-placement）のモジュール土台。
//!
//! ゴースト定義（shell dir＋ghost/shell descript の KV）からキャラ窓・バルーン窓の
//! 初期配置を解決し、窓 entity を組み立てる配置パイプラインの器。
//! 座標単位契約（design 正本 U1〜U5）に従い、配置パイプラインの座標・寸法は
//! **すべて物理 px 単一通貨**とする（論理 DIP・`BoxStyle` は持ち込まない）。
//!
//! 依存方向（design「Architecture Pattern & Boundary Map」の強制規約）:
//! `resolver`（純粋・std のみ）← `config`（areka-parsers のみ）←
//! `measure`（emo-atlas/compose）← `spawn`／`follow`（wintf/bevy_ecs）← main.rs シーム。
//! 左のモジュールは右へ import しない。
//!
//! 本ファイルはサブモジュール宣言・失敗型 [`PlacementError`]（task 1）に加え、
//! 配置準備の合成ルート [`prepare_ghost_windows`]（task 6.1・design「main.rs seam」）
//! を持つ。design File Structure の「mod.rs＝モジュール公開面」に従い、
//! source→config→measure→resolver を束ねる準備関数の自然な置き場として
//! ここ（合成ルート）に実装する（シームの結線自体は task 6.2・main.rs 側）。

pub mod config;
pub mod follow;
pub mod measure;
pub mod persist;
pub mod resolver;
pub mod source;
pub mod spawn;

use std::path::{Path, PathBuf};

use areka_parsers::package::MountError;
use tracing::{error, warn};
use wintf::ecs::window::monitor::{Monitor, enumerate_monitors};

use self::config::PlacementConfig;
use self::measure::MeasuredSizes;
use self::resolver::{RectPx, ScopePlacement};
use self::source::GhostTitles;

/// 配置準備パイプライン（resolve→descript 読込→採寸→解決）の観測可能な失敗。
///
/// design「Error Handling」準拠: 安易な panic 禁止・失敗は `error!`＋`Err`。
/// すべて main.rs シームで捕捉され `spawn_dummy_window` フォールバックへ
/// 落ちる（DD14・log-first）。
#[allow(dead_code)] // scaffold（task 1）: 利用側は後続タスクで実装
#[derive(Debug, thiserror::Error)]
pub enum PlacementError {
    /// ゴーストパッケージのマウント解決（`areka_parsers::package::resolve`）失敗。
    ///
    /// `MountError` は `std::error::Error` 未実装のため `#[from]`/`#[source]`
    /// にせず値として保持し `Debug` 表示する。
    #[error("ゴーストのマウント解決に失敗: {0:?}")]
    Mount(MountError),

    /// descript.txt の読み取り失敗（I/O エラー）。
    #[error("descript の読み取りに失敗: {path}")]
    DescriptRead {
        /// 読み取れなかった descript.txt のパス。
        path: PathBuf,
        /// 元の I/O エラー。
        source: std::io::Error,
    },

    /// surface 採寸（emo-atlas/compose による原寸合成）失敗。
    #[error("scope {scope} の surface 採寸に失敗: {reason}")]
    Measure {
        /// 採寸対象のスコープ番号。
        scope: usize,
        /// 失敗理由（下流の詳細を文字列化）。
        reason: String,
    },

    /// モニタ列挙が 0 台で primary work area の出所がない（2.12 の基準を
    /// 満たせない）。架空の既定矩形は発明せず呼び手（シーム）のフォールバックへ
    /// 委ねる（DD14）。task 6.1 で追加（mod.rs＝準備関数の置き場は本タスク境界内）。
    #[error("モニタ列挙に失敗: {reason}")]
    Monitor {
        /// 失敗理由（列挙結果の状況を文字列化）。
        reason: String,
    },
}

/// placement 側の同期準備一括の結果（design「main.rs seam」正本）。
///
/// I/O は [`prepare_ghost_windows`] までで完結し、**Send な素の値のみ**を運ぶ
/// （`Vec<ScopePlacement>`＝Copy 値の列・`GhostTitles`＝`BTreeMap<usize, String>`。
/// COM/WIC 等のスレッド親和リソースは持たない。Send 契約はテストで固定）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlacement {
    /// スコープごとの解決済み配置（`resolve_placement` の出力そのまま・scope 昇順）。
    pub placements: Vec<ScopePlacement>,
    /// 窓タイトルの正本（spawn（task 5.1）が消費）。
    pub titles: GhostTitles,
}

/// 準備パイプラインの中間結果（load→config→measure まで・work area 非依存部）。
///
/// [`prepare_ghost_windows`]（実モニタ列挙）と
/// [`prepare_ghost_windows_with_work_area`]（合成 work area 注入・決定論テスト用）
/// が resolve 直前まで同一経路を共有するための内部型。
struct PreparedStages {
    cfg: PlacementConfig,
    sizes: MeasuredSizes,
    titles: GhostTitles,
}

impl PreparedStages {
    /// work area（物理 px）を与えて配置を確定する（純粋・resolver P1〜P5）。
    fn resolve(self, work_area: RectPx) -> PreparedPlacement {
        let placements = resolver::resolve_placement(&self.cfg, work_area, &self.sizes.scopes);
        PreparedPlacement {
            placements,
            titles: self.titles,
        }
    }
}

/// 準備パイプラインの work area 非依存部を同期実行する:
/// `load_descript_source` → `build_placement_config` → `measure_scope_sizes`。
///
/// 失敗はフォールバックせず [`PlacementError`] のまま呼び手へ返す（DD14:
/// `spawn_dummy_window` フォールバックは main.rs シームの分担）。
/// 位置の記憶・復元（`ghost.dat` 読み書き）は一切行わない（2.11・テストで固定）。
fn prepare_stages(ghost_root: &Path, balloon_root: &Path) -> Result<PreparedStages, PlacementError> {
    let src = source::load_descript_source(ghost_root)?;
    let cfg = config::build_placement_config(&src.ghost_kv, &src.shell_kv);
    let scope_ids: Vec<usize> = cfg.scopes.keys().copied().collect();
    // k₀ は task 4.3（main.rs boot シーム）が primary モニタ DPI ÷ author_dpi から構築して
    // 供給する。それまでは恒等スケール＝従来と同一の native 採寸で挙動不変（task 5・R7.2）。
    let sizes = measure::measure_scope_sizes(
        &src.shell_dir,
        balloon_root,
        &scope_ids,
        &measure::MeasureScaling::IDENTITY,
    )?;
    Ok(PreparedStages {
        cfg,
        sizes,
        titles: src.titles,
    })
}

/// 窓配置の準備処理（design「main.rs seam」・task 6.1）。
///
/// `load_descript_source` → `build_placement_config` → `measure_scope_sizes` →
/// `enumerate_monitors()` の `is_primary` モニタ work area 取得（2.12）→
/// `resolve_placement` の順に**同期実行**し、Send な結果のみの
/// [`PreparedPlacement`] を返す。
///
/// - 準備段階の失敗は `Err(PlacementError)` のまま返す（本関数はフォールバック
///   しない・DD14: フォールバックは呼び手＝main.rs シームの分担）
/// - 位置の記憶・復元（`ghost.dat` 読み書き）は一切行わない（2.11）
/// - 呼び出しスレッドは COM 初期化済みであること（measure の `WicDecoderArm`
///   前提・本番は MTA UI スレッド）
pub fn prepare_ghost_windows(
    ghost_root: &Path,
    balloon_root: &Path,
) -> Result<PreparedPlacement, PlacementError> {
    let stages = prepare_stages(ghost_root, balloon_root)?;
    let monitors = enumerate_monitors();
    let work_area = primary_work_area(&monitors)?;
    Ok(stages.resolve(work_area))
}

/// [`prepare_ghost_windows`] の work area 注入版（決定論テスト用の偽装境界）。
///
/// 実モニタ列挙（`enumerate_monitors`）だけを合成 work area で置き換え、
/// それ以外（load→config→measure→resolve）は本番と同一経路を通す
/// （記憶 prefer-x64-fake-boundary-tests の流儀。headless 環境でも emo2 fixture
/// の観測可能な完了状態を決定論的に検証できる）。
#[allow(dead_code)] // scaffold（task 6.1）: テスト専用の偽装境界（本番は prepare_ghost_windows）
pub fn prepare_ghost_windows_with_work_area(
    ghost_root: &Path,
    balloon_root: &Path,
    work_area: RectPx,
) -> Result<PreparedPlacement, PlacementError> {
    Ok(prepare_stages(ghost_root, balloon_root)?.resolve(work_area))
}

/// モニタ列挙結果から primary モニタの work area（物理 px）を取り出す（2.12）。
///
/// - `is_primary` のモニタの `work_area`（`RECT`・物理 px）を [`RectPx`] へ
///   **単位変換なしで忠実転写**する（U 契約: どちらも物理 px 通貨）
/// - primary フラグ無し（列挙異常）: `warn!` の上で先頭モニタを代替に用いる
///   （窓は生やす方針・design「Error Handling」）
/// - 0 台: `error!`＋`Err(PlacementError::Monitor)`（架空の既定矩形は発明しない・
///   フォールバックはシームの分担）
fn primary_work_area(monitors: &[Monitor]) -> Result<RectPx, PlacementError> {
    let monitor = match monitors.iter().find(|m| m.is_primary) {
        Some(primary) => primary,
        None => match monitors.first() {
            Some(first) => {
                warn!(
                    monitor_count = monitors.len(),
                    "primary フラグを持つモニタが見つからない（列挙異常）——先頭モニタで代替する"
                );
                first
            }
            None => {
                error!("モニタが 1 台も列挙されない——primary work area の出所がない");
                return Err(PlacementError::Monitor {
                    reason: "enumerate_monitors() が 0 台を返した".to_string(),
                });
            }
        },
    };
    let wa = monitor.work_area;
    Ok(RectPx {
        left: wa.left,
        top: wa.top,
        right: wa.right,
        bottom: wa.bottom,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    use windows::Win32::Foundation::RECT;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
    use wintf::ecs::window::monitor::Monitor;

    use super::resolver::{PointPx, RectPx, SizePx};
    use super::*;

    /// COM 初期化下でクロージャを実行する（measure が `WicDecoderArm` を要求・
    /// measure.rs テストと同一パターン）。
    fn with_com_initialized<F: FnOnce()>(f: F) {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        f();
        unsafe {
            CoUninitialize();
        }
    }

    /// emo2 fixture ルート（source.rs／measure.rs テストと同一アンカー規約）。
    fn emo2_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2")
    }

    /// emo2 fixture のバルーンルート（task 4.1 テストの規約を踏襲）。
    fn balloon_root() -> PathBuf {
        emo2_root().join("emo2-kakukaku")
    }

    /// 決定論テスト用の合成 work area（物理 px・resolver T-R 群と同流儀）。
    const WA: RectPx = RectPx {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
    };

    // ── テスト用一時ディレクトリ（measure.rs テストと同じ std-only 最小実装）──

    static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Drop 時に自身を再帰削除する一時ディレクトリ。
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "areka-placement-prepare-{}-{}",
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

    /// テスト用合成 Monitor（実 HMONITOR 不要・wintf monitor.rs テストと同流儀）。
    fn make_monitor(handle: isize, work: (i32, i32, i32, i32), is_primary: bool) -> Monitor {
        let (left, top, right, bottom) = work;
        Monitor {
            handle,
            bounds: RECT {
                left,
                top,
                right,
                bottom: bottom + 40, // work_area はタスクバーぶん狭い想定
            },
            work_area: RECT {
                left,
                top,
                right,
                bottom,
            },
            dpi: 96,
            is_primary,
        }
    }

    // ------------------------------------------------------------------
    // 観測可能な完了状態（tasks 6.1）: emo2 fixture 相当のパスで 2 スコープぶんの
    // ScopePlacement を含む PreparedPlacement が返る
    // ------------------------------------------------------------------

    /// emo2 fixture＋合成 work area で `prepare_ghost_windows_with_work_area` が
    /// 2 スコープの `ScopePlacement`（resolver P1〜P5 の厳密値）と titles を返す。
    ///
    /// 期待値の根拠（emo2 実測: alignment=bottom・defaultx=0×2・
    /// balloon.alignment=left/right・寸法 434×687／336×400／balloon 400×224）:
    /// - scope0: char=(1920−434, 1040−687)=(1486,353)・balloon 左隣=(1086,353)
    /// - scope1: char=(1486−434, 1040−400)=(1052,640)・balloon 右隣=(1388,640)
    #[test]
    fn prepare_emo2_returns_two_scope_placements() {
        with_com_initialized(|| {
            let p = prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA)
                .expect("emo2 fixture の配置準備は成功する");

            assert_eq!(p.placements.len(), 2, "emo2 は 2 スコープ（DD6）");

            let s0 = &p.placements[0];
            assert_eq!(s0.scope, 0);
            assert_eq!(s0.char_pos, PointPx { x: 1486, y: 353 });
            assert_eq!(s0.char_size, SizePx { w: 434, h: 687 });
            assert_eq!(s0.balloon_pos, PointPx { x: 1086, y: 353 });
            assert_eq!(s0.balloon_size, SizePx { w: 400, h: 224 });
            assert_eq!(s0.balloon_offset, PointPx { x: -400, y: 0 });

            let s1 = &p.placements[1];
            assert_eq!(s1.scope, 1);
            assert_eq!(s1.char_pos, PointPx { x: 1052, y: 640 });
            assert_eq!(s1.char_size, SizePx { w: 336, h: 400 });
            assert_eq!(s1.balloon_pos, PointPx { x: 1388, y: 640 });
            assert_eq!(s1.balloon_offset, PointPx { x: 336, y: 0 });

            // titles は MountModel.names 由来（source T-I5 と同値）
            assert_eq!(p.titles.title(0), "むらさき");
            assert_eq!(p.titles.title(1), "エモ");
        });
    }

    /// 薄いラッパ `prepare_ghost_windows` は実モニタ列挙で primary work area を
    /// 取得し、同じ work area を与えた合成経路と同一の結果を返す。
    /// モニタ 0 台の環境（headless）では `PlacementError::Monitor` が返る
    /// （環境寛容: どちらの分岐でも決定論的に検証する）。
    #[test]
    fn prepare_ghost_windows_uses_primary_monitor() {
        with_com_initialized(|| {
            let monitors = wintf::ecs::window::monitor::enumerate_monitors();
            match prepare_ghost_windows(&emo2_root(), &balloon_root()) {
                Ok(p) => {
                    assert!(
                        !monitors.is_empty(),
                        "モニタ 0 台で Ok は誤成功（work area の出所がない）"
                    );
                    assert_eq!(p.placements.len(), 2);
                    // 同一 work area を注入した合成経路と一致（薄いラッパは委譲のみ）
                    let wa = primary_work_area(&monitors).expect("primary work area");
                    let q =
                        prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), wa)
                            .expect("合成経路も成功する");
                    assert_eq!(p.placements, q.placements);
                }
                Err(PlacementError::Monitor { .. }) => {
                    assert!(
                        monitors.is_empty(),
                        "モニタが存在するのに Monitor エラーが返った"
                    );
                }
                Err(other) => panic!("Monitor 以外の Err は契約外: {other:?}"),
            }
        });
    }

    // ------------------------------------------------------------------
    // primary_work_area（純粋・合成 Monitor で決定論）
    // ------------------------------------------------------------------

    /// モニタ 0 台は `PlacementError::Monitor`（架空の既定矩形を発明しない）。
    #[test]
    fn primary_work_area_empty_is_monitor_err() {
        let err = primary_work_area(&[]).expect_err("0 台は Err");
        assert!(
            matches!(err, PlacementError::Monitor { .. }),
            "Monitor variant 以外が返った: {err:?}"
        );
    }

    /// `is_primary` のモニタの work area（物理 px）が RectPx へ忠実転写される
    /// （単位変換なし・2.12／U 契約）。
    #[test]
    fn primary_work_area_picks_is_primary() {
        let monitors = [
            make_monitor(1, (-1920, 0, 0, 1040), false),
            make_monitor(2, (0, 0, 2560, 1400), true),
        ];
        let wa = primary_work_area(&monitors).expect("primary あり");
        assert_eq!(
            wa,
            RectPx {
                left: 0,
                top: 0,
                right: 2560,
                bottom: 1400
            }
        );
    }

    /// primary フラグ無し（列挙異常）は先頭モニタで代替（warn・窓は生やす方針）。
    #[test]
    fn primary_work_area_no_primary_substitutes_first() {
        let monitors = [
            make_monitor(1, (0, 0, 1920, 1040), false),
            make_monitor(2, (1920, 0, 3840, 1040), false),
        ];
        let wa = primary_work_area(&monitors).expect("非空なら Ok");
        assert_eq!(
            wa,
            RectPx {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040
            }
        );
    }

    // ------------------------------------------------------------------
    // MonitorSnapshot::from_monitors（task 8.1・DD15・実モニタ→snapshot 忠実転写）
    // ------------------------------------------------------------------

    /// `MonitorSnapshot::from_monitors` は全モニタの work area（物理 px）を列挙順の
    /// まま**単位変換なしで忠実転写**する（`primary_work_area` と同じ U 契約）。
    #[test]
    fn monitor_snapshot_from_monitors_transcribes_all_work_areas_in_order() {
        let monitors = [
            make_monitor(1, (-1920, -40, 0, 1000), false),
            make_monitor(2, (0, 0, 2560, 1400), true),
        ];
        let snapshot = follow::MonitorSnapshot::from_monitors(&monitors);
        assert_eq!(
            snapshot.work_areas,
            vec![
                RectPx {
                    left: -1920,
                    top: -40,
                    right: 0,
                    bottom: 1000
                },
                RectPx {
                    left: 0,
                    top: 0,
                    right: 2560,
                    bottom: 1400
                },
            ]
        );
    }

    /// 0 台では空 snapshot（panic しない・消費側 `work_area_for_window` が None 防御）。
    #[test]
    fn monitor_snapshot_from_monitors_empty_is_empty() {
        assert!(
            follow::MonitorSnapshot::from_monitors(&[])
                .work_areas
                .is_empty()
        );
    }

    // ------------------------------------------------------------------
    // Send 契約・失敗経路・永続化なし（2.11）
    // ------------------------------------------------------------------

    /// `PreparedPlacement` は Send（design「main.rs seam」: Send な結果のみ返す）。
    #[test]
    fn prepared_placement_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<PreparedPlacement>();
    }

    /// 準備段階の失敗（ghost_root 不在→Mount）はこの関数内でフォールバックせず
    /// `PlacementError` として呼び手（シーム）へ返る（DD14 の分担）。
    /// 失敗は load（モニタ列挙より前）で起きるため headless でも決定論。
    #[test]
    fn prepare_missing_root_returns_mount_err_to_caller() {
        let root = std::env::temp_dir()
            .join("areka_placement_prepare_missing_root")
            .join("no_such_ghost");
        let err =
            prepare_ghost_windows(&root, &balloon_root()).expect_err("不在 root は Err");
        assert!(
            matches!(err, PlacementError::Mount(_)),
            "Mount variant 以外が返った: {err:?}"
        );
    }

    /// `prepare_ghost_windows` は永続を一切読み書きしない（2.11・A1）:
    /// (a) 実行後どこにも ghost.dat を生成しない（書き込みなし）
    /// (b) 偽の保存位置を持つ ghost.dat を plant しても出力が変わらない（読み込みなし）
    ///
    /// 永続ストアの実体は現在、統一プロパティシステム（sylphya）の別ファイル
    /// `sylphya.toml` にある——これは本 spec（areka-P0-position-persist）が
    /// **消費**する別系統であり、SSP 由来の `ghost.dat` とは無関係。復元の結線は
    /// placement シームの [`persist`] モジュール（と後続タスクの merge シーム）が担い、
    /// `prepare_ghost_windows` 自身は依然として永続を一切読み書きしない（A1＝prepare
    /// 不触は真のまま）。本檻は legacy な ghost.dat probe でその不触性を固定し続ける
    /// （sylphya.toml 系統でも同じく prepare は不触）。
    #[test]
    fn prepare_never_reads_or_writes_ghost_dat() {
        with_com_initialized(|| {
            // 最小の合成ゴースト（emo2 実 PNG を 2 枚だけ複写・descript は emo2 同値キー）
            let root = TempDir::new();
            let ghost_master = root.path().join("ghost").join("master");
            let shell_master = root.path().join("shell").join("master");
            fs::create_dir_all(&ghost_master).expect("create ghost/master");
            fs::create_dir_all(&shell_master).expect("create shell/master");
            fs::write(
                ghost_master.join("descript.txt"),
                "charset,UTF-8\nname,えも\nsakura.name,むらさき\nkero.name,エモ\n",
            )
            .expect("ghost descript");
            fs::write(
                shell_master.join("descript.txt"),
                "charset,UTF-8\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,0\nkero.defaultx,0\nsakura.balloon.alignment,left\nkero.balloon.alignment,right\n",
            )
            .expect("shell descript");
            fs::write(
                shell_master.join("surfaces.txt"),
                "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface10\n{\nelement0,overlay,surface10.png,0,0\n}\n",
            )
            .expect("surfaces.txt");
            fs::copy(
                emo2_root().join("shell/master/surface0.png"),
                shell_master.join("surface0.png"),
            )
            .expect("surface0.png 複写");
            fs::copy(
                emo2_root().join("shell/master/surface10.png"),
                shell_master.join("surface10.png"),
            )
            .expect("surface10.png 複写");

            let before =
                prepare_ghost_windows_with_work_area(root.path(), &balloon_root(), WA)
                    .expect("合成ゴーストの準備は成功する");

            // (a) 書き込みなし: ghost.dat がどこにも生成されていない
            for dir in [root.path(), ghost_master.as_path(), shell_master.as_path()] {
                assert!(
                    !dir.join("ghost.dat").exists(),
                    "ghost.dat が生成された: {}",
                    dir.display()
                );
            }

            // (b) 読み込みなし: 偽の保存位置を plant しても出力不変（復元しない）
            let junk = "position.0.x,9999\r\nposition.0.y,9999\r\n";
            fs::write(root.path().join("ghost.dat"), junk).expect("plant root ghost.dat");
            fs::write(ghost_master.join("ghost.dat"), junk).expect("plant ghost ghost.dat");

            let after =
                prepare_ghost_windows_with_work_area(root.path(), &balloon_root(), WA)
                    .expect("plant 後も準備は成功する");
            assert_eq!(
                before.placements, after.placements,
                "ghost.dat の有無で配置が変わった（復元経路が存在する疑い・2.11 違反）"
            );
            assert_eq!(before.titles, after.titles);
        });
    }
}
