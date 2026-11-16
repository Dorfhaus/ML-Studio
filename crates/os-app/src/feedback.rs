//! User-initiated link to the ML Studio Discord community.
pub const URL: &str = "https://discord.gg/CDYs4rQEk";
pub fn address() -> String { URL.into() }
pub fn open()->Result<(),String>{crate::donate::open_url(&address())}
#[cfg(test)]mod tests{
    #[test]fn feedback_opens_discord_and_button_is_visible(){
        let address=super::address();assert!(address.starts_with(super::URL));assert!(!address.contains('?'));assert_eq!(address, "https://discord.gg/CDYs4rQEk");
        let mut app=crate::app::App::new(None);
        for width in [1024,1280,1920]{let mut canvas=os_ui::Canvas::new(width,900,1.).unwrap();app.paint(&mut canvas,None);let button=app.hits.iter().find(|h|h.action==crate::app::Action::ReportBug).expect("Visible report button");assert!(button.rect.x>=0.&&button.rect.right()<width as f32);for h in &app.hits{if matches!(h.action,crate::app::Action::Updates|crate::app::Action::Donate){assert!(button.rect.intersect(&h.rect).is_empty(),"toolbar panels must not overlap across either row");}}}
    }
}
