use bevy_ecs::component::Component;
use windows::Win32::Graphics::Direct2D::ID2D1CommandList;

/// Direct2D描画命令リスト
#[derive(Component, Debug, Clone, PartialEq)]
pub struct GraphicsCommandList {
    command_list: Option<ID2D1CommandList>,
}

impl GraphicsCommandList {
    /// 新しいCommandListコンポーネントを作成
    pub fn new(command_list: ID2D1CommandList) -> Self {
        Self {
            command_list: Some(command_list),
        }
    }

    /// 空のCommandListコンポーネントを作成
    pub fn empty() -> Self {
        Self { command_list: None }
    }

    /// CommandListへの参照を取得
    pub fn command_list(&self) -> Option<&ID2D1CommandList> {
        self.command_list.as_ref()
    }
}

// SAFETY 条件: windows-rs の COM スマートポインタは自動では Send/Sync にならない
// （だからこそこの unsafe impl が必要）。健全性は対象 API の同期特性に依拠する:
// ID2D1CommandList は D2D1_FACTORY_TYPE_MULTI_THREADED ファクトリ系列
// （GraphicsCore::new → create_d2d_factory）のオブジェクトであり、同一ファクトリ
// 系列への COM 呼び出しは D2D が内部で直列化するため、跨スレッド移動・参照共有とも安全。
unsafe impl Send for GraphicsCommandList {}
unsafe impl Sync for GraphicsCommandList {}
