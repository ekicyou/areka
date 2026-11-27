//! 今の側: 借りた `areka-emo-text`（`windows` 0.62）の公開の API だけで縦書きの 1 列を描き、
//! 供給面の premultiplied BGRA を読み戻す（問い 7）。
//!
//! 絵の大きさ・原点・バルーン定義は本線の縦書きの試験
//! `crates/areka-emo-text/tests/decoration_readback/vertical.rs` の `shoot_column`
//! （と `decoration_readback/mod.rs` の `model_with`・`shoot`）と同じ値を使う。
//! 供給面は validrect の内側だけを持つので、列の先頭の字の枠の上端が面の y=0。

use areka_emo_text::actor::{
    ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame,
};
use areka_emo_text::state::TextLayerConfig;
use areka_parsers::balloon::parse_str;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue, TimePoint, TimeSpan};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};

use crate::diff::Shot;

/// 比べの文（開き括弧・小書きかな・句読点・カラー絵文字）。
pub const TEXT: &str = "「ちょっと、待って。🍣」";
/// ぱすたのバルーン定義の書体の候補列。
pub const PASTA_FONT: &str =
    "游ゴシック Medium,Yu Gothic Medium,游ゴシック,Yu Gothic,BIZ UDPゴシック,メイリオ";
/// 文字の大きさ（image px）＝字の枠の高さ。
pub const FONT_HEIGHT: u32 = 20;
/// バルーン画像の原寸（image px・k=1）。幅は本線の試験と同じ 320。高さは本線の 200 だと
/// 面が 190 px で 12 字（240 px）が 2 列に折り返すので、1 列に収まる 260 にする（面は 250 px）。
pub const IMAGE: (u32, u32) = (320, 260);
/// validrect の内側への寄せ（4 辺とも 5 px）。面は (IMAGE.0-10, IMAGE.1-10)。
pub const VALIDRECT_INSET: u32 = 5;
/// 全グリフが出そろう十分に後の時刻（本線の試験の `SETTLED` と同じ）。
const SETTLED: TimePoint = TimePoint::from_ticks(100 * 10_000_000);

/// 本線の試験の `model_with` と同じ行で、縦書き（右から左）のバルーン定義の文字列を組む。
fn balloon_descript() -> String {
    format!(
        "charset,UTF-8\n\
         font.name,{PASTA_FONT}\n\
         font.height,{FONT_HEIGHT}\n\
         font.color.r,0\n\
         font.color.g,0\n\
         font.color.b,0\n\
         validrect.top,{VALIDRECT_INSET}\n\
         validrect.left,{VALIDRECT_INSET}\n\
         validrect.right,-{VALIDRECT_INSET}\n\
         validrect.bottom,-{VALIDRECT_INSET}\n\
         writing_mode,vertical_rl\n"
    )
}

/// `text` を今の側で 1 フレーム描き、供給面を読み戻す。
///
/// 呼ぶスレッドで COM を MTA で始める（本線の試験の `init_com_for_this_thread` と同じ）。
/// 画面の外の GPU（ハードウェア）だけを使い、窓は出さない。
pub fn draw(text: &str) -> Result<Shot, String> {
    // COM の MTA。借りた側は 0.62 の `windows` を直に持たないので、OS の同じ関数を bind-next の束縛で呼ぶ。
    // 既に初期化済み（S_FALSE）などの戻り値は本線の試験と同じく見ない。
    // SAFETY: 予約の引数は None・COINIT_MULTITHREADED（0）。
    let _ = unsafe { bind_next::Windows::Win32::CoInitializeEx(None, 0) };

    let core = GraphicsCore::new().map_err(|e| format!("GraphicsCore::new: {e:?}"))?;
    let d2d = core
        .d2d_device()
        .ok_or("GraphicsCore::d2d_device が None")?;
    let wuc =
        WucGraphicsResource::new(d2d).map_err(|e| format!("WucGraphicsResource::new: {e:?}"))?;
    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);

    // emo-present の予約スロットの最小の形（本線の試験の `spawn_reserved_slot` と同じ）。
    let window = world.spawn_empty().id();
    let slot = world
        .spawn((
            Name::new("emo-text-layer-slot"),
            Visual::default(),
            ChildOf(window),
        ))
        .id();
    world.flush();

    let model = parse_str(&balloon_descript(), None);
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE),
        ResolvedBalloonText::resolve(&model, IMAGE),
    );
    rt.apply_cue(&TalkCue::scheduled(
        TimePoint::ZERO,
        TimeSpan::ZERO,
        actor.clone(),
        CueCommand::Text(text.to_owned()),
        TimeSpan::ZERO,
    ));
    present_frame(&mut rt, &mut world, SETTLED).map_err(|e| format!("present_frame: {e:?}"))?;

    let surface = rt.surface(&actor).ok_or("装着した actor の供給面が無い")?;
    let (width, height) = surface.size();
    let bgra = surface
        .read_back()
        .map_err(|e| format!("read_back: {e:?}"))?;
    Ok(Shot {
        width,
        height,
        bgra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 墨（α≠0）のある列（x）と行（y）。
    fn ink_lanes(s: &Shot) -> (Vec<u32>, Vec<u32>) {
        let ink = |x: u32, y: u32| s.bgra[((y * s.width + x) * 4 + 3) as usize] != 0;
        let xs = (0..s.width)
            .filter(|&x| (0..s.height).any(|y| ink(x, y)))
            .collect();
        let ys = (0..s.height)
            .filter(|&y| (0..s.width).any(|x| ink(x, y)))
            .collect();
        (xs, ys)
    }

    /// 黒い字の墨で色の付いた画素（B・G・R が揃わない）の数。
    fn tinted(s: &Shot) -> usize {
        s.bgra
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] != 0 && (p[0] != p[1] || p[1] != p[2]))
            .count()
    }

    /// 実 GPU で比べの文を描き、面が validrect の内側の大きさで、墨が 1 列に収まり、
    /// 先頭の字が面の上端から始まることを確かめる。絵文字を除いた列に色の付いた画素が
    /// 無い（＝文字はグレースケールの塗り・ClearType ではない）ことも測る。
    #[test]
    fn old_side_draws_the_vertical_column() {
        let shot = draw(TEXT).expect("今の側で描く");
        assert_eq!(
            (shot.width, shot.height),
            (IMAGE.0 - 2 * VALIDRECT_INSET, IMAGE.1 - 2 * VALIDRECT_INSET)
        );
        assert_eq!(shot.bgra.len(), (shot.width * shot.height * 4) as usize);
        let (xs, ys) = ink_lanes(&shot);
        assert!(!xs.is_empty(), "墨が無い");
        let span = xs[xs.len() - 1] - xs[0] + 1;
        assert!(
            span <= FONT_HEIGHT * 3 / 2,
            "墨が 1 列に収まらない（折り返した）: x {xs:?}"
        );
        assert!(
            ys[0] < FONT_HEIGHT,
            "先頭の字が面の上端から始まらない: y={}",
            ys[0]
        );
        let ink = shot
            .bgra
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] != 0)
            .count();
        let plain = draw("「ちょっと、待って。」").expect("絵文字なしで描く");
        eprintln!(
            "[q7] old ink={ink} x={}..{} y={}..{} tinted={} plain_tinted={}",
            xs[0],
            xs[xs.len() - 1],
            ys[0],
            ys[ys.len() - 1],
            tinted(&shot),
            tinted(&plain)
        );
        assert!(tinted(&shot) > 0, "絵文字が色つきで出ていない");
        assert_eq!(tinted(&plain), 0, "黒い字だけの列に色の付いた画素がある");
    }
}
