pub mod bitmap_source;
pub mod brushes;
// 骨組みに載る描く係は 2.2 以降に来る（載ったらこの expect が外れを知らせる）
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "描く係がまだ骨組みに載っていない")
)]
pub(crate) mod record;
pub mod shapes;
pub mod text;

pub use bitmap_source::{
    BitmapSource, BitmapSourceGraphics, BitmapSourceResource, BoxedCommand, CommandSender, WicCore,
    WintfTaskPool, draw_bitmap_sources,
};

pub use brushes::{Brush, BrushInherit, Brushes, DEFAULT_BACKGROUND, DEFAULT_FOREGROUND};

pub use text::{
    Typewriter, TypewriterEvent, TypewriterEventKind, TypewriterState, TypewriterTalk,
    TypewriterTimeline, TypewriterToken, draw_typewriters, update_typewriters,
};
