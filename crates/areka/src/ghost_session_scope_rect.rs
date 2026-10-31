//! 窓の矩形の読み手（spec mcp-scope-rect-property 要件 1〜5）。
//!
//! `currentghost.scope(ID).rect`・`currentghost.balloon.scope(ID).rect` は、読まれた時に
//! スコープ → 窓の印の表で窓を引き、OS に 1 回だけ今の矩形を聞いて「左,上,右,下」で答える。
//! 値は写しにも保存にも載せない（プル）。画面に出ているかは見ない（要件 1.8・2.4）。

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use areka_ghost::GhostRuntime;
use areka_sylphya::{PropPath, PullShape, PullSource, Selector, SylphyaReader};
use bevy_ecs::prelude::*;
use wintf::ecs::{FrameFinalize, WindowHandle};

use crate::placement::spawn::GhostWindows;

/// 窓の印（HWND の生の値）。HWND はスレッドをまたいで送れないので値で持つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowKey(pub(crate) isize);

/// 画面の矩形（物理の画素・仮想画面の座標・右と下は外の最初の画素）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// 窓の印から今の矩形を 1 回で読む関数。`None` は窓が無い（壊れた・読めない）。
pub(crate) type WindowRectFn = Arc<dyn Fn(WindowKey) -> Option<ScreenRect> + Send + Sync>;

/// 読み手が答える窓の種類。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RectTarget {
    Character,
    Balloon,
}

/// スコープ 1 つの窓の印（印が無い＝窓ができる前）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScopeWindowKeys {
    pub character: Option<WindowKey>,
    pub balloon: Option<WindowKey>,
}

/// スコープ → 窓の印の表（可視の欄は持たない）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScopeWindowTable(BTreeMap<usize, ScopeWindowKeys>);

impl ScopeWindowTable {
    /// `GhostWindows` の各スコープについて、窓の entity から窓の印を引いて組む（純粋）。
    pub(crate) fn build(
        windows: &GhostWindows,
        key_of: impl Fn(Entity) -> Option<WindowKey>,
    ) -> Self {
        Self(
            windows
                .scopes()
                .map(|scope| {
                    let keys = ScopeWindowKeys {
                        character: windows.char_window(scope).and_then(&key_of),
                        balloon: windows.balloon_window(scope).and_then(&key_of),
                    };
                    (scope, keys)
                })
                .collect(),
        )
    }
}

/// 窓の印を値で写す。
fn window_key(handle: &WindowHandle) -> WindowKey {
    WindowKey(handle.hwnd.0 as isize)
}

/// 今の実行系の読み手と共有する置き場（`wire` が実行系ごとに作り直す）。
#[derive(Resource)]
pub(crate) struct ScopeRectWiring {
    table: Arc<RwLock<ScopeWindowTable>>,
}

/// テスト用の OS の読み取りの差し替え口（本番は入れない＝本物の `os_window_rect`）。
#[derive(Resource, Clone)]
pub(crate) struct ScopeRectReadSeam(pub(crate) WindowRectFn);

/// 実行系に読み手を登録し、今の窓の一式から表を組む（`rewire_runtime_consumers` から・同期）。
pub(crate) fn wire(world: &mut World, runtime: &GhostRuntime) {
    wire_reader(world, runtime.sylphya_reader());
}

/// `wire` の中身（テストは sylphya の読み口を直に渡せる）。
pub(crate) fn wire_reader(world: &mut World, reader: &SylphyaReader) {
    let table = match world.get_resource::<GhostWindows>() {
        Some(windows) => {
            ScopeWindowTable::build(windows, |e| world.get::<WindowHandle>(e).map(window_key))
        }
        None => ScopeWindowTable::default(),
    };
    let table = Arc::new(RwLock::new(table));
    let read_rect: WindowRectFn = match world.get_resource::<ScopeRectReadSeam>() {
        Some(seam) => seam.0.clone(),
        None => Arc::new(os_window_rect),
    };
    for (target, pattern) in [
        (RectTarget::Character, "currentghost.scope(*).rect"),
        (RectTarget::Balloon, "currentghost.balloon.scope(*).rect"),
    ] {
        let shape = PullShape::parse(pattern).expect("固定の形は読める");
        let source = ScopeRectSource::new(target, table.clone(), read_rect.clone());
        reader.register_pull(shape, Arc::new(source));
    }
    world.insert_resource(ScopeRectWiring { table });
}

/// 表を出す系（`FrameFinalize`）。窓の一式の入れ直し・窓の印の付け外しの時だけ組む（プッシュ）。
pub(crate) fn sync_scope_window_table(
    windows: Option<Res<GhostWindows>>,
    handles: Query<&WindowHandle>,
    added: Query<(), Added<WindowHandle>>,
    mut removed: RemovedComponents<WindowHandle>,
    wiring: Option<Res<ScopeRectWiring>>,
) {
    // 外れた印は毎回読み捨てる（置き場が無い間の分を後へ持ち越さない）。
    let any_removed = removed.read().count() > 0;
    let changed = windows.as_ref().is_some_and(|w| w.is_changed());
    if !(changed || any_removed || !added.is_empty()) {
        return;
    }
    let Some(wiring) = wiring else {
        return;
    };
    let table = match windows.as_deref() {
        Some(windows) => ScopeWindowTable::build(windows, |e| handles.get(e).ok().map(window_key)),
        None => ScopeWindowTable::default(),
    };
    let mut current = wiring.table.write().unwrap_or_else(|e| e.into_inner());
    if *current != table {
        tracing::debug!(
            event = "scope_rect_table_updated",
            scopes = table.0.len(),
            "スコープ → 窓の印の表を差し替えた"
        );
        *current = table;
    }
}

/// 系をプロセスに 1 回だけ載せる（`ghost_session::register_systems` から）。
pub(crate) fn register(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(FrameFinalize, sync_scope_window_table);
}

/// 窓の矩形の読み手（キャラクター用とバルーン用で 1 つずつ登録する）。
pub(crate) struct ScopeRectSource {
    target: RectTarget,
    table: Arc<RwLock<ScopeWindowTable>>,
    read_rect: WindowRectFn,
}

impl ScopeRectSource {
    pub(crate) fn new(
        target: RectTarget,
        table: Arc<RwLock<ScopeWindowTable>>,
        read_rect: WindowRectFn,
    ) -> Self {
        Self {
            target,
            table,
            read_rect,
        }
    }

    /// 窓を持たないスコープの答え（キャラクターは `0,0,0,0`・バルーンは値なし）。
    fn no_window(&self) -> Option<String> {
        match self.target {
            RectTarget::Character => Some("0,0,0,0".to_string()),
            RectTarget::Balloon => None,
        }
    }
}

/// 名前の選択子の読み。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeSelector {
    /// 番号のスコープ。
    Index(usize),
    /// 負の番号＝窓を持たないスコープ。
    Negative,
}

/// 区切り「scope」の選択子を読む。`None` は書式の誤り（`x`・空・`+1` など・要件 3.6）。
pub(crate) fn scope_selector(path: &PropPath) -> Option<ScopeSelector> {
    let seg = path.segs.iter().find(|seg| seg.name == "scope")?;
    match seg.selector.as_ref()? {
        Selector::ByIndex(n) => Some(ScopeSelector::Index(*n as usize)),
        Selector::ByName(s) => {
            let digits = s.strip_prefix('-')?;
            (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
                .then_some(ScopeSelector::Negative)
        }
    }
}

impl PullSource for ScopeRectSource {
    fn read(&self, path: &PropPath) -> Option<String> {
        let ScopeSelector::Index(scope) = scope_selector(path)? else {
            return self.no_window();
        };
        // 表の鍵は窓の印を写す間だけ持つ（OS は鍵の外で呼ぶ）。poison でも中身で続ける。
        let key = {
            let table = self.table.read().unwrap_or_else(|e| e.into_inner());
            table.0.get(&scope).and_then(|keys| match self.target {
                RectTarget::Character => keys.character,
                RectTarget::Balloon => keys.balloon,
            })
        };
        let Some(key) = key else {
            return self.no_window();
        };
        match (self.read_rect)(key) {
            Some(rect) => Some(rect_text(rect)),
            None => {
                tracing::debug!(
                    event = "scope_rect_window_gone",
                    scope,
                    target = ?self.target,
                    "窓の矩形を OS から読めなかった"
                );
                self.no_window()
            }
        }
    }
}

/// 4 つの数を `左,上,右,下` にする（`i32` の 10 進・空白なし）。
pub(crate) fn rect_text(rect: ScreenRect) -> String {
    format!("{},{},{},{}", rect.left, rect.top, rect.right, rect.bottom)
}

/// 窓の印から OS の今の窓の矩形を 1 回で読む（本物・読み手の既定の読み取り）。
///
/// `GetWindowRect` は呼んだスレッドの DPI の扱いで値が変わるので、呼ぶ間だけ
/// モニタごとの v2 にして物理の画素で読む（要件 1.3）。窓のスレッドは待たない（要件 5.1）。
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#currentghost.scope_28ID_29.rect:1
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#currentghost.balloon.scope_28ID_29.rect:1
pub(crate) fn os_window_rect(key: WindowKey) -> Option<ScreenRect> {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut r = RECT::default();
    let read = unsafe {
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let read = GetWindowRect(HWND(key.0 as *mut _), &mut r);
        if previous.is_invalid() {
            tracing::debug!(
                event = "scope_rect_dpi_context_unchanged",
                "DPI の扱いを変えられなかった（今の扱いのまま読んだ）"
            );
        } else {
            SetThreadDpiAwarenessContext(previous);
        }
        read
    };
    read.ok().map(|()| ScreenRect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    })
}

#[cfg(test)]
#[path = "ghost_session_scope_rect_tests.rs"]
mod tests;
