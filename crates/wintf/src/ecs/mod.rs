mod app;
pub mod common;
pub mod clickthrough;
pub mod dola;
pub mod drag;
mod graphics;
pub mod layout;
pub mod pointer;
#[cfg(test)]
pub(crate) mod test_support;
pub mod types;
pub mod widget;
pub mod window;
mod window_proc;
pub mod world;

pub use types::{Point, PointF, Rect, SizeI};
pub use dola::{DolaAnimator, tick_dola_animators};

pub use app::*;
pub use bevy_ecs::hierarchy::{ChildOf, Children};
pub use common::tree_system::*;
pub use drag::{
    DragConfig, DragConstraint, DragEndEvent, DragEvent, DragStartEvent, DraggingState, OnDrag,
    OnDragEnd, OnDragStart, WindowDragContext, WindowDragContextResource, WindowDragging,
    cleanup_drag_state, dispatch_drag_events,
};
pub use graphics::FrameTime;
pub use graphics::calculate_surface_size_from_global_arrangement;
pub use graphics::*;
pub use layout::*;
pub use pointer::{
    CursorVelocity, DoubleClick, EventHandler, OnPointerEntered, OnPointerExited, OnPointerMoved,
    OnPointerPressed, OnPointerReleased, Phase, PhysicalPoint, PointerButton, PointerEventHandler,
    PointerLeave, PointerState, WheelDelta, WindowPointerTracking, clear_transient_pointer_state,
    debug_pointer_leave, debug_pointer_state_changes, dispatch_pointer_events,
};
pub use window::monitor::*;
pub use widget::{
    BitmapSource, BitmapSourceGraphics, BitmapSourceResource, BoxedCommand, CommandSender, WicCore,
    WintfTaskPool, draw_bitmap_sources,
};
pub use widget::{
    Typewriter, TypewriterEvent, TypewriterEventKind, TypewriterState, TypewriterTalk,
    TypewriterTimeline, TypewriterToken, draw_typewriters, update_typewriters,
};
pub use window::{
    DPI, DpiChangeContext, DpiSuggestedRectPolicy, SetWindowPosCommand, Window, WindowHandle,
    WindowPos, WindowStyle, ZOrder, find_owner_window, flush_window_pos_commands,
    guarded_set_window_pos, is_self_initiated,
};
pub(crate) use window_proc::dispatch_window_message;
pub use world::{
    FrameCount, FrameFinalize, Input, Layout, PostLayout, PreLayout, PreRenderSurface, UISetup,
    Update,
};
