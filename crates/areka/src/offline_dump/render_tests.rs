//! `render.rs` の試験（下地の画素・大きさと位置・2 度で同じ・ファイル名）。
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
use super::{Canvas, RenderError, render_png, render_surface};

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
    assert_eq!(got, Err(RenderError::Compose));
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
    assert_eq!(got, Err(RenderError::CanvasOverflow));
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
