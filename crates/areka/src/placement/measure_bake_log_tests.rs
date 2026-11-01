//! 起動の準備 1 回の行の数の檻（spec: emo-bake-log-overlap 要件 1.1〜1.6・2.1・2.2・3.1・3.3・
//! 3.5・4.1・6.1・6.3〜6.6、design「Testing Strategy」6・7・System Flows の行の数の表）。
//!
//! 本番と同じ順（採寸 `measure_with_reader` → シェルの資産 `build_shell_assets` → バルーンの資産
//! `build_balloon_assets`）に scope 0・1 で通し、焼く段と面の表の記録を文言ごとに数える
//! （宛先まるごとで数えない。`limits.rs` の 1 度だけの `warn!` を拾わないため・要件 6.5）。
//! 検体はファイルの形だけを `target\` の下に置き、絵の中身は `MemoryDecoder` が返す
//! （時間・実の画面・COM に依らない）。
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::Path;

use areka_emo_atlas::{
    AnimatedImage, AnimationFrame, AnimationInfo, DecodedImage, LoopCount, MemoryDecoder,
};
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::measure_with_reader;
use crate::emo2_boot::assets::{build_balloon_assets, build_shell_assets};
use crate::placement::PlacementError;
use crate::placement::descript_window_keys::DefaultSurfaces;

/// 黙らせる記録の文言（焼く段 `areka_emo_atlas`・面の表 `areka_emo_compose`）。
const ZERO_SIZED: &str = "bake: element の元画像が 0 寸です";
const ALL_CLEAR: &str = "全透明（α=0）でトリム後 0 寸です";
const BIND_NONE: &str = "着せ替え（bind）のアニメーションからだけ参照される面の絵です";
const KEY_COLOR: &str = "bake: element を抜き色（左上の 1 画素と同じ色）で透過しました";
const ANIM_UNREADABLE: &str = "bake: 動く絵として読めなかったので 1 枚だけ読みます";
const APPEND_NO_TARGET: &str = "surface.append 対象 id が未存在";
const ANIM_REDEFINED: &str = "append が既存 animation id を再定義";
const NEEDLES: [&str; 7] = [
    ZERO_SIZED,
    ALL_CLEAR,
    BIND_NONE,
    KEY_COLOR,
    ANIM_UNREADABLE,
    APPEND_NO_TARGET,
    ANIM_REDEFINED,
];

/// 採寸と資産に渡す scope（emo2 とぱすたと同じ 2 つ）。
const SCOPES: [usize; 2] = [0, 1];

const OPAQUE: [u8; 4] = [10, 20, 30, 255];
const OTHER: [u8; 4] = [40, 50, 60, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// 2×2 の α つきの絵（画素は全部 `px`）。
fn image(px: [u8; 4]) -> DecodedImage {
    DecodedImage {
        width: 2,
        height: 2,
        stride: 8,
        bgra: [px; 4].concat(),
        has_alpha: true,
    }
}

fn info() -> AnimationInfo {
    AnimationInfo {
        width: 2,
        height: 2,
        frame_count: 2,
    }
}

/// 2 コマの動く絵の全コマ。
fn frames(px0: [u8; 4], px1: [u8; 4]) -> AnimatedImage {
    AnimatedImage {
        frames: [px0, px1]
            .iter()
            .map(|&px| AnimationFrame {
                image: image(px),
                delay_ms: 100,
            })
            .collect(),
        loop_count: LoopCount::Finite(NonZeroU32::new(1).unwrap()),
    }
}

/// シェルの面の記述: 面 0・10 は見える絵、面 1 が記録を起こす絵を持ち、面 1 の bind の
/// アニメーションだけが面 3000（`null.png`）を指す。追記は相手の無い面 5 と、面 1 の
/// アニメーション 1 の書き直し。
const SURFACES_TXT: &str = concat!(
    "charset,UTF-8\n",
    "surface0\n{\nelement0,overlay,body.png,0,0\n}\n",
    "surface10\n{\nelement0,overlay,body.png,0,0\n}\n",
    "surface1\n{\n",
    "element0,overlay,zero.png,0,0\n",
    "element1,overlay,clear.png,0,0\n",
    "element2,overlay,key.png,0,0\n",
    "element3,overlay,fade.png,0,0\n",
    "element4,overlay,broken.png,0,0\n",
    "animation0.interval,bind\n",
    "animation0.pattern0,overlay,3000,0,0,0\n",
    "animation1.interval,random,5\n",
    "animation1.pattern0,overlay,0,0,0,0\n",
    "}\n",
    "surface3000\n{\nelement0,overlay,null.png,0,0\n}\n",
    "surface.append5\n{\nelement0,overlay,body.png,0,0\n}\n",
    "surface.append1\n{\nanimation1.interval,runonce\nanimation1.pattern0,overlay,0,0,0,0\n}\n",
);

const SHELL_IMAGES: [&str; 7] = [
    "body.png",
    "zero.png",
    "clear.png",
    "key.png",
    "fade.png",
    "broken.png",
    "null.png",
];

/// 検体のゴースト（`ghost\master`・`shell\master`・`balloon`）。絵のファイルは名前だけ。
fn ghost() -> TempPath {
    let root = TempPath::under_target("measure-bake-log");
    let write = |rel: &str, text: &str| {
        let path = root.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).expect("フォルダを作る");
        std::fs::write(path, text).expect("検体を書く");
    };
    write(
        "ghost/master/descript.txt",
        "charset,UTF-8\r\nname,g_bake_log\r\n",
    );
    write(
        "shell/master/descript.txt",
        "charset,UTF-8\nseriko.use_self_alpha,1\n",
    );
    write("shell/master/surfaces.txt", SURFACES_TXT);
    for name in SHELL_IMAGES {
        write(&format!("shell/master/{name}"), "");
    }
    write("balloon/descript.txt", "charset,UTF-8\nuse_self_alpha,1\n");
    write("balloon/balloons0.png", "");
    write("balloon/balloons1.png", "");
    root
}

/// 絵の中身。`face0_corrupt` ならバルーンの面 0 を読めない絵にする。
fn decoder(shell_dir: &Path, balloon_dir: &Path, face0_corrupt: bool) -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    let mut still = |path: std::path::PathBuf, img: DecodedImage| {
        dec.insert(
            path,
            img.width,
            img.height,
            img.stride,
            img.bgra,
            img.has_alpha,
        );
    };
    still(shell_dir.join("body.png"), image(OPAQUE));
    still(shell_dir.join("clear.png"), image(CLEAR));
    still(shell_dir.join("null.png"), image(CLEAR));
    // α の無い 2 色の絵＝左上の色を抜いても 1 画素残る。
    let key = |px| DecodedImage {
        width: 2,
        height: 1,
        stride: 8,
        bgra: [OPAQUE, px].concat(),
        has_alpha: false,
    };
    still(shell_dir.join("key.png"), key(OTHER));
    still(balloon_dir.join("balloons1.png"), image(CLEAR));
    dec.insert(shell_dir.join("zero.png"), 0, 0, 0, Vec::new(), true);
    // 1 枚目だけ全透明の動く絵と、全コマを読めない（1 枚目は読める）動く絵。
    dec.insert_animated(
        shell_dir.join("fade.png"),
        info(),
        Ok(image(CLEAR)),
        Ok(frames(CLEAR, OTHER)),
    );
    dec.insert_animated(
        shell_dir.join("broken.png"),
        info(),
        Ok(image(OPAQUE)),
        Err("broken frames".into()),
    );
    let face0 = balloon_dir.join("balloons0.png");
    if face0_corrupt {
        dec.insert_corrupt(face0, "corrupt");
    } else {
        let img = key(OTHER);
        dec.insert(face0, 2, 1, 8, img.bgra, false);
    }
    dec
}

/// 黙らせる記録を（文言, 絵の名前）ごとに数える（絵の名前の無い記録は "-"）。
fn counts(events: &[CapturedEvent]) -> BTreeMap<(&'static str, String), usize> {
    let mut out = BTreeMap::new();
    for e in events {
        if let Some(needle) = NEEDLES.iter().find(|n| e.message().contains(**n)) {
            let rel = e.field_str("rel_path").unwrap_or("-").to_string();
            *out.entry((*needle, rel)).or_default() += 1;
        }
    }
    out
}

fn want(rows: &[(&'static str, &str, usize)]) -> BTreeMap<(&'static str, String), usize> {
    rows.iter()
        .map(|&(needle, rel, n)| ((needle, rel.to_string()), n))
        .collect()
}

/// 要件 1.1〜1.6・2.1・2.2・3.1・3.3・4.1・6.1・6.3: 起動の準備 1 回の行の数。
/// - 採寸の窓: 焼く段と面の表の記録が 0 行（1 枚目だけ全透明の動く絵の「全透明」も出ない）
/// - シェルの資産の窓: 畳み込みの 2 件が 1 行ずつ（scope の数によらない）
/// - 起動の準備 1 回: シェルの 0 寸・全透明・「無し」・抜き色・動く絵の読めない `warn!` が絵ごとに
///   1 行、`fade.png` は 0 行、バルーンの面 0 の抜き色と面 1 の全透明が scope ごとに 1 行（計 2 行）
#[test]
fn boot_preparation_records_each_once() {
    let root = ghost();
    let shell_dir = root.path().join("shell").join("master");
    let balloon_dir = root.path().join("balloon");
    let dec = decoder(&shell_dir, &balloon_dir, false);
    let defaults = DefaultSurfaces::builtin();
    let scopes: Vec<u32> = SCOPES.iter().map(|&s| s as u32).collect();

    let (measured, measure_events) =
        capture(|| measure_with_reader(&dec, &shell_dir, &balloon_dir, &SCOPES, &defaults));
    measured.expect("採寸は通る");
    let (shell, shell_events) =
        capture(|| build_shell_assets(root.path(), None, &scopes, 96, &defaults, &dec));
    shell.unwrap_or_else(|e| panic!("シェルの資産は組める: {e:?}"));
    let (balloon, balloon_events) =
        capture(|| build_balloon_assets(&balloon_dir, &scopes, 96, &dec));
    balloon.unwrap_or_else(|e| panic!("バルーンの資産は組める: {e:?}"));

    assert_eq!(counts(&measure_events), want(&[]), "採寸の窓");
    let folds = want(&[(APPEND_NO_TARGET, "-", 1), (ANIM_REDEFINED, "-", 1)]);
    let shell_counts = counts(&shell_events);
    assert_eq!(
        shell_counts
            .iter()
            .filter(|((needle, _), _)| [APPEND_NO_TARGET, ANIM_REDEFINED].contains(needle))
            .map(|(k, v)| (k.clone(), *v))
            .collect::<BTreeMap<_, _>>(),
        folds,
        "シェルの資産の窓の畳み込み"
    );

    let all: Vec<CapturedEvent> = [measure_events, shell_events, balloon_events].concat();
    assert_eq!(
        counts(&all),
        want(&[
            (ZERO_SIZED, "zero.png", 1),
            (ALL_CLEAR, "clear.png", 1),
            (BIND_NONE, "null.png", 1),
            (KEY_COLOR, "key.png", 1),
            (ANIM_UNREADABLE, "broken.png", 1),
            (APPEND_NO_TARGET, "-", 1),
            (ANIM_REDEFINED, "-", 1),
            (KEY_COLOR, "balloons0.png", 2),
            (ALL_CLEAR, "balloons1.png", 2),
        ]),
        "起動の準備 1 回（fade.png の「全透明」は 0 行）"
    );
}

/// 要件 3.5・6.4: バルーンの面 0 を読めない絵にして採寸を通すと scope 0 の採寸の失敗が返り、
/// バルーンの「面画像の bake に失敗」の `error!` と採寸の面 0 の失敗の `error!` が 1 行ずつ出る
/// （公開の採寸の口は本物の読み手を作るので使わない）。
#[test]
fn unreadable_balloon_face0_stops_measure_with_one_error_each() {
    let root = ghost();
    let shell_dir = root.path().join("shell").join("master");
    let balloon_dir = root.path().join("balloon");
    let dec = decoder(&shell_dir, &balloon_dir, true);
    let defaults = DefaultSurfaces::builtin();

    let (result, events) =
        capture(|| measure_with_reader(&dec, &shell_dir, &balloon_dir, &SCOPES, &defaults));
    assert!(
        matches!(result, Err(PlacementError::Measure { scope: 0, .. })),
        "{result:?}"
    );
    let errors: Vec<(&str, &str)> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .map(|e| (e.target.as_str(), e.message()))
        .collect();
    assert_eq!(
        errors,
        vec![
            (
                "areka_emo_present::balloon",
                "balloon: 面画像の bake に失敗"
            ),
            (
                "areka::placement::measure",
                "measure: scope のバルーン面 0 の採寸に失敗"
            ),
        ],
        "{events:?}"
    );
}
