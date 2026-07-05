//! emo2 fixture を用いた bake パイプラインの統合テスト（shell・balloon 横断）。
//!
//! task 3.2（要件 **1.1, 1.4, 2.2, 5.6, 6.1, 6.2**）。
//!
//! 既に構築済みの bake パイプライン（マニフェスト導出 → デコード → 正規化 → トリム →
//! packing → 焼付）を、実 fixture emo2 の shell surface 一式・balloon 画像・欠落パス混在
//! 入力で駆動し、成功/失敗を自動実行環境で明確に判定する。production コードは変更せず
//! （本モジュールは `#[cfg(test)]`）、WIC 腕（COM 隔離）越しに実 PNG を復号する。
//!
//! - shell E2E: `surfaces.txt` を `areka_parsers::shell::parse` で読み、SurfaceSet として
//!   bake。全 element が索引表に載り頁が生成される（1.1/6.1/6.2）。
//! - balloon E2E: balloon 画像を surface 表現に**手組み**（balloon→surface 適応は本層外・
//!   design D1）し、shell と同一の bake 経路で処理されることを実証（1.4）。
//! - 欠落パス混在: 存在しないパスは当該エントリのみ errors・resolve→None、他エントリは
//!   継続（2.2/5.6）。
//!
//! in-source `#[cfg(test)]`（regular dep `areka-parsers` / `windows` へ full access・
//! WIC 腕テスト task 2.2 の前例に倣う）。

use std::path::PathBuf;

use areka_parsers::shell::{Element, ElementPath, Surface};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

use crate::{
    bake, AlphaParams, BakeError, ManifestDeriver, PackConfig, SetId, SurfaceSet, UseSelfAlpha,
    WicDecoderArm,
};

/// COM 初期化下でクロージャを実行するヘルパ（WIC は CPU-only だが COM init 必須）。
/// `decode/wic_arm.rs` の同名ヘルパと同一パターン（MTA・RPC_E_CHANGED_MODE 許容）。
fn with_com_initialized<F: FnOnce()>(f: F) {
    unsafe {
        // 既に初期化済みでも RPC_E_CHANGED_MODE を許容し、続行する。
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    f();
    unsafe {
        CoUninitialize();
    }
}

/// emo2 fixture 実資産のパスを組む。
/// `CARGO_MANIFEST_DIR` = `crates/areka-emo-atlas`。fixtures はパイロット crate 配下。
fn emo2(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
        .join(rel)
}

/// emo2 の shell/master 基準 dir（surfaces.txt の element 相対パス起点）。
fn shell_master_dir() -> PathBuf {
    emo2("shell/master")
}

/// AlphaParams { use_self_alpha: On }（emo2 の descript は seriko.use_self_alpha,1）。
fn on_params() -> AlphaParams {
    AlphaParams {
        use_self_alpha: UseSelfAlpha::On,
    }
}

/// balloon 画像 1 枚を 1 element として持つ手組み surface（layer0・x/y=0・collision/animation 空）。
/// balloon→surface 適応は本層の責務外（design D1）ゆえ、テストが surface 表現を手構築する。
fn hand_surface(id: u32, rel: &str) -> Surface {
    Surface {
        id,
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new(rel.to_string()),
            x: 0,
            y: 0,
        }],
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

/// emo2 shell の実測フィクスチャ事実（本統合テストが依拠する）。
///
/// `surfaces.txt` が参照する 55 の distinct element 画像は全て実在するが、そのうち
/// ちょうど 1 枚 `purple/a/null.png` は **α チャンネルを持たない** PNG（かつ同名
/// `.pna` 兄弟も無い）。`use_self_alpha=On` の下では正規化器が優先順位（α ＞ .pna ＞
/// キーカラー）に従い KeyColor 腕を選ぶが、本層は AlphaChannel 腕のみ実装ゆえ、この
/// 1 エントリだけが `BakeError::Normalize { source: Unsupported(KeyColor) }` として
/// 分離される（R2.2/5.6 の per-entry 分離を実 fixture で行使）。他の全 element は生存する。
const SHELL_NORMALIZE_SEAM_KEY: &str = "purple/a/null.png";

/// テスト①: emo2 shell — 全 element が（ドキュメント済み seam 1 件を除き）索引表に載り
/// 頁が生成される（1.1/6.1/6.2＋2.2/5.6 を実 fixture で行使）。
///
/// 実 `surfaces.txt` を parse → SurfaceSet として bake。参照 element は全て実在するため
/// デコード失敗は無く、唯一 α 無しの `purple/a/null.png` のみ正規化 seam として分離され、
/// 他の全 manifest key は生存・resolve→Some、頁生成、surface0.png が配置される。
#[test]
fn emo2_shell_all_elements_baked() {
    let content = std::fs::read_to_string(emo2("shell/master/surfaces.txt"))
        .expect("emo2 surfaces.txt must be readable (UTF-8)");
    let shell = areka_parsers::shell::parse(&content);

    // parse がサーフェスを生んだこと（UTF-8 read_to_string で十分）。
    assert!(
        !shell.surfaces.is_empty(),
        "parse produced surfaces from surfaces.txt"
    );

    let base = shell_master_dir();

    // manifest を先に導出（生存判定の基準・既知 key の存在確認も兼ねる）。
    let manifest = {
        let set = SurfaceSet {
            surfaces: &shell.surfaces,
            base_dir: &base,
            alpha_params: on_params(),
        };
        ManifestDeriver.derive(std::slice::from_ref(&set))
    };
    // 既知 key surface0.png が manifest に現れる＝parse が使える surface を出した証左。
    assert!(
        manifest
            .keys
            .iter()
            .any(|k| k.set == SetId(0) && k.rel_path == "surface0.png"),
        "manifest contains known key surface0.png"
    );
    // fixture 事実の前提: seam key が manifest に含まれる（含まれなければ本テストの前提が崩れる）。
    assert!(
        manifest
            .keys
            .iter()
            .any(|k| k.set == SetId(0) && k.rel_path == SHELL_NORMALIZE_SEAM_KEY),
        "manifest contains the documented normalize-seam key {SHELL_NORMALIZE_SEAM_KEY}"
    );

    with_com_initialized(|| {
        let dec = WicDecoderArm::new().expect("WIC factory creates under COM init");

        let set = SurfaceSet {
            surfaces: &shell.surfaces,
            base_dir: &base,
            alpha_params: on_params(),
        };
        let result = bake(std::slice::from_ref(&set), &dec, PackConfig::default());

        // 実在ファイルのみ参照ゆえデコード失敗は 0。唯一の失敗は α 無し null.png の
        // 正規化 seam（KeyColor）で、それ以外は生存する（per-entry 分離・R2.2）。
        for e in &result.errors {
            match e {
                BakeError::Normalize { key, .. }
                    if key.set == SetId(0) && key.rel_path == SHELL_NORMALIZE_SEAM_KEY => {}
                other => panic!(
                    "unexpected bake error (only the documented null.png normalize seam is tolerated): {other:?}"
                ),
            }
        }

        // 索引表は「全 manifest key − 失敗数」= 生存件数で密に構築される（1.1 の全列挙＋
        // per-entry 分離）。null.png 1 件のみ脱落する前提を件数で固定する。
        assert_eq!(
            result.table.len(),
            manifest.keys.len() - result.errors.len(),
            "survivors = all manifest keys minus isolated failures"
        );

        // seam key を除く全 manifest key が resolve→Some（他エントリの継続・往復整合）。
        for key in &manifest.keys {
            if key.set == SetId(0) && key.rel_path == SHELL_NORMALIZE_SEAM_KEY {
                // seam entry は索引表に不在（分離された）。
                assert_eq!(
                    result.table.resolve(key.set, &key.rel_path),
                    None,
                    "the normalize-seam key is absent from the table"
                );
            } else {
                assert!(
                    result.table.resolve(key.set, &key.rel_path).is_some(),
                    "manifest key {key:?} resolves in the table (continuation)"
                );
            }
        }

        // 頁が生成されている（6.1）。
        assert!(
            !result.table.pages().is_empty(),
            "at least one atlas page generated"
        );
        assert!(result.table.page(0).is_some(), "page(0) present");

        // surface0.png は実在の非透明画像 → resolve→Some・placement あり（配置された）。
        let id = result
            .table
            .resolve(SetId(0), "surface0.png")
            .expect("surface0.png resolves");
        assert!(
            result.table.entry(id).placement.is_some(),
            "surface0.png is a real non-transparent image → placed"
        );
    });
}

/// テスト②: emo2 balloon — shell と同一の bake 経路で処理される（1.4）。
///
/// balloon 画像を surface 表現に手組みし（design D1）、shell set と **1 度の bake** で
/// 併せて処理。両 set の代表 key が別 SetId で resolve→placement を持つことで、balloon が
/// shell と同一機構で処理されることを実証。
#[test]
fn emo2_balloon_same_bake_path_as_shell() {
    let content = std::fs::read_to_string(emo2("shell/master/surfaces.txt"))
        .expect("emo2 surfaces.txt readable");
    let shell = areka_parsers::shell::parse(&content);
    assert!(!shell.surfaces.is_empty());

    let shell_base = shell_master_dir();
    let balloon_base = emo2("emo2-kakukaku");

    // balloon の surface 表現を手組み（本層外の適応をテストが肩代わり・D1）。
    let balloon_surfaces = vec![
        hand_surface(0, "balloons0.png"),
        hand_surface(1, "balloonk0.png"),
    ];

    with_com_initialized(|| {
        let dec = WicDecoderArm::new().expect("WIC factory under COM init");

        let shell_set = SurfaceSet {
            surfaces: &shell.surfaces,
            base_dir: &shell_base,
            alpha_params: on_params(),
        };
        let balloon_set = SurfaceSet {
            surfaces: &balloon_surfaces,
            base_dir: &balloon_base,
            alpha_params: on_params(),
        };

        // shell(set0) と balloon(set1) を 1 度の bake で同一機構により処理（1.4）。
        let result = bake(&[shell_set, balloon_set], &dec, PackConfig::default());

        // balloon 画像（RGBA）はデコード・正規化を通過。shell 側の既知 seam
        // （null.png・set0）だけは分離されるが、balloon 側（set1）に失敗は出ない。
        for e in &result.errors {
            match e {
                BakeError::Normalize { key, .. }
                    if key.set == SetId(0) && key.rel_path == SHELL_NORMALIZE_SEAM_KEY => {}
                other => panic!(
                    "combined bake: only the documented shell null.png seam is tolerated, got {other:?}"
                ),
            }
        }

        // shell 側代表 key（set0）。
        let shell_id = result
            .table
            .resolve(SetId(0), "surface0.png")
            .expect("shell surface0.png resolves in combined bake");
        assert!(
            result.table.entry(shell_id).placement.is_some(),
            "shell surface0.png placed"
        );

        // balloon 側 key（set1）— 同一 bake 経路で処理された証明（1.4）。
        for rel in ["balloons0.png", "balloonk0.png"] {
            let id = result
                .table
                .resolve(SetId(1), rel)
                .unwrap_or_else(|| panic!("balloon {rel} resolves in set 1"));
            assert!(
                result.table.entry(id).placement.is_some(),
                "balloon {rel} loaded via the SAME bake path as shell → placed"
            );
        }

        // 頁は非空。
        assert!(
            !result.table.pages().is_empty(),
            "pages materialized for combined bake"
        );
    });
}

/// テスト③: 存在しないパスを含む入力 → 当該エントリのみ errors・他は継続（2.2/5.6）。
///
/// 実在 element（surface0.png）と bogus パスを混在させ、bogus は resolve→None かつ
/// errors に Decode が 1 件以上、実在 element は resolve→Some＋placement（継続）。
#[test]
fn emo2_nonexistent_path_isolated_others_continue() {
    let base = shell_master_dir();

    // 実在 + bogus を 1 surface に混在（両者とも shell/master 基準）。
    let surfaces = vec![Surface {
        id: 0,
        elements: vec![
            Element {
                layer: 0,
                path: ElementPath::new("surface0.png".to_string()),
                x: 0,
                y: 0,
            },
            Element {
                layer: 1,
                path: ElementPath::new("this_file_does_not_exist_xyz.png".to_string()),
                x: 0,
                y: 0,
            },
        ],
        collisions: Vec::new(),
        animations: Vec::new(),
    }];

    with_com_initialized(|| {
        let dec = WicDecoderArm::new().expect("WIC factory under COM init");
        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: &base,
            alpha_params: on_params(),
        };
        let result = bake(std::slice::from_ref(&set), &dec, PackConfig::default());

        // bogus key は索引表に不在（脱落）。
        assert_eq!(
            result.table.resolve(SetId(0), "this_file_does_not_exist_xyz.png"),
            None,
            "nonexistent path absent from table"
        );

        // errors に当該 Decode 失敗が集約されている（≥1）。
        assert!(
            !result.errors.is_empty(),
            "nonexistent path recorded as an error"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| matches!(e, BakeError::Decode(_))),
            "errors contain a BakeError::Decode for the bogus path, got {:?}",
            result.errors
        );

        // 実在 element は継続処理される（脱落しない・placement あり）。
        let ok_id = result
            .table
            .resolve(SetId(0), "surface0.png")
            .expect("real surface0.png still resolves (continuation)");
        assert!(
            result.table.entry(ok_id).placement.is_some(),
            "real surface0.png placed despite sibling failure"
        );
    });
}
