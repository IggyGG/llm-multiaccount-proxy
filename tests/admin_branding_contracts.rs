const LOGIN_PAGE: &str = include_str!("../src/admin_login.html");
const DASHBOARD_PAGE: &str = include_str!("../src/admin_dashboard.html");

#[test]
fn branded_admin_pages_embed_a_favicon_without_an_extra_request() {
    for page in [LOGIN_PAGE, DASHBOARD_PAGE] {
        assert!(page.contains("rel=\"icon\""));
        assert!(page.contains("href=\"data:image/svg+xml,"));
    }
}
