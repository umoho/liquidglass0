---
生成时间:   2026-06-30 12:55:26 +0800
上一份日志: 2026-06-29_shadow-debug-and-config-exposure.md
关联的提交: 827bcba
---

# 2026-06-30 — Phase 4 交互调试

> **完整性说明**：本日志于会话 compaction 后撰写，部分细节可能遗失，仅能根据 git 记录和残留的 uncommitted diff 重建。

## 工作内容

1. **验证 Phase 4 提交（827bcba）** — 构建并运行，测试光标悬停高光、弹簧按压变形、拖拽回弹、滚动抬起
2. **CPU 端确认正常** — 添加 debug eprintln! 后确认：
   - 点击 → `disp=-0.800 pressed=true`
   - 释放 → 阻尼振荡回归 0
   - 光标坐标持续更新
   - 滚动抬起 → 阴影变化肉眼可见（scroll lift 通道 OK）
3. **Shader 端调试** — 发现按压变形在屏幕上**完全不可见**。尝试以下 shader 改动均无效果：
   - `debug_tint = vec3f(1.0, 0.0, 0.0)` 乘到最后输出上
   - 背景区域直接 `return vec4f(1.0, 0.0, 0.0, 1.0)`
   - `mix(color, red, -displacement * 0.5)` 按压暗化
   - 二进制（strings / rg 确认）包含修改后的 shader 源码
   - `cargo build --release` 确认 `liquidglass0-render` 被重新编译

## 关键发现

- CPU → uniform 通路正常（scroll lift 可工作证明 render loop 和 buffer write 没问题）
- 二进制嵌入的 shader 源码是最新版
- 但 shader 修改在屏幕上**零可见变化**，暗示 naga_oil `Composer::make_naga_module` 或 wgpu pipeline 缓存导致旧 IR/旧 pipeline 被重用

## 当前状态

- **Blocked**：按压变形的视觉效果不可见，原因未知
- 已排除：CPU 侧逻辑、uniform 上传、render loop 整体
- 锁定范围：naga_oil 编译缓存 / wgpu pipeline 缓存

## 待调查

- [ ] naga_oil `Composer` 内部是否有 `make_naga_module` 的哈希缓存
- [ ] 换用 `EmbeddedLoader` 绕过 naga_oil 直接编译 WGSL 确认效果
- [ ] 确认 pipeline 创建序列中是否使用了缓存的 shader module

---

*下一份日志应引用本文件：`2026-06-30_phase4-interaction-debug.md`*
