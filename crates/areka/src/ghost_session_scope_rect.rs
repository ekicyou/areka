//! 窓の矩形の読み手（spec mcp-scope-rect-property 要件 1〜5）。
//!
//! `currentghost.scope(ID).rect`・`currentghost.balloon.scope(ID).rect` は、読まれた時に
//! スコープ → 窓の印の表で窓を引き、OS に 1 回だけ今の矩形を聞いて「左,上,右,下」で答える。
//! 値は写しにも保存にも載せない（プル）。画面に出ているかは見ない（要件 1.8・2.4）。
// 呼び手（結線と系の登録）は 3.1 で付く。それまで本番のビルドでは使われない。
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use areka_sylphya::{PropPath, PullSource, Selector};

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
pub(crate) struct ScopeWindowTable(pub(crate) BTreeMap<usize, ScopeWindowKeys>);

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
enum ScopeSelector {
    /// 番号のスコープ。
    Index(usize),
    /// 負の番号＝窓を持たないスコープ。
    Negative,
}

/// 区切り「scope」の選択子を読む。`None` は書式の誤り（`x`・空・`+1` など・要件 3.6）。
fn scope_selector(path: &PropPath) -> Option<ScopeSelector> {
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

#[cfg(test)]
#[path = "ghost_session_scope_rect_tests.rs"]
mod tests;
