// SPDX-License-Identifier: GPL-3.0-or-later
use crate::{Color, Palette};

/// Colors with skin-specific defaults. Only an authored override replaces native artwork colors.
macro_rules! roles {
    ($($role:ident => $field:ident),* $(,)?) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[repr(u8)]
        pub enum Chrome {
            $($role),*
        }

        impl Chrome {
            pub(crate) fn for_key(key: &str) -> Option<Self> {
                Some(match key {
                    $(stringify!($field) => Self::$role,)*
                    _ => return None,
                })
            }

            pub(crate) const fn bit(self) -> u64 { 1 << self as u8 }

            pub fn get(self, palette: &Palette) -> Option<Color> {
                if palette.appearance.chrome_overrides & self.bit() == 0 { return None; }
                Some(match self {
                    $(Self::$role => palette.$field),*
                })
            }

            pub fn or(self, palette: &Palette, inherited: Color) -> Color {
                self.get(palette).unwrap_or(inherited)
            }
        }
    };
}

roles! {
    Text => text,
    ActiveButton => active_button_face,
    ActiveInk => active_button_text,
    MixerName => mixer_channel_name,
    MixerNumber => mixer_channel_number,
    MixerSelectedNumber => mixer_channel_number_selected,
    MixerHeader => mixer_channel_header,
    MixerSelectedHeader => mixer_channel_header_selected,
    MixerAction => mixer_action_face,
    MixerActionHover => mixer_action_hover,
    MixerActionPressed => mixer_action_pressed,
    MixerActionEdge => mixer_action_border,
    MixerActionInk => mixer_action_text,
    SettingsHeading => settings_group_title,
    SettingsNote => settings_group_note,
    SettingsGroup => settings_group_face,
    SettingsGroupEdge => settings_group_border,
    SettingsTabText => settings_tab_text,
    SettingsTabSelected => settings_tab_selected_text,
    SettingsTabFace => settings_tab_face,
    SettingsTabSelectedFace => settings_tab_selected_face,
    PlaylistBar => playlist_bar_face,
    PlaylistBarInk => playlist_bar_text,
    PlaylistBarDivider => playlist_bar_divider,
    PlaylistButton => playlist_bar_button,
    PlaylistButtonHover => playlist_bar_hover,
    PlaylistActive => playlist_bar_selected,
    PlaylistActiveInk => playlist_bar_selected_text,
    PlaylistRail => playlist_overview_track,
    PlaylistThumb => playlist_overview_thumb,
    PlaylistThumbEdge => playlist_overview_thumb_edge,
}
