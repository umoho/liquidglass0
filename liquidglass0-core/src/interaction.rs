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
/// 使用 `press()` / `release()` / `update()` 驱动弹簧模拟。
#[derive(Debug, Clone)]
pub struct InteractionState {
    /// 鼠标位置，归一化到 [0, 1]。
    pub cursor_pos: Vec2,

    /// 鼠标是否按下。
    pub is_pressed: bool,

    /// 当前弹簧状态。
    pub deformation_state: DeformationState,

    /// 弹簧变形量（-1 ~ 1），负值表示表面下陷。
    pub displacement: f32,

    /// 变形速度。
    pub velocity: f32,

    /// 自按下起经过的秒数。
    pub time: f32,

    /// 滚动抬起偏移量（像素），随时间衰减。
    pub lift_offset: f32,
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
            lift_offset: 0.0,
        }
    }
}

impl InteractionState {
    /// 按下：目标位移 -0.8，状态切为 Pressed。
    pub fn press(&mut self) {
        self.is_pressed = true;
        self.deformation_state = DeformationState::Pressed;
        self.displacement = -0.8;
        self.velocity = 0.0;
        self.time = 0.0;
    }

    /// 释放：弹簧目标回到 0，状态切为 Releasing。
    pub fn release(&mut self) {
        self.is_pressed = false;
        self.deformation_state = DeformationState::Releasing;
    }

    /// 对滚动事件施加抬起冲量。
    ///
    /// `delta` 为滚轮行数绝对值。
    pub fn apply_scroll(&mut self, delta: f32) {
        self.lift_offset = (self.lift_offset + delta * 0.5).min(10.0);
    }

    /// 弹簧物理更新。
    ///
    /// 使用阻尼谐振子模型：`F = k * (target - x) - b * v`，质量 m = 1。
    /// 按下时 target = -0.8，释放后 target = 0。
    /// 振荡收敛后自动切回 Idle。
    ///
    /// # 参数
    ///
    /// * `dt` - 帧间隔（秒）
    /// * `spring_k` - 弹簧刚度（越大回弹越快）
    /// * `damping_b` - 阻尼系数（越大振荡衰减越快）
    pub fn update(&mut self, dt: f32, spring_k: f32, damping_b: f32) {
        let target = if self.is_pressed { -0.8 } else { 0.0 };
        let force = spring_k * (target - self.displacement) - damping_b * self.velocity;
        self.velocity += force * dt;
        self.displacement += self.velocity * dt;
        self.displacement = self.displacement.clamp(-1.0, 1.0);
        self.time += dt;

        // 抬起衰减（指数衰减）
        self.lift_offset *= (1.0 - 3.0 * dt).max(0.0);
        if self.lift_offset < 0.05 {
            self.lift_offset = 0.0;
        }

        // 释放后判定是否稳定
        if !self.is_pressed
            && self.deformation_state == DeformationState::Releasing
            && self.displacement.abs() < 0.005
            && self.velocity.abs() < 1.0
        {
            self.displacement = 0.0;
            self.velocity = 0.0;
            self.deformation_state = DeformationState::Idle;
        }
    }
}
