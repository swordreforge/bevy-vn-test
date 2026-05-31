CG 处理问题
1. 实体内存泄漏 — despawn() 未递归
cleanup_gallery (gallery.rs:527)、choice_ui_cleanup (choice.rs:106)、cleanup_dialogue (dialogue.rs:121)、cleanup_save_load_ui (save_load.rs:239) 都用 .despawn() 而非 .despawn_recursive()。Bevy 的 despawn() 不销毁子实体，导致每次状态切换都泄漏子节点。
画廊翻页 (gallery.rs:390-394) 更严重——只逐个 despawn grid 的直接子节点，但 locked 缩略图有 with_child(Text) 孙节点，这些孙节点不会被清理。
2. 子目录 CG 不在画廊中
build.rs:57-76 的 all_cg_files() 只扫描 assets/image/ev/ 顶层文件，子目录图片通过 scan_ev_subdir 只进入 ext_map 而不进入 top_ev_files。画廊里永远看不到子目录中的 CG。
3. CG 全屏查看路径不处理子目录
handle_thumbnail_click (gallery.rs:282) 用 format!("image/ev/{}", file) 拼路径，但没有子目录前缀。即使 all_cg_files() 修好包含了子目录文件，这里的路径拼接仍会是错的。
4. CG fade-out 完成后实体残留
update_cg_fade (rendering.rs:683-686) 在 FadeOut 完成后只设 Visibility::Hidden，不 despawn 实体，也不重置 cg_state.active 和 cg_state.texture。实体永远留在世界中也占用 TextureCache。
对话选择问题
5. 点击任意位置可跳过选项（严重）
process_advance (script_runner.rs:188-205) 的选择处理逻辑：
if choice_state.active {
    for ev in choice_ev.read() {
        // 处理选择...
    }
    choice_state.active = false;  // ← 无条件执行
    choice_state.options.clear(); // ← 无条件执行
    continue;
}
choice_state.active = false 和 options.clear() 在 for ev 循环外面，无论是否读到 ChoiceSelectedMessage 都会执行。而 handle_global_input (inputs.rs:51) 在任何左键点击时都发 AdvanceEvent。结果：点击选项按钮以外的任意位置都会导致选择被跳过，脚本继续走未分支的下一行。
6. 选项处理与 AdvanceEvent 紧耦合
选择处理的入口嵌在 for _ in advance_ev.read() 循环内。理论上如果某帧没有 AdvanceEvent，即使 ChoiceSelectedMessage 已经写入也不会被处理。目前靠"点按钮=点屏幕=同时产生 AdvanceEvent"这个巧合运行，设计脆弱。
7. 选项 UI 系统 choice_ui_spawn 每帧运行
choice_ui_spawn 的 run_if 只检查 state.active，不检查 Changed<ChoiceState>，每帧都查询 Query<Entity, With<ChoiceUiRoot>>。虽然有 guard if !existing.is_empty() { return; } 防止重复生成，但浪费查询。
