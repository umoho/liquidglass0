use glam::Vec2;

/// 弹簧变形状态。
///
/// 参见 [`LIQUID_GLASS.md`] §5.3。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeformationState {
    /// 闲置，无交互。
    Idle,

    /// 按下，表面下陷。
    Pressed,

    /// 拖拽中，高光跟随。
    Dragging,

    /// 释放回弹。
    Releasing,
}

/// 交互状态。
///
/// 记录鼠标位置、按压状态和弹簧物理量。
/// 弹簧计算逻辑在 Phase 4 实现。
#[derive(Debug, Clone)]
pub struct InteractionState {
    /// 鼠标位置，归一化到 [0, 1]。
    pub cursor_pos: Vec2,

    /// 鼠标是否按下。
    pub is_pressed: bool,

    /// 当前弹簧状态。
    pub deformation_state: DeformationState,

    /// 目标变形量。
    pub displacement: f32,

    /// 变形速度。
    pub velocity: f32,

    /// 自按下起经过的秒数。
    pub time: f32,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            cursor_pos: Vec2::ZERO,
            is_pressed: false,
            deformation_state: DeformationState::Idle,
            displacement: 0.0,
            velocity: 0.0,
            time: 0.0,
        }
    }
}
