//! 採寸と資産の組み立てで動く絵を焼く回数の檻（spec: placement-measure-bake-once 要件 1.1・1.2・
//! 1.4・4.1・4.2・4.3・4.6、design「Integration Tests」1・2・D5・System Flows の数の表）。
//!
//! 本物の読み手 `WicDecoderArm` を数える読み手 [`CountingDecoder`] で包み、本番と同じ順
//! （採寸 `measure_with_reader` → 資産 `build_shell_assets`・`build_balloon_assets`）に
//! 別々の数える読み手を通して、5 つの口の呼ばれた回数をファイルごとに固定する。
//! 採寸の側の焼きの回数は、包み `FirstFrameDecoder` の `decode` が 1 回につき 1 度だけ呼ぶ
//! `probe_animation` で数える（資産の側は焼きの段が 1 枚につき 1 度呼ぶ）。
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_emo_atlas::{
    AnimatedImage, AnimationInfo, DecodeError, DecodedImage, ElementDecoder, WicDecoderArm,
};
use areka_emo_present::balloon::resolve_balloon_faces;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::{MeasureScaling, measure_baked, measure_scope_sizes, measure_with_reader};
use crate::emo2_boot::assets::{build_balloon_assets, build_shell_assets};
use crate::placement::PlacementError;
use crate::placement::descript_window_keys::DefaultSurfaces;
use crate::placement::shared_test_support::{balloon_root, emo2_root, with_com_initialized};

/// 1 つのファイルについて、5 つの口が呼ばれた回数。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Calls {
    decode: u32,
    probe_pna: u32,
    probe_animation: u32,
    decode_frames: u32,
    decode_first_frame: u32,
}

/// 本物の読み手を包み、5 つの口の呼ばれた回数をファイルごとに数える（このファイルの中だけで使う）。
struct CountingDecoder {
    inner: WicDecoderArm,
    calls: RefCell<BTreeMap<PathBuf, Calls>>,
}

impl CountingDecoder {
    fn new() -> Self {
        Self {
            inner: WicDecoderArm::new().expect("WIC の読み手（COM 初期化済み）"),
            calls: RefCell::default(),
        }
    }

    fn bump(&self, path: &Path, f: impl FnOnce(&mut Calls)) {
        f(self
            .calls
            .borrow_mut()
            .entry(path.to_path_buf())
            .or_default());
    }

    /// `dir` の直下のファイルの回数を、ファイル名で引ける形で返す。
    fn in_dir(&self, dir: &Path) -> BTreeMap<String, Calls> {
        self.calls
            .borrow()
            .iter()
            .filter(|(p, _)| p.parent() == Some(dir))
            .map(|(p, c)| (p.file_name().unwrap().to_string_lossy().into_owned(), *c))
            .collect()
    }
}

impl ElementDecoder for CountingDecoder {
    fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        self.bump(path, |c| c.decode += 1);
        self.inner.decode(path)
    }
    fn probe_pna(&self, path: &Path) -> bool {
        self.bump(path, |c| c.probe_pna += 1);
        self.inner.probe_pna(path)
    }
    fn probe_animation(&self, path: &Path) -> Option<AnimationInfo> {
        self.bump(path, |c| c.probe_animation += 1);
        self.inner.probe_animation(path)
    }
    fn decode_frames(
        &self,
        path: &Path,
        info: AnimationInfo,
    ) -> Result<AnimatedImage, DecodeError> {
        self.bump(path, |c| c.decode_frames += 1);
        self.inner.decode_frames(path, info)
    }
    fn decode_first_frame(
        &self,
        path: &Path,
        info: AnimationInfo,
    ) -> Result<DecodedImage, DecodeError> {
        self.bump(path, |c| c.decode_first_frame += 1);
        self.inner.decode_first_frame(path, info)
    }
}

/// 採寸と資産に渡す scope（資産と同じ `[0, 1]` に揃える・design「バルーンの数え方」）。
const SCOPES: [usize; 2] = [0, 1];

/// 本番と同じ順（採寸 → 資産）で、別々の数える読み手を通す。戻りは（採寸, 資産）。
fn measure_then_assets(
    ghost_root: &Path,
    shell_dir: &Path,
    balloon_dir: &Path,
) -> (CountingDecoder, CountingDecoder) {
    let defaults = DefaultSurfaces::builtin();
    let measure = CountingDecoder::new();
    measure_with_reader(&measure, shell_dir, balloon_dir, &SCOPES, &defaults).expect("採寸は通る");
    let assets = CountingDecoder::new();
    let scopes: Vec<u32> = SCOPES.iter().map(|&s| s as u32).collect();
    build_shell_assets(ghost_root, None, &scopes, 96, &defaults, &assets)
        .unwrap_or_else(|e| panic!("シェルの資産は組める: {e:?}"));
    build_balloon_assets(balloon_dir, &scopes, 96, &assets)
        .unwrap_or_else(|e| panic!("バルーンの資産は組める: {e:?}"));
    (measure, assets)
}

/// バルーンの面のファイルごとの焼きの期待（採寸＝面 0 に使う scope の数・資産＝使う scope の数）。
///
/// 数える読み手は scope を知らないので、scope ごとの 1 回をファイルごとの数の和で表す。
fn balloon_bakes_per_file(balloon_dir: &Path) -> BTreeMap<String, (u32, u32)> {
    let mut want: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for &scope in &SCOPES {
        for face in resolve_balloon_faces(balloon_dir, scope as u32).expect("系列の解決") {
            let entry = want.entry(face.file_name.clone()).or_default();
            entry.1 += 1;
            if face.surface_id == 0 {
                entry.0 += 1;
            }
        }
    }
    want
}

/// 動く絵の検体を `target\` の下の小さなゴーストの形へ写す（`ghost\master`・`shell\master`・
/// `balloon`）。
fn animated_ghost() -> TempPath {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../areka-emo-compose/tests/fixtures/animated-playback");
    let root = TempPath::under_target("measure-bake-count");
    let ghost_master = root.path().join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\r\nname,g_bake_count\r\n",
    )
    .expect("ghost の descript");
    for (from, to) in [("shell", "shell/master"), ("balloon", "balloon")] {
        let dst = root.path().join(to);
        std::fs::create_dir_all(&dst).expect("写し先");
        for entry in std::fs::read_dir(fixture.join(from)).expect("検体の一覧") {
            let entry = entry.expect("検体の一覧の 1 件");
            std::fs::copy(entry.path(), dst.join(entry.file_name())).expect("検体を写す");
        }
    }
    root
}

/// 採寸の 1 回の焼き（`FirstFrameDecoder::decode` が `probe_animation` を 1 度呼び、動く絵は
/// 1 枚目読みへ・動かない絵は 1 枚読みへ）。
fn measured(animated: bool) -> Calls {
    Calls {
        decode: u32::from(!animated),
        probe_pna: 1,
        probe_animation: 1,
        decode_frames: 0,
        decode_first_frame: u32::from(animated),
    }
}

/// 資産の 1 回の焼き（焼きの段が `probe_animation` を 1 度呼び、動く絵は全コマ読みへ）。
fn assembled(animated: bool) -> Calls {
    Calls {
        decode: u32::from(!animated),
        probe_pna: 1,
        probe_animation: 1,
        decode_frames: u32::from(animated),
        decode_first_frame: 0,
    }
}

/// `n` 回分の回数。
fn times(c: Calls, n: u32) -> Calls {
    Calls {
        decode: c.decode * n,
        probe_pna: c.probe_pna * n,
        probe_animation: c.probe_animation * n,
        decode_frames: c.decode_frames * n,
        decode_first_frame: c.decode_first_frame * n,
    }
}

/// 要件 1.1・1.2・4.1・4.2・4.6: 動く絵の検体で、採寸は動く絵 1 つの 1 回の焼きにつき 1 枚目読み
/// 1 回・全コマ読み 0 回、資産はシェルの動く絵 1 つにつき全コマ読み 1 回。シェルの絵 1 枚ごとの
/// 焼きは採寸 1・資産 1、バルーンの面の絵はファイルごとに「採寸＝面 0 に使う scope の数・
/// 資産＝使う scope の数」。
#[test]
fn animated_fixture_bakes_once_per_stage_and_reads_full_frames_once() {
    with_com_initialized(|| {
        let ghost = animated_ghost();
        let shell_dir = ghost.path().join("shell").join("master");
        let balloon_dir = ghost.path().join("balloon");
        let (measure, assets) = measure_then_assets(ghost.path(), &shell_dir, &balloon_dir);

        // シェルの焼く絵（`surface2.png` は element定義が在るので土台に使われず焼かれない）。
        let shell: [(&str, bool); 8] = [
            ("alpha.webp", true),
            ("body.png", false),
            ("cyan.png", false),
            ("part.png", false),
            ("rgb.apng", true),
            ("rgb.webp", true),
            ("surface1.png", true),
            ("yellow.png", false),
        ];
        let want = |stage: fn(bool) -> Calls| -> BTreeMap<String, Calls> {
            shell
                .iter()
                .map(|&(name, animated)| (name.to_string(), stage(animated)))
                .collect()
        };
        assert_eq!(measure.in_dir(&shell_dir), want(measured), "採寸のシェル");
        assert_eq!(assets.in_dir(&shell_dir), want(assembled), "資産のシェル");

        // バルーンは本体側の系列だけ（scope 1 は本体側へ縮退）で、2 つとも動く絵。
        // 期待は系列の解決から導き、導いた値そのものも数で固定する。
        let per_file = balloon_bakes_per_file(&balloon_dir);
        assert_eq!(
            per_file,
            BTreeMap::from([
                ("balloons0.png".to_string(), (2, 2)),
                ("balloons1.png".to_string(), (0, 2)),
            ]),
            "面 0 は 2 つの scope とも balloons0.png・資産は 2 つの scope とも 2 面"
        );
        let want_balloon = |pick: fn((u32, u32)) -> u32, stage: fn(bool) -> Calls| {
            per_file
                .iter()
                .filter(|(_, n)| pick(**n) > 0)
                .map(|(name, n)| (name.clone(), times(stage(true), pick(*n))))
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(
            measure.in_dir(&balloon_dir),
            want_balloon(|n| n.0, measured),
            "採寸のバルーン（面 0 だけ・scope ごとに 1 回）"
        );
        assert_eq!(
            assets.in_dir(&balloon_dir),
            want_balloon(|n| n.1, assembled),
            "資産のバルーン（全部の面・scope ごとに 1 回）"
        );
    });
}

/// 要件 1.4・4.3: emo2（動かない絵だけ）で、動く絵の口は 0 回、絵 1 枚ごとの焼きは採寸 1・資産 1、
/// バルーンの面の絵はファイルごとに「採寸＝面 0 に使う scope の数・資産＝使う scope の数」。
#[test]
fn emo2_bakes_each_still_image_once_per_stage() {
    with_com_initialized(|| {
        let root = emo2_root();
        let shell_dir = root.join("shell").join("master");
        let balloon_dir = balloon_root();
        let (measure, assets) = measure_then_assets(&root, &shell_dir, &balloon_dir);

        let measured_shell = measure.in_dir(&shell_dir);
        assert!(!measured_shell.is_empty(), "emo2 のシェルの絵を焼く");
        let want = |stage: fn(bool) -> Calls| -> BTreeMap<String, Calls> {
            measured_shell
                .keys()
                .map(|name| (name.clone(), stage(false)))
                .collect()
        };
        assert_eq!(measured_shell, want(measured), "採寸のシェル");
        assert_eq!(assets.in_dir(&shell_dir), want(assembled), "資産のシェル");

        let per_file = balloon_bakes_per_file(&balloon_dir);
        let want_balloon = |pick: fn((u32, u32)) -> u32, stage: fn(bool) -> Calls| {
            per_file
                .iter()
                .filter(|(_, n)| pick(**n) > 0)
                .map(|(name, n)| (name.clone(), times(stage(false), pick(*n))))
                .collect::<BTreeMap<_, _>>()
        };
        let face0 = want_balloon(|n| n.0, measured);
        assert_eq!(
            face0.keys().map(String::as_str).collect::<Vec<_>>(),
            ["balloonk0.png", "balloons0.png"],
            "scope 0 と 1 は別の系列の面 0 を焼く"
        );
        assert_eq!(measure.in_dir(&balloon_dir), face0, "採寸のバルーン");
        // 資産はほかに箱の目印（arrow*.png）を焼きでなく 1 枚読みで読むので、面の絵だけを比べ、
        // 動く絵の口が 0 回であることはフォルダの全部で確かめる。
        let assets_balloon = assets.in_dir(&balloon_dir);
        let faces: BTreeMap<String, Calls> = assets_balloon
            .iter()
            .filter(|(name, _)| per_file.contains_key(*name))
            .map(|(name, c)| (name.clone(), *c))
            .collect();
        assert_eq!(
            faces,
            want_balloon(|n| n.1, assembled),
            "資産のバルーンの面"
        );
        for (stage, calls) in [
            ("採寸", measure.in_dir(&balloon_dir)),
            ("資産", assets_balloon),
        ] {
            for (name, c) in calls {
                assert_eq!(
                    (c.decode_frames, c.decode_first_frame),
                    (0, 0),
                    "{stage}のバルーン {name}: 動く絵の口は呼ばれない"
                );
            }
        }
    });
}

/// 要件 2.1・2.2: 採寸の読み手を通した測りは、全コマを焼く今の測り（包まない読み手で
/// `measure_baked`）と scope ごとの寸法が等しい（動く絵の検体と emo2）。
#[test]
fn first_frame_measure_matches_full_frame_measure() {
    with_com_initialized(|| {
        let ghost = animated_ghost();
        let emo2 = emo2_root();
        let cases = [
            (
                "動く絵の検体",
                ghost.path().join("shell").join("master"),
                ghost.path().join("balloon"),
            ),
            ("emo2", emo2.join("shell").join("master"), balloon_root()),
        ];
        let arm = WicDecoderArm::new().expect("WIC の読み手（COM 初期化済み）");
        let defaults = DefaultSurfaces::builtin();
        for (name, shell_dir, balloon_dir) in cases {
            let first = measure_with_reader(&arm, &shell_dir, &balloon_dir, &SCOPES, &defaults)
                .unwrap_or_else(|e| panic!("{name}: 採寸の読み手で測れる: {e:?}"));
            let full = measure_baked(&arm, &shell_dir, &balloon_dir, &SCOPES, &defaults)
                .unwrap_or_else(|e| panic!("{name}: 全コマを焼いて測れる: {e:?}"));
            assert_eq!(first.len(), SCOPES.len(), "{name}: scope の数");
            assert_eq!(first, full, "{name}: scope ごとの寸法");
        }
    });
}

/// シェルとバルーンの読み込みの記録の宛先（読み込みの権威）。
const LOAD_TARGETS: [&str; 3] = [
    "areka_emo_present::shell_target",
    "areka_emo_present::balloon",
    "areka_emo_present::self_alpha",
];

/// 採寸自身の記録の宛先（`measure.rs`）。
const MEASURE_TARGET: &str = "areka::placement::measure";

/// `target` の記録を（水準, 文言）の並びにする。
fn records_of(events: &[CapturedEvent], targets: &[&str]) -> Vec<(tracing::Level, String)> {
    events
        .iter()
        .filter(|e| targets.contains(&e.target.as_str()))
        .map(|e| (e.level, e.message().to_string()))
        .collect()
}

/// 要件 3.6・4.5: 動く絵の検体で公開の口 `measure_scope_sizes` を通すと、シェルとバルーンの
/// 読み込みの記録は 0 行。採寸自身の記録は全コマを焼く今の測り（`measure_baked`）と同じで、
/// この検体では 0 行（scope 0 と 1 の既定の面 0・10 がどちらもシェルに在り、合成も通る）。
#[test]
fn public_measure_emits_no_load_records_and_same_own_records() {
    with_com_initialized(|| {
        let ghost = animated_ghost();
        let shell_dir = ghost.path().join("shell").join("master");
        let balloon_dir = ghost.path().join("balloon");
        let defaults = DefaultSurfaces::builtin();
        let (sizes, events) = capture(|| {
            measure_scope_sizes(
                &shell_dir,
                &balloon_dir,
                &SCOPES,
                &defaults,
                &MeasureScaling::IDENTITY,
            )
        });
        sizes.expect("採寸は通る");
        assert_eq!(
            records_of(&events, &LOAD_TARGETS),
            vec![],
            "採寸の道の読み込みの記録"
        );

        let arm = WicDecoderArm::new().expect("WIC の読み手（COM 初期化済み）");
        let (_, full_events) =
            capture(|| measure_baked(&arm, &shell_dir, &balloon_dir, &SCOPES, &defaults));
        let own = records_of(&events, &[MEASURE_TARGET]);
        assert_eq!(
            own,
            records_of(&full_events, &[MEASURE_TARGET]),
            "採寸自身の記録は全コマを焼く今の測りと同じ"
        );
        assert_eq!(own, vec![], "この検体での採寸自身の記録");
    });
}

/// `error!` の行の数を（読み込みの権威, 採寸）で数える。
fn error_counts(events: &[CapturedEvent]) -> (usize, usize) {
    let errors = |targets: &[&str]| {
        records_of(events, targets)
            .iter()
            .filter(|(level, _)| *level == tracing::Level::ERROR)
            .count()
    };
    (errors(&LOAD_TARGETS), errors(&[MEASURE_TARGET]))
}

/// 要件 2.5・3.6: 読めないときは記録を出さない道でも失敗の `error!` が出て `Err`。
/// - シェルのフォルダが無い: 権威の一覧の失敗 `error!` 1 行・採寸は出し直さない（0 行・
///   `build_shell_assets` の説明「真因のログは権威が出す」）。`Err` は scope 0。
/// - バルーンの面 0 が無い: 権威の面 0 の `error!` 1 行・採寸の面 0 の失敗 `error!` 1 行
///   （scope 0 で止まるので 1 回ずつ）。`Err` は scope 0・`balloon:` の理由。
#[test]
fn unreadable_shell_or_balloon_face0_errors_once_and_fails() {
    with_com_initialized(|| {
        let ghost = animated_ghost();
        let shell_dir = ghost.path().join("shell").join("master");
        let balloon_dir = ghost.path().join("balloon");
        let defaults = DefaultSurfaces::builtin();
        let measure = |shell: &Path, balloon: &Path| {
            capture(|| {
                measure_scope_sizes(
                    shell,
                    balloon,
                    &SCOPES,
                    &defaults,
                    &MeasureScaling::IDENTITY,
                )
            })
        };

        let (result, events) = measure(&ghost.path().join("shell").join("absent"), &balloon_dir);
        assert!(
            matches!(result, Err(PlacementError::Measure { scope: 0, .. })),
            "シェルのフォルダが無い: {result:?}"
        );
        assert_eq!(error_counts(&events), (1, 0), "シェルのフォルダが無い");
        assert!(
            records_of(&events, &LOAD_TARGETS)[0]
                .1
                .contains("シェルのフォルダの一覧に失敗"),
            "権威の一覧の失敗: {events:?}"
        );

        // 面 0 の無いバルーン（面 1 だけ）。
        std::fs::remove_file(balloon_dir.join("balloons0.png")).expect("面 0 を外す");
        let (result, events) = measure(&shell_dir, &balloon_dir);
        match &result {
            Err(PlacementError::Measure { scope: 0, reason }) => {
                assert!(reason.starts_with("balloon: "), "理由: {reason}")
            }
            other => panic!("バルーンの面 0 が無い: {other:?}"),
        }
        assert_eq!(
            error_counts(&events),
            (1, 1),
            "バルーンの面 0 が無い: {events:?}"
        );
    });
}
