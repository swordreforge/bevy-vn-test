#[derive(Debug, Clone)]
pub struct ViewEntry {
    pub name_file: &'static str,
    pub base_file: &'static str,
    pub mask_file: &'static str,
    pub name_w: u16,
    pub name_x: u16,
    pub pen_type: u8,
    pub window_color: u8,
}

#[derive(Debug, Clone)]
pub struct ViewTweenEntry {
    pub waypoints: &'static [(f32, f32)],
    pub step_wait_ms: u64,
    pub step_duration_ms: u64,
    pub reveal_time_ms: u64,
}

pub const VIEW_TABLE: &[(&str, ViewEntry)] = &[
    ("OTH", ViewEntry { name_file: "view_name00", base_file: "view_base00", mask_file: "view_mask02", name_w: 460, name_x: 718, pen_type: 4, window_color: 2 }),
    ("EUS", ViewEntry { name_file: "view_name01", base_file: "view_base01", mask_file: "view_mask02", name_w: 460, name_x: 718, pen_type: 2, window_color: 3 }),
    ("ERI", ViewEntry { name_file: "view_name02", base_file: "view_base02", mask_file: "view_mask02", name_w: 460, name_x: 718, pen_type: 2, window_color: 3 }),
    ("IRE", ViewEntry { name_file: "view_name03", base_file: "view_base03", mask_file: "view_mask02", name_w: 460, name_x: 718, pen_type: 2, window_color: 3 }),
    ("LIC", ViewEntry { name_file: "view_name04", base_file: "view_base04", mask_file: "view_mask01", name_w: 580, name_x: 598, pen_type: 1, window_color: 3 }),
    ("FIO", ViewEntry { name_file: "view_name05", base_file: "view_base05", mask_file: "view_mask02", name_w: 460, name_x: 718, pen_type: 2, window_color: 3 }),
    ("EUSTIA", ViewEntry { name_file: "view_name06", base_file: "view_base01", mask_file: "view_mask03", name_w: 220, name_x: 959, pen_type: 3, window_color: 3 }),
    ("COL", ViewEntry { name_file: "view_name07", base_file: "view_base07", mask_file: "view_mask03", name_w: 220, name_x: 959, pen_type: 3, window_color: 3 }),
    ("LAV", ViewEntry { name_file: "view_name13", base_file: "view_base13", mask_file: "view_mask03", name_w: 220, name_x: 959, pen_type: 3, window_color: 3 }),
    ("LUC", ViewEntry { name_file: "view_name32", base_file: "view_base32", mask_file: "view_mask01", name_w: 580, name_x: 598, pen_type: 1, window_color: 1 }),
    ("VAR", ViewEntry { name_file: "view_name41", base_file: "view_base41", mask_file: "view_mask02", name_w: 460, name_x: 718, pen_type: 2, window_color: 1 }),
    ("ViewEnd", ViewEntry { name_file: "view_name99", base_file: "viewend_base", mask_file: "view_mask02", name_w: 460, name_x: 718, pen_type: 2, window_color: 0 }),
];

const TWEEN_1_WAYPOINTS: &[(f32, f32)] = &[
    (650.0, -100.0), (300.0, 50.0),
    (-10.0, 81.0), (0.0, 70.0), (30.0, 90.0), (100.0, 70.0),
    (150.0, 97.0), (140.0, 62.0), (162.0, 106.0), (245.0, 97.0),
    (230.0, 65.0), (268.0, 106.0), (354.0, 93.0), (421.0, 60.0),
    (405.0, 103.0), (430.0, 72.0), (465.0, 110.0), (650.0, 102.0),
];

const TWEEN_2_WAYPOINTS: &[(f32, f32)] = &[
    (650.0, -100.0), (300.0, 50.0),
    (136.0, 81.0), (180.0, 62.0), (214.0, 106.0), (218.0, 82.0),
    (389.0, 106.0), (401.0, 63.0), (485.0, 72.0), (495.0, 110.0),
    (650.0, 62.0),
];

const TWEEN_3_WAYPOINTS: &[(f32, f32)] = &[
    (650.0, -100.0), (300.0, 50.0),
    (305.0, 81.0), (360.0, 62.0), (368.0, 106.0), (401.0, 72.0),
    (380.0, 93.0), (421.0, 80.0), (461.0, 103.0), (495.0, 62.0),
    (650.0, 102.0),
];

const TWEEN_4_WAYPOINTS: &[(f32, f32)] = &[
    (650.0, -100.0), (300.0, 50.0),
    (136.0, 81.0), (180.0, 62.0), (214.0, 106.0), (218.0, 82.0),
    (389.0, 106.0), (401.0, 93.0), (485.0, 102.0), (495.0, 62.0),
    (650.0, 102.0),
];

pub const VIEW_TWEEN_TABLE: &[ViewTweenEntry] = &[
    ViewTweenEntry { waypoints: TWEEN_1_WAYPOINTS, step_wait_ms: 200, step_duration_ms: 81, reveal_time_ms: 1800 },
    ViewTweenEntry { waypoints: TWEEN_2_WAYPOINTS, step_wait_ms: 200, step_duration_ms: 111, reveal_time_ms: 1500 },
    ViewTweenEntry { waypoints: TWEEN_3_WAYPOINTS, step_wait_ms: 200, step_duration_ms: 111, reveal_time_ms: 1500 },
    ViewTweenEntry { waypoints: TWEEN_4_WAYPOINTS, step_wait_ms: 200, step_duration_ms: 111, reveal_time_ms: 1500 },
];

pub fn lookup_view_entry(char_id: &str) -> Option<&'static ViewEntry> {
    VIEW_TABLE.iter().find(|(id, _)| *id == char_id).map(|(_, entry)| entry)
}

pub fn lookup_tween_entry(pen_type: u8) -> Option<&'static ViewTweenEntry> {
    VIEW_TWEEN_TABLE.get(pen_type as usize - 1)
}

pub const VIEW_PATH_PREFIX: &str = "image/view/";
