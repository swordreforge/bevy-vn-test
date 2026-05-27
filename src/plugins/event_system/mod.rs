pub mod view;
pub mod view_data;
pub mod view_material;

use bevy::prelude::*;
use bevy::ui_render::UiMaterialPlugin;
pub use view::{ViewPhase, ViewState};
pub use view_material::ViewMaskMaterial;

pub struct EventSystemPlugin;

impl Plugin for EventSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiMaterialPlugin::<ViewMaskMaterial>::default())
            .add_plugins(view::ViewPlugin);
    }
}
