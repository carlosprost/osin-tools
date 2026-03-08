#[allow(dead_code)]
pub fn scrape_url_stealth(_url: &str) -> Result<String, String> {
    Err("Scraper deshabilitado temporalmente. Se requiere headless_chrome y configuración de Chromium para activar esta función.".into())
}
