//! `render.rs` の試験（下地の画素・大きさと位置・2 度で同じ・ファイルの名前と書き出し）。
//!
//! 検体は `offline_dump_test_support::variety_shell`（土台 16×8・負の座標の絵が -2,-2）。期待の絵は
//! 画面と同じ入力（同じ面・同じ着せ替え・`settle_surface` の同じ状態）で `Composer::compose` した絵を
//! 試験の側で 1 画素ずつ `(-left, -top)` に置いて作る。

use areka_emo_atlas::WicDecoderArm;
use areka_emo_compose::{BindSet, Composer, EmoWorld, shell_bounds};
use areka_emo_present::{ShellTarget, load_shell_target};
use areka_seriko::{AnimationTable, settle_surface, settle_time};
use log_capture_kit::capture;
use temp_path_kit::TempPath;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::super::offline_dump_test_support as fixture;
use std::path::{Path, PathBuf};

use super::{
    Canvas, DEFAULT_PREFIX, RenderError, png_path, render_png, render_surface, write_surface,
};

/// 読み込んだ検体のシェル（根は Drop まで残す）。
struct Loaded {
    _root: TempPath,
    target: ShellTarget,
    world: EmoWorld,
    table: AnimationTable,
    canvas: Canvas,
}

fn load(label: &str) -> Loaded {
    // SAFETY: テストスレッドごとの COM 初期化。結果は既初期化でも続けてよいので捨てる。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let root = fixture::build_ghost(
        label,
        Some(&["name,g_render"]),
        &[fixture::variety_shell("master")],
    );
    let arm = WicDecoderArm::new().expect("WIC の工場が作れる");
    let (target, _) = capture(|| load_shell_target(&root.path().join("shell/master"), &arm));
    let target = target.expect("検体のシェルは読める");
    let world = target.build_world();
    let table = AnimationTable::from_world(&world);
    let canvas = Canvas::from_bounds(shell_bounds(&world, target.atlas()));
    Loaded {
        _root: root,
        target,
        world,
        table,
        canvas,
    }
}

impl Loaded {
    fn render(&self, binds: &BindSet, id: u32) -> Result<Vec<u8>, RenderError> {
        render_surface(
            &mut Composer::new(),
            &self.world,
            self.target.atlas(),
            &self.table,
            binds,
            0,
            id,
            &self.canvas,
        )
    }

    fn png(&self, binds: &BindSet, id: u32) -> Vec<u8> {
        render_png(
            &mut Composer::new(),
            &self.world,
            self.target.atlas(),
            &self.table,
            binds,
            0,
            id,
            &self.canvas,
        )
        .unwrap_or_else(|e| panic!("面 {id} を PNG にできる: {e:?}"))
    }

    /// 面 `id` をスコープ `scope` で `dir` へ頭の文字列 `prefix` で書く。
    fn write(&self, scope: u32, id: u32, dir: &Path, prefix: &str) -> Result<PathBuf, RenderError> {
        write_surface(
            &mut Composer::new(),
            &self.world,
            self.target.atlas(),
            &self.table,
            &default_binds(),
            scope,
            id,
            &self.canvas,
            dir,
            prefix,
        )
    }

    /// 画面と同じ入力で組んだ絵を、下地の `(-left, -top)` に 1 画素ずつ置いた期待の絵。
    fn expected(&self, binds: &BindSet, id: u32) -> Vec<u8> {
        let pattern = settle_surface(&self.table, 0, binds, id);
        let pic = Composer::new()
            .compose(&self.world, self.target.atlas(), id, binds, &pattern)
            .unwrap_or_else(|e| panic!("面 {id} は組める: {e:?}"));
        let (cw, ch) = (self.canvas.width as usize, self.canvas.height as usize);
        let (ox, oy) = (-self.canvas.left as usize, -self.canvas.top as usize);
        let mut out = vec![0u8; cw * ch * 4];
        for y in 0..pic.height() as usize {
            for x in 0..pic.width() as usize {
                let src = y * pic.stride() as usize + x * 4;
                let dst = ((oy + y) * cw + ox + x) * 4;
                out[dst..dst + 4].copy_from_slice(&pic.bytes()[src..src + 4]);
            }
        }
        out
    }

    /// 下地の座標 `(x, y)` の乗算済み BGRA。
    fn pixel(&self, bgra: &[u8], x: usize, y: usize) -> [u8; 4] {
        let at = (y * self.canvas.width as usize + x) * 4;
        bgra[at..at + 4].try_into().unwrap()
    }
}

/// 中身の無い面を除く検体の面。
fn drawable() -> impl Iterator<Item = u32> {
    fixture::VARIETY_SURFACES
        .into_iter()
        .filter(|&id| id != fixture::EMPTY)
}

/// 既定の着せ替え（head の hat・acc の a と b）。
fn default_binds() -> BindSet {
    BindSet::from_ids([20, 30, 31])
}

const GRAY: [u8; 4] = [128, 128, 128, 255];
const RED: [u8; 4] = [0, 0, 255, 255];
const BLUE: [u8; 4] = [255, 0, 0, 255];

/// 下地の画素は、同じ面・同じ着せ替え・同じ状態で合成した絵を `(-left,-top)` に置いたものと同じ（4.5）。
#[test]
fn pixels_are_the_screen_composition_placed_at_minus_left_top() {
    let shell = load("offline-render-pixels");
    for binds in [BindSet::default(), default_binds()] {
        for id in drawable() {
            let got = shell.render(&binds, id).expect("描ける");
            assert!(
                got == shell.expected(&binds, id),
                "面 {id}・着せ替え {:?} の画素が画面の合成と違う",
                binds.ids()
            );
        }
    }
}

/// 下地の大きさと位置はシェルで 1 つで、どの面も同じ（4.3）。土台の原点は下地の (2,2)。
#[test]
fn canvas_size_and_position_are_the_same_for_every_surface() {
    let shell = load("offline-render-canvas");
    assert_eq!(
        shell.canvas,
        Canvas {
            left: -2,
            top: -2,
            width: 18,
            height: 10
        }
    );
    for id in drawable() {
        let got = shell.render(&default_binds(), id).expect("描ける");
        assert_eq!(got.len(), 18 * 10 * 4, "面 {id} の下地の大きさ");
        // 土台の原点は下地の (2,2)。負の座標の絵（-2〜2）の外の (5,5) は土台の灰。
        if id < fixture::PARTS[0] && id != fixture::RUNONCE && id != fixture::BIND {
            assert_eq!(shell.pixel(&got, 5, 5), GRAY, "面 {id} の土台の位置");
        }
    }
    // 合成は負の座標の部分を切る（design の Risks）ので、下地の左上の余白は透明で、絵の残りが
    // 土台の原点 (2,2) に重なる（画面の絵と同じ・4.5）。
    let corner = shell.render(&default_binds(), fixture::NEGATIVE).unwrap();
    assert_eq!(shell.pixel(&corner, 0, 0), [0, 0, 0, 0]);
    assert_ne!(
        shell.pixel(&corner, 2, 2),
        GRAY,
        "負の座標の絵の残りが土台の原点に在る"
    );
    assert_eq!(shell.pixel(&corner, 4, 4), GRAY);
}

/// 絵の無い画素は `(0,0,0,0)`（4.4）。
#[test]
fn uncovered_pixels_are_transparent_zero() {
    let shell = load("offline-render-transparent");
    let red = shell.render(&default_binds(), fixture::PARTS[0]).unwrap();
    assert_eq!(shell.pixel(&red, 2, 2), RED);
    for (x, y) in [(0, 0), (1, 9), (10, 2), (17, 9)] {
        assert_eq!(shell.pixel(&red, x, y), [0, 0, 0, 0], "({x},{y})");
    }
}

/// PNG は MCP と同じ `png_image::encode_png` から出る（4.6）。
#[test]
fn png_comes_from_the_function_mcp_uses() {
    let shell = load("offline-render-png");
    for id in drawable() {
        let bgra = shell.render(&default_binds(), id).unwrap();
        let want = crate::png_image::encode_png(&bgra, shell.canvas.width, shell.canvas.height);
        assert_eq!(shell.png(&default_binds(), id), want, "面 {id}");
    }
}

/// runonce の面は 1 周の最後のコマ（102＝青）で出る（4.1）。
#[test]
fn runonce_shows_its_last_frame() {
    let shell = load("offline-render-runonce");
    assert_eq!(settle_time(&shell.table, fixture::RUNONCE), 300);
    let got = shell.render(&default_binds(), fixture::RUNONCE).unwrap();
    assert_eq!(shell.pixel(&got, 2, 2), BLUE, "最後のコマの 102");
    assert_eq!(shell.pixel(&got, 10, 2), GRAY, "102 の右は土台");
}

/// always の面は時点 T（runonce が無いので 0）のコマで出る。コマ 0 は待ち 50 の後なので T=0 では
/// 何も出ない（SSP 2.9.23 の測り・research.md §10「always だけ」は灰だけ・4.7）。
#[test]
fn always_shows_the_frame_at_t() {
    let shell = load("offline-render-always");
    assert_eq!(settle_time(&shell.table, fixture::ALWAYS), 0);
    let got = shell.render(&default_binds(), fixture::ALWAYS).unwrap();
    assert_eq!(
        shell.pixel(&got, 10, 2),
        GRAY,
        "T=0 では always のコマはまだ出ない"
    );
}

/// random は抽選されず、土台だけで出る（4.7・4.9）。
#[test]
fn random_never_appears() {
    let shell = load("offline-render-random");
    let got = shell.render(&default_binds(), fixture::RANDOM).unwrap();
    assert_eq!(shell.pixel(&got, 10, 2), GRAY, "random の 8,0 は土台のまま");
}

/// periodic は T まで進む・この面は runonce が無く T=0 なのでまだ出ない（research §10.1 の面 4・4.7）。
#[test]
fn periodic_advances_to_t_and_has_not_fired_at_zero() {
    let shell = load("offline-render-periodic");
    assert_eq!(settle_time(&shell.table, fixture::PERIODIC), 0);
    let got = shell.render(&default_binds(), fixture::PERIODIC).unwrap();
    assert_eq!(
        shell.pixel(&got, 10, 2),
        GRAY,
        "T=0 では periodic のコマはまだ出ない"
    );
}

/// 2 度描くと同じバイト列（4.9）。
#[test]
fn rendering_twice_gives_the_same_bytes() {
    let shell = load("offline-render-twice");
    for id in drawable() {
        assert_eq!(
            shell.png(&default_binds(), id),
            shell.png(&default_binds(), id),
            "面 {id}"
        );
    }
}

/// 合成の失敗は合成の error 1 行に info を足すだけで、`RenderError::Compose`（4.8）。
#[test]
fn compose_failure_adds_only_info() {
    let shell = load("offline-render-compose-failure");
    let (got, events) = capture(|| shell.render(&default_binds(), fixture::EMPTY));
    assert!(matches!(got, Err(RenderError::Compose)), "{got:?}");
    let errors = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .count();
    assert_eq!(errors, 1, "error は合成の 1 行だけ: {events:?}");
    assert!(
        events.iter().any(|e| e.level == tracing::Level::INFO
            && e.field_str("event") == Some("offline_dump_surface_skipped")),
        "飛ばした info: {events:?}"
    );
}

/// 下地からはみ出す絵は error 1 行で `RenderError::CanvasOverflow`。
#[test]
fn overflow_is_an_error() {
    let mut shell = load("offline-render-overflow");
    shell.canvas = Canvas {
        left: 0,
        top: 0,
        width: 4,
        height: 4,
    };
    let (got, events) = capture(|| shell.render(&default_binds(), fixture::RUNONCE));
    assert!(matches!(got, Err(RenderError::CanvasOverflow)), "{got:?}");
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "{events:?}");
    assert_eq!(
        errors[0].field_str("event"),
        Some("offline_dump_canvas_overflow")
    );
}

/// 書き出しの試験で書く面（赤の部品）。
const WRITTEN: u32 = fixture::PARTS[0];

/// 既定の名前は `surface<番号>.png` で、中身は `render_png` の PNG（6.1・6.3）。
#[test]
fn default_name_is_surface_and_number() {
    let shell = load("offline-write-default");
    let out = TempPath::under_target("offline-write-default-out");
    assert_eq!(DEFAULT_PREFIX, "surface");
    let got = shell
        .write(0, WRITTEN, out.path(), DEFAULT_PREFIX)
        .expect("書ける");
    assert_eq!(got, out.child(&format!("surface{WRITTEN}.png")));
    assert_eq!(got, png_path(out.path(), DEFAULT_PREFIX, WRITTEN));
    assert_eq!(
        std::fs::read(&got).unwrap(),
        shell.png(&default_binds(), WRITTEN)
    );
}

/// 頭の文字列は既定を置き換える（付け足さない・6.2）。
#[test]
fn prefix_replaces_the_default() {
    let shell = load("offline-write-prefix");
    let out = TempPath::under_target("offline-write-prefix-out");
    let got = shell.write(0, WRITTEN, out.path(), "x_").expect("書ける");
    assert_eq!(got, out.child(&format!("x_{WRITTEN}.png")));
    assert!(!out.child(&format!("surface{WRITTEN}.png")).exists());
    assert!(!out.child(&format!("x_surface{WRITTEN}.png")).exists());
}

/// 無い出力先は書く前に作る（深い道も・6.4）。
#[test]
fn missing_output_dir_is_created() {
    let shell = load("offline-write-mkdir");
    let out = TempPath::under_target("offline-write-mkdir-out");
    let dir = out.path().join("a").join("b");
    let got = shell
        .write(0, WRITTEN, &dir, DEFAULT_PREFIX)
        .expect("書ける");
    assert_eq!(got, dir.join(format!("surface{WRITTEN}.png")));
    assert!(got.is_file());
}

/// 同じ名前のファイルは書いた絵で置き換える（6.5）。
#[test]
fn same_name_is_replaced() {
    let shell = load("offline-write-replace");
    let out = TempPath::under_target("offline-write-replace-out");
    let path = out.child(&format!("surface{WRITTEN}.png"));
    std::fs::write(&path, b"old and longer than nothing").unwrap();
    shell
        .write(0, WRITTEN, out.path(), DEFAULT_PREFIX)
        .expect("書ける");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        shell.png(&default_binds(), WRITTEN)
    );
}

/// 頭の文字列の `\`・`/` は出力先からの相対の道で、在るフォルダへは書ける（6.8）。
#[test]
fn prefix_with_separators_writes_into_existing_subfolder() {
    let shell = load("offline-write-subdir");
    let out = TempPath::under_target("offline-write-subdir-out");
    std::fs::create_dir(out.child("sub")).unwrap();
    for prefix in [r"sub\p", "sub/q"] {
        let got = shell.write(0, WRITTEN, out.path(), prefix).expect("書ける");
        assert!(got.is_file(), "{prefix}: {}", got.display());
    }
    assert!(out.child("sub").join(format!("p{WRITTEN}.png")).is_file());
    assert!(out.child("sub").join(format!("q{WRITTEN}.png")).is_file());
}

/// 書けなかったら error 1 行（道筋と OS の誤り）で `RenderError::Write` を返す。
fn assert_write_failed(shell: &Loaded, dir: &Path, prefix: &str) {
    let (got, events) = capture(|| shell.write(0, WRITTEN, dir, prefix));
    assert!(
        matches!(got, Err(RenderError::Write(_))),
        "{prefix}: {got:?}"
    );
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "{prefix}: {events:?}");
    assert_eq!(
        errors[0].field_str("event"),
        Some("offline_dump_png_write_failed")
    );
    let names = errors[0].field_names_sorted();
    assert!(
        names.contains(&"path") && names.contains(&"error"),
        "{names:?}"
    );
}

/// 頭の文字列の下の無いフォルダは作らず、書けなかった error（6.8・6.6）。
#[test]
fn prefix_with_missing_subfolder_fails_without_creating_it() {
    let shell = load("offline-write-nosub");
    let out = TempPath::under_target("offline-write-nosub-out");
    assert_write_failed(&shell, out.path(), r"none\p");
    assert!(!out.child("none").exists(), "その下のフォルダは作らない");
}

/// 名前にファイル名として使えない文字が入ると書けなかった error（6.6）。
#[test]
fn invalid_file_name_fails() {
    let shell = load("offline-write-invalid");
    let out = TempPath::under_target("offline-write-invalid-out");
    assert_write_failed(&shell, out.path(), "a?");
}

/// スコープの番号は名前に入らない（6.7）。
#[test]
fn scope_is_not_in_the_name() {
    let shell = load("offline-write-scope");
    let out = TempPath::under_target("offline-write-scope-out");
    let got = shell
        .write(1, WRITTEN, out.path(), DEFAULT_PREFIX)
        .expect("書ける");
    assert_eq!(got, out.child(&format!("surface{WRITTEN}.png")));
}
